use borsh::BorshDeserialize;
use light_compressed_account::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
    },
    constants::{
        ACCOUNT_COMPRESSION_PROGRAM_ID, CREATE_CPI_CONTEXT_ACCOUNT, LIGHT_REGISTRY_PROGRAM_ID,
        LIGHT_SYSTEM_PROGRAM_ID, REGISTERED_PROGRAM_PDA,
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
use light_token_interface::{
    instructions::{
        extensions::ExtensionInstructionData, transfer2::CompressedTokenInstructionDataTransfer2,
    },
    LIGHT_TOKEN_PROGRAM_ID, TRANSFER2,
};
use light_zero_copy::traits::ZeroCopyAt;

use super::{
    error::ParseIndexerEventError,
    event::{
        AtaOwnerInfo, BatchNullifyContext, BatchPublicTransactionEvent, MerkleTreeSequenceNumber,
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
    /// Index of the token program instruction (if present, only when called from registry)
    pub token: Option<usize>,
    /// Whether registry program was found in the CPI chain (required for token instruction tracking)
    pub found_registry: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ProgramId {
    LightSystem,
    AccountCompression,
    SolanaSystem,
    LightToken,
    Registry,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
struct AssociatedInstructions<'a> {
    pub executing_system_instruction: ExecutingSystemInstruction<'a>,
    pub cpi_context_outputs: Vec<OutputCompressedAccountWithPackedContext>,
    pub insert_into_queues_instruction: InsertIntoQueuesInstructionData<'a>,
    pub accounts: &'a [Pubkey],
    /// Token instruction data and accounts for ATA owner extraction
    pub token_instruction: Option<TokenInstructionData<'a>>,
}

/// Parsed token instruction data for extracting ATA owner info
#[derive(Debug, Clone, PartialEq)]
struct TokenInstructionData<'a> {
    /// Raw instruction data
    pub data: &'a [u8],
    /// Accounts for this instruction
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

    // Get token instruction data if present
    let token_instruction = indices.token.map(|token_idx| TokenInstructionData {
        data: &instructions[token_idx],
        accounts: &accounts[token_idx],
    });

    Ok(AssociatedInstructions {
        executing_system_instruction: exec_instruction,
        cpi_context_outputs,
        insert_into_queues_instruction: insert_queues_instruction,
        // Remove signer and register program accounts.
        accounts: &accounts[indices.insert_into_queues][2..],
        token_instruction,
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
    // Track tentative token index - will only be confirmed if registry is found
    let mut tentative_token: Option<usize> = None;

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
        } else if index_account.found_system && matches!(program_id, ProgramId::LightToken) {
            // Token program Transfer2 instruction in the CPI chain.
            // Track tentatively - will only be confirmed if registry is found later.
            // Only track the first one (closest to system instruction).
            if tentative_token.is_none() {
                tentative_token = Some(index);
            }
        } else if index_account.found_system && matches!(program_id, ProgramId::Registry) {
            // Registry program instruction - confirms token tracking for ATA owner extraction.
            // Since we search backwards, registry is found after token in the search order,
            // but registry is the outer caller in the actual CPI chain.
            index_account.found_registry = true;
            // Confirm the tentative token index now that we found registry
            if index_account.token.is_none() {
                index_account.token = tentative_token;
            }
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
        } else if program_id == &LIGHT_SYSTEM_PROGRAM_ID {
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
        } else if program_id == &Pubkey::from(LIGHT_TOKEN_PROGRAM_ID) {
            // Token program Transfer2 instruction
            if !instruction.is_empty() && instruction[0] == TRANSFER2 {
                vec.push(ProgramId::LightToken);
            } else {
                vec.push(ProgramId::Unknown);
            }
        } else if program_id == &LIGHT_REGISTRY_PROGRAM_ID {
            vec.push(ProgramId::Registry);
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

/// Extract ATA owner info from token instruction's out_tlv.
/// Returns a Vec of (output_index, wallet_owner) for ATAs.
fn extract_ata_owners(token_instruction: &TokenInstructionData) -> Vec<AtaOwnerInfo> {
    let mut ata_owners = Vec::new();

    // Token instruction format: [discriminator (1 byte)] [serialized data]
    if token_instruction.data.is_empty() || token_instruction.data[0] != TRANSFER2 {
        return ata_owners;
    }

    // Skip discriminator byte and deserialize using borsh
    let data = &token_instruction.data[1..];
    let Ok(transfer_data) = CompressedTokenInstructionDataTransfer2::deserialize(&mut &data[..])
    else {
        return ata_owners;
    };

    // Check if there's out_tlv data
    let Some(out_tlv) = transfer_data.out_tlv.as_ref() else {
        return ata_owners;
    };

    // Iterate over output TLV entries (one per output token account)
    for (output_index, tlv_extensions) in out_tlv.iter().enumerate() {
        // Look for CompressedOnly extension with is_ata=true
        for ext in tlv_extensions.iter() {
            if let ExtensionInstructionData::CompressedOnly(compressed_only) = ext {
                if compressed_only.is_ata {
                    // Get wallet owner from packed_accounts using owner_index.
                    // owner_index is an index into packed_accounts, which starts at position 7
                    // in the Transfer2 accounts array (after the 7 system accounts).
                    const TRANSFER2_PACKED_ACCOUNTS_OFFSET: usize = 7;
                    let owner_idx =
                        compressed_only.owner_index as usize + TRANSFER2_PACKED_ACCOUNTS_OFFSET;
                    if owner_idx < token_instruction.accounts.len() {
                        ata_owners.push(AtaOwnerInfo {
                            output_index: output_index as u8,
                            wallet_owner: token_instruction.accounts[owner_idx],
                        });
                    }
                }
            }
        }
    }

    ata_owners
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
            ata_owners: associated_instructions
                .token_instruction
                .as_ref()
                .map(extract_ata_owners)
                .unwrap_or_default(),
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
                token: None,
                found_registry: false,
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
                token: None,
                found_registry: false,
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
                    token: None,
                    found_registry: false,
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
                    token: None,
                    found_registry: false,
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
                        token: None,
                        found_registry: false,
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
                        token: None,
                        found_registry: false,
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
                    token: None,
                    found_registry: false,
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
                    token: None,
                    found_registry: false,
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
                    token: None,
                    found_registry: false,
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
                    token: None,
                    found_registry: false,
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

    // ==========================================================================
    // Tests for extract_ata_owners
    // ==========================================================================

    use borsh::BorshSerialize;
    use light_token_interface::instructions::{
        extensions::CompressedOnlyExtensionInstructionData,
        transfer2::{CompressedTokenInstructionDataTransfer2, MultiTokenTransferOutputData},
    };

    /// Helper to create valid Transfer2 instruction data with ATA extensions
    fn create_transfer2_with_ata(owner_index: u8, is_ata: bool) -> Vec<u8> {
        let transfer_data = CompressedTokenInstructionDataTransfer2 {
            with_transaction_hash: false,
            with_lamports_change_account_merkle_tree_index: false,
            lamports_change_account_merkle_tree_index: 0,
            lamports_change_account_owner_index: 0,
            output_queue: 0,
            max_top_up: 0,
            cpi_context: None,
            compressions: None,
            proof: None,
            in_token_data: vec![],
            out_token_data: vec![MultiTokenTransferOutputData {
                owner: owner_index,
                amount: 1000,
                has_delegate: false,
                delegate: 0,
                mint: 0,
                version: 3,
            }],
            in_lamports: None,
            out_lamports: None,
            in_tlv: None,
            out_tlv: Some(vec![vec![ExtensionInstructionData::CompressedOnly(
                CompressedOnlyExtensionInstructionData {
                    delegated_amount: 0,
                    withheld_transfer_fee: 0,
                    is_frozen: false,
                    compression_index: 0,
                    is_ata,
                    bump: 255,
                    owner_index,
                },
            )]]),
        };
        let mut data = vec![TRANSFER2]; // discriminator
        data.extend(transfer_data.try_to_vec().unwrap());
        data
    }

    #[test]
    fn test_extract_ata_owners_empty_data() {
        let token_instruction = TokenInstructionData {
            data: &[],
            accounts: &[],
        };
        let result = extract_ata_owners(&token_instruction);
        assert!(result.is_empty(), "Empty data should return empty vec");
    }

    #[test]
    fn test_extract_ata_owners_wrong_discriminator() {
        let token_instruction = TokenInstructionData {
            data: &[0xFF, 0x00, 0x00], // Wrong discriminator
            accounts: &[],
        };
        let result = extract_ata_owners(&token_instruction);
        assert!(
            result.is_empty(),
            "Wrong discriminator should return empty vec"
        );
    }

    #[test]
    fn test_extract_ata_owners_only_discriminator() {
        let token_instruction = TokenInstructionData {
            data: &[TRANSFER2], // Only discriminator, no data
            accounts: &[],
        };
        let result = extract_ata_owners(&token_instruction);
        assert!(
            result.is_empty(),
            "Only discriminator should return empty vec (deserialization fails)"
        );
    }

    #[test]
    fn test_extract_ata_owners_malformed_data() {
        // Random garbage after discriminator
        let token_instruction = TokenInstructionData {
            data: &[TRANSFER2, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            accounts: &[],
        };
        let result = extract_ata_owners(&token_instruction);
        assert!(
            result.is_empty(),
            "Malformed data should return empty vec (deserialization fails)"
        );
    }

    #[test]
    fn test_extract_ata_owners_valid_non_ata() {
        let data = create_transfer2_with_ata(0, false); // is_ata = false
        let accounts = vec![Pubkey::default(); 10];
        let token_instruction = TokenInstructionData {
            data: &data,
            accounts: &accounts,
        };
        let result = extract_ata_owners(&token_instruction);
        assert!(
            result.is_empty(),
            "Non-ATA accounts should not produce ATA owner info"
        );
    }

    #[test]
    fn test_extract_ata_owners_valid_ata() {
        let owner_index = 2u8; // Index into packed_accounts
        let data = create_transfer2_with_ata(owner_index, true);

        // Create accounts array: 7 system accounts + packed_accounts
        // owner_index=2 means packed_accounts[2] = accounts[7+2] = accounts[9]
        let mut accounts = vec![Pubkey::default(); 10];
        let expected_owner = Pubkey::new_from_array([42u8; 32]);
        accounts[7 + owner_index as usize] = expected_owner;

        let token_instruction = TokenInstructionData {
            data: &data,
            accounts: &accounts,
        };
        let result = extract_ata_owners(&token_instruction);
        assert_eq!(result.len(), 1, "Should extract one ATA owner");
        assert_eq!(result[0].output_index, 0);
        assert_eq!(result[0].wallet_owner, expected_owner);
    }

    #[test]
    fn test_extract_ata_owners_owner_index_out_of_bounds() {
        let owner_index = 100u8; // Way beyond accounts array
        let data = create_transfer2_with_ata(owner_index, true);

        // Only 10 accounts, but owner_index + 7 = 107
        let accounts = vec![Pubkey::default(); 10];

        let token_instruction = TokenInstructionData {
            data: &data,
            accounts: &accounts,
        };
        let result = extract_ata_owners(&token_instruction);
        assert!(
            result.is_empty(),
            "Out of bounds owner_index should be safely skipped"
        );
    }

    #[test]
    fn test_extract_ata_owners_boundary_owner_index() {
        // Test with owner_index at the boundary
        let owner_index = 2u8;
        let data = create_transfer2_with_ata(owner_index, true);

        // Create exactly enough accounts: 7 system + 3 packed (indices 0, 1, 2)
        // owner_index=2 needs accounts[9], so we need 10 accounts total
        let mut accounts = vec![Pubkey::default(); 10];
        let expected_owner = Pubkey::new_from_array([99u8; 32]);
        accounts[9] = expected_owner;

        let token_instruction = TokenInstructionData {
            data: &data,
            accounts: &accounts,
        };
        let result = extract_ata_owners(&token_instruction);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].wallet_owner, expected_owner);

        // Now with one less account - should be skipped
        let accounts_short = vec![Pubkey::default(); 9];
        let token_instruction_short = TokenInstructionData {
            data: &data,
            accounts: &accounts_short,
        };
        let result_short = extract_ata_owners(&token_instruction_short);
        assert!(
            result_short.is_empty(),
            "Boundary case with insufficient accounts should be skipped"
        );
    }

    #[test]
    fn test_extract_ata_owners_max_owner_index() {
        // Test with u8::MAX owner_index
        let owner_index = u8::MAX;
        let data = create_transfer2_with_ata(owner_index, true);

        // 255 + 7 = 262, need 263 accounts
        let accounts = vec![Pubkey::default(); 10]; // Way too few

        let token_instruction = TokenInstructionData {
            data: &data,
            accounts: &accounts,
        };
        let result = extract_ata_owners(&token_instruction);
        assert!(
            result.is_empty(),
            "u8::MAX owner_index with small accounts array should be safely skipped"
        );
    }

    // ==========================================================================
    // Tests for wrap_program_ids with LightToken and Registry
    // ==========================================================================

    #[test]
    fn test_wrap_program_ids_light_token_transfer2() {
        let program_ids = vec![Pubkey::from(LIGHT_TOKEN_PROGRAM_ID)];
        let mut instruction_data = vec![0u8; 12]; // Minimum size
        instruction_data[0] = TRANSFER2;
        let instructions = vec![instruction_data];
        let accounts = vec![vec![]];

        let result = wrap_program_ids(&program_ids, &instructions, &accounts);
        assert_eq!(result, vec![ProgramId::LightToken]);
    }

    #[test]
    fn test_wrap_program_ids_light_token_non_transfer2() {
        let program_ids = vec![Pubkey::from(LIGHT_TOKEN_PROGRAM_ID)];
        let mut instruction_data = vec![0u8; 12];
        instruction_data[0] = 0xFF; // Not TRANSFER2
        let instructions = vec![instruction_data];
        let accounts = vec![vec![]];

        let result = wrap_program_ids(&program_ids, &instructions, &accounts);
        assert_eq!(result, vec![ProgramId::Unknown]);
    }

    #[test]
    fn test_wrap_program_ids_registry() {
        let program_ids = vec![Pubkey::from(LIGHT_REGISTRY_PROGRAM_ID)];
        let instruction_data = vec![0u8; 12];
        let instructions = vec![instruction_data];
        let accounts = vec![vec![]];

        let result = wrap_program_ids(&program_ids, &instructions, &accounts);
        assert_eq!(result, vec![ProgramId::Registry]);
    }

    #[test]
    fn test_wrap_program_ids_instruction_too_small() {
        let program_ids = vec![Pubkey::from(LIGHT_TOKEN_PROGRAM_ID)];
        let instruction_data = vec![TRANSFER2; 5]; // Less than 12 bytes
        let instructions = vec![instruction_data];
        let accounts = vec![vec![]];

        let result = wrap_program_ids(&program_ids, &instructions, &accounts);
        assert_eq!(
            result,
            vec![ProgramId::Unknown],
            "Instructions smaller than 12 bytes should be Unknown"
        );
    }

    // ==========================================================================
    // Tests for find_cpi_pattern with Registry and Token tracking
    // ==========================================================================

    #[test]
    fn test_find_cpi_pattern_with_registry_and_token() {
        // Pattern: Registry -> Token -> LightSystem -> SolanaSystem -> AccountCompression
        let program_ids = vec![
            ProgramId::Registry,
            ProgramId::LightToken,
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let (res, _) = find_cpi_pattern(4, &program_ids);
        assert!(res.is_some());
        let indices = res.unwrap();
        assert!(indices.found_registry, "Should find registry");
        assert_eq!(
            indices.token,
            Some(1),
            "Should track token when registry is present"
        );
        assert_eq!(indices.system, 2);
    }

    #[test]
    fn test_find_cpi_pattern_token_without_registry() {
        // Pattern: Token -> LightSystem -> SolanaSystem -> AccountCompression
        // No registry means token should NOT be tracked
        let program_ids = vec![
            ProgramId::LightToken,
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let (res, _) = find_cpi_pattern(3, &program_ids);
        assert!(res.is_some());
        let indices = res.unwrap();
        assert!(!indices.found_registry, "Should not find registry");
        assert_eq!(
            indices.token, None,
            "Should NOT track token without registry"
        );
    }

    #[test]
    fn test_find_cpi_pattern_registry_without_token() {
        // Registry can call LightSystem directly without Token
        // Pattern: Registry -> LightSystem -> SolanaSystem -> AccountCompression
        let program_ids = vec![
            ProgramId::Registry,
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let (res, _) = find_cpi_pattern(3, &program_ids);
        assert!(res.is_some());
        let indices = res.unwrap();
        assert!(indices.found_registry, "Should find registry");
        assert_eq!(indices.token, None, "No token instruction in this pattern");
    }

    #[test]
    fn test_find_cpi_pattern_multiple_tokens_only_first_tracked() {
        // Only the first (closest to system) token should be tracked
        // Pattern: Registry -> Token1 -> Token2 -> LightSystem -> SolanaSystem -> AccountCompression
        let program_ids = vec![
            ProgramId::Registry,
            ProgramId::LightToken, // Token1 - outer
            ProgramId::LightToken, // Token2 - inner, should be tracked
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let (res, _) = find_cpi_pattern(5, &program_ids);
        assert!(res.is_some());
        let indices = res.unwrap();
        assert!(indices.found_registry);
        // The inner token (index 2) should be tracked as it's first when searching backwards
        assert_eq!(
            indices.token,
            Some(2),
            "Should track the token closest to system instruction"
        );
    }

    // ==========================================================================
    // Additional ATA and Program ID filtering edge case tests
    // ==========================================================================

    #[test]
    fn test_find_cpi_pattern_token_after_account_compression_not_tracked() {
        // Token appearing after AccountCompression should not be part of this pattern
        // Pattern: Registry -> LightSystem -> SolanaSystem -> AccountCompression -> Token
        let program_ids = vec![
            ProgramId::Registry,
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
            ProgramId::LightToken, // After AccountCompression - not part of this pattern
        ];
        let (res, _) = find_cpi_pattern(3, &program_ids);
        assert!(res.is_some());
        let indices = res.unwrap();
        assert!(indices.found_registry);
        assert_eq!(
            indices.token, None,
            "Token after AccountCompression should not be tracked in this pattern"
        );
    }

    #[test]
    fn test_find_cpi_pattern_registry_after_account_compression_not_found() {
        // Registry appearing after AccountCompression should not validate token tracking
        // Pattern: Token -> LightSystem -> SolanaSystem -> AccountCompression -> Registry
        let program_ids = vec![
            ProgramId::LightToken,
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
            ProgramId::Registry, // After AccountCompression - not part of this pattern
        ];
        let (res, _) = find_cpi_pattern(3, &program_ids);
        assert!(res.is_some());
        let indices = res.unwrap();
        assert!(
            !indices.found_registry,
            "Registry after AccountCompression should not be found"
        );
        assert_eq!(
            indices.token, None,
            "Token should not be tracked without registry before AccountCompression"
        );
    }

    #[test]
    fn test_find_cpi_pattern_token_between_unknown_programs() {
        // Token surrounded by Unknown programs, with Registry present
        // Pattern: Registry -> Unknown -> Token -> Unknown -> LightSystem -> SolanaSystem -> AccountCompression
        let program_ids = vec![
            ProgramId::Registry,
            ProgramId::Unknown,
            ProgramId::LightToken,
            ProgramId::Unknown,
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let (res, _) = find_cpi_pattern(6, &program_ids);
        assert!(res.is_some());
        let indices = res.unwrap();
        assert!(indices.found_registry);
        assert_eq!(
            indices.token,
            Some(2),
            "Token should be tracked even with Unknown programs around it"
        );
    }

    #[test]
    fn test_find_cpi_pattern_empty_program_ids() {
        let program_ids: Vec<ProgramId> = vec![];
        let patterns = find_cpi_patterns(&program_ids);
        assert!(
            patterns.is_empty(),
            "Empty program IDs should return no patterns"
        );
    }

    #[test]
    fn test_find_cpi_pattern_single_account_compression() {
        let program_ids = vec![ProgramId::AccountCompression];
        let (res, _) = find_cpi_pattern(0, &program_ids);
        assert!(
            res.is_none(),
            "Single AccountCompression without system should not match"
        );
    }

    #[test]
    fn test_find_cpi_pattern_registry_token_no_system() {
        // Registry and Token without LightSystem - invalid pattern
        let program_ids = vec![
            ProgramId::Registry,
            ProgramId::LightToken,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let (res, _) = find_cpi_pattern(3, &program_ids);
        assert!(
            res.is_none(),
            "Pattern without LightSystem should not match"
        );
    }

    #[test]
    fn test_find_cpi_pattern_token_at_position_zero_not_tracked() {
        // Token at position 0 (outermost in CPI chain) - this is NOT a valid real-world pattern.
        // In the actual protocol, Registry is always the outermost caller (Registry -> Token -> LightSystem).
        // Pattern: Token -> Registry -> LightSystem -> SolanaSystem -> AccountCompression
        //
        // When searching backwards, we encounter Registry (index 1) BEFORE Token (index 0).
        // At the point we find Registry, tentative_token is still None, so we don't confirm a token.
        // Then we find Token at index 0, but Registry has already been processed.
        //
        // This behavior is CORRECT because Token being outermost is invalid - Registry must be outer.
        let program_ids = vec![
            ProgramId::LightToken, // Position 0 - invalid as outermost
            ProgramId::Registry,   // Position 1
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let (res, _) = find_cpi_pattern(4, &program_ids);
        assert!(res.is_some());
        let indices = res.unwrap();
        assert!(indices.found_registry);
        // Token at position 0 is NOT tracked because it appears AFTER Registry in backwards search.
        // This is correct behavior - Token must be between Registry and LightSystem.
        assert_eq!(
            indices.token, None,
            "Token at position 0 (before Registry in array) should NOT be tracked - invalid CPI order"
        );
    }

    #[test]
    fn test_find_cpi_pattern_multiple_registries() {
        // Multiple Registry programs - behavior verification
        // Pattern: Registry -> Registry -> Token -> LightSystem -> SolanaSystem -> AccountCompression
        let program_ids = vec![
            ProgramId::Registry, // First Registry
            ProgramId::Registry, // Second Registry
            ProgramId::LightToken,
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let (res, _) = find_cpi_pattern(5, &program_ids);
        assert!(res.is_some());
        let indices = res.unwrap();
        assert!(indices.found_registry, "Should find at least one registry");
        assert_eq!(
            indices.token,
            Some(2),
            "Token should be tracked with registry present"
        );
    }

    #[test]
    fn test_find_cpi_pattern_token_before_system_instruction() {
        // Token appearing before finding system instruction in backwards search
        // Pattern: LightSystem -> SolanaSystem -> Token -> AccountCompression
        // When searching backwards from AccountCompression, we find Token before system
        let program_ids = vec![
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::LightToken, // Between SolanaSystem and AccountCompression
            ProgramId::AccountCompression,
        ];
        let (res, _) = find_cpi_pattern(3, &program_ids);
        // This should fail because we need SolanaSystem right before AccountCompression
        assert!(
            res.is_none(),
            "Token breaking the SolanaSystem -> AccountCompression chain should fail"
        );
    }

    #[test]
    fn test_find_cpi_pattern_registry_between_system_and_solana_system() {
        // Registry between LightSystem and SolanaSystem
        // Pattern: Registry -> LightSystem -> Registry -> SolanaSystem -> AccountCompression
        let program_ids = vec![
            ProgramId::Registry,
            ProgramId::LightSystem,
            ProgramId::Registry, // Between LightSystem and SolanaSystem
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let (res, _) = find_cpi_pattern(4, &program_ids);
        // Registry between should break the pattern
        assert!(
            res.is_none(),
            "Registry between LightSystem and SolanaSystem should break pattern"
        );
    }

    // ==========================================================================
    // Additional extract_ata_owners edge case tests
    // ==========================================================================

    /// Helper to create Transfer2 instruction data with multiple outputs
    fn create_transfer2_with_multiple_outputs(
        outputs: Vec<(u8, bool)>, // (owner_index, is_ata)
    ) -> Vec<u8> {
        let out_token_data: Vec<MultiTokenTransferOutputData> = outputs
            .iter()
            .map(|(owner_index, _)| MultiTokenTransferOutputData {
                owner: *owner_index,
                amount: 1000,
                has_delegate: false,
                delegate: 0,
                mint: 0,
                version: 3,
            })
            .collect();

        let out_tlv: Vec<Vec<ExtensionInstructionData>> = outputs
            .iter()
            .map(|(owner_index, is_ata)| {
                vec![ExtensionInstructionData::CompressedOnly(
                    CompressedOnlyExtensionInstructionData {
                        delegated_amount: 0,
                        withheld_transfer_fee: 0,
                        is_frozen: false,
                        compression_index: 0,
                        is_ata: *is_ata,
                        bump: 255,
                        owner_index: *owner_index,
                    },
                )]
            })
            .collect();

        let transfer_data = CompressedTokenInstructionDataTransfer2 {
            with_transaction_hash: false,
            with_lamports_change_account_merkle_tree_index: false,
            lamports_change_account_merkle_tree_index: 0,
            lamports_change_account_owner_index: 0,
            output_queue: 0,
            max_top_up: 0,
            cpi_context: None,
            compressions: None,
            proof: None,
            in_token_data: vec![],
            out_token_data,
            in_lamports: None,
            out_lamports: None,
            in_tlv: None,
            out_tlv: Some(out_tlv),
        };
        let mut data = vec![TRANSFER2];
        data.extend(transfer_data.try_to_vec().unwrap());
        data
    }

    #[test]
    fn test_extract_ata_owners_multiple_outputs_all_ata() {
        // Multiple outputs, all are ATAs
        let data = create_transfer2_with_multiple_outputs(vec![
            (0, true), // output 0: ATA with owner at packed_accounts[0]
            (1, true), // output 1: ATA with owner at packed_accounts[1]
            (2, true), // output 2: ATA with owner at packed_accounts[2]
        ]);

        let mut accounts = vec![Pubkey::default(); 12]; // 7 system + 5 packed
        let owner0 = Pubkey::new_from_array([10u8; 32]);
        let owner1 = Pubkey::new_from_array([11u8; 32]);
        let owner2 = Pubkey::new_from_array([12u8; 32]);
        accounts[7] = owner0;
        accounts[8] = owner1;
        accounts[9] = owner2;

        let token_instruction = TokenInstructionData {
            data: &data,
            accounts: &accounts,
        };
        let result = extract_ata_owners(&token_instruction);

        assert_eq!(result.len(), 3, "Should extract 3 ATA owners");
        assert_eq!(result[0].output_index, 0);
        assert_eq!(result[0].wallet_owner, owner0);
        assert_eq!(result[1].output_index, 1);
        assert_eq!(result[1].wallet_owner, owner1);
        assert_eq!(result[2].output_index, 2);
        assert_eq!(result[2].wallet_owner, owner2);
    }

    #[test]
    fn test_extract_ata_owners_multiple_outputs_mixed() {
        // Mixed: some ATA, some not
        let data = create_transfer2_with_multiple_outputs(vec![
            (0, false), // output 0: NOT an ATA
            (1, true),  // output 1: ATA
            (2, false), // output 2: NOT an ATA
            (3, true),  // output 3: ATA
        ]);

        let mut accounts = vec![Pubkey::default(); 12];
        let owner1 = Pubkey::new_from_array([21u8; 32]);
        let owner3 = Pubkey::new_from_array([23u8; 32]);
        accounts[8] = owner1; // packed_accounts[1]
        accounts[10] = owner3; // packed_accounts[3]

        let token_instruction = TokenInstructionData {
            data: &data,
            accounts: &accounts,
        };
        let result = extract_ata_owners(&token_instruction);

        assert_eq!(result.len(), 2, "Should only extract ATA outputs");
        assert_eq!(result[0].output_index, 1);
        assert_eq!(result[0].wallet_owner, owner1);
        assert_eq!(result[1].output_index, 3);
        assert_eq!(result[1].wallet_owner, owner3);
    }

    #[test]
    fn test_extract_ata_owners_multiple_outputs_none_ata() {
        // All outputs are non-ATA
        let data = create_transfer2_with_multiple_outputs(vec![(0, false), (1, false), (2, false)]);

        let accounts = vec![Pubkey::default(); 12];
        let token_instruction = TokenInstructionData {
            data: &data,
            accounts: &accounts,
        };
        let result = extract_ata_owners(&token_instruction);

        assert!(
            result.is_empty(),
            "Should not extract any owners when no ATAs"
        );
    }

    #[test]
    fn test_extract_ata_owners_same_owner_multiple_atas() {
        // Multiple ATAs pointing to the same owner (same owner_index)
        let data = create_transfer2_with_multiple_outputs(vec![
            (0, true), // output 0: ATA with owner at packed_accounts[0]
            (0, true), // output 1: ATA with SAME owner
            (0, true), // output 2: ATA with SAME owner
        ]);

        let mut accounts = vec![Pubkey::default(); 10];
        let shared_owner = Pubkey::new_from_array([77u8; 32]);
        accounts[7] = shared_owner;

        let token_instruction = TokenInstructionData {
            data: &data,
            accounts: &accounts,
        };
        let result = extract_ata_owners(&token_instruction);

        assert_eq!(result.len(), 3, "Should extract all 3 ATA entries");
        assert!(
            result.iter().all(|r| r.wallet_owner == shared_owner),
            "All should have the same owner"
        );
        assert_eq!(result[0].output_index, 0);
        assert_eq!(result[1].output_index, 1);
        assert_eq!(result[2].output_index, 2);
    }

    #[test]
    fn test_extract_ata_owners_partial_out_of_bounds() {
        // Some outputs have valid owner_index, some are out of bounds
        let data = create_transfer2_with_multiple_outputs(vec![
            (0, true),   // output 0: Valid owner_index
            (100, true), // output 1: Out of bounds
            (1, true),   // output 2: Valid owner_index
        ]);

        let mut accounts = vec![Pubkey::default(); 10];
        let owner0 = Pubkey::new_from_array([30u8; 32]);
        let owner1 = Pubkey::new_from_array([31u8; 32]);
        accounts[7] = owner0;
        accounts[8] = owner1;

        let token_instruction = TokenInstructionData {
            data: &data,
            accounts: &accounts,
        };
        let result = extract_ata_owners(&token_instruction);

        assert_eq!(result.len(), 2, "Should only extract valid owner indices");
        assert_eq!(result[0].output_index, 0);
        assert_eq!(result[0].wallet_owner, owner0);
        assert_eq!(result[1].output_index, 2);
        assert_eq!(result[1].wallet_owner, owner1);
    }

    #[test]
    fn test_extract_ata_owners_zero_packed_accounts() {
        // Edge case: exactly 7 accounts (no packed_accounts at all)
        let data = create_transfer2_with_ata(0, true); // Wants packed_accounts[0] which doesn't exist

        let accounts = vec![Pubkey::default(); 7]; // Only system accounts

        let token_instruction = TokenInstructionData {
            data: &data,
            accounts: &accounts,
        };
        let result = extract_ata_owners(&token_instruction);

        assert!(
            result.is_empty(),
            "Should not extract ATA when no packed_accounts exist"
        );
    }

    #[test]
    fn test_extract_ata_owners_exactly_one_packed_account() {
        // Edge case: exactly 8 accounts (only one packed_account at index 0)
        let data = create_transfer2_with_ata(0, true);

        let mut accounts = vec![Pubkey::default(); 8];
        let owner = Pubkey::new_from_array([55u8; 32]);
        accounts[7] = owner;

        let token_instruction = TokenInstructionData {
            data: &data,
            accounts: &accounts,
        };
        let result = extract_ata_owners(&token_instruction);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].wallet_owner, owner);
    }

    // ==========================================================================
    // Tests for wrap_program_ids edge cases
    // ==========================================================================

    #[test]
    fn test_wrap_program_ids_empty_instruction_data() {
        let program_ids = vec![Pubkey::from(LIGHT_TOKEN_PROGRAM_ID)];
        let instructions = vec![vec![]]; // Empty instruction data
        let accounts = vec![vec![]];

        let result = wrap_program_ids(&program_ids, &instructions, &accounts);
        assert_eq!(
            result,
            vec![ProgramId::Unknown],
            "Empty instruction should be Unknown"
        );
    }

    #[test]
    fn test_wrap_program_ids_exactly_12_bytes() {
        // Boundary: exactly 12 bytes is valid
        let program_ids = vec![Pubkey::from(LIGHT_TOKEN_PROGRAM_ID)];
        let mut instruction_data = vec![0u8; 12];
        instruction_data[0] = TRANSFER2;
        let instructions = vec![instruction_data];
        let accounts = vec![vec![]];

        let result = wrap_program_ids(&program_ids, &instructions, &accounts);
        assert_eq!(result, vec![ProgramId::LightToken]);
    }

    #[test]
    fn test_wrap_program_ids_11_bytes() {
        // Boundary: 11 bytes is too small
        let program_ids = vec![Pubkey::from(LIGHT_TOKEN_PROGRAM_ID)];
        let mut instruction_data = vec![0u8; 11];
        instruction_data[0] = TRANSFER2;
        let instructions = vec![instruction_data];
        let accounts = vec![vec![]];

        let result = wrap_program_ids(&program_ids, &instructions, &accounts);
        assert_eq!(result, vec![ProgramId::Unknown], "11 bytes is too small");
    }

    #[test]
    fn test_wrap_program_ids_mixed_valid_invalid() {
        // Mix of valid and invalid instructions
        let program_ids = vec![
            Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
            Pubkey::from(LIGHT_REGISTRY_PROGRAM_ID),
            Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
            Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
        ];

        let mut valid_transfer = vec![0u8; 12];
        valid_transfer[0] = TRANSFER2;

        let instructions = vec![
            valid_transfer.clone(), // Valid Token + TRANSFER2
            vec![0u8; 12],          // Valid Registry (any 12+ bytes)
            vec![0xFF; 12],         // Token but not TRANSFER2
            vec![TRANSFER2; 5],     // Token + TRANSFER2 but too short
        ];
        let accounts = vec![vec![], vec![], vec![], vec![]];

        let result = wrap_program_ids(&program_ids, &instructions, &accounts);
        assert_eq!(
            result,
            vec![
                ProgramId::LightToken,
                ProgramId::Registry,
                ProgramId::Unknown,
                ProgramId::Unknown,
            ]
        );
    }

    #[test]
    fn test_wrap_program_ids_account_compression_missing_registered_pda() {
        // AccountCompression with wrong registered PDA
        let program_ids = vec![Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID)];
        let mut instruction_data = vec![0u8; 12];
        instruction_data[0..8].copy_from_slice(&DISCRIMINATOR_INSERT_INTO_QUEUES);
        let instructions = vec![instruction_data];
        // accounts[1] should be REGISTERED_PROGRAM_PDA but we use a different pubkey
        let accounts = vec![vec![
            Pubkey::default(),
            Pubkey::new_from_array([99u8; 32]), // Wrong PDA
            Pubkey::default(),
        ]];

        let result = wrap_program_ids(&program_ids, &instructions, &accounts);
        assert_eq!(
            result,
            vec![ProgramId::Unknown],
            "AccountCompression with wrong registered PDA should be Unknown"
        );
    }

    #[test]
    fn test_wrap_program_ids_account_compression_valid() {
        // AccountCompression with correct setup
        let program_ids = vec![Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID)];
        let mut instruction_data = vec![0u8; 12];
        instruction_data[0..8].copy_from_slice(&DISCRIMINATOR_INSERT_INTO_QUEUES);
        let instructions = vec![instruction_data];
        let accounts = vec![vec![
            Pubkey::default(),
            Pubkey::from(REGISTERED_PROGRAM_PDA), // Correct PDA
            Pubkey::default(),
        ]];

        let result = wrap_program_ids(&program_ids, &instructions, &accounts);
        assert_eq!(result, vec![ProgramId::AccountCompression]);
    }

    #[test]
    fn test_wrap_program_ids_account_compression_insufficient_accounts() {
        // AccountCompression with too few accounts
        let program_ids = vec![Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID)];
        let mut instruction_data = vec![0u8; 12];
        instruction_data[0..8].copy_from_slice(&DISCRIMINATOR_INSERT_INTO_QUEUES);
        let instructions = vec![instruction_data];
        let accounts = vec![vec![Pubkey::default()]]; // Only 1 account, need 3

        let result = wrap_program_ids(&program_ids, &instructions, &accounts);
        assert_eq!(
            result,
            vec![ProgramId::Unknown],
            "AccountCompression with insufficient accounts should be Unknown"
        );
    }
}
