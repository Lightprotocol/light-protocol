#![cfg(not(target_os = "solana"))]
use account_compression::{self, utils::constants::GROUP_AUTHORITY_SEED, ID};
use anchor_lang::{system_program, InstructionData};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

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
