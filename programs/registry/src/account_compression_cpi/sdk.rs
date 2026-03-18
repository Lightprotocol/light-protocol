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
pub const NULLIFY_STATE_V1_MULTI_MAX_NODES: usize = 27;

#[derive(Clone, Debug, PartialEq)]
pub struct CreateNullifyStateV1MultiInstructionInputs {
    pub authority: Pubkey,
    pub nullifier_queue: Pubkey,
    pub merkle_tree: Pubkey,
    pub change_log_index: u16,
    pub queue_indices: [u16; 4],
    pub leaf_indices: [u32; 4],
    pub proof_bitvecs: [u32; 4],
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
        proof_bitvecs: inputs.proof_bitvecs,
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

/// Result of compressing 2-4 Merkle proofs into a deduplicated node pool.
pub struct CompressedProofs {
    /// Bitvecs for proofs 2-4, each selecting 16 nodes from the pool.
    /// proof_1 is always nodes[0..16].
    pub proof_bitvecs: [u32; 4],
    pub nodes: Vec<[u8; 32]>,
}

/// Compresses 2-4 full 16-node Merkle proofs into a deduplicated node pool.
/// The pool is built level-by-level so that iterating set bits in ascending
/// order produces nodes in proof-level order.
/// Proof 1 is always nodes[0..16]. Proofs 2-4 each have a bitvec selecting
/// which pool nodes form that proof.
/// Returns `None` if fewer than 2, more than 4 proofs, or too many unique nodes.
pub fn compress_proofs(proofs: &[&[[u8; 32]; 16]]) -> Option<CompressedProofs> {
    use bitvec::prelude::*;

    if proofs.len() < 2 || proofs.len() > 4 {
        return None;
    }

    // Build level-ordered deduplicated pool. For each level, add unique
    // nodes across all proofs. Ascending pool index == ascending level.
    let mut nodes: Vec<[u8; 32]> = Vec::new();
    let mut pool_indices = [[0usize; 16]; 4];

    for level in 0..16 {
        for (proof_idx, proof) in proofs.iter().enumerate() {
            if let Some(idx) = nodes.iter().position(|n| *n == proof[level]) {
                pool_indices[proof_idx][level] = idx;
            } else {
                pool_indices[proof_idx][level] = nodes.len();
                nodes.push(proof[level]);
            }
        }
    }

    if nodes.len() > NULLIFY_STATE_V1_MULTI_MAX_NODES || nodes.len() > 32 {
        return None;
    }

    let mut proof_bitvecs = [0u32; 4];
    for (proof_idx, _) in proofs.iter().enumerate() {
        let bv = proof_bitvecs[proof_idx].view_bits_mut::<Lsb0>();
        for level in 0..16 {
            bv.set(pool_indices[proof_idx][level], true);
        }
    }

    Some(CompressedProofs {
        proof_bitvecs,
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
    use bitvec::prelude::*;

    use super::*;

    /// Simulates on-chain reconstruction for testing round-trips.
    fn reconstruct_proof(nodes: &[[u8; 32]], bits: u32) -> [[u8; 32]; 16] {
        let bv = bits.view_bits::<Lsb0>();
        let mut proof = [[0u8; 32]; 16];
        let mut proof_idx = 0;
        for (i, node) in nodes.iter().enumerate() {
            if bv[i] {
                proof[proof_idx] = *node;
                proof_idx += 1;
            }
        }
        assert_eq!(proof_idx, 16, "bitvec must select exactly 16 nodes");
        proof
    }

    #[test]
    fn test_nullify_state_v1_multi_instruction_data_size() {
        let instruction_data = crate::instruction::NullifyStateV1Multi {
            change_log_index: 0,
            queue_indices: [0; 4],
            leaf_indices: [0; 4],
            proof_bitvecs: [0; 4],
            nodes: vec![[0u8; 32]; NULLIFY_STATE_V1_MULTI_MAX_NODES],
        };
        let data = instruction_data.data();
        // 8 disc + 2 changelog + 8 queue_indices + 16 leaf_indices + 16 proof_bitvecs
        // + 4 vec_prefix + N*32 nodes
        let expected = 8 + 2 + 8 + 16 + 16 + 4 + NULLIFY_STATE_V1_MULTI_MAX_NODES * 32;
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
            proof_bitvecs: [0; 4],
            nodes: vec![[0u8; 32]; 16],
            derivation: authority,
            is_metadata_forester: false,
        };
        let ix = create_nullify_state_v1_multi_instruction(inputs, 0);
        assert_eq!(ix.accounts.len(), 8, "expected 8 accounts");
    }

    #[test]
    fn test_compress_proofs_round_trip() {
        let mut proof_1 = [[0u8; 32]; 16];
        let mut proof_2 = [[0u8; 32]; 16];
        let mut proof_3 = [[0u8; 32]; 16];
        let mut proof_4 = [[0u8; 32]; 16];

        for (i, elem) in proof_1.iter_mut().enumerate() {
            *elem = [i as u8 + 1; 32];
        }

        // proof_2: differs at levels 0-3, shares 4-15 (total: 16 + 4 = 20)
        for i in 0..16 {
            if i < 4 {
                proof_2[i] = [i as u8 + 100; 32];
            } else {
                proof_2[i] = proof_1[i];
            }
        }

        // proof_3: differs at levels 0-2, shares 3-15 (total: 20 + 3 = 23)
        for i in 0..16 {
            if i < 3 {
                proof_3[i] = [i as u8 + 200; 32];
            } else {
                proof_3[i] = proof_1[i];
            }
        }

        // proof_4: differs at levels 0-1, shares 2-15 (total: 23 + 2 = 25)
        for i in 0..16 {
            if i < 2 {
                proof_4[i] = [(i as u8).wrapping_add(250); 32];
            } else {
                proof_4[i] = proof_1[i];
            }
        }

        let proofs: Vec<&[[u8; 32]; 16]> = vec![&proof_1, &proof_2, &proof_3, &proof_4];
        let result = compress_proofs(&proofs);
        assert!(result.is_some(), "compress_proofs should succeed");
        let compressed = result.unwrap();

        let r_proof_1 = reconstruct_proof(&compressed.nodes, compressed.proof_bitvecs[0]);
        assert_eq!(r_proof_1, proof_1);

        let r_proof_2 = reconstruct_proof(&compressed.nodes, compressed.proof_bitvecs[1]);
        assert_eq!(r_proof_2, proof_2);

        let r_proof_3 = reconstruct_proof(&compressed.nodes, compressed.proof_bitvecs[2]);
        assert_eq!(r_proof_3, proof_3);

        let r_proof_4 = reconstruct_proof(&compressed.nodes, compressed.proof_bitvecs[3]);
        assert_eq!(r_proof_4, proof_4);
    }

    #[test]
    fn test_compress_proofs_returns_none_when_too_many_nodes() {
        let make_proof = |base: u8| -> [[u8; 32]; 16] {
            let mut p = [[0u8; 32]; 16];
            for (i, slot) in p.iter_mut().enumerate() {
                *slot = [base.wrapping_add(i as u8); 32];
            }
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
        let mut proof_1 = [[0u8; 32]; 16];
        let mut proof_2 = [[0u8; 32]; 16];
        for i in 0..16 {
            proof_1[i] = [i as u8 + 1; 32];
            if i % 2 == 0 {
                proof_2[i] = proof_1[i];
            } else {
                proof_2[i] = [i as u8 + 100; 32];
            }
        }

        let proofs: Vec<&[[u8; 32]; 16]> = vec![&proof_1, &proof_2];
        let result = compress_proofs(&proofs);
        assert!(result.is_some(), "2 proofs should compress");
        let compressed = result.unwrap();

        // Unused bitvecs should be 0
        assert_eq!(compressed.proof_bitvecs[2], 0);
        assert_eq!(compressed.proof_bitvecs[3], 0);

        // 16 for proof_1 + 8 unique for proof_2 (odd indices)
        assert_eq!(compressed.nodes.len(), 16 + 8);

        // Round-trip
        let r_proof_1 = reconstruct_proof(&compressed.nodes, compressed.proof_bitvecs[0]);
        assert_eq!(r_proof_1, proof_1);

        let r_proof_2 = reconstruct_proof(&compressed.nodes, compressed.proof_bitvecs[1]);
        assert_eq!(r_proof_2, proof_2);
    }

    #[test]
    fn test_compress_proofs_3_proofs() {
        let mut proof_1 = [[0u8; 32]; 16];
        let mut proof_2 = [[0u8; 32]; 16];
        let mut proof_3 = [[0u8; 32]; 16];
        for i in 0..16 {
            proof_1[i] = [i as u8 + 1; 32];
            if i % 2 == 0 {
                proof_2[i] = proof_1[i];
            } else {
                proof_2[i] = [i as u8 + 50; 32];
            }
            if i % 3 == 0 {
                proof_3[i] = proof_1[i];
            } else {
                proof_3[i] = proof_2[i];
            }
        }

        let proofs: Vec<&[[u8; 32]; 16]> = vec![&proof_1, &proof_2, &proof_3];
        let result = compress_proofs(&proofs);
        assert!(result.is_some(), "3 proofs should compress");
        let compressed = result.unwrap();
        assert_eq!(
            compressed.proof_bitvecs[3], 0,
            "proof_4 bitvec should be 0 for 3 proofs"
        );

        let r_proof_1 = reconstruct_proof(&compressed.nodes, compressed.proof_bitvecs[0]);
        assert_eq!(r_proof_1, proof_1);

        let r_proof_2 = reconstruct_proof(&compressed.nodes, compressed.proof_bitvecs[1]);
        assert_eq!(r_proof_2, proof_2);

        let r_proof_3 = reconstruct_proof(&compressed.nodes, compressed.proof_bitvecs[2]);
        assert_eq!(r_proof_3, proof_3);
    }
}
