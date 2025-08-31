use light_sdk_types::{
    CpiAccountsSmall as GenericCpiAccountsSmall, ACCOUNT_COMPRESSION_AUTHORITY_PDA,
    ACCOUNT_COMPRESSION_PROGRAM_ID, REGISTERED_PROGRAM_PDA, SMALL_SYSTEM_ACCOUNTS_LEN,
    SOL_POOL_PDA,
};

use crate::{
    error::{LightSdkError, Result},
    AccountInfo, AccountMeta, Pubkey,
};

#[derive(Debug)]
pub struct CpiInstructionConfigSmall<'a, 'info> {
    pub fee_payer: Pubkey,
    pub cpi_signer: Pubkey,
    pub sol_pool_pda: bool,
    pub sol_compression_recipient_pubkey: Option<Pubkey>,
    pub cpi_context_pubkey: Option<Pubkey>,
    pub packed_accounts: &'a [AccountInfo<'info>],
}

pub type CpiAccountsSmall<'c, 'info> = GenericCpiAccountsSmall<'c, AccountInfo<'info>>;

pub fn get_account_metas_from_config_small(
    config: CpiInstructionConfigSmall<'_, '_>,
) -> Vec<AccountMeta> {
    let mut account_metas = Vec::with_capacity(1 + SMALL_SYSTEM_ACCOUNTS_LEN);

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

    // 4. Account Compression Authority (readonly) - hardcoded constant
    account_metas.push(AccountMeta {
        pubkey: Pubkey::from(ACCOUNT_COMPRESSION_AUTHORITY_PDA),
        is_signer: false,
        is_writable: false,
    });

    // 5. Account Compression Program (readonly) - hardcoded constant
    account_metas.push(AccountMeta {
        pubkey: Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID),
        is_signer: false,
        is_writable: false,
    });

    // 6. System Program (readonly) - always default pubkey
    account_metas.push(AccountMeta {
        pubkey: Pubkey::default(),
        is_signer: false,
        is_writable: false,
    });

    // Optional accounts based on config
    if config.sol_pool_pda {
        account_metas.push(AccountMeta {
            pubkey: Pubkey::from(SOL_POOL_PDA),
            is_signer: false,
            is_writable: true,
        });
    }

    if let Some(sol_compression_recipient_pubkey) = config.sol_compression_recipient_pubkey {
        account_metas.push(AccountMeta {
            pubkey: sol_compression_recipient_pubkey,
            is_signer: false,
            is_writable: true,
        });
    }

    if let Some(cpi_context_pubkey) = config.cpi_context_pubkey {
        account_metas.push(AccountMeta {
            pubkey: cpi_context_pubkey,
            is_signer: false,
            is_writable: true,
        });
    }

    // Add tree accounts
    for acc in config.packed_accounts {
        account_metas.push(AccountMeta {
            pubkey: *acc.key,
            is_signer: false,
            is_writable: acc.is_writable,
        });
    }

    account_metas
}

impl<'a, 'info> TryFrom<&'a CpiAccountsSmall<'a, 'info>> for CpiInstructionConfigSmall<'a, 'info> {
    type Error = LightSdkError;

    fn try_from(cpi_accounts: &'a CpiAccountsSmall<'a, 'info>) -> Result<Self> {
        Ok(CpiInstructionConfigSmall {
            fee_payer: *cpi_accounts.fee_payer().key,
            cpi_signer: cpi_accounts.config().cpi_signer().into(),
            sol_pool_pda: cpi_accounts.config().sol_pool_pda,
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

pub fn to_account_metas_small(cpi_accounts: CpiAccountsSmall<'_, '_>) -> Result<Vec<AccountMeta>> {
    let config = CpiInstructionConfigSmall::try_from(&cpi_accounts)?;
    Ok(get_account_metas_from_config_small(config))
}
