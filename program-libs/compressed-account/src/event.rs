use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::{Hasher, Poseidon};
use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError};
use solana_program::pubkey::Pubkey;

use super::discriminators::*;
use crate::{
    address::{derive_address, derive_address_legacy},
    compressed_account::PackedCompressedAccountWithMerkleContext,
    constants::{
        ACCOUNT_COMPRESSION_PROGRAM_ID, CREATE_CPI_CONTEXT_ACCOUNT, REGISTERED_PROGRAM_PDA,
        SYSTEM_PROGRAM_ID,
    },
    instruction_data::{
        data::{InstructionDataInvoke, OutputCompressedAccountWithPackedContext},
        insert_into_queues::InsertIntoQueuesInstructionData,
        invoke_cpi::InstructionDataInvokeCpiWithReadOnly,
    },
};

// Separate type because U64 doesn't implement BorshSerialize
#[derive(Debug, Clone, Copy, BorshSerialize, BorshDeserialize, Default, PartialEq)]
pub struct MerkleTreeSequenceNumber {
    pub pubkey: Pubkey,
    pub seq: u64,
}

impl MerkleTreeSequenceNumber {
    pub fn new(
        seq: &crate::instruction_data::insert_into_queues::MerkleTreeSequenceNumber,
    ) -> Self {
        Self {
            pubkey: seq.pubkey.into(),
            seq: seq.seq.into(),
        }
    }
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NewAddress {
    pub address: [u8; 32],
    pub mt_pubkey: Pubkey,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BatchPublicTransactionEvent {
    pub event: PublicTransactionEvent,
    pub new_addresses: Vec<NewAddress>,
    pub input_sequence_numbers: Vec<MerkleTreeSequenceNumber>,
    pub address_sequence_numbers: Vec<MerkleTreeSequenceNumber>,
    pub tx_hash: [u8; 32],
    pub batch_input_accounts: Vec<BatchNullifyContext>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct BatchNullifyContext {
    pub tx_hash: [u8; 32],
    pub account_hash: [u8; 32],
    pub nullifier: [u8; 32],
    pub nullifier_queue_index: u64,
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
    program_ids: &[Pubkey],
    instructions: &[Vec<u8>],
    remaining_accounts: Vec<Vec<Pubkey>>,
) -> Result<Option<BatchPublicTransactionEvent>, ZeroCopyError> {
    let mut event = PublicTransactionEvent::default();
    let mut ix_set_cpi_context = false;
    let mut input_merkle_tree_indices = Vec::new();
    let found_event = instructions
        .iter()
        .zip(program_ids.iter())
        .zip(remaining_accounts.iter())
        // .filter(|((_, program_id), _)| **program_id == SYSTEM_PROGRAM_ID)
        .position(|((x, program_id), accounts)| {
            if *program_id != SYSTEM_PROGRAM_ID {
                return false;
            }
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
    println!("found event {}", found_event.is_some());
    if found_event.is_none() {
        return Ok(None);
    }
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
        println!("added cpi context to event {}", found_event.is_some());
    }
    // New addresses in batched trees.
    let mut new_addresses = Vec::new();
    let mut input_sequence_numbers = Vec::new();
    let mut address_sequence_numbers = Vec::new();
    let mut tx_hash = [0u8; 32];
    let mut batch_input_accounts = vec![];
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
            &mut batch_input_accounts,
        )?;
        if res {
            pos = Some(i);
            break;
        }
    }
    if event == PublicTransactionEvent::default()
        && address_sequence_numbers.is_empty()
        && input_sequence_numbers.is_empty()
        && new_addresses.is_empty()
    {
        return Ok(None);
    }
    println!("pos {:?}", pos);
    if let Some(pos) = found_event {
        println!("remaining accounts {:?}", remaining_accounts);
        let discriminator = instructions[pos][0..8].try_into().unwrap();
        match discriminator {
            DISCRIMINATOR_INVOKE => {
                event.pubkey_array = remaining_accounts[pos][9..].to_vec();
            }
            _ => {
                event.pubkey_array = remaining_accounts[pos][11..].to_vec();
            }
        }
        println!("event pubkey array {:?}", event.pubkey_array);
        println!("input_sequence_numbers {:?}", input_sequence_numbers);
        println!("address_sequence_numbers {:?}", address_sequence_numbers);

        // Nullifier queue indices are continous similar to sequence numbers.
        // The emitted sequence number marks the first insertion into the queue in this tx.
        // Iterate over all sequence numbers, match with input accounts Merkle tree and increment the sequence number.
        // u64::MAX means it is a v1 account and it doesn't have a queue index.
        let mut nullifier_queue_indices =
            vec![u64::MAX; event.input_compressed_account_hashes.len()];
        println!("legacyinternal_input_sequence_numbers ");
        let mut internal_input_sequence_numbers = input_sequence_numbers.clone();
        println!(
            "internal_input_sequence_numbers {:?}",
            internal_input_sequence_numbers
        );
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

        assert_eq!(
            nullifier_queue_indices
                .iter()
                .filter(|x| **x != u64::MAX)
                .count(),
            batch_input_accounts.len()
        );
        for (index, context) in nullifier_queue_indices
            .iter()
            .zip(batch_input_accounts.iter_mut())
        {
            context.nullifier_queue_index = *index;
        }
        println!("input_merkle_tree_indices {:?}", input_merkle_tree_indices);
        println!("nullifier_queue_indices {:?}", nullifier_queue_indices);
        println!("input_sequence_numbers {:?}", input_sequence_numbers);

        Ok(Some(BatchPublicTransactionEvent {
            event,
            new_addresses,
            input_sequence_numbers,
            address_sequence_numbers,
            tx_hash,
            batch_input_accounts,
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
    batch_input_accounts: &mut Vec<BatchNullifyContext>,
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
            // event
            //     .output_compressed_accounts
            //     .iter_mut()
            //     .zip(data.leaves.iter())
            //     .for_each(|(x, y)| {
            //         x.merkle_tree_index = y.account_index;
            //     });
            data.addresses.iter().for_each(|x| {
                if x.tree_index == x.queue_index {
                    new_addresses.push(NewAddress {
                        address: x.address,
                        mt_pubkey: accounts[x.queue_index as usize],
                    });
                }
            });
            data.input_sequence_numbers.iter().for_each(|x| {
                // Skip accounts nullified in legacy trees (x.pubkey == Pubkey::default())
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
                let tree_pubkey = &accounts[n.tree_index as usize];
                if input_sequence_numbers
                    .iter()
                    .any(|x| x.pubkey == *tree_pubkey)
                {
                    let nullifier = {
                        let mut leaf_index_bytes = [0u8; 32];
                        leaf_index_bytes[28..]
                            .copy_from_slice(u32::from(n.leaf_index).to_be_bytes().as_slice());
                        // Inclusion of the tx_hash enables zk proofs of how a value was spent.
                        Poseidon::hashv(&[n.account_hash.as_slice(), &leaf_index_bytes, tx_hash])
                            .unwrap()
                    };
                    batch_input_accounts.push(BatchNullifyContext {
                        tx_hash: *tx_hash,
                        account_hash: n.account_hash,
                        nullifier,
                        nullifier_queue_index: u64::MAX,
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
            let data = InstructionDataInvoke::deserialize(&mut &instruction[..])
                .map_err(|_| ZeroCopyError::Size)?;
            event.output_compressed_accounts = data.output_compressed_accounts;
            event.is_compress = data.is_compress;
            event.relay_fee = data.relay_fee;
            event.compress_or_decompress_lamports = data.compress_or_decompress_lamports;
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
            let data = crate::instruction_data::invoke_cpi::InstructionDataInvokeCpi::deserialize(
                &mut &instruction[..],
            )
            .map_err(|_| ZeroCopyError::Size)?;
            // We are only interested in remaining account which start after 11 static accounts.
            let remaining_accounts = accounts.split_at(11).1;
            // We need to find the instruction that executed the verification first.
            // If cpi context was set we need to find those instructions afterwards and add them to the event.
            if let Some(cpi_context) = data.cpi_context {
                *ix_set_cpi_context = true;
                if (cpi_context.first_set_context || cpi_context.set_context) && !set_cpi_context {
                    return Ok(false);
                } else {
                    data.output_compressed_accounts.iter().for_each(|x| {
                        event.output_compressed_accounts.push(x.clone());
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
            event.output_compressed_accounts = data.output_compressed_accounts;
            event.is_compress = data.is_compress;
            event.relay_fee = data.relay_fee;
            event.compress_or_decompress_lamports = data.compress_or_decompress_lamports;
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
            let data = InstructionDataInvokeCpiWithReadOnly::deserialize(&mut &instruction[..])
                .map_err(|_| ZeroCopyError::Size)?;
            let data = data.invoke_cpi;
            // We are only interested in remaining account which start after 11 static accounts.
            let remaining_accounts = accounts.split_at(11).1;
            // We need to find the instruction that executed the verification first.
            // If cpi context was set we need to find those instructions afterwards and add them to the event.
            if let Some(cpi_context) = data.cpi_context {
                *ix_set_cpi_context = true;
                if (cpi_context.first_set_context || cpi_context.set_context) && !set_cpi_context {
                    return Ok(false);
                } else {
                    data.output_compressed_accounts.iter().for_each(|x| {
                        event.output_compressed_accounts.push(x.clone());
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
            event.output_compressed_accounts = data.output_compressed_accounts;
            event.is_compress = data.is_compress;
            event.relay_fee = data.relay_fee;
            event.compress_or_decompress_lamports = data.compress_or_decompress_lamports;
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

#[derive(Debug, Clone, PartialEq)]
pub struct DeserializedInstructions<'a> {
    pub executing_system_instruction: Vec<ExecutingSystemInstruction<'a>>,
    pub cpi_system_instruction: Vec<CpiSystemInstruction<'a>>,
    pub insert_into_queues_instruction: Vec<(InsertIntoQueuesInstructionData<'a>, &'a [Pubkey])>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutingSystemInstruction<'a> {
    output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    input_compressed_accounts: Vec<PackedCompressedAccountWithMerkleContext>,
    new_address: Vec<NewAddress>,
    is_compress: bool,
    relay_fee: Option<u64>,
    compress_or_decompress_lamports: Option<u64>,
    execute_cpi_context: bool,
    discriminator: [u8; 8],
    accounts: &'a [Pubkey],
}

#[derive(Debug, Clone, PartialEq)]
pub struct CpiSystemInstruction<'a> {
    output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    input_compressed_accounts: Vec<PackedCompressedAccountWithMerkleContext>,
    accounts: &'a [Pubkey],
}

fn deserialize_instruction<'a>(
    instruction: &'a [u8],
    accounts: &'a [Pubkey],
) -> Result<ExecutingSystemInstruction<'a>, ZeroCopyError> {
    if instruction.len() < 12 {
        return Err(ZeroCopyError::Size);
    }
    let instruction_discriminator = instruction[0..8]
        .try_into()
        .map_err(|_| ZeroCopyError::Size)?;
    let instruction = instruction.split_at(12).1;

    match instruction_discriminator {
        // Cannot be exucted with cpi context -> executing tx
        DISCRIMINATOR_INVOKE => {
            let accounts = accounts.split_at(9).1;
            let data = InstructionDataInvoke::deserialize(&mut &instruction[..])
                .map_err(|_| ZeroCopyError::Size)?;
            Ok(ExecutingSystemInstruction {
                output_compressed_accounts: data.output_compressed_accounts,
                input_compressed_accounts: data.input_compressed_accounts_with_merkle_context,
                is_compress: data.is_compress,
                relay_fee: data.relay_fee,
                compress_or_decompress_lamports: data.compress_or_decompress_lamports,
                execute_cpi_context: false,
                discriminator: DISCRIMINATOR_INVOKE,
                new_address: data
                    .new_address_params
                    .iter()
                    .map(|x| NewAddress {
                        address: derive_address_legacy(
                            &accounts[x.address_merkle_tree_account_index as usize],
                            &x.seed,
                        )
                        .unwrap(),
                        mt_pubkey: accounts[x.address_merkle_tree_account_index as usize],
                    })
                    .collect::<Vec<_>>(),
                accounts,
            })
        }
        DISCRIMINATOR_INVOKE_CPI => {
            println!("Found invoke cpi");
            println!("accounts: {:?}", accounts);
            let invoking_program_id = &accounts[6].to_bytes();
            let accounts = accounts.split_at(11).1;
            let data = crate::instruction_data::invoke_cpi::InstructionDataInvokeCpi::deserialize(
                &mut &instruction[..],
            )
            .map_err(|_| ZeroCopyError::Size)?;
            Ok(ExecutingSystemInstruction {
                output_compressed_accounts: data.output_compressed_accounts,
                input_compressed_accounts: data.input_compressed_accounts_with_merkle_context,
                is_compress: data.is_compress,
                relay_fee: data.relay_fee,
                compress_or_decompress_lamports: data.compress_or_decompress_lamports,
                discriminator: DISCRIMINATOR_INVOKE_CPI,
                execute_cpi_context: data.cpi_context.is_some(),
                new_address: data
                    .new_address_params
                    .iter()
                    .map(|x| NewAddress {
                        address: derive_address(
                            &x.seed,
                            &accounts[x.address_merkle_tree_account_index as usize].to_bytes(),
                            invoking_program_id,
                        ),
                        mt_pubkey: accounts[x.address_merkle_tree_account_index as usize],
                    })
                    .collect::<Vec<_>>(),
                accounts,
            })
        }
        DISCRIMINATOR_INVOKE_CPI_WITH_READ_ONLY => {
            let invoking_program_id = &accounts[6].to_bytes();
            let accounts = accounts.split_at(11).1;
            let data = InstructionDataInvokeCpiWithReadOnly::deserialize(&mut &instruction[..])
                .map_err(|_| ZeroCopyError::Size)?;

            Ok(ExecutingSystemInstruction {
                output_compressed_accounts: data.invoke_cpi.output_compressed_accounts,
                input_compressed_accounts: data
                    .invoke_cpi
                    .input_compressed_accounts_with_merkle_context,
                is_compress: data.invoke_cpi.is_compress,
                relay_fee: data.invoke_cpi.relay_fee,
                compress_or_decompress_lamports: data.invoke_cpi.compress_or_decompress_lamports,
                discriminator: DISCRIMINATOR_INVOKE_CPI_WITH_READ_ONLY,
                execute_cpi_context: data.invoke_cpi.cpi_context.is_some(),
                new_address: data
                    .invoke_cpi
                    .new_address_params
                    .iter()
                    .map(|x| NewAddress {
                        address: derive_address(
                            &x.seed,
                            &accounts[x.address_merkle_tree_account_index as usize].to_bytes(),
                            invoking_program_id,
                        ),
                        mt_pubkey: accounts[x.address_merkle_tree_account_index as usize],
                    })
                    .collect::<Vec<_>>(),
                accounts,
            })
        }
        _ => Err(ZeroCopyError::Size),
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Indices {
    pub system: usize,
    pub cpi: Vec<usize>,
    pub insert_into_queues: usize,
    pub found_solana_system_program_instruction: bool,
    pub found_system: bool,
}

/// Filter all system instructions which create cpi context accounts,
/// so that we can infer that a system program instruction is a light transaction.
/// Create new AssociatedInstructions when we find a system instruction
/// if next instruct is solana system program isntruction followed by insert into queues is executable instruction
/// else is cpi instruction
/// only push into vec if insert into queues instruction is found
pub fn find_patterns(program_ids: &[ProgramId]) -> Vec<Indices> {
    let mut vec = Vec::new();
    let mut next_index = usize::MAX;
    for (last_index, program_id) in (0..program_ids.len()).rev().zip(program_ids.iter().rev()) {
        // skip last found pattern
        if last_index > next_index {
            continue;
        }
        // In case that we encounter more than one account compression program ix
        // before finding one or more system program ix we just overwrite.
        if let ProgramId::AccountCompression = program_id {
            let (res, last_index) = find_pattern(last_index, program_ids);
            next_index = last_index;
            if let Some(res) = res {
                vec.push(res);
            };
        }
    }
    vec
}

/// Pattern, SYSTEM_PROGRAM_ID.., default ids .., account compression program id
/// We search for the pattern in reverse because there can be multiple system instructions
/// but only one account compression instruction.
/// Start index points to ACCOUNT_COMPRESSION_PROGRAM_ID
pub fn find_pattern(start_index: usize, program_ids: &[ProgramId]) -> (Option<Indices>, usize) {
    let mut index_account = Indices {
        insert_into_queues: start_index,
        ..Default::default()
    };
    for (index, program_id) in (0..start_index)
        .rev()
        .zip(program_ids[..start_index].iter().rev())
    {
        if let ProgramId::SolanaSystem = program_id {
            index_account.found_solana_system_program_instruction = true;
            continue;
        } else if matches!(program_id, ProgramId::LightSystem)
            && index_account.found_solana_system_program_instruction
            && !index_account.found_system
        {
            index_account.system = index;
            index_account.found_system = true;
        } else if index_account.found_system && matches!(program_id, ProgramId::LightSystem) {
            index_account.cpi.push(index);
        } else if matches!(program_id, ProgramId::AccountCompression) && index_account.found_system
        {
            // Possibly found next light transaction.
            return (Some(index_account), index);
        } else if !index_account.found_system {
            // If no system program found pattern incomplete.
            // Else search for cpi instructions until we find account compression program id.
            return (None, index);
        }
    }
    if index_account.found_system {
        (Some(index_account), 0)
    } else {
        (None, 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProgramId {
    LightSystem,
    AccountCompression,
    SolanaSystem,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssociatedInstructions<'a> {
    pub executing_system_instruction: ExecutingSystemInstruction<'a>,
    pub cpi_system_instructions: Vec<CpiSystemInstruction<'a>>,
    pub insert_into_queues_instruction: InsertIntoQueuesInstructionData<'a>,
    pub accounts: &'a [Pubkey],
}

pub fn wrap_program_ids(
    program_ids: &[Pubkey],
    instructions: &[Vec<u8>],
    accounts: &[Vec<Pubkey>],
) -> Vec<ProgramId> {
    let mut vec = Vec::new();
    for ((instruction, program_id), accounts) in instructions
        .iter()
        .zip(program_ids.iter())
        .zip(accounts.iter())
    {
        if instruction.len() < 12 {
            vec.push(ProgramId::Unknown);
            continue;
        }
        let discriminator: [u8; 8] = instruction[0..8].try_into().unwrap();
        if program_id == &Pubkey::default() {
            vec.push(ProgramId::SolanaSystem);
        } else if program_id == &SYSTEM_PROGRAM_ID {
            if discriminator == CREATE_CPI_CONTEXT_ACCOUNT {
                vec.push(ProgramId::Unknown);
            } else {
                vec.push(ProgramId::LightSystem);
            }
        } else if program_id == &ACCOUNT_COMPRESSION_PROGRAM_ID {
            if discriminator == DISCRIMINATOR_INSERT_INTO_QUEUES
                && accounts.len() > 2
                && accounts[1] == REGISTERED_PROGRAM_PDA
            {
                vec.push(ProgramId::AccountCompression);
            } else {
                vec.push(ProgramId::Unknown);
            }
        } else {
            vec.push(ProgramId::Unknown);
        }
    }
    vec
}

/// 0. Wrap program ids of instructions to filter but not change the pattern
///         system program cpi context creation ixs
///         insert into queue ixs not by the system program
///         instructions with less than 12 bytes ix data
/// 1. Find associated instructions by cpi pattern.
/// 2. Deserialize associated instructions.
/// 3. Create batched transaction events.
pub fn event_from_light_transaction_new(
    program_ids: &[Pubkey],
    instructions: &[Vec<u8>],
    remaining_accounts: Vec<Vec<Pubkey>>,
) -> Result<Option<Vec<BatchPublicTransactionEvent>>, ZeroCopyError> {
    let program_ids = wrap_program_ids(program_ids, instructions, &remaining_accounts);
    println!("program_ids {:?}", program_ids);
    let mut patterns = find_patterns(&program_ids);
    if patterns.is_empty() {
        return Ok(None);
    }
    println!("patterns {:?}", patterns);
    // We deserialized from the last pattern to the first.
    //      -> reverse to be in order
    patterns.reverse();
    let associated_instructions = patterns
        .iter()
        .map(|pattern| {
            deserialize_associated_instructions(pattern, instructions, &remaining_accounts)
        })
        .collect::<Result<Vec<_>, _>>()?;
    println!("associated_instructions {:?}", associated_instructions);
    // Create batched transaction events
    Ok(Some(
        associated_instructions
            .iter()
            .map(|associated_instruction| create_batched_transaction_event(associated_instruction))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

pub fn deserialize_associated_instructions<'a>(
    indices: &Indices,
    instructions: &'a [Vec<u8>],
    accounts: &'a [Vec<Pubkey>],
) -> Result<AssociatedInstructions<'a>, ZeroCopyError> {
    let insert_queues_instruction = {
        let ix = &instructions[indices.insert_into_queues];
        let discriminator: [u8; 8] = ix[0..8].try_into().map_err(|_| ZeroCopyError::Size)?;
        if discriminator == DISCRIMINATOR_INSERT_INTO_QUEUES {
            let (data, _) = InsertIntoQueuesInstructionData::zero_copy_at(&ix[12..])?;
            Ok(data)
        } else {
            Err(ZeroCopyError::Size)
        }
    }?;
    let exec_instruction =
        deserialize_instruction(&instructions[indices.system], &accounts[indices.system])?;
    let mut cpi_instructions = Vec::new();
    for cpi_index in indices.cpi.iter() {
        let cpi_instruction =
            deserialize_cpi_instruction(&instructions[*cpi_index], &accounts[*cpi_index])?;
        cpi_instructions.push(cpi_instruction);
    }
    Ok(AssociatedInstructions {
        executing_system_instruction: exec_instruction,
        cpi_system_instructions: cpi_instructions,
        insert_into_queues_instruction: insert_queues_instruction,
        // Remove signer and register program accounts.
        accounts: &accounts[indices.insert_into_queues][2..],
    })
}

fn deserialize_cpi_instruction<'a>(
    instruction: &'a [u8],
    accounts: &'a [Pubkey],
) -> Result<CpiSystemInstruction<'a>, ZeroCopyError> {
    let discriminator = instruction[0..8]
        .try_into()
        .map_err(|_| ZeroCopyError::Size)?;
    match discriminator {
        DISCRIMINATOR_INVOKE_CPI => {
            let accounts = accounts.split_at(11).1;
            let data = crate::instruction_data::invoke_cpi::InstructionDataInvokeCpi::deserialize(
                &mut &instruction[12..],
            )
            .map_err(|_| ZeroCopyError::Size)?;
            Ok(CpiSystemInstruction {
                output_compressed_accounts: data.output_compressed_accounts,
                input_compressed_accounts: data.input_compressed_accounts_with_merkle_context,
                accounts,
            })
        }
        _ => Err(ZeroCopyError::Size),
    }
}

pub fn create_batched_transaction_event(
    associated_instructions: &AssociatedInstructions,
) -> Result<BatchPublicTransactionEvent, ZeroCopyError> {
    let input_sequence_numbers = associated_instructions
        .insert_into_queues_instruction
        .input_sequence_numbers
        .iter()
        .filter(|x| Pubkey::from(x.pubkey) != Pubkey::default())
        .map(MerkleTreeSequenceNumber::new)
        .collect::<Vec<MerkleTreeSequenceNumber>>();
    let mut batched_transaction_event = BatchPublicTransactionEvent {
        event: PublicTransactionEvent {
            input_compressed_account_hashes: associated_instructions
                .insert_into_queues_instruction
                .nullifiers
                .iter()
                .map(|x| x.account_hash)
                .collect(),
            output_compressed_account_hashes: associated_instructions
                .insert_into_queues_instruction
                .leaves
                .iter()
                .map(|x| x.leaf)
                .collect(),
            output_compressed_accounts: associated_instructions
                .executing_system_instruction
                .output_compressed_accounts
                .clone(),
            output_leaf_indices: associated_instructions
                .insert_into_queues_instruction
                .output_leaf_indices
                .iter()
                .map(|x| u32::from(*x))
                .collect(),
            sequence_numbers: associated_instructions
                .insert_into_queues_instruction
                .output_sequence_numbers
                .iter()
                .map(|x| MerkleTreeSequenceNumber {
                    pubkey: x.pubkey.into(),
                    seq: x.seq.into(),
                })
                .collect(),
            relay_fee: associated_instructions
                .executing_system_instruction
                .relay_fee,
            is_compress: associated_instructions
                .executing_system_instruction
                .is_compress,
            compress_or_decompress_lamports: associated_instructions
                .executing_system_instruction
                .compress_or_decompress_lamports,
            pubkey_array: associated_instructions
                .executing_system_instruction
                .accounts
                .to_vec(),
            message: None,
        },
        tx_hash: associated_instructions
            .insert_into_queues_instruction
            .tx_hash,
        new_addresses: associated_instructions
            .executing_system_instruction
            .new_address
            .clone(),
        address_sequence_numbers: associated_instructions
            .insert_into_queues_instruction
            .address_sequence_numbers
            .iter()
            .filter(|x| Pubkey::from(x.pubkey) != Pubkey::default())
            .map(MerkleTreeSequenceNumber::new)
            .collect::<Vec<MerkleTreeSequenceNumber>>(),
        batch_input_accounts: associated_instructions
            .insert_into_queues_instruction
            .nullifiers
            .iter()
            .filter(|x| {
                input_sequence_numbers.iter().any(|y| {
                    println!(
                        "input seq pubkey {:?} nullifier context tree {:?}",
                        y.pubkey, associated_instructions.accounts[x.tree_index as usize]
                    );
                    println!(
                        "input seq pubkey {:?} nullifier context queue {:?}",
                        y.pubkey, associated_instructions.accounts[x.queue_index as usize]
                    );

                    y.pubkey == associated_instructions.accounts[x.tree_index as usize]
                })
            })
            .map(|n| {
                BatchNullifyContext {
                    tx_hash: associated_instructions
                        .insert_into_queues_instruction
                        .tx_hash,
                    account_hash: n.account_hash,
                    nullifier: {
                        let nullifier = {
                            let mut leaf_index_bytes = [0u8; 32];
                            leaf_index_bytes[28..]
                                .copy_from_slice(u32::from(n.leaf_index).to_be_bytes().as_slice());
                            // Inclusion of the tx_hash enables zk proofs of how a value was spent.
                            Poseidon::hashv(&[
                                n.account_hash.as_slice(),
                                &leaf_index_bytes,
                                &associated_instructions
                                    .insert_into_queues_instruction
                                    .tx_hash,
                            ])
                            .unwrap()
                        };
                        nullifier
                    },
                    nullifier_queue_index: u64::MAX,
                }
            })
            .collect(),
        input_sequence_numbers,
    };
    let nullifier_queue_indices =
        create_nullifier_queue_indices(associated_instructions, &batched_transaction_event);

    batched_transaction_event
        .batch_input_accounts
        .iter_mut()
        .zip(nullifier_queue_indices.iter())
        .for_each(|(context, index)| {
            context.nullifier_queue_index = *index;
        });
    for cpi_instruction in associated_instructions.cpi_system_instructions.iter() {
        batched_transaction_event
            .event
            .output_compressed_accounts
            .extend_from_slice(cpi_instruction.output_compressed_accounts.as_slice());
    }

    Ok(batched_transaction_event)
}

pub fn create_nullifier_queue_indices(
    associated_instructions: &AssociatedInstructions,
    batched_transaction_event: &BatchPublicTransactionEvent,
) -> Vec<u64> {
    let input_merkle_tree_pubkeys = associated_instructions
        .executing_system_instruction
        .input_compressed_accounts
        .iter()
        .map(|x| {
            associated_instructions
                .executing_system_instruction
                .accounts[x.merkle_context.merkle_tree_pubkey_index as usize]
        })
        .collect::<Vec<_>>();
    let mut nullifier_queue_indices =
        vec![u64::MAX; batched_transaction_event.batch_input_accounts.len()];
    let mut internal_input_sequence_numbers = associated_instructions
        .insert_into_queues_instruction
        .input_sequence_numbers
        .to_vec();
    println!(
        "internal_input_sequence_numbers {:?}",
        internal_input_sequence_numbers
    );
    internal_input_sequence_numbers.iter_mut().for_each(|seq| {
        for (i, merkle_tree_pubkey) in input_merkle_tree_pubkeys.iter().enumerate() {
            println!(
                " seq pubkey {:?} == merkle tree pubkey {:?}",
                Pubkey::from(seq.pubkey),
                *merkle_tree_pubkey
            );
            if *merkle_tree_pubkey == seq.pubkey.into() {
                nullifier_queue_indices[i] = seq.seq.into();
                seq.seq += 1;
            }
        }
    });
    // TODO: remove for prod
    assert_eq!(
        nullifier_queue_indices
            .iter()
            .filter(|x| **x != u64::MAX)
            .count(),
        batched_transaction_event.batch_input_accounts.len(),
        " {:?}",
        batched_transaction_event.batch_input_accounts
    );
    nullifier_queue_indices
}

#[cfg(test)]
mod test {
    use rand::{
        rngs::{StdRng, ThreadRng},
        Rng, RngCore, SeedableRng,
    };

    use super::*;
    fn get_rnd_program_id<R: Rng>(rng: &mut R, with_system_program: bool) -> ProgramId {
        let vec = [
            ProgramId::Unknown,
            ProgramId::AccountCompression,
            ProgramId::LightSystem,
        ];
        let len = if with_system_program { 3 } else { 2 };
        let index = rng.gen_range(0..len);
        vec[index]
    }
    fn get_rnd_program_ids<R: Rng>(
        rng: &mut R,
        len: usize,
        with_system_program: bool,
    ) -> Vec<ProgramId> {
        (0..len)
            .map(|_| get_rnd_program_id(rng, with_system_program))
            .collect()
    }

    #[test]
    fn test_rnd_functional() {
        let mut thread_rng = ThreadRng::default();
        let seed = thread_rng.next_u64();
        // Keep this print so that in case the test fails
        // we can use the seed to reproduce the error.
        println!("\n\ntest seed {}\n\n", seed);
        let mut rng = StdRng::seed_from_u64(seed);
        let num_iters = 100000;
        for _ in 0..num_iters {
            let len_pre = rng.gen_range(0..6);
            let rnd_vec_pre = get_rnd_program_ids(&mut rng, len_pre, false);
            let len_post = rng.gen_range(0..6);
            let rnd_vec_post = get_rnd_program_ids(&mut rng, len_post, false);
            let num_mid = rng.gen_range(1..6);

            let program_ids = [
                rnd_vec_pre.as_slice(),
                [ProgramId::LightSystem].as_slice(),
                vec![ProgramId::SolanaSystem; num_mid].as_slice(),
                [ProgramId::AccountCompression].as_slice(),
                rnd_vec_post.as_slice(),
            ]
            .concat();
            let start_index = program_ids.len() - 1 - len_post;
            let system_index = program_ids.len() - 1 - len_post - num_mid - 1;
            let vec = find_patterns(&program_ids);
            let expected = Indices {
                system: system_index,
                cpi: vec![],
                insert_into_queues: start_index,
                found_solana_system_program_instruction: true,
                found_system: true,
            };
            assert!(
                vec.contains(&expected),
                "program ids {:?} parsed events  {:?} expected {:?} ",
                program_ids,
                vec,
                expected,
            );
        }

        for _ in 0..num_iters {
            let len_pre = rng.gen_range(0..6);
            let rnd_vec_pre = get_rnd_program_ids(&mut rng, len_pre, true);
            let len_post = rng.gen_range(0..6);
            let rnd_vec_post = get_rnd_program_ids(&mut rng, len_post, true);
            let num_mid = rng.gen_range(1..6);

            let program_ids = [
                rnd_vec_pre.as_slice(),
                [ProgramId::LightSystem].as_slice(),
                vec![ProgramId::SolanaSystem; num_mid].as_slice(),
                [ProgramId::AccountCompression].as_slice(),
                rnd_vec_post.as_slice(),
            ]
            .concat();
            let start_index = program_ids.len() - 1 - len_post;
            let system_index = program_ids.len() - 1 - len_post - num_mid - 1;
            let vec = find_patterns(&program_ids);
            let expected = Indices {
                system: system_index,
                cpi: vec![],
                insert_into_queues: start_index,
                found_solana_system_program_instruction: true,
                found_system: true,
            };
            assert!(
                vec.iter().any(|x| x.system == expected.system
                    && x.insert_into_queues == expected.insert_into_queues),
                "program ids {:?} parsed events  {:?} expected {:?} ",
                program_ids,
                vec,
                expected,
            );
        }
    }

    #[test]
    fn test_rnd_failing() {
        let mut thread_rng = ThreadRng::default();
        let seed = thread_rng.next_u64();
        // Keep this print so that in case the test fails
        // we can use the seed to reproduce the error.
        println!("\n\ntest seed {}\n\n", seed);
        let mut rng = StdRng::seed_from_u64(seed);
        let num_iters = 100000;
        for _ in 0..num_iters {
            let len = rng.gen_range(0..20);
            let mut program_ids = get_rnd_program_ids(&mut rng, len, true);
            // if any ProgramId::LightSystem is followed by ProgramId::SolanaSystem overwrite ProgramId::SolanaSystem with ProgramId::Unknown
            for i in 0..program_ids.len().saturating_sub(1) {
                if matches!(program_ids[i], ProgramId::LightSystem)
                    && matches!(program_ids[i + 1], ProgramId::SolanaSystem)
                {
                    program_ids[i + 1] = ProgramId::Unknown;
                }
            }

            let vec = find_patterns(&program_ids);

            assert!(
                vec.is_empty(),
                "program_ids {:?} result {:?}",
                program_ids,
                vec
            );
        }
    }
    #[test]
    fn test_find_two_patterns() {
        // Std pattern
        {
            let program_ids = vec![
                ProgramId::Unknown,
                ProgramId::LightSystem,
                ProgramId::SolanaSystem,
                ProgramId::AccountCompression,
                ProgramId::Unknown,
                ProgramId::LightSystem,
                ProgramId::SolanaSystem,
                ProgramId::AccountCompression,
            ];
            let vec = find_patterns(&program_ids);
            assert_eq!(vec.len(), 2);
            assert_eq!(
                vec[0],
                Indices {
                    system: 5,
                    cpi: vec![],
                    insert_into_queues: 7,
                    found_solana_system_program_instruction: true,
                    found_system: true,
                }
            );
            assert_eq!(
                vec[1],
                Indices {
                    system: 1,
                    cpi: vec![],
                    insert_into_queues: 3,
                    found_solana_system_program_instruction: true,
                    found_system: true,
                }
            );
            // Modify only second event is valid
            {
                let mut program_ids = program_ids.clone();
                program_ids[2] = ProgramId::Unknown;
                let vec = find_patterns(&program_ids);
                assert_eq!(vec.len(), 1);
                assert_eq!(
                    vec[0],
                    Indices {
                        system: 5,
                        cpi: vec![],
                        insert_into_queues: 7,
                        found_solana_system_program_instruction: true,
                        found_system: true,
                    }
                );
            }
            // Modify only first event is valid
            {
                let mut program_ids = program_ids;
                program_ids[6] = ProgramId::Unknown;
                let vec = find_patterns(&program_ids);
                assert_eq!(vec.len(), 1);
                assert_eq!(
                    vec[0],
                    Indices {
                        system: 1,
                        cpi: vec![],
                        insert_into_queues: 3,
                        found_solana_system_program_instruction: true,
                        found_system: true,
                    }
                );
            }
        }
    }

    #[test]
    fn test_find_pattern() {
        // Std pattern
        {
            let program_ids = vec![
                ProgramId::Unknown,
                ProgramId::LightSystem,
                ProgramId::SolanaSystem,
                ProgramId::AccountCompression,
            ];
            let (res, last_index) = find_pattern(3, &program_ids);
            assert_eq!(last_index, 0);
            assert_eq!(
                res,
                Some(Indices {
                    system: 1,
                    cpi: vec![],
                    insert_into_queues: 3,
                    found_solana_system_program_instruction: true,
                    found_system: true,
                })
            );
        }
        {
            let program_ids = vec![
                ProgramId::Unknown,
                ProgramId::LightSystem,
                ProgramId::SolanaSystem,
                ProgramId::SolanaSystem,
                ProgramId::SolanaSystem,
                ProgramId::AccountCompression,
            ];
            let start_index = program_ids.len() - 1;
            let (res, last_index) = find_pattern(start_index, &program_ids);
            assert_eq!(last_index, 0);
            assert_eq!(
                res,
                Some(Indices {
                    system: 1,
                    cpi: vec![],
                    insert_into_queues: start_index,
                    found_solana_system_program_instruction: true,
                    found_system: true,
                })
            );
        }
        {
            let program_ids = vec![
                ProgramId::Unknown,
                ProgramId::LightSystem,
                ProgramId::SolanaSystem,
                ProgramId::Unknown,
                ProgramId::SolanaSystem,
                ProgramId::AccountCompression,
            ];
            let start_index = program_ids.len() - 1;
            let (res, last_index) = find_pattern(start_index, &program_ids);
            assert_eq!(last_index, 3);
            assert_eq!(res, None);
        }
        // With cpi context
        {
            let program_ids = vec![
                ProgramId::Unknown,
                ProgramId::LightSystem,
                ProgramId::Unknown,
                ProgramId::LightSystem,
                ProgramId::SolanaSystem,
                ProgramId::SolanaSystem,
                ProgramId::SolanaSystem,
                ProgramId::AccountCompression,
            ];
            let start_index = program_ids.len() - 1;
            let (res, last_index) = find_pattern(start_index, &program_ids);
            assert_eq!(last_index, 0);
            assert_eq!(
                res,
                Some(Indices {
                    system: 3,
                    cpi: vec![1],
                    insert_into_queues: start_index,
                    found_solana_system_program_instruction: true,
                    found_system: true,
                })
            );
            // Failing
            {
                let mut program_ids = program_ids;
                program_ids[5] = ProgramId::Unknown;
                let (res, last_index) = find_pattern(start_index, &program_ids);
                assert_eq!(last_index, 5);
                assert_eq!(res, None);
            }
        }
        // With cpi context
        {
            let program_ids = vec![
                ProgramId::Unknown,
                ProgramId::LightSystem,
                ProgramId::LightSystem,
                ProgramId::SolanaSystem,
                ProgramId::SolanaSystem,
                ProgramId::SolanaSystem,
                ProgramId::AccountCompression,
            ];
            let start_index = program_ids.len() - 1;
            let (res, last_index) = find_pattern(start_index, &program_ids);
            assert_eq!(last_index, 0);
            assert_eq!(
                res,
                Some(Indices {
                    system: 2,
                    cpi: vec![1],
                    insert_into_queues: start_index,
                    found_solana_system_program_instruction: true,
                    found_system: true,
                })
            );
            // Failing
            {
                let mut program_ids = program_ids;
                program_ids[4] = ProgramId::Unknown;
                let (res, last_index) = find_pattern(start_index, &program_ids);
                assert_eq!(last_index, 4);
                assert_eq!(res, None);
            }
        }
    }
}
