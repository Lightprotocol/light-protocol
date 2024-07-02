#![cfg(not(target_os = "solana"))]
use crate::get_forester_epoch_pda_address;
use account_compression::{
    self, utils::constants::GROUP_AUTHORITY_SEED, AddressMerkleTreeConfig, AddressQueueConfig,
    NullifierQueueConfig, StateMerkleTreeConfig, ID,
};
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use light_macros::pubkey;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
// TODO: move to non program sdk
pub const NOOP_PROGRAM_ID: Pubkey = pubkey!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

pub fn create_initialize_group_authority_instruction(
    signer_pubkey: Pubkey,
    group_accounts: Pubkey,
    seed_pubkey: Pubkey,
    authority: Pubkey,
) -> Instruction {
    let instruction_data = account_compression::instruction::InitializeGroupAuthority { authority };

    Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new(seed_pubkey, true),
            AccountMeta::new(group_accounts, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: instruction_data.data(),
    }
}

pub fn create_update_authority_instruction(
    signer_pubkey: Pubkey,
    new_authority: Pubkey,
) -> Instruction {
    let authority_pda = get_governance_authority_pda();
    let update_authority_ix = crate::instruction::UpdateGovernanceAuthority {
        bump: authority_pda.1,
        new_authority,
    };

    // update with new authority
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new(authority_pda.0, false),
        ],
        data: update_authority_ix.data(),
    }
}

pub fn create_register_program_instruction(
    signer_pubkey: Pubkey,
    authority_pda: (Pubkey, u8),
    group_account: Pubkey,
    program_id_to_be_registered: Pubkey,
) -> (Instruction, Pubkey) {
    let cpi_authority_pda = get_cpi_authority_pda();
    let registered_program_pda =
        Pubkey::find_program_address(&[program_id_to_be_registered.to_bytes().as_slice()], &ID).0;

    let register_program_ix = crate::instruction::RegisterSystemProgram {
        bump: cpi_authority_pda.1,
    };

    let instruction = Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new(authority_pda.0, false),
            AccountMeta::new(cpi_authority_pda.0, false),
            AccountMeta::new(group_account, false),
            AccountMeta::new_readonly(ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new(registered_program_pda, false),
            AccountMeta::new(program_id_to_be_registered, true),
        ],
        data: register_program_ix.data(),
    };
    (instruction, registered_program_pda)
}

pub fn get_governance_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[crate::AUTHORITY_PDA_SEED], &crate::ID)
}

pub fn get_cpi_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[crate::CPI_AUTHORITY_PDA_SEED], &crate::ID)
}

pub fn create_initialize_governance_authority_instruction(
    signer_pubkey: Pubkey,
    authority: Pubkey,
) -> Instruction {
    let authority_pda = get_governance_authority_pda();
    let ix = crate::instruction::InitializeGovernanceAuthority {
        bump: authority_pda.1,
        authority,
        rewards: vec![],
    };

    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new(authority_pda.0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: ix.data(),
    }
}
pub fn get_group_pda(seed: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[GROUP_AUTHORITY_SEED, seed.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0
}

pub fn create_register_forester_instruction(
    governance_authority: &Pubkey,
    forester_authority: &Pubkey,
) -> Instruction {
    let (forester_epoch_pda, _bump) = get_forester_epoch_pda_address(forester_authority);
    let instruction_data = crate::instruction::RegisterForester {
        _bump: 0,
        authority: *forester_authority,
    };
    let (authority_pda, _) = get_governance_authority_pda();
    let accounts = crate::accounts::RegisterForester {
        forester_epoch_pda,
        signer: *governance_authority,
        authority_pda,
        system_program: solana_sdk::system_program::id(),
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_update_forester_instruction(
    forester_authority: &Pubkey,
    new_authority: &Pubkey,
) -> Instruction {
    let (forester_epoch_pda, _bump) = get_forester_epoch_pda_address(forester_authority);
    let instruction_data = crate::instruction::UpdateForesterEpochPda {
        authority: *new_authority,
    };
    let accounts = crate::accounts::UpdateForesterEpochPda {
        forester_epoch_pda,
        signer: *forester_authority,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub struct CreateNullifyInstructionInputs {
    pub authority: Pubkey,
    pub nullifier_queue: Pubkey,
    pub merkle_tree: Pubkey,
    pub change_log_indices: Vec<u64>,
    pub leaves_queue_indices: Vec<u16>,
    pub indices: Vec<u64>,
    pub proofs: Vec<Vec<[u8; 32]>>,
    pub derivation: Pubkey,
}

pub fn create_nullify_instruction(inputs: CreateNullifyInstructionInputs) -> Instruction {
    let register_program_pda = get_registered_program_pda(&crate::ID);
    let registered_forester_pda = get_forester_epoch_pda_address(&inputs.derivation).0;
    log::info!("registered_forester_pda: {:?}", registered_forester_pda);
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
        log_wrapper: NOOP_PROGRAM_ID,
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
    pub new_queue: Pubkey,
    pub new_merkle_tree: Pubkey,
    pub old_queue: Pubkey,
    pub old_merkle_tree: Pubkey,
}

pub fn create_rollover_address_merkle_tree_instruction(
    inputs: CreateRolloverMerkleTreeInstructionInputs,
) -> Instruction {
    let (_, bump) = crate::sdk::get_cpi_authority_pda();

    let instruction_data = crate::instruction::RolloverAddressMerkleTreeAndQueue { bump };
    create_rollover_instruction(instruction_data.data(), inputs)
}

pub fn create_rollover_state_merkle_tree_instruction(
    inputs: CreateRolloverMerkleTreeInstructionInputs,
) -> Instruction {
    let (_, bump) = crate::sdk::get_cpi_authority_pda();

    let instruction_data = crate::instruction::RolloverStateMerkleTreeAndQueue { bump };
    create_rollover_instruction(instruction_data.data(), inputs)
}

pub fn create_rollover_instruction(
    data: Vec<u8>,
    inputs: CreateRolloverMerkleTreeInstructionInputs,
) -> Instruction {
    let (cpi_authority, _) = crate::sdk::get_cpi_authority_pda();
    let registered_program_pda = get_registered_program_pda(&crate::ID);
    let registered_forester_pda = get_forester_epoch_pda_address(&inputs.authority).0;
    let accounts = crate::accounts::RolloverMerkleTreeAndQueue {
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
        data,
    }
}

pub struct UpdateAddressMerkleTreeInstructionInputs {
    pub authority: Pubkey,
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
}

pub fn create_update_address_merkle_tree_instruction(
    instructions: UpdateAddressMerkleTreeInstructionInputs,
) -> Instruction {
    let register_program_pda = get_registered_program_pda(&crate::ID);
    let registered_forester_pda = get_forester_epoch_pda_address(&instructions.authority).0;

    let (cpi_authority, bump) = get_cpi_authority_pda();
    let instruction_data = crate::instruction::UpdateAddressMerkleTree {
        bump,
        changelog_index: instructions.changelog_index,
        indexed_changelog_index: instructions.indexed_changelog_index,
        value: instructions.value,
        low_address_index: instructions.low_address_index,
        low_address_value: instructions.low_address_value,
        low_address_next_index: instructions.low_address_next_index,
        low_address_next_value: instructions.low_address_next_value,
        low_address_proof: instructions.low_address_proof,
    };

    let accounts = crate::accounts::UpdateAddressMerkleTree {
        authority: instructions.authority,
        registered_forester_pda,
        registered_program_pda: register_program_pda,
        merkle_tree: instructions.address_merkle_tree,
        queue: instructions.address_queue,
        log_wrapper: NOOP_PROGRAM_ID,
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
    index: u64,
    payer: Pubkey,
    program_owner: Option<Pubkey>,
    merkle_tree_pubkey: Pubkey,
    queue_pubkey: Pubkey,
    address_merkle_tree_config: AddressMerkleTreeConfig,
    address_queue_config: AddressQueueConfig,
) -> Instruction {
    let register_program_pda = get_registered_program_pda(&crate::ID);
    let (cpi_authority, bump) = crate::sdk::get_cpi_authority_pda();

    let instruction_data = crate::instruction::InitializeAddressMerkleTree {
        bump,
        index,
        program_owner,
        merkle_tree_config: address_merkle_tree_config,
        queue_config: address_queue_config,
    };
    let accounts = crate::accounts::InitializeAddressMerkleTreeAndQueue {
        authority: payer,
        registered_program_pda: register_program_pda,
        merkle_tree: merkle_tree_pubkey,
        queue: queue_pubkey,
        cpi_authority,
        account_compression_program: account_compression::ID,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_initialize_merkle_tree_instruction(
    payer: Pubkey,
    merkle_tree_pubkey: Pubkey,
    nullifier_queue_pubkey: Pubkey,
    state_merkle_tree_config: StateMerkleTreeConfig,
    nullifier_queue_config: NullifierQueueConfig,
    program_owner: Option<Pubkey>,
    index: u64,
    additional_rent: u64,
) -> Instruction {
    let register_program_pda = get_registered_program_pda(&crate::ID);
    let (cpi_authority, bump) = crate::sdk::get_cpi_authority_pda();

    let instruction_data = crate::instruction::InitializeStateMerkleTree {
        bump,
        index,
        program_owner,
        merkle_tree_config: state_merkle_tree_config,
        queue_config: nullifier_queue_config,
        additional_rent,
    };
    let accounts = crate::accounts::InitializeAddressMerkleTreeAndQueue {
        authority: payer,
        registered_program_pda: register_program_pda,
        merkle_tree: merkle_tree_pubkey,
        queue: nullifier_queue_pubkey,
        cpi_authority,
        account_compression_program: account_compression::ID,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}
