use borsh::BorshDeserialize;
use light_compressed_account::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
    },
    constants::{
        ACCOUNT_COMPRESSION_PROGRAM_ID, CREATE_CPI_CONTEXT_ACCOUNT, REGISTERED_PROGRAM_PDA,
        SYSTEM_PROGRAM_ID,
    },
    discriminators::*,
    instruction_data::{
        data::{InstructionDataInvoke, OutputCompressedAccountWithPackedContext},
        insert_into_queues::InsertIntoQueuesInstructionData,
        with_account_info::InstructionDataInvokeCpiWithAccountInfo,
        with_readonly::InstructionDataInvokeCpiWithReadOnly,
    },
    nullifier::create_nullifier,
    Pubkey,
};
use light_zero_copy::traits::ZeroCopyAt;

use super::{
    error::ParseIndexerEventError,
    event::{
        BatchNullifyContext, BatchPublicTransactionEvent, MerkleTreeSequenceNumber,
        MerkleTreeSequenceNumberV1, NewAddress, PublicTransactionEvent,
    },
};

#[derive(Debug, Clone, PartialEq)]
struct ExecutingSystemInstruction<'a> {
    output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    input_compressed_accounts: Vec<PackedCompressedAccountWithMerkleContext>,
    is_compress: bool,
    relay_fee: Option<u64>,
    compress_or_decompress_lamports: Option<u64>,
    execute_cpi_context: bool,
    accounts: &'a [Pubkey],
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Indices {
    pub system: usize,
    pub cpi: Vec<usize>,
    pub insert_into_queues: usize,
    pub found_solana_system_program_instruction: bool,
    pub found_system: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ProgramId {
    LightSystem,
    AccountCompression,
    SolanaSystem,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
struct AssociatedInstructions<'a> {
    pub executing_system_instruction: ExecutingSystemInstruction<'a>,
    pub cpi_context_outputs: Vec<OutputCompressedAccountWithPackedContext>,
    pub insert_into_queues_instruction: InsertIntoQueuesInstructionData<'a>,
    pub accounts: &'a [Pubkey],
}

/// We piece the event together from 2 instructions:
/// 1. light_system_program::{Invoke, InvokeCpi, InvokeCpiReadOnly} (one of the 3)
/// 2. account_compression::InsertIntoQueues
/// - We return new addresses in batched trees separately
///   because from the PublicTransactionEvent there
///   is no way to know which addresses are new and
///   for batched address trees we need to index the queue of new addresses
///   the tree&queue account only contains bloomfilters, roots and metadata.
///
/// Steps:
/// 0. Wrap program ids of instructions to filter but not change the pattern
///         system program cpi context creation ixs
///         insert into queue ixs not by the system program
///         instructions with less than 12 bytes ix data
/// 1. Find associated instructions by cpi pattern.
/// 2. Deserialize associated instructions.
/// 3. Create batched transaction events.
pub fn event_from_light_transaction(
    program_ids: &[Pubkey],
    instructions: &[Vec<u8>],
    accounts: Vec<Vec<Pubkey>>,
) -> Result<Option<Vec<BatchPublicTransactionEvent>>, ParseIndexerEventError> {
    // 0. Wrap program ids of instructions to filter but not change the pattern.
    let program_ids = wrap_program_ids(program_ids, instructions, &accounts);
    // 1. Find associated instructions by cpi pattern.
    let mut patterns = find_cpi_patterns(&program_ids);
    if patterns.is_empty() {
        return Ok(None);
    }
    // We searched from the last pattern to the first.
    //      -> reverse to be in order
    patterns.reverse();
    // 2. Deserialize associated instructions.
    let associated_instructions = patterns
        .iter()
        .map(|pattern| deserialize_associated_instructions(pattern, instructions, &accounts))
        .collect::<Result<Vec<_>, _>>()?;
    // 3. Create batched transaction events.
    let batched_transaction_events = associated_instructions
        .iter()
        .map(|associated_instruction| create_batched_transaction_event(associated_instruction))
        .collect::<Result<Vec<_>, _>>()?;

    // // Sanity checks:
    // // - this must not throw in production because indexing just works if all instructions are in the same transaction.
    // // - It's ok if someone misues the cpi context account but transaction data will not be available in photon.
    // // - if we would throw an error it would brick photon because we would not be able to index a transaction that changed queue state.
    // // - I could add extra data to the account compression cpi to make this impossible. -> this makes sense it is more robust.
    // // TODO: make debug
    // batched_transaction_events.iter().for_each(|event| {
    //     println!("event: {:?}", event);
    //     assert_eq!(
    //         event.event.input_compressed_account_hashes.len(),
    //         event.batch_input_accounts.len(),
    //         "Input hashes and input accounts length mismatch "
    //     );
    //     assert_eq!(
    //         event.event.output_compressed_account_hashes.len(),
    //         event.event.output_leaf_indices.len(),
    //         "Output hashes and output leaf indices length mismatch "
    //     );
    //     assert_eq!(
    //         event.event.output_compressed_account_hashes.len(),
    //         event.event.output_compressed_accounts.len(),
    //         "Output hashes and output compressed accounts length mismatch "
    //     );
    // });
    Ok(Some(batched_transaction_events))
}

fn deserialize_associated_instructions<'a>(
    indices: &Indices,
    instructions: &'a [Vec<u8>],
    accounts: &'a [Vec<Pubkey>],
) -> Result<AssociatedInstructions<'a>, ParseIndexerEventError> {
    let (insert_queues_instruction, cpi_context_outputs) = {
        let ix = &instructions[indices.insert_into_queues];
        if ix.len() < 12 {
            return Err(ParseIndexerEventError::InstructionDataTooSmall(
                ix.len(),
                12,
            ));
        }
        let discriminator: [u8; 8] = ix[0..8].try_into().unwrap();
        if discriminator == DISCRIMINATOR_INSERT_INTO_QUEUES {
            let (data, bytes) = InsertIntoQueuesInstructionData::zero_copy_at(&ix[12..])?;
            let cpi_context_outputs =
                Vec::<OutputCompressedAccountWithPackedContext>::deserialize(&mut &bytes[..])?;
            Ok((data, cpi_context_outputs))
        } else {
            Err(ParseIndexerEventError::DeserializeAccountLightSystemCpiInputsError)
        }
    }?;
    let exec_instruction =
        deserialize_instruction(&instructions[indices.system], &accounts[indices.system])?;
    Ok(AssociatedInstructions {
        executing_system_instruction: exec_instruction,
        cpi_context_outputs,
        insert_into_queues_instruction: insert_queues_instruction,
        // Remove signer and register program accounts.
        accounts: &accounts[indices.insert_into_queues][2..],
    })
}

/// Filter all system instructions which create cpi context accounts,
/// so that we can infer that a system program instruction is a light transaction.
/// Create new AssociatedInstructions when we find a system instruction
/// if next instruct is solana system program isntruction followed by insert into queues is executable instruction
/// else is cpi instruction
/// only push into vec if insert into queues instruction is found
fn find_cpi_patterns(program_ids: &[ProgramId]) -> Vec<Indices> {
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
            let (res, last_index) = find_cpi_pattern(last_index, program_ids);
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
fn find_cpi_pattern(start_index: usize, program_ids: &[ProgramId]) -> (Option<Indices>, usize) {
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

fn wrap_program_ids(
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

fn deserialize_instruction<'a>(
    instruction: &'a [u8],
    accounts: &'a [Pubkey],
) -> Result<ExecutingSystemInstruction<'a>, ParseIndexerEventError> {
    if instruction.len() < 12 {
        return Err(ParseIndexerEventError::InstructionDataTooSmall(
            instruction.len(),
            12,
        ));
    }
    let instruction_discriminator = instruction[0..8].try_into().unwrap();
    let instruction = instruction.split_at(8).1;
    match instruction_discriminator {
        // Cannot be exucted with cpi context -> executing tx
        DISCRIMINATOR_INVOKE => {
            if accounts.len() < 9 {
                return Err(ParseIndexerEventError::DeserializeSystemInstructionError);
            }
            let accounts = accounts.split_at(9).1;
            // Skips vec size bytes
            let data = InstructionDataInvoke::deserialize(&mut &instruction[4..])?;
            Ok(ExecutingSystemInstruction {
                output_compressed_accounts: data.output_compressed_accounts,
                input_compressed_accounts: data.input_compressed_accounts_with_merkle_context,
                is_compress: data.is_compress,
                relay_fee: data.relay_fee,
                compress_or_decompress_lamports: data.compress_or_decompress_lamports,
                execute_cpi_context: false,
                accounts,
            })
        }
        DISCRIMINATOR_INVOKE_CPI => {
            if accounts.len() < 11 {
                return Err(ParseIndexerEventError::DeserializeSystemInstructionError);
            }
            let accounts = accounts.split_at(11).1;
            let data = light_compressed_account::instruction_data::invoke_cpi::InstructionDataInvokeCpi::deserialize(
                &mut &instruction[4..],
            )?;
            Ok(ExecutingSystemInstruction {
                output_compressed_accounts: data.output_compressed_accounts,
                input_compressed_accounts: data.input_compressed_accounts_with_merkle_context,
                is_compress: data.is_compress,
                relay_fee: data.relay_fee,
                compress_or_decompress_lamports: data.compress_or_decompress_lamports,
                execute_cpi_context: data.cpi_context.is_some(),
                accounts,
            })
        }
        DISCRIMINATOR_INVOKE_CPI_WITH_READ_ONLY => {
            // Min len for a small instruction 3 accounts + 1 tree or queue
            // Fee payer + authority + registered program + account compression program + account compression authority
            if accounts.len() < 5 {
                return Err(ParseIndexerEventError::DeserializeSystemInstructionError);
            }
            let data: InstructionDataInvokeCpiWithReadOnly =
                InstructionDataInvokeCpiWithReadOnly::deserialize(&mut &instruction[..])?;
            let system_accounts_len = if data.mode == 0 {
                11
            } else {
                let mut len = 6; // fee_payer + authority + registered_program + account_compression_program + account_compression_authority + system_program
                if data.compress_or_decompress_lamports > 0 {
                    len += 1;
                }
                if !data.is_compress && data.compress_or_decompress_lamports > 0 {
                    len += 1;
                }
                if data.with_cpi_context {
                    len += 1;
                }
                len
            };

            let accounts = accounts.split_at(system_accounts_len).1;
            Ok(ExecutingSystemInstruction {
                output_compressed_accounts: data.output_compressed_accounts,
                input_compressed_accounts: data
                    .input_compressed_accounts
                    .iter()
                    .map(|x| {
                        x.into_packed_compressed_account_with_merkle_context(
                            data.invoking_program_id,
                        )
                    })
                    .collect::<Vec<_>>(),
                is_compress: data.is_compress && data.compress_or_decompress_lamports > 0,
                relay_fee: None,
                compress_or_decompress_lamports: if data.compress_or_decompress_lamports == 0 {
                    None
                } else {
                    Some(data.compress_or_decompress_lamports)
                },
                execute_cpi_context: data.with_cpi_context,
                accounts,
            })
        }
        INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION => {
            // Min len for a small instruction 4 accounts + 1 tree or queue
            // Fee payer + authority + registered program + account compression program + account compression authority
            if accounts.len() < 5 {
                return Err(ParseIndexerEventError::DeserializeSystemInstructionError);
            }
            let data: InstructionDataInvokeCpiWithAccountInfo =
                InstructionDataInvokeCpiWithAccountInfo::deserialize(&mut &instruction[..])?;
            let system_accounts_len = if data.mode == 0 {
                11
            } else {
                let mut len = 6; // fee_payer + authority + registered_program + account_compression_program + account_compression_authority + system_program
                if data.compress_or_decompress_lamports > 0 {
                    len += 1;
                }
                if !data.is_compress && data.compress_or_decompress_lamports > 0 {
                    len += 1;
                }
                if data.with_cpi_context {
                    len += 1;
                }
                len
            };
            let accounts = accounts.split_at(system_accounts_len).1;

            let instruction = ExecutingSystemInstruction {
                output_compressed_accounts: data
                    .account_infos
                    .iter()
                    .filter(|x| x.output.is_some())
                    .map(|x| {
                        let account = x.output.as_ref().unwrap();
                        OutputCompressedAccountWithPackedContext {
                            compressed_account: CompressedAccount {
                                address: x.address,
                                owner: data.invoking_program_id,
                                lamports: account.lamports,
                                data: Some(CompressedAccountData {
                                    discriminator: account.discriminator,
                                    data: account.data.clone(),
                                    data_hash: account.data_hash,
                                }),
                            },
                            merkle_tree_index: account.output_merkle_tree_index,
                        }
                    })
                    .collect::<Vec<_>>(),
                input_compressed_accounts: data
                    .account_infos
                    .iter()
                    .filter(|x| x.input.is_some())
                    .map(|x| {
                        let account = x.input.as_ref().unwrap();
                        PackedCompressedAccountWithMerkleContext {
                            compressed_account: CompressedAccount {
                                address: x.address,
                                owner: data.invoking_program_id,
                                lamports: account.lamports,
                                data: Some(CompressedAccountData {
                                    discriminator: account.discriminator,
                                    data: vec![],
                                    data_hash: account.data_hash,
                                }),
                            },
                            read_only: false,
                            root_index: account.root_index,
                            merkle_context: account.merkle_context,
                        }
                    })
                    .collect::<Vec<_>>(),
                is_compress: data.is_compress && data.compress_or_decompress_lamports > 0,
                relay_fee: None,
                compress_or_decompress_lamports: if data.compress_or_decompress_lamports == 0 {
                    None
                } else {
                    Some(data.compress_or_decompress_lamports)
                },
                execute_cpi_context: data.with_cpi_context,
                accounts,
            };

            Ok(instruction)
        }
        _ => Err(ParseIndexerEventError::DeserializeSystemInstructionError),
    }
}

fn create_batched_transaction_event(
    associated_instructions: &AssociatedInstructions,
) -> Result<BatchPublicTransactionEvent, ParseIndexerEventError> {
    let input_sequence_numbers = associated_instructions
        .insert_into_queues_instruction
        .input_sequence_numbers
        .iter()
        .map(From::from)
        .filter(|x: &MerkleTreeSequenceNumber| !(*x).is_empty())
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
            output_compressed_accounts: [
                associated_instructions.cpi_context_outputs.clone(),
                associated_instructions
                    .executing_system_instruction
                    .output_compressed_accounts
                    .clone(),
            ]
            .concat(),
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
                .map(From::from)
                .filter(|x: &MerkleTreeSequenceNumber| !(*x).is_empty())
                .map(|x| MerkleTreeSequenceNumberV1 {
                    seq: x.seq,
                    tree_pubkey: x.tree_pubkey,
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
            .insert_into_queues_instruction
            .addresses
            .iter()
            .map(|x| NewAddress {
                address: x.address,
                mt_pubkey: associated_instructions.accounts[x.tree_index as usize],
                queue_index: u64::MAX,
            })
            .collect::<Vec<_>>(),
        address_sequence_numbers: associated_instructions
            .insert_into_queues_instruction
            .address_sequence_numbers
            .iter()
            .map(From::from)
            .filter(|x: &MerkleTreeSequenceNumber| !(*x).is_empty())
            .collect::<Vec<MerkleTreeSequenceNumber>>(),
        batch_input_accounts: associated_instructions
            .insert_into_queues_instruction
            .nullifiers
            .iter()
            .filter(|x| {
                input_sequence_numbers.iter().any(|y| {
                    y.tree_pubkey == associated_instructions.accounts[x.tree_index as usize]
                })
            })
            .map(|n| {
                Ok(BatchNullifyContext {
                    tx_hash: associated_instructions
                        .insert_into_queues_instruction
                        .tx_hash,
                    account_hash: n.account_hash,
                    nullifier: {
                        // The nullifier is computed inside the account compression program.
                        // -> it is not part of the cpi system to account compression program that we index.
                        // -> we need to compute the nullifier here.
                        create_nullifier(
                            &n.account_hash,
                            n.leaf_index.into(),
                            &associated_instructions
                                .insert_into_queues_instruction
                                .tx_hash,
                        )?
                    },
                    nullifier_queue_index: u64::MAX,
                })
            })
            .collect::<Result<Vec<_>, ParseIndexerEventError>>()?,
        input_sequence_numbers,
    };

    let nullifier_queue_indices = create_nullifier_queue_indices(
        associated_instructions,
        batched_transaction_event.batch_input_accounts.len(),
    );

    batched_transaction_event
        .batch_input_accounts
        .iter_mut()
        .zip(nullifier_queue_indices.iter())
        .for_each(|(context, index)| {
            context.nullifier_queue_index = *index;
        });

    let address_queue_indices = create_address_queue_indices(
        associated_instructions,
        batched_transaction_event.new_addresses.len(),
    );

    batched_transaction_event
        .new_addresses
        .iter_mut()
        .zip(address_queue_indices.iter())
        .for_each(|(context, index)| {
            context.queue_index = *index;
        });

    Ok(batched_transaction_event)
}

fn create_nullifier_queue_indices(
    associated_instructions: &AssociatedInstructions,
    len: usize,
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
    let mut nullifier_queue_indices = vec![u64::MAX; len];
    let mut internal_input_sequence_numbers = associated_instructions
        .insert_into_queues_instruction
        .input_sequence_numbers
        .to_vec();
    // For every sequence number:
    // 1. Find every input compressed account
    // 2. assign sequence number as nullifier queue index
    // 3. increment the sequence number
    internal_input_sequence_numbers.iter_mut().for_each(|seq| {
        for (i, merkle_tree_pubkey) in input_merkle_tree_pubkeys.iter().enumerate() {
            if *merkle_tree_pubkey == seq.tree_pubkey {
                nullifier_queue_indices[i] = seq.seq.into();
                seq.seq += 1;
            }
        }
    });
    nullifier_queue_indices
}

fn create_address_queue_indices(
    associated_instructions: &AssociatedInstructions,
    len: usize,
) -> Vec<u64> {
    let address_merkle_tree_pubkeys = associated_instructions
        .insert_into_queues_instruction
        .addresses
        .iter()
        .map(|x| associated_instructions.accounts[x.tree_index as usize])
        .collect::<Vec<_>>();
    let mut address_queue_indices = vec![u64::MAX; len];
    let mut internal_address_sequence_numbers = associated_instructions
        .insert_into_queues_instruction
        .address_sequence_numbers
        .to_vec();
    internal_address_sequence_numbers
        .iter_mut()
        .for_each(|seq| {
            for (i, merkle_tree_pubkey) in address_merkle_tree_pubkeys.iter().enumerate() {
                if *merkle_tree_pubkey == seq.tree_pubkey {
                    address_queue_indices[i] = seq.seq.into();
                    seq.seq += 1;
                }
            }
        });
    address_queue_indices
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
            let vec = find_cpi_patterns(&program_ids);
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
            let vec = find_cpi_patterns(&program_ids);
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

            let vec = find_cpi_patterns(&program_ids);

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
            let vec = find_cpi_patterns(&program_ids);
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
                let vec = find_cpi_patterns(&program_ids);
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
                let vec = find_cpi_patterns(&program_ids);
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
            let (res, last_index) = find_cpi_pattern(3, &program_ids);
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
            let (res, last_index) = find_cpi_pattern(start_index, &program_ids);
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
            let (res, last_index) = find_cpi_pattern(start_index, &program_ids);
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
            let (res, last_index) = find_cpi_pattern(start_index, &program_ids);
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
                let (res, last_index) = find_cpi_pattern(start_index, &program_ids);
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
            let (res, last_index) = find_cpi_pattern(start_index, &program_ids);
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
                let (res, last_index) = find_cpi_pattern(start_index, &program_ids);
                assert_eq!(last_index, 4);
                assert_eq!(res, None);
            }
        }
    }
}
