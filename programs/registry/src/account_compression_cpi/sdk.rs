#![cfg(not(target_os = "solana"))]
use account_compression::{
    utils::constants::NOOP_PUBKEY, AddressMerkleTreeConfig, AddressQueueConfig, MigrateLeafParams,
    NullifierQueueConfig, StateMerkleTreeConfig,
};
use anchor_lang::{prelude::*, InstructionData};
use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
};
use light_system_program::program::LightSystemProgram;
use solana_sdk::instruction::Instruction;

use crate::utils::{
    get_cpi_authority_pda, get_forester_epoch_pda_from_authority, get_protocol_config_pda_address,
};
pub struct CreateNullifyInstructionInputs {
    pub authority: Pubkey,
    pub nullifier_queue: Pubkey,
    pub merkle_tree: Pubkey,
    pub change_log_indices: Vec<u64>,
    pub leaves_queue_indices: Vec<u16>,
    pub indices: Vec<u64>,
    pub proofs: Vec<Vec<[u8; 32]>>,
    pub derivation: Pubkey,
    pub is_metadata_forester: bool,
}

pub fn create_nullify_instruction(
    inputs: CreateNullifyInstructionInputs,
    epoch: u64,
) -> Instruction {
    let register_program_pda = get_registered_program_pda(&crate::ID);
    let registered_forester_pda = if inputs.is_metadata_forester {
        None
    } else {
        Some(get_forester_epoch_pda_from_authority(&inputs.derivation, epoch).0)
    };
    let (cpi_authority, bump) = get_cpi_authority_pda();
    let instruction_data = crate::instruction::Nullify {
        bump,
        change_log_indices: inputs.change_log_indices,
        leaves_queue_indices: inputs.leaves_queue_indices,
        indices: inputs.indices,
        proofs: inputs.proofs,
    };

    let accounts = crate::accounts::NullifyLeaves {
        authority: inputs.authority,
        registered_forester_pda,
        registered_program_pda: register_program_pda,
        nullifier_queue: inputs.nullifier_queue,
        merkle_tree: inputs.merkle_tree,
        log_wrapper: NOOP_PUBKEY.into(),
        cpi_authority,
        account_compression_program: account_compression::ID,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

/// Returns the base accounts for populating an address lookup table
/// for nullify v0 transactions.
fn nullify_lookup_table_accounts_base(merkle_tree: Pubkey, nullifier_queue: Pubkey) -> Vec<Pubkey> {
    let (cpi_authority, _) = get_cpi_authority_pda();
    let registered_program_pda = get_registered_program_pda(&crate::ID);
    vec![
        cpi_authority,
        registered_program_pda,
        account_compression::ID,
        Pubkey::new_from_array(NOOP_PUBKEY),
        merkle_tree,
        nullifier_queue,
        crate::ID,
    ]
}

/// Max number of 32-byte nodes in the dedup encoding vec.
/// Verified by tx size test (forester/tests/test_nullify_state_v1_multi_tx_size.rs).
/// With ALT, SetComputeUnitLimit + SetComputeUnitPrice ixs, and worst-case nodes,
/// the tx fits within the 1232 byte limit.
pub const NULLIFY_STATE_V1_MULTI_MAX_NODES: usize = 26;

#[derive(Clone, Debug, PartialEq)]
pub struct CreateNullifyStateV1MultiInstructionInputs {
    pub authority: Pubkey,
    pub nullifier_queue: Pubkey,
    pub merkle_tree: Pubkey,
    pub change_log_index: u16,
    pub queue_indices: [u16; 4],
    pub leaf_indices: [u32; 4],
    pub proof_2_shared: u16,
    pub proof_3_source: u32,
    pub proof_4_source: u32,
    pub shared_top_node: [u8; 32],
    pub nodes: Vec<[u8; 32]>,
    pub derivation: Pubkey,
    pub is_metadata_forester: bool,
}

pub fn create_nullify_state_v1_multi_instruction(
    inputs: CreateNullifyStateV1MultiInstructionInputs,
    epoch: u64,
) -> Instruction {
    let register_program_pda = get_registered_program_pda(&crate::ID);
    let registered_forester_pda = if inputs.is_metadata_forester {
        None
    } else {
        Some(get_forester_epoch_pda_from_authority(&inputs.derivation, epoch).0)
    };
    let (cpi_authority, _bump) = get_cpi_authority_pda();
    let instruction_data = crate::instruction::NullifyStateV1Multi {
        change_log_index: inputs.change_log_index,
        queue_indices: inputs.queue_indices,
        leaf_indices: inputs.leaf_indices,
        proof_2_shared: inputs.proof_2_shared,
        proof_3_source: inputs.proof_3_source,
        proof_4_source: inputs.proof_4_source,
        shared_top_node: inputs.shared_top_node,
        nodes: inputs.nodes,
    };

    let accounts = crate::accounts::NullifyLeaves {
        authority: inputs.authority,
        registered_forester_pda,
        registered_program_pda: register_program_pda,
        nullifier_queue: inputs.nullifier_queue,
        merkle_tree: inputs.merkle_tree,
        log_wrapper: NOOP_PUBKEY.into(),
        cpi_authority,
        account_compression_program: account_compression::ID,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

/// Result of compressing 2-4 Merkle proofs into the dedup encoding.
pub struct CompressedProofs {
    pub proof_2_shared: u16,
    pub proof_3_source: u32,
    pub proof_4_source: u32,
    pub shared_top_node: [u8; 32],
    pub nodes: Vec<[u8; 32]>,
}

/// Compresses 2-4 full 16-node Merkle proofs into the dedup encoding.
/// Returns the compressed proof data,
/// or `None` if compression is impossible (different top nodes, too many unique nodes, or
/// fewer than 2 or more than 4 proofs).
pub fn compress_proofs(proofs: &[&[[u8; 32]; 16]]) -> Option<CompressedProofs> {
    if proofs.len() < 2 || proofs.len() > 4 {
        return None;
    }

    // All proofs must share the same node at index 15
    let shared_top_node = proofs[0][15];
    for p in &proofs[1..] {
        if p[15] != shared_top_node {
            return None;
        }
    }

    let mut nodes: Vec<[u8; 32]> = Vec::new();

    // proof_1: levels 0..14
    for i in 0..15 {
        nodes.push(proofs[0][i]);
    }

    // proof_2: bitvec
    let mut proof_2_shared: u16 = 0;
    for i in 0..15 {
        if proofs[1][i] == proofs[0][i] {
            proof_2_shared |= 1 << i;
        } else {
            nodes.push(proofs[1][i]);
        }
    }

    // proof_3
    let mut proof_3_source: u32 = 0;
    if proofs.len() >= 3 {
        for i in 0..15 {
            if proofs[2][i] == proofs[0][i] {
                // 00 = proof_1
            } else if proofs[2][i] == proofs[1][i] {
                proof_3_source |= 0b01 << (i * 2);
            } else {
                proof_3_source |= 0b10 << (i * 2);
                nodes.push(proofs[2][i]);
            }
        }
    }

    // proof_4
    let mut proof_4_source: u32 = 0;
    if proofs.len() >= 4 {
        for i in 0..15 {
            if proofs[3][i] == proofs[0][i] {
                // 00 = proof_1
            } else if proofs[3][i] == proofs[1][i] {
                proof_4_source |= 0b01 << (i * 2);
            } else if proofs[3][i] == proofs[2][i] {
                proof_4_source |= 0b10 << (i * 2);
            } else {
                proof_4_source |= 0b11 << (i * 2);
                nodes.push(proofs[3][i]);
            }
        }
    }

    if nodes.len() > NULLIFY_STATE_V1_MULTI_MAX_NODES {
        return None;
    }

    Some(CompressedProofs {
        proof_2_shared,
        proof_3_source,
        proof_4_source,
        shared_top_node,
        nodes,
    })
}

/// Returns the known accounts for populating an address lookup table
/// for nullify_state_v1_multi v0 transactions. Includes ComputeBudget program ID
/// since nullify_state_v1_multi transactions also include a SetComputeUnitLimit instruction.
pub fn nullify_state_v1_multi_lookup_table_accounts(
    merkle_tree: Pubkey,
    nullifier_queue: Pubkey,
) -> Vec<Pubkey> {
    let mut accounts = nullify_lookup_table_accounts_base(merkle_tree, nullifier_queue);
    accounts.push(solana_sdk::compute_budget::ID);
    accounts
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateMigrateStateInstructionInputs {
    pub authority: Pubkey,
    pub output_queue: Pubkey,
    pub merkle_tree: Pubkey,
    pub inputs: MigrateLeafParams,
    pub derivation: Pubkey,
    pub is_metadata_forester: bool,
}

pub fn create_migrate_state_instruction(
    inputs: CreateMigrateStateInstructionInputs,
    epoch: u64,
) -> Instruction {
    let register_program_pda = get_registered_program_pda(&crate::ID);
    let registered_forester_pda =
        get_forester_epoch_pda_from_authority(&inputs.derivation, epoch).0;
    let (cpi_authority, bump) = get_cpi_authority_pda();
    let instruction_data = crate::instruction::MigrateState {
        bump,
        inputs: inputs.inputs,
    };

    let accounts = crate::accounts::MigrateState {
        authority: inputs.authority,
        registered_forester_pda,
        registered_program_pda: register_program_pda,
        output_queue: inputs.output_queue,
        merkle_tree: inputs.merkle_tree,
        log_wrapper: NOOP_PUBKEY.into(),
        cpi_authority,
        account_compression_program: account_compression::ID,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn get_registered_program_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[program_id.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0
}

pub struct CreateRolloverMerkleTreeInstructionInputs {
    pub authority: Pubkey,
    pub derivation: Pubkey,
    pub new_queue: Pubkey,
    pub new_merkle_tree: Pubkey,
    pub old_queue: Pubkey,
    pub old_merkle_tree: Pubkey,
    pub cpi_context_account: Option<Pubkey>,
    pub is_metadata_forester: bool,
}

pub fn create_rollover_address_merkle_tree_instruction(
    inputs: CreateRolloverMerkleTreeInstructionInputs,
    epoch: u64,
) -> Instruction {
    let (_, bump) = get_cpi_authority_pda();

    let instruction_data = crate::instruction::RolloverAddressMerkleTreeAndQueue { bump };
    let (cpi_authority, _) = get_cpi_authority_pda();
    let registered_program_pda = get_registered_program_pda(&crate::ID);
    let registered_forester_pda = if inputs.is_metadata_forester {
        None
    } else {
        Some(get_forester_epoch_pda_from_authority(&inputs.derivation, epoch).0)
    };

    let accounts = crate::accounts::RolloverAddressMerkleTreeAndQueue {
        account_compression_program: account_compression::ID,
        registered_forester_pda,
        cpi_authority,
        authority: inputs.authority,
        registered_program_pda,
        new_merkle_tree: inputs.new_merkle_tree,
        new_queue: inputs.new_queue,
        old_merkle_tree: inputs.old_merkle_tree,
        old_queue: inputs.old_queue,
    };

    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_rollover_state_merkle_tree_instruction(
    inputs: CreateRolloverMerkleTreeInstructionInputs,
    epoch: u64,
) -> Instruction {
    let (_, bump) = get_cpi_authority_pda();

    let instruction_data = crate::instruction::RolloverStateMerkleTreeAndQueue { bump };
    let (cpi_authority, _) = get_cpi_authority_pda();
    let registered_program_pda = get_registered_program_pda(&crate::ID);
    let registered_forester_pda = if inputs.is_metadata_forester {
        None
    } else {
        Some(get_forester_epoch_pda_from_authority(&inputs.derivation, epoch).0)
    };
    let protocol_config_pda = get_protocol_config_pda_address().0;

    let accounts = crate::accounts::RolloverStateMerkleTreeAndQueue {
        account_compression_program: account_compression::ID,
        registered_forester_pda,
        cpi_authority,
        authority: inputs.authority,
        registered_program_pda,
        new_merkle_tree: inputs.new_merkle_tree,
        new_queue: inputs.new_queue,
        old_merkle_tree: inputs.old_merkle_tree,
        old_queue: inputs.old_queue,
        cpi_context_account: inputs.cpi_context_account.unwrap(),
        light_system_program: LightSystemProgram::id(),
        protocol_config_pda,
    };

    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub struct UpdateAddressMerkleTreeInstructionInputs {
    pub authority: Pubkey,
    pub derivation: Pubkey,
    pub address_merkle_tree: Pubkey,
    pub address_queue: Pubkey,
    pub changelog_index: u16,
    pub indexed_changelog_index: u16,
    pub value: u16,
    pub low_address_index: u64,
    pub low_address_value: [u8; 32],
    pub low_address_next_index: u64,
    pub low_address_next_value: [u8; 32],
    pub low_address_proof: [[u8; 32]; 16],
    pub is_metadata_forester: bool,
}

pub fn create_update_address_merkle_tree_instruction(
    inputs: UpdateAddressMerkleTreeInstructionInputs,
    epoch: u64,
) -> Instruction {
    let register_program_pda = get_registered_program_pda(&crate::ID);
    let registered_forester_pda = if inputs.is_metadata_forester {
        None
    } else {
        Some(get_forester_epoch_pda_from_authority(&inputs.derivation, epoch).0)
    };

    let (cpi_authority, bump) = get_cpi_authority_pda();
    let instruction_data = crate::instruction::UpdateAddressMerkleTree {
        bump,
        changelog_index: inputs.changelog_index,
        indexed_changelog_index: inputs.indexed_changelog_index,
        value: inputs.value,
        low_address_index: inputs.low_address_index,
        low_address_value: inputs.low_address_value,
        low_address_next_index: inputs.low_address_next_index,
        low_address_next_value: inputs.low_address_next_value,
        low_address_proof: inputs.low_address_proof,
    };

    let accounts = crate::accounts::UpdateAddressMerkleTree {
        authority: inputs.authority,
        registered_forester_pda,
        registered_program_pda: register_program_pda,
        merkle_tree: inputs.address_merkle_tree,
        queue: inputs.address_queue,
        log_wrapper: NOOP_PUBKEY.into(),
        cpi_authority,
        account_compression_program: account_compression::ID,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_initialize_address_merkle_tree_and_queue_instruction(
    payer: Pubkey,
    forester: Option<Pubkey>,
    program_owner: Option<Pubkey>,
    merkle_tree_pubkey: Pubkey,
    queue_pubkey: Pubkey,
    address_merkle_tree_config: AddressMerkleTreeConfig,
    address_queue_config: AddressQueueConfig,
) -> Instruction {
    let register_program_pda = get_registered_program_pda(&crate::ID);
    let (cpi_authority, bump) = get_cpi_authority_pda();

    let instruction_data = crate::instruction::InitializeAddressMerkleTree {
        bump,
        program_owner,
        forester,
        merkle_tree_config: address_merkle_tree_config,
        queue_config: address_queue_config,
    };
    let protocol_config_pda = get_protocol_config_pda_address().0;
    let accounts = crate::accounts::InitializeMerkleTreeAndQueue {
        authority: payer,
        registered_program_pda: register_program_pda,
        merkle_tree: merkle_tree_pubkey,
        queue: queue_pubkey,
        cpi_authority,
        account_compression_program: account_compression::ID,
        protocol_config_pda,
        light_system_program: None,
        cpi_context_account: None,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_initialize_merkle_tree_instruction(
    authority: Pubkey,
    merkle_tree_pubkey: Pubkey,
    nullifier_queue_pubkey: Pubkey,
    cpi_context_pubkey: Pubkey,
    state_merkle_tree_config: StateMerkleTreeConfig,
    nullifier_queue_config: NullifierQueueConfig,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
) -> Instruction {
    let register_program_pda = get_registered_program_pda(&crate::ID);
    let (cpi_authority, bump) = get_cpi_authority_pda();
    let protocol_config_pda = get_protocol_config_pda_address().0;
    let instruction_data = crate::instruction::InitializeStateMerkleTree {
        bump,
        program_owner,
        forester,
        merkle_tree_config: state_merkle_tree_config,
        queue_config: nullifier_queue_config,
    };
    let accounts = crate::accounts::InitializeMerkleTreeAndQueue {
        authority,
        registered_program_pda: register_program_pda,
        merkle_tree: merkle_tree_pubkey,
        queue: nullifier_queue_pubkey,
        cpi_authority,
        account_compression_program: account_compression::ID,
        protocol_config_pda,
        light_system_program: Some(LightSystemProgram::id()),
        cpi_context_account: Some(cpi_context_pubkey),
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_initialize_batched_merkle_tree_instruction(
    authority: Pubkey,
    merkle_tree_pubkey: Pubkey,
    queue_pubkey: Pubkey,
    cpi_context_pubkey: Pubkey,
    params: InitStateTreeAccountsInstructionData,
) -> Instruction {
    let register_program_pda = get_registered_program_pda(&crate::ID);
    let (cpi_authority, bump) = get_cpi_authority_pda();
    let protocol_config_pda = get_protocol_config_pda_address().0;
    let instruction_data = crate::instruction::InitializeBatchedStateMerkleTree {
        bump,
        params: params.try_to_vec().unwrap(),
    };
    let accounts = crate::accounts::InitializeBatchedStateMerkleTreeAndQueue {
        authority,
        registered_program_pda: register_program_pda,
        merkle_tree: merkle_tree_pubkey,
        queue: queue_pubkey,
        cpi_authority,
        account_compression_program: account_compression::ID,
        protocol_config_pda,
        light_system_program: LightSystemProgram::id(),
        cpi_context_account: cpi_context_pubkey,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_batch_append_instruction(
    forester: Pubkey,
    derivation_pubkey: Pubkey,
    merkle_tree_pubkey: Pubkey,
    output_queue_pubkey: Pubkey,
    epoch: u64,
    data: Vec<u8>,
) -> Instruction {
    let forester_epoch_pda = get_forester_epoch_pda_from_authority(&derivation_pubkey, epoch).0;
    let registered_program_pda = get_registered_program_pda(&crate::ID);

    let (cpi_authority_pda, bump) = get_cpi_authority_pda();
    let accounts = crate::accounts::BatchAppend {
        authority: forester,
        merkle_tree: merkle_tree_pubkey,
        output_queue: output_queue_pubkey,
        cpi_authority: cpi_authority_pda,
        registered_forester_pda: Some(forester_epoch_pda),
        registered_program_pda,
        account_compression_program: account_compression::ID,
        log_wrapper: NOOP_PUBKEY.into(),
        fee_payer: forester,
    };
    let instruction_data = crate::instruction::BatchAppend { bump, data };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_batch_nullify_instruction(
    forester: Pubkey,
    derivation_pubkey: Pubkey,
    merkle_tree_pubkey: Pubkey,
    epoch: u64,
    data: Vec<u8>,
) -> Instruction {
    let forester_epoch_pda = get_forester_epoch_pda_from_authority(&derivation_pubkey, epoch).0;
    let registered_program_pda = get_registered_program_pda(&crate::ID);

    let (cpi_authority_pda, bump) = get_cpi_authority_pda();
    let accounts = crate::accounts::BatchNullify {
        authority: forester,
        merkle_tree: merkle_tree_pubkey,
        cpi_authority: cpi_authority_pda,
        registered_forester_pda: Some(forester_epoch_pda),
        registered_program_pda,
        account_compression_program: account_compression::ID,
        log_wrapper: NOOP_PUBKEY.into(),
    };
    let instruction_data = crate::instruction::BatchNullify { bump, data };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_rollover_batch_state_tree_instruction(
    forester: Pubkey,
    derivation_pubkey: Pubkey,
    old_state_merkle_tree: Pubkey,
    new_state_merkle_tree: Pubkey,
    old_output_queue: Pubkey,
    new_output_queue: Pubkey,
    cpi_context_account: Pubkey,
    epoch: u64,
    light_forester: bool,
) -> Instruction {
    let register_program_pda = get_registered_program_pda(&crate::ID);
    let registered_forester_pda =
        get_forester_epoch_pda_from_authority(&derivation_pubkey, epoch).0;
    let (cpi_authority, bump) = get_cpi_authority_pda();
    let instruction_data = crate::instruction::RolloverBatchedStateMerkleTree { bump };
    let registered_forester_pda = if !light_forester {
        None
    } else {
        Some(registered_forester_pda)
    };

    let accounts = crate::accounts::RolloverBatchedStateMerkleTree {
        authority: forester,
        registered_forester_pda,
        registered_program_pda: register_program_pda,
        old_state_merkle_tree,
        new_state_merkle_tree,
        old_output_queue,
        new_output_queue,
        cpi_context_account,
        cpi_authority,
        account_compression_program: account_compression::ID,
        protocol_config_pda: get_protocol_config_pda_address().0,
        light_system_program: LightSystemProgram::id(),
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_initialize_batched_address_merkle_tree_instruction(
    authority: Pubkey,
    merkle_tree_pubkey: Pubkey,
    params: InitAddressTreeAccountsInstructionData,
) -> Instruction {
    let register_program_pda = get_registered_program_pda(&crate::ID);
    let (cpi_authority, bump) = get_cpi_authority_pda();

    let instruction_data = crate::instruction::InitializeBatchedAddressMerkleTree {
        bump,
        params: params.try_to_vec().unwrap(),
    };
    let protocol_config_pda = get_protocol_config_pda_address().0;
    let accounts = crate::accounts::InitializeBatchedAddressTree {
        authority,
        registered_program_pda: register_program_pda,
        merkle_tree: merkle_tree_pubkey,
        cpi_authority,
        account_compression_program: account_compression::ID,
        protocol_config_pda,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_batch_update_address_tree_instruction(
    forester: Pubkey,
    derivation_pubkey: Pubkey,
    merkle_tree_pubkey: Pubkey,
    epoch: u64,
    data: Vec<u8>,
) -> Instruction {
    let forester_epoch_pda = get_forester_epoch_pda_from_authority(&derivation_pubkey, epoch).0;
    let registered_program_pda = get_registered_program_pda(&crate::ID);

    let (cpi_authority_pda, bump) = get_cpi_authority_pda();
    let accounts = crate::accounts::BatchUpdateAddressTree {
        authority: forester,
        merkle_tree: merkle_tree_pubkey,
        cpi_authority: cpi_authority_pda,
        registered_forester_pda: Some(forester_epoch_pda),
        registered_program_pda,
        account_compression_program: account_compression::ID,
        log_wrapper: NOOP_PUBKEY.into(),
        fee_payer: forester,
    };
    let instruction_data = crate::instruction::BatchUpdateAddressTree { bump, data };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_rollover_batch_address_tree_instruction(
    forester: Pubkey,
    derivation_pubkey: Pubkey,
    old_merkle_tree: Pubkey,
    new_merkle_tree: Pubkey,
    epoch: u64,
) -> Instruction {
    let forester_epoch_pda = get_forester_epoch_pda_from_authority(&derivation_pubkey, epoch).0;
    let registered_program_pda = get_registered_program_pda(&crate::ID);

    let (cpi_authority_pda, bump) = get_cpi_authority_pda();
    let accounts = crate::accounts::RolloverBatchedAddressMerkleTree {
        authority: forester,
        new_address_merkle_tree: new_merkle_tree,
        old_address_merkle_tree: old_merkle_tree,
        cpi_authority: cpi_authority_pda,
        registered_forester_pda: Some(forester_epoch_pda),
        registered_program_pda,
        account_compression_program: account_compression::ID,
        protocol_config_pda: get_protocol_config_pda_address().0,
    };
    let instruction_data = crate::instruction::RolloverBatchedAddressMerkleTree { bump };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nullify_state_v1_multi_instruction_data_size() {
        // Worst case: max_nodes unique nodes
        let instruction_data = crate::instruction::NullifyStateV1Multi {
            change_log_index: 0,
            queue_indices: [0; 4],
            leaf_indices: [0; 4],
            proof_2_shared: 0,
            proof_3_source: 0,
            proof_4_source: 0,
            shared_top_node: [0u8; 32],
            nodes: vec![[0u8; 32]; NULLIFY_STATE_V1_MULTI_MAX_NODES],
        };
        let data = instruction_data.data();
        // 8 disc + 2 changelog + 8 queue_indices + 16 leaf_indices + 2 proof_2_shared
        // + 4 proof_3_source + 4 proof_4_source + 32 shared_top_node
        // + 4 vec_prefix + N*32 nodes
        let expected = 8 + 2 + 8 + 16 + 2 + 4 + 4 + 32 + 4 + NULLIFY_STATE_V1_MULTI_MAX_NODES * 32;
        assert_eq!(
            data.len(),
            expected,
            "nullify_state_v1_multi instruction data must be exactly {} bytes, got {}",
            expected,
            data.len()
        );
    }

    #[test]
    fn test_nullify_state_v1_multi_instruction_accounts() {
        let authority = Pubkey::new_unique();
        let inputs = CreateNullifyStateV1MultiInstructionInputs {
            authority,
            nullifier_queue: Pubkey::new_unique(),
            merkle_tree: Pubkey::new_unique(),
            change_log_index: 0,
            queue_indices: [0, 1, 2, 3],
            leaf_indices: [0, 1, 2, 3],
            proof_2_shared: 0,
            proof_3_source: 0,
            proof_4_source: 0,
            shared_top_node: [0u8; 32],
            nodes: vec![[0u8; 32]; 15],
            derivation: authority,
            is_metadata_forester: false,
        };
        let ix = create_nullify_state_v1_multi_instruction(inputs, 0);
        assert_eq!(ix.accounts.len(), 8, "expected 8 accounts");
    }

    #[test]
    fn test_compress_proofs_round_trip() {
        // Create 4 proofs with sharing patterns that fit within MAX_NODES (27).
        // Budget: 15 (proof_1) + 5 (proof_2 unique) + 5 (proof_3 unique) + 2 (proof_4 unique) = 27
        let shared_top = [0xCC; 32];
        let mut proof_1 = [[0u8; 32]; 16];
        let mut proof_2 = [[0u8; 32]; 16];
        let mut proof_3 = [[0u8; 32]; 16];
        let mut proof_4 = [[0u8; 32]; 16];

        for (i, slot) in proof_1.iter_mut().enumerate().take(15) {
            *slot = [i as u8 + 1; 32];
        }
        proof_1[15] = shared_top;

        // proof_2: 10 shared with proof_1, 5 unique (levels 0-4)
        for (i, slot) in proof_2.iter_mut().enumerate().take(15) {
            if i < 5 {
                *slot = [i as u8 + 100; 32]; // unique
            } else {
                *slot = proof_1[i]; // shared
            }
        }
        proof_2[15] = shared_top;

        // proof_3: 5 from proof_1, 5 new (levels 5-9), 5 from proof_2
        for (i, slot) in proof_3.iter_mut().enumerate().take(15) {
            if i < 5 {
                *slot = proof_1[i]; // same as proof_1
            } else if i < 10 {
                *slot = [i as u8 + 200; 32]; // new
            } else {
                *slot = proof_2[i]; // same as proof_2 (and proof_1)
            }
        }
        proof_3[15] = shared_top;

        // proof_4: 4 from proof_1, 4 from proof_2, 5 from proof_3, 2 new
        for (i, slot) in proof_4.iter_mut().enumerate().take(15) {
            if i < 4 {
                *slot = proof_1[i]; // from proof_1
            } else if i < 8 {
                *slot = proof_2[i]; // from proof_2
            } else if i < 13 {
                *slot = proof_3[i]; // from proof_3
            } else {
                *slot = [(i as u8).wrapping_add(250); 32]; // new
            }
        }
        proof_4[15] = shared_top;

        let proofs: Vec<&[[u8; 32]; 16]> = vec![&proof_1, &proof_2, &proof_3, &proof_4];
        let result = compress_proofs(&proofs);
        assert!(result.is_some(), "compress_proofs should succeed");
        let CompressedProofs {
            proof_2_shared: p2_shared,
            proof_3_source: p3_source,
            proof_4_source: p4_source,
            shared_top_node: top,
            nodes,
        } = result.unwrap();

        // Simulate on-chain reconstruction
        let mut cursor = 0usize;

        // Reconstruct proof_1
        let mut r_proof_1 = [[0u8; 32]; 16];
        r_proof_1[..15].copy_from_slice(&nodes[cursor..cursor + 15]);
        r_proof_1[15] = top;
        cursor += 15;
        assert_eq!(r_proof_1, proof_1);

        // Reconstruct proof_2
        let mut r_proof_2 = [[0u8; 32]; 16];
        for i in 0..15 {
            if (p2_shared >> i) & 1 == 1 {
                r_proof_2[i] = r_proof_1[i];
            } else {
                r_proof_2[i] = nodes[cursor];
                cursor += 1;
            }
        }
        r_proof_2[15] = top;
        assert_eq!(r_proof_2, proof_2);

        // Reconstruct proof_3
        let mut r_proof_3 = [[0u8; 32]; 16];
        for i in 0..15 {
            let src = (p3_source >> (i * 2)) & 0b11;
            match src {
                0b00 => r_proof_3[i] = r_proof_1[i],
                0b01 => r_proof_3[i] = r_proof_2[i],
                0b10 => {
                    r_proof_3[i] = nodes[cursor];
                    cursor += 1;
                }
                _ => panic!("unexpected source 0b11 for proof_3"),
            }
        }
        r_proof_3[15] = top;
        assert_eq!(r_proof_3, proof_3);

        // Reconstruct proof_4
        let mut r_proof_4 = [[0u8; 32]; 16];
        for i in 0..15 {
            let src = (p4_source >> (i * 2)) & 0b11;
            match src {
                0b00 => r_proof_4[i] = r_proof_1[i],
                0b01 => r_proof_4[i] = r_proof_2[i],
                0b10 => r_proof_4[i] = r_proof_3[i],
                0b11 => {
                    r_proof_4[i] = nodes[cursor];
                    cursor += 1;
                }
                _ => unreachable!(),
            }
        }
        r_proof_4[15] = top;
        assert_eq!(r_proof_4, proof_4);

        assert_eq!(cursor, nodes.len(), "all nodes should be consumed");
    }

    #[test]
    fn test_compress_proofs_returns_none_when_too_many_nodes() {
        // All 4 proofs with completely unique nodes at every level = 15 + 15 + 15 + 15 = 60 nodes
        let shared_top = [0xCC; 32];
        let make_proof = |base: u8| -> [[u8; 32]; 16] {
            let mut p = [[0u8; 32]; 16];
            for (i, slot) in p.iter_mut().enumerate().take(15) {
                *slot = [base.wrapping_add(i as u8); 32];
            }
            p[15] = shared_top;
            p
        };
        let p1 = make_proof(1);
        let p2 = make_proof(50);
        let p3 = make_proof(100);
        let p4 = make_proof(150);

        let proofs: Vec<&[[u8; 32]; 16]> = vec![&p1, &p2, &p3, &p4];
        let result = compress_proofs(&proofs);
        assert!(
            result.is_none(),
            "should return None when no sharing leads to > MAX_NODES"
        );
    }

    #[test]
    fn test_compress_proofs_2_proofs() {
        let shared_top = [0xCC; 32];
        let mut proof_1 = [[0u8; 32]; 16];
        let mut proof_2 = [[0u8; 32]; 16];
        for i in 0..15 {
            proof_1[i] = [i as u8 + 1; 32];
            // Share half the nodes
            if i % 2 == 0 {
                proof_2[i] = proof_1[i];
            } else {
                proof_2[i] = [i as u8 + 100; 32];
            }
        }
        proof_1[15] = shared_top;
        proof_2[15] = shared_top;

        let proofs: Vec<&[[u8; 32]; 16]> = vec![&proof_1, &proof_2];
        let result = compress_proofs(&proofs);
        assert!(result.is_some(), "2 proofs should compress");
        let CompressedProofs {
            proof_2_shared: p2_shared,
            proof_3_source: p3_source,
            proof_4_source: p4_source,
            shared_top_node: top,
            nodes,
        } = result.unwrap();

        // proof_3_source and proof_4_source should be 0 (unused)
        assert_eq!(p3_source, 0);
        assert_eq!(p4_source, 0);
        assert_eq!(top, shared_top);

        // Verify proof_2_shared bitvec
        for i in 0..15 {
            if i % 2 == 0 {
                assert_eq!((p2_shared >> i) & 1, 1, "level {} should be shared", i);
            } else {
                assert_eq!((p2_shared >> i) & 1, 0, "level {} should not be shared", i);
            }
        }

        // 15 for proof_1 + 7 unique for proof_2 (odd indices 1,3,5,7,9,11,13)
        assert_eq!(nodes.len(), 15 + 7);
    }

    #[test]
    fn test_compress_proofs_3_proofs() {
        let shared_top = [0xCC; 32];
        let mut proof_1 = [[0u8; 32]; 16];
        let mut proof_2 = [[0u8; 32]; 16];
        let mut proof_3 = [[0u8; 32]; 16];
        for i in 0..15 {
            proof_1[i] = [i as u8 + 1; 32];
            // proof_2 shares some levels with proof_1 to stay within MAX_NODES
            if i % 2 == 0 {
                proof_2[i] = proof_1[i]; // shared
            } else {
                proof_2[i] = [i as u8 + 50; 32];
            }
            // proof_3 alternates between proof_1 and proof_2
            if i % 3 == 0 {
                proof_3[i] = proof_1[i];
            } else if i % 3 == 1 {
                proof_3[i] = proof_2[i];
            } else {
                proof_3[i] = [i as u8 + 100; 32]; // new
            }
        }
        proof_1[15] = shared_top;
        proof_2[15] = shared_top;
        proof_3[15] = shared_top;

        let proofs: Vec<&[[u8; 32]; 16]> = vec![&proof_1, &proof_2, &proof_3];
        let result = compress_proofs(&proofs);
        assert!(result.is_some(), "3 proofs should compress");
        let CompressedProofs {
            proof_4_source: p4_source,
            ..
        } = result.unwrap();
        assert_eq!(p4_source, 0, "proof_4_source should be 0 for 3 proofs");
    }
}
