#![cfg(not(target_os = "solana"))]
use account_compression::{self, ID};
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use crate::{
    protocol_config::state::ProtocolConfig,
    utils::{
        get_cpi_authority_pda, get_epoch_pda_address, get_forester_epoch_pda_from_authority,
        get_forester_epoch_pda_from_derivation, get_forester_pda, get_protocol_config_pda_address,
    },
    ForesterConfig,
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

pub fn create_update_protocol_config_instruction(
    signer_pubkey: Pubkey,
    new_authority: Option<Pubkey>,
    protocol_config: Option<ProtocolConfig>,
) -> Instruction {
    let protocol_config_pda = get_protocol_config_pda_address();
    let update_authority_ix = crate::instruction::UpdateProtocolConfig { protocol_config };
    let accounts = crate::accounts::UpdateProtocolConfig {
        protocol_config_pda: protocol_config_pda.0,
        authority: signer_pubkey,
        fee_payer: signer_pubkey,
        new_authority,
    };

    // update with new authority
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: update_authority_ix.data(),
    }
}

pub fn create_register_program_instruction(
    signer_pubkey: Pubkey,
    protocol_config_pda: (Pubkey, u8),
    group_account: Pubkey,
    program_id_to_be_registered: Pubkey,
) -> (Instruction, Pubkey) {
    let cpi_authority_pda = get_cpi_authority_pda();
    let registered_program_pda =
        Pubkey::find_program_address(&[program_id_to_be_registered.to_bytes().as_slice()], &ID).0;

    let register_program_ix = crate::instruction::RegisterSystemProgram {
        bump: cpi_authority_pda.1,
    };
    let register_program_accounts = crate::accounts::RegisterProgram {
        authority: signer_pubkey,
        program_to_be_registered: program_id_to_be_registered,
        registered_program_pda,
        protocol_config_pda: protocol_config_pda.0,
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

pub fn create_deregister_program_instruction(
    signer_pubkey: Pubkey,
    protocol_config_pda: (Pubkey, u8),
    group_account: Pubkey,
    program_id_to_be_deregistered: Pubkey,
) -> (Instruction, Pubkey) {
    let cpi_authority_pda = get_cpi_authority_pda();
    let registered_program_pda =
        Pubkey::find_program_address(&[program_id_to_be_deregistered.to_bytes().as_slice()], &ID).0;

    let register_program_ix = crate::instruction::DeregisterSystemProgram {
        bump: cpi_authority_pda.1,
    };
    let register_program_accounts = crate::accounts::DeregisterProgram {
        authority: signer_pubkey,
        registered_program_pda,
        protocol_config_pda: protocol_config_pda.0,
        group_pda: group_account,
        cpi_authority: cpi_authority_pda.0,
        account_compression_program: ID,
    };

    let instruction = Instruction {
        program_id: crate::ID,
        accounts: register_program_accounts.to_account_metas(Some(true)),
        data: register_program_ix.data(),
    };
    (instruction, registered_program_pda)
}

pub fn create_initialize_governance_authority_instruction(
    fee_payer: Pubkey,
    authority: Pubkey,
    protocol_config: ProtocolConfig,
) -> Instruction {
    let protocol_config_pda = get_protocol_config_pda_address();
    let ix = crate::instruction::InitializeProtocolConfig {
        bump: protocol_config_pda.1,
        protocol_config,
    };

    let accounts = crate::accounts::InitializeProtocolConfig {
        protocol_config_pda: protocol_config_pda.0,
        fee_payer,
        authority,
        system_program: system_program::ID,
        self_program: crate::ID,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: ix.data(),
    }
}

pub fn create_register_forester_instruction(
    fee_payer: &Pubkey,
    governance_authority: &Pubkey,
    forester_authority: &Pubkey,
    config: ForesterConfig,
) -> Instruction {
    let (forester_pda, _bump) = get_forester_pda(forester_authority);
    let instruction_data = crate::instruction::RegisterForester {
        _bump,
        authority: *forester_authority,
        config,
        weight: Some(1),
    };
    let (protocol_config_pda, _) = get_protocol_config_pda_address();
    let accounts = crate::accounts::RegisterForester {
        forester_pda,
        fee_payer: *fee_payer,
        authority: *governance_authority,
        protocol_config_pda,
        system_program: solana_sdk::system_program::id(),
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_update_forester_pda_weight_instruction(
    forester_authority: &Pubkey,
    protocol_authority: &Pubkey,
    new_weight: u64,
) -> Instruction {
    let (forester_pda, _bump) = get_forester_pda(forester_authority);
    let protocol_config_pda = get_protocol_config_pda_address().0;
    let instruction_data = crate::instruction::UpdateForesterPdaWeight { new_weight };
    let accounts = crate::accounts::UpdateForesterPdaWeight {
        forester_pda,
        authority: *protocol_authority,
        protocol_config_pda,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_update_forester_pda_instruction(
    forester_authority: &Pubkey,
    derivation_key: &Pubkey,
    new_authority: Option<Pubkey>,
    config: Option<ForesterConfig>,
) -> Instruction {
    let (forester_pda, _) = get_forester_pda(derivation_key);
    let instruction_data = crate::instruction::UpdateForesterPda { config };
    let accounts = crate::accounts::UpdateForesterPda {
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
    derivation: &Pubkey,
    epoch: u64,
) -> Instruction {
    let (forester_epoch_pda, _bump) = get_forester_epoch_pda_from_authority(derivation, epoch);
    let (forester_pda, _) = get_forester_pda(derivation);
    let epoch_pda = get_epoch_pda_address(epoch);
    let protocol_config_pda = get_protocol_config_pda_address().0;
    let instruction_data = crate::instruction::RegisterForesterEpoch { epoch };
    let accounts = crate::accounts::RegisterForesterEpoch {
        fee_payer: *authority,
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

pub fn create_finalize_registration_instruction(
    authority: &Pubkey,
    derivation: &Pubkey,
    epoch: u64,
) -> Instruction {
    let (forester_epoch_pda, _bump) = get_forester_epoch_pda_from_derivation(derivation, epoch);
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

pub fn create_report_work_instruction(
    authority: &Pubkey,
    derivation: &Pubkey,
    epoch: u64,
) -> Instruction {
    let (forester_epoch_pda, _bump) = get_forester_epoch_pda_from_authority(derivation, epoch);
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
