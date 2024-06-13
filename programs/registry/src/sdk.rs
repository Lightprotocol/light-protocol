#![cfg(not(target_os = "solana"))]
use crate::{
    protocol_config::state::ProtocolConfig,
    utils::{
        get_cpi_authority_pda, get_epoch_pda_address, get_forester_epoch_pda_address,
        get_forester_pda_address, get_protocol_config_pda_address,
    },
    ForesterConfig,
};
use account_compression::{self, ID};
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
    let authority_pda = get_protocol_config_pda_address();
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
    let register_program_accounts = crate::accounts::RegisteredProgram {
        authority: signer_pubkey,
        program_to_be_registered: program_id_to_be_registered,
        registered_program_pda,
        authority_pda: authority_pda.0,
        group_pda: group_account,
        cpi_authority: cpi_authority_pda.0,
        account_compression_program: ID,
        system_program: system_program::ID,
    };

    let instruction = Instruction {
        program_id: crate::ID,
        accounts: register_program_accounts.to_account_metas(Some(true)),
        data: register_program_ix.data(),
    };
    (instruction, registered_program_pda)
}

pub fn create_initialize_governance_authority_instruction(
    signer_pubkey: Pubkey,
    protocol_config: ProtocolConfig,
) -> Instruction {
    let authority_pda = get_protocol_config_pda_address();
    let ix = crate::instruction::InitializeGovernanceAuthority {
        bump: authority_pda.1,
        protocol_config,
    };

    let accounts = crate::accounts::InitializeAuthority {
        authority_pda: authority_pda.0,
        authority: signer_pubkey,
        system_program: system_program::ID,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: ix.data(),
    }
}

pub fn create_register_forester_instruction(
    governance_authority: &Pubkey,
    forester_authority: &Pubkey,
    config: ForesterConfig,
) -> Instruction {
    let (forester_pda, _bump) = get_forester_pda_address(forester_authority);
    let instruction_data = crate::instruction::RegisterForester {
        _bump: 0,
        authority: *forester_authority,
        config,
    };
    let (authority_pda, _) = get_protocol_config_pda_address();
    let accounts = crate::accounts::RegisterForester {
        forester_pda,
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

pub fn create_update_forester_epoch_pda_instruction(
    forester_authority: &Pubkey,
    new_authority: &Pubkey,
) -> Instruction {
    let (forester_epoch_pda, _bump) = get_forester_pda_address(forester_authority);
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

pub fn create_update_forester_pda_instruction(
    forester_authority: &Pubkey,
    new_authority: Option<Pubkey>,
    config: ForesterConfig,
) -> Instruction {
    let (forester_pda, _) = get_forester_pda_address(forester_authority);
    let instruction_data = crate::instruction::UpdateForester { config };
    let accounts = crate::accounts::UpdateForester {
        forester_pda,
        authority: *forester_authority,
        new_authority,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_register_forester_epoch_pda_instruction(
    authority: &Pubkey,
    epoch: u64,
) -> Instruction {
    let (forester_epoch_pda, _bump) = get_forester_epoch_pda_address(authority, epoch);
    let (forester_pda, _) = get_forester_pda_address(authority);
    let epoch_pda = get_epoch_pda_address(epoch);
    let protocol_config_pda = get_protocol_config_pda_address().0;
    let instruction_data = crate::instruction::RegisterForesterEpoch { epoch };
    let accounts = crate::accounts::RegisterForesterEpoch {
        forester_epoch_pda,
        forester_pda,
        authority: *authority,
        epoch_pda,
        protocol_config: protocol_config_pda,
        system_program: solana_sdk::system_program::id(),
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_finalize_registration_instruction(authority: &Pubkey, epoch: u64) -> Instruction {
    let (forester_epoch_pda, _bump) = get_forester_epoch_pda_address(authority, epoch);
    let epoch_pda = get_epoch_pda_address(epoch);
    let instruction_data = crate::instruction::FinalizeRegistration {};
    let accounts = crate::accounts::FinalizeRegistration {
        forester_epoch_pda,
        authority: *authority,
        epoch_pda,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_report_work_instruction(authority: &Pubkey, epoch: u64) -> Instruction {
    let (forester_epoch_pda, _bump) = get_forester_epoch_pda_address(authority, epoch);
    let epoch_pda = get_epoch_pda_address(epoch);
    let instruction_data = crate::instruction::ReportWork {};
    let accounts = crate::accounts::ReportWork {
        authority: *authority,
        forester_epoch_pda,
        epoch_pda,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}
