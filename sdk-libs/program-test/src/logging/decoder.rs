//! Instruction decoder for Light Protocol and common Solana programs

use borsh::BorshDeserialize;
use light_compressed_account::instruction_data::{
    data::InstructionDataInvoke, invoke_cpi::InstructionDataInvokeCpi,
    with_account_info::InstructionDataInvokeCpiWithAccountInfo,
    with_readonly::InstructionDataInvokeCpiWithReadOnly,
};
use solana_sdk::{instruction::AccountMeta, pubkey::Pubkey, system_program};

use super::types::ParsedInstructionData;

/// Helper to resolve merkle tree and queue pubkeys from instruction accounts
/// For InvokeCpi instructions, tree accounts start 2 positions after the system program
fn resolve_tree_and_queue_pubkeys(
    accounts: &[AccountMeta],
    merkle_tree_index: Option<u8>,
    nullifier_queue_index: Option<u8>,
) -> (Option<Pubkey>, Option<Pubkey>) {
    let mut tree_pubkey = None;
    let mut queue_pubkey = None;

    // Find the system program account position
    let mut system_program_pos = None;
    for (i, account) in accounts.iter().enumerate() {
        if account.pubkey == system_program::ID {
            system_program_pos = Some(i);
            break;
        }
    }

    if let Some(system_pos) = system_program_pos {
        // Tree accounts start 2 positions after system program
        let tree_accounts_start = system_pos + 2;

        if let Some(tree_idx) = merkle_tree_index {
            let tree_account_pos = tree_accounts_start + tree_idx as usize;
            if tree_account_pos < accounts.len() {
                tree_pubkey = Some(accounts[tree_account_pos].pubkey);
            }
        }

        if let Some(queue_idx) = nullifier_queue_index {
            let queue_account_pos = tree_accounts_start + queue_idx as usize;
            if queue_account_pos < accounts.len() {
                queue_pubkey = Some(accounts[queue_account_pos].pubkey);
            }
        }
    }

    (tree_pubkey, queue_pubkey)
}

/// Decode instruction data for known programs
pub fn decode_instruction(
    program_id: &Pubkey,
    data: &[u8],
    accounts: &[AccountMeta],
) -> Option<ParsedInstructionData> {
    match program_id.to_string().as_str() {
        // Light System Program
        "SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7" => {
            decode_light_system_instruction(data, accounts, program_id)
        }

        // Compute Budget Program
        "ComputeBudget111111111111111111111111111111" => decode_compute_budget_instruction(data),

        // System Program
        id if id == system_program::ID.to_string() => decode_system_instruction(data),

        // Account Compression Program
        "compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq" => decode_compression_instruction(data),

        // Compressed Token Program
        "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m" => decode_compressed_token_instruction(data),

        _ => Some(ParsedInstructionData::Unknown {
            program_name: get_program_name(program_id),
            data_preview: bs58::encode(&data[..data.len().min(16)]).into_string(),
        }),
    }
}

/// Decode Light System Program instructions
fn decode_light_system_instruction(
    data: &[u8],
    accounts: &[AccountMeta],
    program_id: &Pubkey,
) -> Option<ParsedInstructionData> {
    if data.is_empty() {
        return None;
    }

    // Light System Program uses 8-byte discriminators
    if data.len() < 8 {
        return Some(ParsedInstructionData::LightSystemProgram {
            instruction_type: "Invalid".to_string(),
            compressed_accounts: None,
            proof_info: None,
            address_params: None,
            fee_info: None,
            input_account_data: None,
            output_account_data: None,
        });
    }

    // Extract the 8-byte discriminator
    let discriminator: [u8; 8] = data[0..8].try_into().unwrap();

    // Light Protocol discriminators from compressed-account/src/discriminators.rs
    let (
        instruction_type,
        compressed_accounts,
        proof_info,
        address_params,
        fee_info,
        input_account_data,
        output_account_data,
    ) = match discriminator {
        [26, 16, 169, 7, 21, 202, 242, 25] => {
            // DISCRIMINATOR_INVOKE
            match parse_invoke_instruction(&data[8..], accounts) {
                Ok(parsed) => parsed,
                Err(_) => (
                    "Invoke (parse error)".to_string(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
            }
        }
        [49, 212, 191, 129, 39, 194, 43, 196] => {
            // DISCRIMINATOR_INVOKE_CPI
            match parse_invoke_cpi_instruction(&data[8..], accounts) {
                Ok(parsed) => parsed,
                Err(_) => (
                    "InvokeCpi (parse error)".to_string(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
            }
        }
        [86, 47, 163, 166, 21, 223, 92, 8] => {
            // DISCRIMINATOR_INVOKE_CPI_WITH_READ_ONLY
            match parse_invoke_cpi_readonly_instruction(&data[8..], accounts) {
                Ok(parsed) => parsed,
                Err(_) => (
                    "InvokeCpiWithReadOnly (parse error)".to_string(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
            }
        }
        [228, 34, 128, 84, 47, 139, 86, 240] => {
            // INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION
            match parse_invoke_cpi_account_info_instruction(&data[8..], accounts, program_id) {
                Ok(parsed) => parsed,
                Err(_) => (
                    "InvokeCpiWithAccountInfo (parse error)".to_string(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
            }
        }
        _ => {
            // Unknown discriminator - show the discriminator bytes for debugging
            let discriminator_str = format!("{:?}", discriminator);
            (
                format!("Unknown({})", discriminator_str),
                None,
                None,
                None,
                None,
                None,
                None,
            )
        }
    };

    Some(ParsedInstructionData::LightSystemProgram {
        instruction_type,
        compressed_accounts,
        proof_info,
        address_params,
        fee_info,
        input_account_data,
        output_account_data,
    })
}

type InstructionParseResult = Result<
    (
        String,
        Option<super::types::CompressedAccountSummary>,
        Option<super::types::ProofSummary>,
        Option<Vec<super::types::AddressParam>>,
        Option<super::types::FeeSummary>,
        Option<Vec<super::types::InputAccountData>>,
        Option<Vec<super::types::OutputAccountData>>,
    ),
    Box<dyn std::error::Error>,
>;

/// Parse Invoke instruction data - display data hashes directly
fn parse_invoke_instruction(data: &[u8], accounts: &[AccountMeta]) -> InstructionParseResult {
    // Skip the 4-byte vec length prefix that Anchor adds
    if data.len() < 4 {
        return Err("Instruction data too short for Anchor prefix".into());
    }
    let instruction_data = InstructionDataInvoke::try_from_slice(&data[4..])?;

    let compressed_accounts = Some(super::types::CompressedAccountSummary {
        input_accounts: instruction_data
            .input_compressed_accounts_with_merkle_context
            .len(),
        output_accounts: instruction_data.output_compressed_accounts.len(),
        lamports_change: instruction_data
            .compress_or_decompress_lamports
            .map(|l| l as i64),
    });

    let proof_info = instruction_data
        .proof
        .as_ref()
        .map(|_| super::types::ProofSummary {
            proof_type: "Validity".to_string(),
            has_validity_proof: true,
        });

    // Extract actual address parameters with values
    let address_params = if !instruction_data.new_address_params.is_empty() {
        Some(
            instruction_data
                .new_address_params
                .iter()
                .map(|param| {
                    let tree_idx = Some(param.address_merkle_tree_account_index);
                    let queue_idx = Some(param.address_queue_account_index);
                    let (tree_pubkey, queue_pubkey) =
                        resolve_tree_and_queue_pubkeys(accounts, tree_idx, queue_idx);

                    super::types::AddressParam {
                        seed: param.seed,
                        address_queue_index: queue_idx,
                        address_queue_pubkey: queue_pubkey,
                        merkle_tree_index: tree_idx,
                        address_merkle_tree_pubkey: tree_pubkey,
                        root_index: Some(param.address_merkle_tree_root_index),
                        derived_address: None,
                        assigned_account_index: super::types::AddressAssignment::V1,
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    // Extract input account data
    let input_account_data = if !instruction_data
        .input_compressed_accounts_with_merkle_context
        .is_empty()
    {
        Some(
            instruction_data
                .input_compressed_accounts_with_merkle_context
                .iter()
                .map(|acc| {
                    let tree_idx = Some(acc.merkle_context.merkle_tree_pubkey_index);
                    let queue_idx = Some(acc.merkle_context.queue_pubkey_index);
                    let (tree_pubkey, queue_pubkey) =
                        resolve_tree_and_queue_pubkeys(accounts, tree_idx, queue_idx);

                    super::types::InputAccountData {
                        lamports: acc.compressed_account.lamports,
                        owner: Some(acc.compressed_account.owner.into()),
                        merkle_tree_index: tree_idx,
                        merkle_tree_pubkey: tree_pubkey,
                        queue_index: queue_idx,
                        queue_pubkey,
                        address: acc.compressed_account.address,
                        data_hash: if let Some(ref data) = acc.compressed_account.data {
                            data.data_hash.to_vec()
                        } else {
                            vec![]
                        },
                        discriminator: if let Some(ref data) = acc.compressed_account.data {
                            data.discriminator.to_vec()
                        } else {
                            vec![]
                        },
                        leaf_index: Some(acc.merkle_context.leaf_index),
                        root_index: Some(acc.root_index),
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    // Extract output account data
    let output_account_data = if !instruction_data.output_compressed_accounts.is_empty() {
        Some(
            instruction_data
                .output_compressed_accounts
                .iter()
                .map(|acc| {
                    let tree_idx = Some(acc.merkle_tree_index);
                    let (tree_pubkey, _queue_pubkey) =
                        resolve_tree_and_queue_pubkeys(accounts, tree_idx, None);

                    super::types::OutputAccountData {
                        lamports: acc.compressed_account.lamports,
                        data: acc.compressed_account.data.as_ref().map(|d| d.data.clone()),
                        owner: Some(acc.compressed_account.owner.into()),
                        merkle_tree_index: tree_idx,
                        merkle_tree_pubkey: tree_pubkey,
                        queue_index: None,
                        queue_pubkey: None,
                        address: acc.compressed_account.address,
                        data_hash: if let Some(ref data) = acc.compressed_account.data {
                            data.data_hash.to_vec()
                        } else {
                            vec![]
                        },
                        discriminator: if let Some(ref data) = acc.compressed_account.data {
                            data.discriminator.to_vec()
                        } else {
                            vec![]
                        },
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    let fee_info = instruction_data
        .relay_fee
        .map(|fee| super::types::FeeSummary {
            relay_fee: Some(fee),
            compression_fee: None,
        });

    Ok((
        "Invoke".to_string(),
        compressed_accounts,
        proof_info,
        address_params,
        fee_info,
        input_account_data,
        output_account_data,
    ))
}

/// Parse InvokeCpi instruction data - display data hashes directly
fn parse_invoke_cpi_instruction(data: &[u8], accounts: &[AccountMeta]) -> InstructionParseResult {
    // Skip the 4-byte vec length prefix that Anchor adds
    if data.len() < 4 {
        return Err("Instruction data too short for Anchor prefix".into());
    }
    let instruction_data = InstructionDataInvokeCpi::try_from_slice(&data[4..])?;

    let compressed_accounts = Some(super::types::CompressedAccountSummary {
        input_accounts: instruction_data
            .input_compressed_accounts_with_merkle_context
            .len(),
        output_accounts: instruction_data.output_compressed_accounts.len(),
        lamports_change: instruction_data
            .compress_or_decompress_lamports
            .map(|l| l as i64),
    });

    let proof_info = instruction_data
        .proof
        .as_ref()
        .map(|_| super::types::ProofSummary {
            proof_type: "Validity".to_string(),
            has_validity_proof: true,
        });

    // Extract actual address parameters with values
    let address_params = if !instruction_data.new_address_params.is_empty() {
        Some(
            instruction_data
                .new_address_params
                .iter()
                .map(|param| {
                    let tree_idx = Some(param.address_merkle_tree_account_index);
                    let queue_idx = Some(param.address_queue_account_index);
                    let (tree_pubkey, queue_pubkey) =
                        resolve_tree_and_queue_pubkeys(accounts, tree_idx, queue_idx);

                    super::types::AddressParam {
                        seed: param.seed,
                        address_queue_index: queue_idx,
                        address_queue_pubkey: queue_pubkey,
                        merkle_tree_index: tree_idx,
                        address_merkle_tree_pubkey: tree_pubkey,
                        root_index: Some(param.address_merkle_tree_root_index),
                        derived_address: None,
                        assigned_account_index: super::types::AddressAssignment::V1,
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    // Extract input account data
    let input_account_data = if !instruction_data
        .input_compressed_accounts_with_merkle_context
        .is_empty()
    {
        Some(
            instruction_data
                .input_compressed_accounts_with_merkle_context
                .iter()
                .map(|acc| {
                    let tree_idx = Some(acc.merkle_context.merkle_tree_pubkey_index);
                    let queue_idx = Some(acc.merkle_context.queue_pubkey_index);
                    let (tree_pubkey, queue_pubkey) =
                        resolve_tree_and_queue_pubkeys(accounts, tree_idx, queue_idx);

                    super::types::InputAccountData {
                        lamports: acc.compressed_account.lamports,
                        owner: Some(acc.compressed_account.owner.into()),
                        merkle_tree_index: tree_idx,
                        merkle_tree_pubkey: tree_pubkey,
                        queue_index: queue_idx,
                        queue_pubkey,
                        address: acc.compressed_account.address,
                        data_hash: if let Some(ref data) = acc.compressed_account.data {
                            data.data_hash.to_vec()
                        } else {
                            vec![]
                        },
                        discriminator: if let Some(ref data) = acc.compressed_account.data {
                            data.discriminator.to_vec()
                        } else {
                            vec![]
                        },
                        leaf_index: Some(acc.merkle_context.leaf_index),
                        root_index: Some(acc.root_index),
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    // Extract output account data
    let output_account_data = if !instruction_data.output_compressed_accounts.is_empty() {
        Some(
            instruction_data
                .output_compressed_accounts
                .iter()
                .map(|acc| {
                    let tree_idx = Some(acc.merkle_tree_index);
                    let (tree_pubkey, _queue_pubkey) =
                        resolve_tree_and_queue_pubkeys(accounts, tree_idx, None);

                    super::types::OutputAccountData {
                        lamports: acc.compressed_account.lamports,
                        data: acc.compressed_account.data.as_ref().map(|d| d.data.clone()),
                        owner: Some(acc.compressed_account.owner.into()),
                        merkle_tree_index: tree_idx,
                        merkle_tree_pubkey: tree_pubkey,
                        queue_index: None,
                        queue_pubkey: None,
                        address: acc.compressed_account.address,
                        data_hash: if let Some(ref data) = acc.compressed_account.data {
                            data.data_hash.to_vec()
                        } else {
                            vec![]
                        },
                        discriminator: if let Some(ref data) = acc.compressed_account.data {
                            data.discriminator.to_vec()
                        } else {
                            vec![]
                        },
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    let fee_info = instruction_data
        .relay_fee
        .map(|fee| super::types::FeeSummary {
            relay_fee: Some(fee),
            compression_fee: None,
        });

    Ok((
        "InvokeCpi".to_string(),
        compressed_accounts,
        proof_info,
        address_params,
        fee_info,
        input_account_data,
        output_account_data,
    ))
}

/// Parse InvokeCpiWithReadOnly instruction data - display data hashes directly
fn parse_invoke_cpi_readonly_instruction(
    data: &[u8],
    accounts: &[AccountMeta],
) -> InstructionParseResult {
    let instruction_data = InstructionDataInvokeCpiWithReadOnly::try_from_slice(data)?;

    let compressed_accounts = Some(super::types::CompressedAccountSummary {
        input_accounts: instruction_data.input_compressed_accounts.len(),
        output_accounts: instruction_data.output_compressed_accounts.len(),
        lamports_change: if instruction_data.compress_or_decompress_lamports > 0 {
            Some(instruction_data.compress_or_decompress_lamports as i64)
        } else {
            None
        },
    });

    let proof_info = Some(super::types::ProofSummary {
        proof_type: "Validity".to_string(),
        has_validity_proof: true,
    });

    // Extract actual address parameters with values
    let mut address_params = Vec::new();

    // Add new address parameters with actual values
    for param in &instruction_data.new_address_params {
        let tree_idx = Some(param.address_merkle_tree_account_index);
        let queue_idx = Some(param.address_queue_account_index);
        let (tree_pubkey, queue_pubkey) =
            resolve_tree_and_queue_pubkeys(accounts, tree_idx, queue_idx);

        address_params.push(super::types::AddressParam {
            seed: param.seed,
            address_queue_index: queue_idx,
            address_queue_pubkey: queue_pubkey,
            merkle_tree_index: tree_idx,
            address_merkle_tree_pubkey: tree_pubkey,
            root_index: Some(param.address_merkle_tree_root_index),
            derived_address: None,
            assigned_account_index: if param.assigned_to_account {
                super::types::AddressAssignment::AssignedIndex(param.assigned_account_index)
            } else {
                super::types::AddressAssignment::None
            },
        });
    }

    // Add readonly address parameters
    for readonly_addr in &instruction_data.read_only_addresses {
        let tree_idx = Some(readonly_addr.address_merkle_tree_account_index);
        let (tree_pubkey, _queue_pubkey) = resolve_tree_and_queue_pubkeys(accounts, tree_idx, None);

        address_params.push(super::types::AddressParam {
            seed: [0; 32], // ReadOnly addresses don't have seeds in the same way
            address_queue_index: None,
            address_queue_pubkey: None,
            merkle_tree_index: tree_idx,
            address_merkle_tree_pubkey: tree_pubkey,
            root_index: Some(readonly_addr.address_merkle_tree_root_index),
            derived_address: Some(readonly_addr.address),
            assigned_account_index: super::types::AddressAssignment::None,
        });
    }

    let address_params = if !address_params.is_empty() {
        Some(address_params)
    } else {
        None
    };

    // Extract input account data - use data_hash from InAccount
    let input_account_data = if !instruction_data.input_compressed_accounts.is_empty() {
        Some(
            instruction_data
                .input_compressed_accounts
                .iter()
                .map(|acc| {
                    let tree_idx = Some(acc.merkle_context.merkle_tree_pubkey_index);
                    let queue_idx = Some(acc.merkle_context.queue_pubkey_index);
                    let (tree_pubkey, queue_pubkey) =
                        resolve_tree_and_queue_pubkeys(accounts, tree_idx, queue_idx);

                    super::types::InputAccountData {
                        lamports: acc.lamports,
                        owner: Some(instruction_data.invoking_program_id.into()), // Use invoking program as owner
                        merkle_tree_index: tree_idx,
                        merkle_tree_pubkey: tree_pubkey,
                        queue_index: queue_idx,
                        queue_pubkey,
                        address: acc.address,
                        data_hash: acc.data_hash.to_vec(),
                        discriminator: acc.discriminator.to_vec(),
                        leaf_index: Some(acc.merkle_context.leaf_index),
                        root_index: Some(acc.root_index),
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    // Extract output account data
    let output_account_data = if !instruction_data.output_compressed_accounts.is_empty() {
        Some(
            instruction_data
                .output_compressed_accounts
                .iter()
                .map(|acc| {
                    let tree_idx = Some(acc.merkle_tree_index);
                    let (tree_pubkey, _queue_pubkey) =
                        resolve_tree_and_queue_pubkeys(accounts, tree_idx, None);

                    super::types::OutputAccountData {
                        lamports: acc.compressed_account.lamports,
                        data: acc.compressed_account.data.as_ref().map(|d| d.data.clone()),
                        owner: Some(instruction_data.invoking_program_id.into()), // Use invoking program as owner for consistency
                        merkle_tree_index: tree_idx,
                        merkle_tree_pubkey: tree_pubkey,
                        queue_index: None,
                        queue_pubkey: None,
                        address: acc.compressed_account.address,
                        data_hash: if let Some(ref data) = acc.compressed_account.data {
                            data.data_hash.to_vec()
                        } else {
                            vec![]
                        },
                        discriminator: if let Some(ref data) = acc.compressed_account.data {
                            data.discriminator.to_vec()
                        } else {
                            vec![]
                        },
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    Ok((
        "InvokeCpiWithReadOnly".to_string(),
        compressed_accounts,
        proof_info,
        address_params,
        None,
        input_account_data,
        output_account_data,
    ))
}

/// Parse InvokeCpiWithAccountInfo instruction data - display data hashes directly
fn parse_invoke_cpi_account_info_instruction(
    data: &[u8],
    accounts: &[AccountMeta],
    program_id: &Pubkey,
) -> InstructionParseResult {
    let instruction_data = InstructionDataInvokeCpiWithAccountInfo::try_from_slice(data)?;

    let input_accounts = instruction_data
        .account_infos
        .iter()
        .filter(|a| a.input.is_some())
        .count();
    let output_accounts = instruction_data
        .account_infos
        .iter()
        .filter(|a| a.output.is_some())
        .count();

    let compressed_accounts = Some(super::types::CompressedAccountSummary {
        input_accounts,
        output_accounts,
        lamports_change: if instruction_data.compress_or_decompress_lamports > 0 {
            Some(instruction_data.compress_or_decompress_lamports as i64)
        } else {
            None
        },
    });

    let proof_info = Some(super::types::ProofSummary {
        proof_type: "Validity".to_string(),
        has_validity_proof: true,
    });

    // Extract actual address parameters with values
    let mut address_params = Vec::new();

    // Add new address parameters with actual values
    for param in &instruction_data.new_address_params {
        let tree_idx = Some(param.address_merkle_tree_account_index);
        let queue_idx = Some(param.address_queue_account_index);
        let (tree_pubkey, queue_pubkey) =
            resolve_tree_and_queue_pubkeys(accounts, tree_idx, queue_idx);

        address_params.push(super::types::AddressParam {
            seed: param.seed,
            address_queue_index: queue_idx,
            address_queue_pubkey: queue_pubkey,
            merkle_tree_index: tree_idx,
            address_merkle_tree_pubkey: tree_pubkey,
            root_index: Some(param.address_merkle_tree_root_index),
            derived_address: None,
            assigned_account_index: if param.assigned_to_account {
                super::types::AddressAssignment::AssignedIndex(param.assigned_account_index)
            } else {
                super::types::AddressAssignment::None
            },
        });
    }

    // Add readonly address parameters
    for readonly_addr in &instruction_data.read_only_addresses {
        let tree_idx = Some(readonly_addr.address_merkle_tree_account_index);
        let (tree_pubkey, _queue_pubkey) = resolve_tree_and_queue_pubkeys(accounts, tree_idx, None);

        address_params.push(super::types::AddressParam {
            seed: [0; 32], // ReadOnly addresses don't have seeds in the same way
            address_queue_index: None,
            address_queue_pubkey: None,
            merkle_tree_index: tree_idx,
            address_merkle_tree_pubkey: tree_pubkey,
            root_index: Some(readonly_addr.address_merkle_tree_root_index),
            derived_address: Some(readonly_addr.address),
            assigned_account_index: super::types::AddressAssignment::None,
        });
    }

    let address_params = if !address_params.is_empty() {
        Some(address_params)
    } else {
        None
    };

    // Extract input account data from account_infos
    let input_account_data = {
        let mut input_data = Vec::new();
        for account_info in &instruction_data.account_infos {
            if let Some(ref input) = account_info.input {
                input_data.push(super::types::InputAccountData {
                    lamports: input.lamports,
                    owner: Some(*program_id), // Use invoking program as owner
                    merkle_tree_index: None, // Note: merkle tree context not available in CompressedAccountInfo
                    merkle_tree_pubkey: None,
                    queue_index: None,
                    queue_pubkey: None,
                    address: account_info.address, // Use address from CompressedAccountInfo
                    data_hash: input.data_hash.to_vec(),
                    discriminator: input.discriminator.to_vec(),
                    leaf_index: Some(input.merkle_context.leaf_index),
                    root_index: Some(input.root_index),
                });
            }
        }
        if !input_data.is_empty() {
            Some(input_data)
        } else {
            None
        }
    };

    // Extract output account data from account_infos
    let output_account_data = {
        let mut output_data = Vec::new();
        for account_info in &instruction_data.account_infos {
            if let Some(ref output) = account_info.output {
                let tree_idx = Some(output.output_merkle_tree_index);
                let (tree_pubkey, _queue_pubkey) =
                    resolve_tree_and_queue_pubkeys(accounts, tree_idx, None);

                output_data.push(super::types::OutputAccountData {
                    lamports: output.lamports,
                    data: if !output.data.is_empty() {
                        Some(output.data.clone())
                    } else {
                        None
                    },
                    owner: Some(*program_id), // Use invoking program as owner
                    merkle_tree_index: tree_idx,
                    merkle_tree_pubkey: tree_pubkey,
                    queue_index: None,
                    queue_pubkey: None,
                    address: account_info.address, // Use address from CompressedAccountInfo
                    data_hash: output.data_hash.to_vec(),
                    discriminator: output.discriminator.to_vec(),
                });
            }
        }
        if !output_data.is_empty() {
            Some(output_data)
        } else {
            None
        }
    };

    Ok((
        "InvokeCpiWithAccountInfo".to_string(),
        compressed_accounts,
        proof_info,
        address_params,
        None,
        input_account_data,
        output_account_data,
    ))
}

/// Decode Compute Budget Program instructions
fn decode_compute_budget_instruction(data: &[u8]) -> Option<ParsedInstructionData> {
    if data.len() < 4 {
        return None;
    }

    let instruction_discriminator = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

    match instruction_discriminator {
        0 => {
            // RequestUnitsDeprecated
            if data.len() >= 12 {
                let units = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as u64;
                let _additional_fee =
                    u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as u64;
                Some(ParsedInstructionData::ComputeBudget {
                    instruction_type: "RequestUnitsDeprecated".to_string(),
                    value: Some(units),
                })
            } else {
                None
            }
        }
        1 => {
            // RequestHeapFrame
            if data.len() >= 8 {
                let bytes = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as u64;
                Some(ParsedInstructionData::ComputeBudget {
                    instruction_type: "RequestHeapFrame".to_string(),
                    value: Some(bytes),
                })
            } else {
                None
            }
        }
        2 => {
            // SetComputeUnitLimit
            if data.len() >= 8 {
                let units = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as u64;
                Some(ParsedInstructionData::ComputeBudget {
                    instruction_type: "SetComputeUnitLimit".to_string(),
                    value: Some(units),
                })
            } else {
                None
            }
        }
        3 => {
            // SetComputeUnitPrice
            if data.len() >= 12 {
                let price = u64::from_le_bytes([
                    data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
                ]);
                Some(ParsedInstructionData::ComputeBudget {
                    instruction_type: "SetComputeUnitPrice".to_string(),
                    value: Some(price),
                })
            } else {
                None
            }
        }
        _ => Some(ParsedInstructionData::ComputeBudget {
            instruction_type: "Unknown".to_string(),
            value: None,
        }),
    }
}

/// Decode System Program instructions
fn decode_system_instruction(data: &[u8]) -> Option<ParsedInstructionData> {
    if data.len() < 4 {
        return None;
    }

    let instruction_type = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

    match instruction_type {
        0 => {
            // CreateAccount
            if data.len() >= 52 {
                let lamports = u64::from_le_bytes([
                    data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
                ]);
                let space = u64::from_le_bytes([
                    data[12], data[13], data[14], data[15], data[16], data[17], data[18], data[19],
                ]);

                Some(ParsedInstructionData::System {
                    instruction_type: "CreateAccount".to_string(),
                    lamports: Some(lamports),
                    space: Some(space),
                    new_account: None,
                })
            } else {
                None
            }
        }
        2 => {
            // Transfer
            if data.len() >= 12 {
                let lamports = u64::from_le_bytes([
                    data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
                ]);

                Some(ParsedInstructionData::System {
                    instruction_type: "Transfer".to_string(),
                    lamports: Some(lamports),
                    space: None,
                    new_account: None,
                })
            } else {
                None
            }
        }
        8 => {
            // Allocate
            if data.len() >= 12 {
                let space = u64::from_le_bytes([
                    data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
                ]);

                Some(ParsedInstructionData::System {
                    instruction_type: "Allocate".to_string(),
                    lamports: None,
                    space: Some(space),
                    new_account: None,
                })
            } else {
                None
            }
        }
        _ => Some(ParsedInstructionData::System {
            instruction_type: "Unknown".to_string(),
            lamports: None,
            space: None,
            new_account: None,
        }),
    }
}

/// Decode Account Compression Program instructions
fn decode_compression_instruction(data: &[u8]) -> Option<ParsedInstructionData> {
    // Return basic instruction info for account compression
    let instruction_name = if data.len() >= 8 {
        // Common account compression operations
        "InsertIntoQueues"
    } else {
        "Unknown"
    };

    Some(ParsedInstructionData::Unknown {
        program_name: "Account Compression".to_string(),
        data_preview: format!("{}({}bytes)", instruction_name, data.len()),
    })
}

/// Decode Compressed Token Program instructions
fn decode_compressed_token_instruction(data: &[u8]) -> Option<ParsedInstructionData> {
    // Return basic instruction info for compressed token operations
    let instruction_name = if data.len() >= 8 {
        // Common compressed token operations
        "TokenOperation"
    } else {
        "Unknown"
    };

    Some(ParsedInstructionData::Unknown {
        program_name: "Compressed Token".to_string(),
        data_preview: format!("{}({}bytes)", instruction_name, data.len()),
    })
}

/// Get human-readable program name
fn get_program_name(program_id: &Pubkey) -> String {
    match program_id.to_string().as_str() {
        id if id == system_program::ID.to_string() => "System Program".to_string(),
        "ComputeBudget111111111111111111111111111111" => "Compute Budget".to_string(),
        "SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7" => "Light System Program".to_string(),
        "compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq" => "Account Compression".to_string(),
        "FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy" => "Test Program".to_string(),
        _ => {
            let pubkey_str = program_id.to_string();
            format!("Program {}", &pubkey_str[..8])
        }
    }
}

/// Extract Light Protocol events from transaction logs and metadata
pub fn extract_light_events(
    logs: &[String],
    _events: &Option<Vec<String>>, // Light Protocol events for future enhancement
) -> Vec<super::types::LightProtocolEvent> {
    let mut light_events = Vec::new();

    // Parse events from logs
    for log in logs {
        if log.contains("PublicTransactionEvent") || log.contains("BatchPublicTransactionEvent") {
            // Parse Light Protocol events from logs
            light_events.push(super::types::LightProtocolEvent {
                event_type: "PublicTransactionEvent".to_string(),
                compressed_accounts: Vec::new(),
                merkle_tree_changes: Vec::new(),
                nullifiers: Vec::new(),
            });
        }
    }

    light_events
}
