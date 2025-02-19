use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError};
use solana_program::pubkey::Pubkey;
use light_hasher::{Hasher, Poseidon};
use super::discriminators::*;
use crate::instruction_data::{
    data::OutputCompressedAccountWithPackedContext,
    insert_into_queues::InsertIntoQueuesInstructionData,
    zero_copy::{
        ZInstructionDataInvoke, ZInstructionDataInvokeCpi, ZInstructionDataInvokeCpiWithReadOnly,
    },
};

// Separate type because U64 doesn't implement BorshSerialize
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Default, PartialEq)]
pub struct MerkleTreeSequenceNumber {
    pub pubkey: Pubkey,
    pub seq: u64,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Default, PartialEq)]
pub struct PublicTransactionEvent {
    pub input_compressed_account_hashes: Vec<[u8; 32]>,
    pub output_compressed_account_hashes: Vec<[u8; 32]>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub output_leaf_indices: Vec<u32>,
    pub sequence_numbers: Vec<MerkleTreeSequenceNumber>,
    pub relay_fee: Option<u64>,
    pub is_compress: bool,
    pub compress_or_decompress_lamports: Option<u64>,
    pub pubkey_array: Vec<Pubkey>,
    pub message: Option<Vec<u8>>,
}
#[derive(Debug, Clone)]
pub struct NewAddress {
    pub address: [u8; 32],
    pub mt_pubkey: Pubkey,
}

#[derive(Debug, Clone)]
pub struct BatchPublicTransactionEvent {
    pub event: PublicTransactionEvent,
    pub new_addresses: Vec<NewAddress>,
    pub input_sequence_numbers: Vec<MerkleTreeSequenceNumber>,
    pub address_sequence_numbers: Vec<MerkleTreeSequenceNumber>,
    pub nullifier_queue_indices: Vec<u64>,
    pub tx_hash: [u8; 32],
    pub nullifiers: Vec<[u8; 32]>
}

/// We piece the event together from 2 instructions:
/// 1. light_system_program::{Invoke, InvokeCpi, InvokeCpiReadOnly} (one of the 3)
/// 2. account_compression::InsertIntoQueues
/// - We return new addresses in batched trees separately
///     because from the PublicTransactionEvent there
///     is no way to know which addresses are new and
///     for batched address trees we need to index the queue of new addresses
///     the tree&queue account only contains bloomfilters, roots and metadata.
///
/// Steps:
/// 1. search instruction which matches one of the system instructions
/// 2. search instruction which matches InsertIntoQueues
/// 3. Populate pubkey array with remaining accounts.
pub fn event_from_light_transaction(
    instructions: &[Vec<u8>],
    remaining_accounts: Vec<Vec<Pubkey>>,
) -> Result<Option<BatchPublicTransactionEvent>, ZeroCopyError> {
    let mut event = PublicTransactionEvent::default();
    let mut ix_set_cpi_context = false;
    let mut input_merkle_tree_indices = Vec::new();
    let found_event = instructions
        .iter()
        .zip(remaining_accounts.iter())
        .any(|(x, accounts)| {
            match_system_program_instruction(
                x,
                false,
                &mut event,
                &mut ix_set_cpi_context,
                &mut input_merkle_tree_indices,
                accounts,
            )
                .unwrap_or_default()
        });
    println!("found event {}", found_event);
    if !found_event {
        return Ok(None);
    }
    println!("ix_set_cpi_context {}", ix_set_cpi_context);
    // If an instruction set the cpi context add the instructions that set the cpi context.
    if ix_set_cpi_context {
        instructions
            .iter()
            .zip(remaining_accounts.iter())
            .try_for_each(|(x, accounts)| -> Result<(), ZeroCopyError> {
                match_system_program_instruction(
                    x,
                    true,
                    &mut event,
                    &mut true,
                    &mut input_merkle_tree_indices,
                    accounts,
                )?;
                Ok(())
            })?;
        println!("added cpi context to event {}", found_event);
    }
    // New addresses in batched trees.
    let mut new_addresses = Vec::new();
    let mut input_sequence_numbers = Vec::new();
    let mut address_sequence_numbers = Vec::new();
    let mut tx_hash = [0u8; 32];
    let mut nullifiers = vec![];
    let mut pos = None;
    for (i, instruction) in instructions.iter().enumerate() {
        if remaining_accounts[i].len() < 3 {
            continue;
        }
        let res = match_account_compression_program_instruction(
            instruction,
            &mut event,
            &mut new_addresses,
            &mut input_sequence_numbers,
            &mut address_sequence_numbers,
            &remaining_accounts[i][2..],
            &mut tx_hash,
            &mut nullifiers,
        )?;
        if res {
            pos = Some(i);
            break;
        }
    };

    println!("pos {:?}", pos);
    if let Some(pos) = pos {
        println!("remaining accounts {:?}", remaining_accounts);
        event.pubkey_array = remaining_accounts[pos][2..].to_vec().clone();
        println!("event pubkey array {:?}", event.pubkey_array);
        println!("input_sequence_numbers {:?}", input_sequence_numbers);
        println!("address_sequence_numbers {:?}", address_sequence_numbers);

        // Nullifier queue indices are continous similar to sequence numbers.
        // The emitted sequence number marks the first insertion into the queue in this tx.
        // Iterate over all sequence numbers, match with input accounts Merkle tree and increment the sequence number.
        // u64::MAX means it is a v1 account and it doesn't have a queue index.
        let mut nullifier_queue_indices =
            vec![u64::MAX; event.input_compressed_account_hashes.len()];
        let mut internal_input_sequence_numbers = input_sequence_numbers.clone();
        internal_input_sequence_numbers.iter_mut().for_each(|seq| {
            for (i, merkle_tree_pubkey) in input_merkle_tree_indices.iter().enumerate() {
                println!(
                    " seq pubkey {:?} == merkle tree pubkey {:?}",
                    seq.pubkey, *merkle_tree_pubkey
                );
                if *merkle_tree_pubkey == seq.pubkey {
                    nullifier_queue_indices[i] = seq.seq;
                    seq.seq += 1;
                }
            }
        });
        println!("input_merkle_tree_indices {:?}", input_merkle_tree_indices);
        println!("nullifier_queue_indices {:?}", nullifier_queue_indices);
        println!("input_sequence_numbers {:?}", input_sequence_numbers);
        Ok(Some(BatchPublicTransactionEvent {
            event,
            new_addresses,
            input_sequence_numbers,
            address_sequence_numbers,
            tx_hash,
            nullifiers,
            nullifier_queue_indices,
        }))
    } else {
        Ok(None)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn match_account_compression_program_instruction(
    instruction: &[u8],
    event: &mut PublicTransactionEvent,
    new_addresses: &mut Vec<NewAddress>,
    input_sequence_numbers: &mut Vec<MerkleTreeSequenceNumber>,
    address_sequence_numbers: &mut Vec<MerkleTreeSequenceNumber>,
    accounts: &[Pubkey],
    tx_hash: &mut [u8; 32],
    nullifiers: &mut Vec<[u8; 32]>,
) -> Result<bool, ZeroCopyError> {
    if instruction.len() < 8 {
        return Ok(false);
    }
    let instruction_discriminator = instruction[0..8].try_into().unwrap();

    match instruction_discriminator {
        DISCRIMINATOR_INSERT_INTO_QUEUES => {
            let (_, instruction) = instruction.split_at(12);
            let (data, _) = InsertIntoQueuesInstructionData::zero_copy_at(instruction)?;
            event.input_compressed_account_hashes =
                data.nullifiers.iter().map(|x| x.account_hash).collect();
            event.output_compressed_account_hashes = data.leaves.iter().map(|x| x.leaf).collect();
            event.sequence_numbers = data
                .output_sequence_numbers
                .iter()
                .map(|x| MerkleTreeSequenceNumber {
                    pubkey: x.pubkey.into(),
                    seq: x.seq.into(),
                })
                .collect();
            event.output_leaf_indices = data
                .output_leaf_indices
                .iter()
                .map(|x| (*x).into())
                .collect();
            // overwrite the merkle tree index in the output accounts
            // because this index is consistent with the pubkey array.
            event
                .output_compressed_accounts
                .iter_mut()
                .zip(data.leaves.iter())
                .for_each(|(x, y)| {
                    x.merkle_tree_index = y.account_index;
                });
            data.addresses.iter().for_each(|x| {
                if x.tree_index == x.queue_index {
                    new_addresses.push(NewAddress {
                        address: x.address,
                        mt_pubkey: accounts[x.queue_index as usize],
                    });
                }
            });
            data.input_sequence_numbers.iter().for_each(|x| {
                if x.pubkey != Pubkey::default().into() {
                    input_sequence_numbers.push(MerkleTreeSequenceNumber {
                        pubkey: x.pubkey.into(),
                        seq: x.seq.into(),
                    });
                }
            });
            data.address_sequence_numbers.iter().for_each(|x| {
                if x.pubkey != Pubkey::default().into() {
                    address_sequence_numbers.push(MerkleTreeSequenceNumber {
                        pubkey: x.pubkey.into(),
                        seq: x.seq.into(),
                    });
                }
            });
            *tx_hash = data.tx_hash;

            data.nullifiers.iter().for_each(|n| {
                let nullifier = {
                    let mut leaf_index_bytes = [0u8; 32];
                    leaf_index_bytes[28..].copy_from_slice(u32::from(n.leaf_index).to_be_bytes().as_slice());
                    // Inclusion of the tx_hash enables zk proofs of how a value was spent.
                    Poseidon::hashv(&[n.account_hash.as_slice(), &leaf_index_bytes, tx_hash]).unwrap()
                };
                nullifiers.push(nullifier);
            });
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub fn match_system_program_instruction(
    instruction: &[u8],
    set_cpi_context: bool,
    event: &mut PublicTransactionEvent,
    ix_set_cpi_context: &mut bool,
    input_merkle_tree_indices: &mut Vec<Pubkey>,
    accounts: &[Pubkey],
) -> Result<bool, ZeroCopyError> {
    if instruction.len() < 12 {
        return Ok(false);
    }
    let instruction_discriminator = instruction[0..8].try_into().unwrap();
    let instruction = instruction.split_at(12).1;
    match instruction_discriminator {
        DISCRIMINATOR_INVOKE => {
            let (data, _) = ZInstructionDataInvoke::zero_copy_at(instruction)?;
            event.output_compressed_accounts = data
                .output_compressed_accounts
                .iter()
                .map(OutputCompressedAccountWithPackedContext::from)
                .collect();
            event.is_compress = data.is_compress;
            event.relay_fee = data.relay_fee.map(|x| (*x).into());
            event.compress_or_decompress_lamports =
                data.compress_or_decompress_lamports.map(|x| (*x).into());
            // We are only interested in remaining account which start after 9 static accounts.
            let remaining_accounts = accounts.split_at(9).1;
            data.input_compressed_accounts_with_merkle_context
                .iter()
                .for_each(|x| {
                    input_merkle_tree_indices.push(
                        remaining_accounts[x.merkle_context.merkle_tree_pubkey_index as usize],
                    );
                });
            Ok(true)
        }
        DISCRIMINATOR_INVOKE_CPI => {
            let (data, _) = ZInstructionDataInvokeCpi::zero_copy_at(instruction)?;
            // We are only interested in remaining account which start after 10 static accounts.
            let remaining_accounts = accounts.split_at(9).1;
            // We need to find the instruction that executed the verification first.
            // If cpi context was set we need to find those instructions afterwards and add them to the event.
            if let Some(cpi_context) = data.cpi_context {
                *ix_set_cpi_context = true;
                if (cpi_context.first_set_context() || cpi_context.set_context())
                    && !set_cpi_context
                {
                    return Ok(false);
                } else {
                    data.output_compressed_accounts.iter().for_each(|x| {
                        event
                            .output_compressed_accounts
                            .push(OutputCompressedAccountWithPackedContext::from(x));
                    });
                    // We are only interested in remaining account which start after 9 static accounts.
                    data.input_compressed_accounts_with_merkle_context
                        .iter()
                        .for_each(|x| {
                            input_merkle_tree_indices.push(
                                remaining_accounts
                                    [x.merkle_context.merkle_tree_pubkey_index as usize],
                            );
                        });
                    return Ok(true);
                }
            }
            event.output_compressed_accounts = data
                .output_compressed_accounts
                .iter()
                .map(OutputCompressedAccountWithPackedContext::from)
                .collect();
            event.is_compress = data.is_compress;
            event.relay_fee = data.relay_fee.map(|x| (*x).into());
            event.compress_or_decompress_lamports =
                data.compress_or_decompress_lamports.map(|x| (*x).into());
            data.input_compressed_accounts_with_merkle_context
                .iter()
                .for_each(|x| {
                    input_merkle_tree_indices.push(
                        remaining_accounts[x.merkle_context.merkle_tree_pubkey_index as usize],
                    );
                });
            Ok(true)
        }
        DISCRIMINATOR_INVOKE_CPI_WITH_READ_ONLY => {
            let (data, _) = ZInstructionDataInvokeCpiWithReadOnly::zero_copy_at(instruction)?;
            let data = data.invoke_cpi;
            // We are only interested in remaining account which start after 10 static accounts.
            let remaining_accounts = accounts.split_at(9).1;
            // We need to find the instruction that executed the verification first.
            // If cpi context was set we need to find those instructions afterwards and add them to the event.
            if let Some(cpi_context) = data.cpi_context {
                *ix_set_cpi_context = true;
                if (cpi_context.first_set_context() || cpi_context.set_context())
                    && !set_cpi_context
                {
                    return Ok(false);
                } else {
                    data.output_compressed_accounts.iter().for_each(|x| {
                        event
                            .output_compressed_accounts
                            .push(OutputCompressedAccountWithPackedContext::from(x));
                    });
                    data.input_compressed_accounts_with_merkle_context
                        .iter()
                        .for_each(|x| {
                            input_merkle_tree_indices.push(
                                remaining_accounts
                                    [x.merkle_context.merkle_tree_pubkey_index as usize],
                            );
                        });
                    return Ok(true);
                }
            }
            event.output_compressed_accounts = data
                .output_compressed_accounts
                .iter()
                .map(OutputCompressedAccountWithPackedContext::from)
                .collect();
            event.is_compress = data.is_compress;
            event.relay_fee = data.relay_fee.map(|x| (*x).into());
            event.compress_or_decompress_lamports =
                data.compress_or_decompress_lamports.map(|x| (*x).into());
            data.input_compressed_accounts_with_merkle_context
                .iter()
                .for_each(|x| {
                    input_merkle_tree_indices.push(
                        remaining_accounts[x.merkle_context.merkle_tree_pubkey_index as usize],
                    );
                });
            Ok(true)
        }
        _ => Ok(false),
    }
}
