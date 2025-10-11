pub use light_sdk_types::cpi_accounts::v1::SYSTEM_ACCOUNTS_LEN;
use light_sdk_types::{
    cpi_accounts::v1::CpiAccounts as GenericCpiAccounts, ACCOUNT_COMPRESSION_AUTHORITY_PDA,
    ACCOUNT_COMPRESSION_PROGRAM_ID, LIGHT_SYSTEM_PROGRAM_ID, NOOP_PROGRAM_ID,
    REGISTERED_PROGRAM_PDA,
};

use crate::{
    error::{LightSdkError, Result},
    AccountInfo, AccountMeta, Pubkey,
};

#[derive(Debug)]
pub struct CpiInstructionConfig<'a, 'info> {
    pub fee_payer: Pubkey,
    pub cpi_signer: Pubkey,
    pub invoking_program: Pubkey,
    pub sol_pool_pda_pubkey: Option<Pubkey>,
    pub sol_compression_recipient_pubkey: Option<Pubkey>,
    pub cpi_context_pubkey: Option<Pubkey>,
    pub packed_accounts: &'a [AccountInfo<'info>],
}

/// Light system program CPI accounts struct.
///
/// Use with [`LightSystemProgramCpi`](super::LightSystemProgramCpi) to invoke the Light system program.
pub type CpiAccounts<'c, 'info> = GenericCpiAccounts<'c, AccountInfo<'info>>;

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

    // 8. Light System Program (readonly) - reused for optional accounts
    let create_light_system_meta = || AccountMeta {
        pubkey: Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID),
        is_signer: false,
        is_writable: false,
    };

    // 9. Sol Pool PDA (writable) OR Light System Program (readonly)
    if let Some(sol_pool_pda_pubkey) = config.sol_pool_pda_pubkey {
        account_metas.push(AccountMeta {
            pubkey: sol_pool_pda_pubkey,
            is_signer: false,
            is_writable: true,
        });
    } else {
        account_metas.push(create_light_system_meta());
    }

    // 10. Sol Compression Recipient (writable) OR Light System Program (readonly)
    if let Some(sol_compression_recipient_pubkey) = config.sol_compression_recipient_pubkey {
        account_metas.push(AccountMeta {
            pubkey: sol_compression_recipient_pubkey,
            is_signer: false,
            is_writable: true,
        });
    } else {
        account_metas.push(create_light_system_meta());
    }

    // 11. System Program (readonly) - always default pubkey
    account_metas.push(AccountMeta {
        pubkey: Pubkey::default(),
        is_signer: false,
        is_writable: false,
    });

    // 12. CPI Context (writable) OR Light System Program (readonly)
    if let Some(cpi_context_pubkey) = config.cpi_context_pubkey {
        account_metas.push(AccountMeta {
            pubkey: cpi_context_pubkey,
            is_signer: false,
            is_writable: true,
        });
    } else {
        account_metas.push(create_light_system_meta());
    }

    for acc in config.packed_accounts {
        account_metas.push(AccountMeta {
            pubkey: *acc.key,
            is_signer: false,
            is_writable: acc.is_writable,
        });
    }

    account_metas
}

impl<'a, 'info> TryFrom<&'a CpiAccounts<'a, 'info>> for CpiInstructionConfig<'a, 'info> {
    type Error = LightSdkError;

    fn try_from(cpi_accounts: &'a CpiAccounts<'a, 'info>) -> Result<Self> {
        Ok(CpiInstructionConfig {
            fee_payer: *cpi_accounts.fee_payer().key,
            cpi_signer: cpi_accounts.config().cpi_signer().into(),
            invoking_program: cpi_accounts.config().cpi_signer.program_id.into(),
            sol_pool_pda_pubkey: if cpi_accounts.config().sol_pool_pda {
                Some(*cpi_accounts.sol_pool_pda()?.key)
            } else {
                None
            },
            sol_compression_recipient_pubkey: if cpi_accounts.config().sol_compression_recipient {
                Some(*cpi_accounts.decompression_recipient()?.key)
            } else {
                None
            },
            cpi_context_pubkey: if cpi_accounts.config().cpi_context {
                Some(*cpi_accounts.cpi_context()?.key)
            } else {
                None
            },
            packed_accounts: cpi_accounts.tree_accounts().unwrap_or(&[]),
        })
    }
}
