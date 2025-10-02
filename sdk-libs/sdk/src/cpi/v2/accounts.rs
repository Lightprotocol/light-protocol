use light_sdk_types::cpi_accounts::v2::{
    CompressionCpiAccountIndex, CpiAccounts as GenericCpiAccounts, PROGRAM_ACCOUNTS_LEN,
};

use crate::{error::Result, AccountInfo, AccountMeta};

/// Light system program CPI accounts struct.
///
/// Use with [`LightSystemProgramCpi`](super::LightSystemProgramCpi) to invoke the Light system program.
pub type CpiAccounts<'c, 'info> = GenericCpiAccounts<'c, AccountInfo<'info>>;

pub fn to_account_metas(cpi_accounts: &CpiAccounts<'_, '_>) -> Result<Vec<AccountMeta>> {
    // TODO: do a version with a const array instead of vector.
    let mut account_metas =
        Vec::with_capacity(1 + cpi_accounts.account_infos().len() - PROGRAM_ACCOUNTS_LEN);

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
        pubkey: *cpi_accounts.system_program()?.key,
        is_signer: false,
        is_writable: false,
    });
    let accounts = cpi_accounts.account_infos();
    let mut index = CompressionCpiAccountIndex::SolPoolPda as usize;

    if cpi_accounts.config().sol_pool_pda {
        let account = cpi_accounts.get_account_info(index)?;
        account_metas.push(AccountMeta {
            pubkey: *account.key,
            is_signer: false,
            is_writable: true,
        });
        index += 1;
    }

    if cpi_accounts.config().sol_compression_recipient {
        let account = cpi_accounts.get_account_info(index)?;
        account_metas.push(AccountMeta {
            pubkey: *account.key,
            is_signer: false,
            is_writable: true,
        });
        index += 1;
    }

    if cpi_accounts.config().cpi_context {
        let account = cpi_accounts.get_account_info(index)?;
        account_metas.push(AccountMeta {
            pubkey: *account.key,
            is_signer: false,
            is_writable: true,
        });
        index += 1;
    }
    assert_eq!(cpi_accounts.system_accounts_end_offset(), index);

    let tree_accounts =
        accounts
            .get(index..)
            .ok_or(crate::error::LightSdkError::CpiAccountsIndexOutOfBounds(
                index,
            ))?;
    tree_accounts.iter().for_each(|acc| {
        account_metas.push(AccountMeta {
            pubkey: *acc.key,
            is_signer: false,
            is_writable: true,
        });
    });
    Ok(account_metas)
}
