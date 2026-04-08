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

    let (reimbursement_pda, _) = get_reimbursement_pda(&inputs.merkle_tree);
    let accounts = crate::accounts::NullifyLeaves {
        authority: inputs.authority,
        registered_forester_pda,
        registered_program_pda: register_program_pda,
        nullifier_queue: inputs.nullifier_queue,
        merkle_tree: inputs.merkle_tree,
        log_wrapper: NOOP_PUBKEY.into(),
        cpi_authority,
        account_compression_program: account_compression::ID,
        reimbursement_pda,
        system_program: solana_sdk::system_program::id(),
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
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

pub fn get_reimbursement_pda(merkle_tree_pubkey: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            crate::fee_reimbursement::initialize::REIMBURSEMENT_PDA_SEED,
            merkle_tree_pubkey.as_ref(),
        ],
        &crate::ID,
    )
}

pub fn create_init_reimbursement_pda_instruction(
    payer: Pubkey,
    merkle_tree_pubkey: Pubkey,
) -> Instruction {
    let (reimbursement_pda, _) = get_reimbursement_pda(&merkle_tree_pubkey);
    let accounts = crate::accounts::InitReimbursementPda {
        payer,
        reimbursement_pda,
        tree: merkle_tree_pubkey,
        system_program: solana_sdk::system_program::id(),
    };
    let instruction_data = crate::instruction::InitReimbursementPda {};
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_claim_fees_wrapper_instruction(
    forester: Pubkey,
    derivation_pubkey: Pubkey,
    merkle_tree_or_queue: Pubkey,
    fee_recipient: Pubkey,
    protocol_config_pda: Pubkey,
    epoch: u64,
) -> Instruction {
    let forester_epoch_pda = get_forester_epoch_pda_from_authority(&derivation_pubkey, epoch).0;
    let registered_program_pda = get_registered_program_pda(&crate::ID);
    let (cpi_authority_pda, bump) = get_cpi_authority_pda();
    let accounts = crate::accounts::ClaimFeesWrapper {
        registered_forester_pda: Some(forester_epoch_pda),
        authority: forester,
        cpi_authority: cpi_authority_pda,
        registered_program_pda,
        account_compression_program: account_compression::ID,
        merkle_tree_or_queue,
        protocol_config_pda,
        fee_recipient,
    };
    let instruction_data = crate::instruction::ClaimFees { bump };
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
    let (reimbursement_pda, _) = get_reimbursement_pda(&merkle_tree_pubkey);

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
        reimbursement_pda,
        system_program: solana_sdk::system_program::id(),
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
    let (reimbursement_pda, _) = get_reimbursement_pda(&merkle_tree_pubkey);
    let accounts = crate::accounts::BatchNullify {
        authority: forester,
        merkle_tree: merkle_tree_pubkey,
        cpi_authority: cpi_authority_pda,
        registered_forester_pda: Some(forester_epoch_pda),
        registered_program_pda,
        account_compression_program: account_compression::ID,
        log_wrapper: NOOP_PUBKEY.into(),
        reimbursement_pda,
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
