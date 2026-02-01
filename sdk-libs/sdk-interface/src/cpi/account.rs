//! Generic CPI accounts trait and implementations.

use light_account_checks::{AccountInfoTrait, CpiMeta};
use light_sdk_types::cpi_accounts::v2::{CompressionCpiAccountIndex, CpiAccounts, PROGRAM_ACCOUNTS_LEN};
use light_sdk_types::cpi_context_write::CpiContextWriteAccounts;

use crate::error::LightPdaError;

/// Trait for types that can provide account infos and metas for Light system program CPI.
///
/// Generic over `AI: AccountInfoTrait` to work with both solana and pinocchio backends.
pub trait CpiAccountsTrait<AI: AccountInfoTrait + Clone> {
    fn to_account_infos(&self) -> Vec<AI>;
    fn to_account_metas(&self) -> Result<Vec<CpiMeta>, LightPdaError>;
    fn get_mode(&self) -> Option<u8>;
}

/// Build `CpiMeta` vec from `CpiAccounts` (v2 mode=1).
impl<'a, AI: AccountInfoTrait + Clone> CpiAccountsTrait<AI> for CpiAccounts<'a, AI> {
    fn to_account_infos(&self) -> Vec<AI> {
        CpiAccounts::to_account_infos(self)
    }

    fn to_account_metas(&self) -> Result<Vec<CpiMeta>, LightPdaError> {
        to_cpi_metas(self)
    }

    fn get_mode(&self) -> Option<u8> {
        Some(1) // v2 mode
    }
}

/// Build `CpiMeta` vec from `CpiContextWriteAccounts` (3-account CPI context write).
impl<'a, AI: AccountInfoTrait + Clone> CpiAccountsTrait<AI> for CpiContextWriteAccounts<'a, AI> {
    fn to_account_infos(&self) -> Vec<AI> {
        self.to_account_infos().to_vec()
    }

    fn to_account_metas(&self) -> Result<Vec<CpiMeta>, LightPdaError> {
        let infos = self.to_account_info_refs();
        Ok(vec![
            CpiMeta {
                pubkey: infos[0].key(),
                is_signer: true,
                is_writable: true,
            },
            CpiMeta {
                pubkey: infos[1].key(),
                is_signer: true,
                is_writable: false,
            },
            CpiMeta {
                pubkey: infos[2].key(),
                is_signer: false,
                is_writable: true,
            },
        ])
    }

    fn get_mode(&self) -> Option<u8> {
        Some(1) // v2 mode
    }
}

/// Convert `CpiAccounts` to a vec of `CpiMeta`, preserving the account layout
/// expected by the Light system program.
fn to_cpi_metas<AI: AccountInfoTrait + Clone>(
    cpi_accounts: &CpiAccounts<'_, AI>,
) -> Result<Vec<CpiMeta>, LightPdaError> {
    let mut metas =
        Vec::with_capacity(1 + cpi_accounts.account_infos().len() - PROGRAM_ACCOUNTS_LEN);

    metas.push(CpiMeta {
        pubkey: cpi_accounts.fee_payer().key(),
        is_signer: true,
        is_writable: true,
    });
    metas.push(CpiMeta {
        pubkey: cpi_accounts.authority()?.key(),
        is_signer: true,
        is_writable: false,
    });
    metas.push(CpiMeta {
        pubkey: cpi_accounts.registered_program_pda()?.key(),
        is_signer: false,
        is_writable: false,
    });
    metas.push(CpiMeta {
        pubkey: cpi_accounts.account_compression_authority()?.key(),
        is_signer: false,
        is_writable: false,
    });
    metas.push(CpiMeta {
        pubkey: cpi_accounts.account_compression_program()?.key(),
        is_signer: false,
        is_writable: false,
    });
    metas.push(CpiMeta {
        pubkey: cpi_accounts.system_program()?.key(),
        is_signer: false,
        is_writable: false,
    });

    let accounts = cpi_accounts.account_infos();
    let mut index = CompressionCpiAccountIndex::SolPoolPda as usize;

    if cpi_accounts.config().sol_pool_pda {
        let account = cpi_accounts.get_account_info(index)?;
        metas.push(CpiMeta {
            pubkey: account.key(),
            is_signer: false,
            is_writable: true,
        });
        index += 1;
    }

    if cpi_accounts.config().sol_compression_recipient {
        let account = cpi_accounts.get_account_info(index)?;
        metas.push(CpiMeta {
            pubkey: account.key(),
            is_signer: false,
            is_writable: true,
        });
        index += 1;
    }

    if cpi_accounts.config().cpi_context {
        let account = cpi_accounts.get_account_info(index)?;
        metas.push(CpiMeta {
            pubkey: account.key(),
            is_signer: false,
            is_writable: true,
        });
        index += 1;
    }
    assert_eq!(cpi_accounts.system_accounts_end_offset(), index);

    let tree_accounts =
        accounts
            .get(index..)
            .ok_or(LightPdaError::CpiAccountsIndexOutOfBounds(index))?;
    tree_accounts.iter().for_each(|acc| {
        metas.push(CpiMeta {
            pubkey: acc.key(),
            is_signer: acc.is_signer(),
            is_writable: acc.is_writable(),
        });
    });
    Ok(metas)
}
