pub use light_sdk_types::CpiAccountsConfig;
use light_sdk_types::{CpiAccounts as GenericCpiAccounts, SYSTEM_ACCOUNTS_LEN};

use crate::{
    error::{LightSdkError, Result},
    AccountInfo, AccountMeta, Pubkey,
};

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
