use light_sdk_types::constants::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, LIGHT_SYSTEM_PROGRAM_ID,
    NOOP_PROGRAM_ID, REGISTERED_PROGRAM_PDA,
};
pub use light_sdk_types::CpiAccountsConfig;
use light_sdk_types::{CpiAccounts as GenericCpiAccounts, SYSTEM_ACCOUNTS_LEN};

use crate::{
    error::{LightSdkError, Result},
    AccountInfo, AccountMeta, Pubkey,
};

#[derive(Debug)]
pub struct CpiInstructionConfig<'a, 'info> {
    pub fee_payer: Pubkey,
    pub cpi_signer: Pubkey, // pre-computed authority
    pub invoking_program: Pubkey,
    pub sol_pool_pda_pubkey: Option<Pubkey>,
    pub sol_compression_recipient_pubkey: Option<Pubkey>,
    pub cpi_context_pubkey: Option<Pubkey>,
    pub packed_accounts: &'a [AccountInfo<'info>], // account info slice
}

pub type CpiAccounts<'c, 'info> = GenericCpiAccounts<'c, AccountInfo<'info>>;

pub fn to_account_metas(cpi_accounts: CpiAccounts<'_, '_>) -> Result<Vec<AccountMeta>> {
    let mut account_metas = Vec::with_capacity(1 + SYSTEM_ACCOUNTS_LEN);
    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.fee_payer().key,
        is_signer: true,
        is_writable: true,
    });
    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.authority()?.key,
        is_signer: true,
        is_writable: false,
    });

    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.registered_program_pda()?.key,
        is_signer: false,
        is_writable: false,
    });
    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.noop_program()?.key,
        is_signer: false,
        is_writable: false,
    });
    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.account_compression_authority()?.key,
        is_signer: false,
        is_writable: false,
    });
    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.account_compression_program()?.key,
        is_signer: false,
        is_writable: false,
    });
    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.invoking_program()?.key,
        is_signer: false,
        is_writable: false,
    });
    let mut current_index = 7;
    let anchor_none_account_meta = AccountMeta {
        pubkey: *cpi_accounts.light_system_program()?.key,
        is_signer: false,
        is_writable: false,
    };
    if !cpi_accounts.config().sol_pool_pda {
        account_metas.push(anchor_none_account_meta.clone());
    } else {
        let account = cpi_accounts.get_account_info(current_index)?;
        account_metas.push(AccountMeta {
            pubkey: *account.key,
            is_signer: false,
            is_writable: true,
        });
        current_index += 1;
    }

    if !cpi_accounts.config().sol_compression_recipient {
        account_metas.push(anchor_none_account_meta.clone());
    } else {
        let account = cpi_accounts.get_account_info(current_index)?;
        account_metas.push(AccountMeta {
            pubkey: *account.key,
            is_signer: false,
            is_writable: true,
        });
        current_index += 1;
    }
    // System program
    account_metas.push(AccountMeta {
        pubkey: Pubkey::default(),
        is_signer: false,
        is_writable: false,
    });
    current_index += 1;

    if !cpi_accounts.config().cpi_context {
        account_metas.push(anchor_none_account_meta);
    } else {
        let account = cpi_accounts.get_account_info(current_index)?;
        account_metas.push(AccountMeta {
            pubkey: *account.key,
            is_signer: false,
            is_writable: true,
        });
        current_index += 1;
    }
    let tree_accounts = cpi_accounts
        .account_infos()
        .get(current_index..)
        .ok_or(LightSdkError::CpiAccountsIndexOutOfBounds(current_index))?;
    tree_accounts.iter().for_each(|acc| {
        account_metas.push(AccountMeta {
            pubkey: *acc.key,
            is_signer: false,
            is_writable: acc.is_writable,
        });
    });
    Ok(account_metas)
}

pub fn get_account_metas_from_config(config: CpiInstructionConfig<'_, '_>) -> Vec<AccountMeta> {
    let mut account_metas = Vec::with_capacity(1 + SYSTEM_ACCOUNTS_LEN);

    // 1. Fee payer (signer, writable)
    account_metas.push(AccountMeta {
        pubkey: config.fee_payer,
        is_signer: true,
        is_writable: true,
    });

    // 2. Authority/CPI Signer (signer, readonly)
    account_metas.push(AccountMeta {
        pubkey: config.cpi_signer,
        is_signer: true,
        is_writable: false,
    });

    // 3. Registered Program PDA (readonly) - hardcoded constant
    account_metas.push(AccountMeta {
        pubkey: Pubkey::from(REGISTERED_PROGRAM_PDA),
        is_signer: false,
        is_writable: false,
    });

    // 4. Noop Program (readonly) - hardcoded constant
    account_metas.push(AccountMeta {
        pubkey: Pubkey::from(NOOP_PROGRAM_ID),
        is_signer: false,
        is_writable: false,
    });

    // 5. Account Compression Authority (readonly) - hardcoded constant
    account_metas.push(AccountMeta {
        pubkey: Pubkey::from(ACCOUNT_COMPRESSION_AUTHORITY_PDA),
        is_signer: false,
        is_writable: false,
    });

    // 6. Account Compression Program (readonly) - hardcoded constant
    account_metas.push(AccountMeta {
        pubkey: Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID),
        is_signer: false,
        is_writable: false,
    });

    // 7. Invoking Program (readonly)
    account_metas.push(AccountMeta {
        pubkey: config.invoking_program,
        is_signer: false,
        is_writable: false,
    });

    // 8. Sol Pool PDA (writable) OR Light System Program (readonly)
    let light_system_program_meta = AccountMeta {
        pubkey: Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID),
        is_signer: false,
        is_writable: false,
    };
    if let Some(sol_pool_pda_pubkey) = config.sol_pool_pda_pubkey {
        account_metas.push(AccountMeta {
            pubkey: sol_pool_pda_pubkey,
            is_signer: false,
            is_writable: true,
        });
    } else {
        account_metas.push(light_system_program_meta.clone());
    }

    // 9. Sol Compression Recipient (writable) OR Light System Program (readonly)
    if let Some(sol_compression_recipient_pubkey) = config.sol_compression_recipient_pubkey {
        account_metas.push(AccountMeta {
            pubkey: sol_compression_recipient_pubkey,
            is_signer: false,
            is_writable: true,
        });
    } else {
        account_metas.push(light_system_program_meta.clone());
    }

    // 10. System Program (readonly) - always default pubkey
    account_metas.push(AccountMeta {
        pubkey: Pubkey::default(),
        is_signer: false,
        is_writable: false,
    });

    // 11. CPI Context (writable) OR Light System Program (readonly)
    if let Some(cpi_context_pubkey) = config.cpi_context_pubkey {
        account_metas.push(AccountMeta {
            pubkey: cpi_context_pubkey,
            is_signer: false,
            is_writable: true,
        });
    } else {
        account_metas.push(light_system_program_meta);
    }

    // 12. Packed accounts (variable number)
    for acc in config.packed_accounts {
        account_metas.push(AccountMeta {
            pubkey: *acc.key,
            is_signer: false,
            is_writable: acc.is_writable,
        });
    }

    account_metas
}
