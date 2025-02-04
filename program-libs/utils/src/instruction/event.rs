use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError};
use solana_program::pubkey::Pubkey;

use super::{discriminators::*, insert_into_queues::AppendNullifyCreateAddressInputsIndexer};
use crate::instruction::{
    instruction_data::OutputCompressedAccountWithPackedContext,
    instruction_data_zero_copy::{
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

pub struct NewAddress {
    pub address: [u8; 32],
    pub mt_pubkey: Pubkey,
}

// TODO: remove unwraps
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
) -> Result<(Option<PublicTransactionEvent>, Option<Vec<NewAddress>>), ZeroCopyError> {
    let mut event = PublicTransactionEvent::default();
    let mut ix_set_cpi_context = false;
    let found_event = instructions.iter().any(|x| {
        match_system_program_instruction(x, false, &mut event, &mut ix_set_cpi_context).unwrap()
    });
    println!("found event {}", found_event);
    if !found_event {
        return Ok((None, None));
    }
    println!("ix_set_cpi_context {}", ix_set_cpi_context);
    // If an instruction set the cpi context add the instructions that set the cpi context.
    if ix_set_cpi_context {
        instructions.iter().for_each(|x| {
            match_system_program_instruction(x, true, &mut event, &mut true).unwrap();
        });
        println!("added cpi context to event {}", found_event);
    }
    // New addresses in batched trees.
    let mut new_addresses = Vec::new();
    let pos = instructions.iter().enumerate().position(|(i, x)| {
        match_account_compression_program_instruction(
            x,
            &mut event,
            &mut new_addresses,
            &remaining_accounts[i][2..],
        )
        .unwrap()
    });

    println!("pos {:?}", pos);
    if let Some(pos) = pos {
        println!("remaining accounts {:?}", remaining_accounts);
        event.pubkey_array = remaining_accounts[pos][2..].to_vec().clone();
        println!("event pubkey array {:?}", event.pubkey_array);
        let new_addresses = if new_addresses.is_empty() {
            None
        } else {
            Some(new_addresses)
        };
        Ok((Some(event), new_addresses))
    } else {
        Ok((None, None))
    }
}

pub fn match_account_compression_program_instruction(
    instruction: &[u8],
    event: &mut PublicTransactionEvent,
    new_addresses: &mut Vec<NewAddress>,
    accounts: &[Pubkey],
) -> Result<bool, ZeroCopyError> {
    if instruction.len() < 8 {
        return Ok(false);
    }
    let instruction_discriminator = instruction[0..8].try_into().unwrap();

    match instruction_discriminator {
        DISCRIMINATOR_INSERT_INTO_QUEUES => {
            let (_, instruction) = instruction.split_at(12);
            let (data, _) = AppendNullifyCreateAddressInputsIndexer::zero_copy_at(instruction)?;
            event.input_compressed_account_hashes =
                data.nullifiers.iter().map(|x| x.account_hash).collect();
            event.output_compressed_account_hashes = data.leaves.iter().map(|x| x.leaf).collect();
            event.sequence_numbers = data
                .sequence_numbers
                .iter()
                .map(|x| MerkleTreeSequenceNumber {
                    pubkey: x.pubkey.to_bytes().into(),
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
                    x.merkle_tree_index = y.index;
                });
            data.addresses.iter().for_each(|x| {
                if x.tree_index == x.queue_index {
                    new_addresses.push(NewAddress {
                        address: x.address,
                        mt_pubkey: accounts[x.queue_index as usize],
                    });
                }
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
            Ok(true)
        }
        DISCRIMINATOR_INVOKE_CPI => {
            let (data, _) = ZInstructionDataInvokeCpi::zero_copy_at(instruction)?;
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
            Ok(true)
        }
        DISCRIMINATOR_INVOKE_CPI_WITH_READ_ONLY => {
            let (data, _) = ZInstructionDataInvokeCpiWithReadOnly::zero_copy_at(instruction)?;
            let data = data.invoke_cpi;
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
            Ok(true)
        }
        _ => Ok(false),
    }
}
