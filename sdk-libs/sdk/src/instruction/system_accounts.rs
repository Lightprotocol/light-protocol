use light_sdk_types::constants::{
    CPI_AUTHORITY_PDA_SEED, PROGRAM_ID_ACCOUNT_COMPRESSION, PROGRAM_ID_LIGHT_SYSTEM,
    PROGRAM_ID_NOOP,
};

use crate::{find_cpi_signer_macro, AccountMeta, Pubkey};

#[derive(Debug, Default, Copy, Clone)]
pub struct SystemAccountMetaConfig {
    pub self_program: Pubkey,
    pub cpi_context: Option<Pubkey>,
    pub sol_compression_recipient: Option<Pubkey>,
    pub sol_pool_pda: Option<Pubkey>,
}

impl SystemAccountMetaConfig {
    pub fn new(self_program: Pubkey) -> Self {
        Self {
            self_program,
            cpi_context: None,
            sol_compression_recipient: None,
            sol_pool_pda: None,
        }
    }

    pub fn new_with_cpi_context(self_program: Pubkey, cpi_context: Pubkey) -> Self {
        Self {
            self_program,
            cpi_context: Some(cpi_context),
            sol_compression_recipient: None,
            sol_pool_pda: None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SystemAccountPubkeys {
    pub light_sytem_program: Pubkey,
    pub system_program: Pubkey,
    pub account_compression_program: Pubkey,
    pub account_compression_authority: Pubkey,
    pub registered_program_pda: Pubkey,
    pub noop_program: Pubkey,
    pub sol_pool_pda: Pubkey,
}

impl Default for SystemAccountPubkeys {
    fn default() -> Self {
        Self {
            light_sytem_program: Pubkey::from(PROGRAM_ID_LIGHT_SYSTEM),
            system_program: Pubkey::default(),
            account_compression_program: Pubkey::from(PROGRAM_ID_ACCOUNT_COMPRESSION),
            account_compression_authority: Pubkey::find_program_address(
                &[CPI_AUTHORITY_PDA_SEED],
                &Pubkey::from(PROGRAM_ID_LIGHT_SYSTEM),
            )
            .0,
            registered_program_pda: Pubkey::find_program_address(
                &[PROGRAM_ID_LIGHT_SYSTEM.as_slice()],
                &Pubkey::from(PROGRAM_ID_ACCOUNT_COMPRESSION),
            )
            .0,
            noop_program: Pubkey::from(PROGRAM_ID_NOOP),
            // TODO: add correct pubkey
            sol_pool_pda: Pubkey::default(),
        }
    }
}

pub fn get_light_system_account_metas(config: SystemAccountMetaConfig) -> Vec<AccountMeta> {
    let cpi_signer = find_cpi_signer_macro!(&config.self_program).0;
    let default_pubkeys = SystemAccountPubkeys::default();

    let mut vec = vec![
        AccountMeta::new_readonly(default_pubkeys.light_sytem_program, false),
        AccountMeta::new_readonly(cpi_signer, false),
        AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
        AccountMeta::new_readonly(default_pubkeys.noop_program, false),
        AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
        AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
        AccountMeta::new_readonly(config.self_program, false),
    ];

    if let Some(pubkey) = config.sol_pool_pda {
        vec.push(AccountMeta {
            pubkey,
            is_signer: false,
            is_writable: true,
        });
    }
    if let Some(pubkey) = config.sol_compression_recipient {
        vec.push(AccountMeta {
            pubkey,
            is_signer: false,
            is_writable: true,
        });
    }
    vec.push(AccountMeta::new_readonly(
        default_pubkeys.system_program,
        false,
    ));
    if let Some(pubkey) = config.cpi_context {
        vec.push(AccountMeta {
            pubkey,
            is_signer: false,
            is_writable: true,
        });
    }
    vec
}

/// Can be used in client to add system account metas.
///
/// We need the program id account infos in the outer instruction.
/// Account Metas:
/// 1. Light System Program
/// 2. Account Compression Program
/// 3. System Program
/// 4. CPI Signer
/// 5. Registered Program PDA
/// 6. Account Compression Authority
#[cfg(feature = "v2")]
pub fn get_light_system_account_metas_small(config: SystemAccountMetaConfig) -> Vec<AccountMeta> {
    let cpi_signer = find_cpi_signer_macro!(&config.self_program).0;
    let default_pubkeys = SystemAccountPubkeys::default();

    let mut vec = vec![
        AccountMeta::new_readonly(default_pubkeys.light_sytem_program, false),
        AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
        AccountMeta::new_readonly(default_pubkeys.system_program, false),
        AccountMeta::new_readonly(cpi_signer, false),
        AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
        AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
    ];

    if let Some(pubkey) = config.sol_pool_pda {
        vec.push(AccountMeta {
            pubkey,
            is_signer: false,
            is_writable: true,
        });
    }
    if let Some(pubkey) = config.sol_compression_recipient {
        vec.push(AccountMeta {
            pubkey,
            is_signer: false,
            is_writable: true,
        });
    }
    if let Some(pubkey) = config.cpi_context {
        vec.push(AccountMeta {
            pubkey,
            is_signer: false,
            is_writable: true,
        });
    }
    vec
}
