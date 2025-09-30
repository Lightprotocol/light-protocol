pub use light_sdk_types::{
    cpi_accounts::{
        v1::{CpiAccounts as GenericCpiAccounts, SYSTEM_ACCOUNTS_LEN},
        CpiAccountsConfig,
    },
    CpiSigner,
};
use pinocchio::{account_info::AccountInfo, instruction::AccountMeta};

use crate::error::{LightSdkError, Result};

pub type CpiAccounts<'a> = GenericCpiAccounts<'a, AccountInfo>;

pub fn to_account_metas<'a>(cpi_accounts: &CpiAccounts<'a>) -> Result<Vec<AccountMeta<'a>>> {
    let mut account_metas = Vec::with_capacity(1 + SYSTEM_ACCOUNTS_LEN);
    account_metas.push(AccountMeta::writable_signer(cpi_accounts.fee_payer().key()));
    account_metas.push(AccountMeta::readonly_signer(
        cpi_accounts.authority()?.key(),
    ));

    account_metas.push(AccountMeta::readonly(
        cpi_accounts.registered_program_pda()?.key(),
    ));
    account_metas.push(AccountMeta::readonly(cpi_accounts.noop_program()?.key()));
    account_metas.push(AccountMeta::readonly(
        cpi_accounts.account_compression_authority()?.key(),
    ));
    account_metas.push(AccountMeta::readonly(
        cpi_accounts.account_compression_program()?.key(),
    ));
    account_metas.push(AccountMeta::readonly(
        cpi_accounts.invoking_program()?.key(),
    ));
    let mut current_index = 7;
    let light_system_program_key = cpi_accounts.light_system_program()?.key();

    if !cpi_accounts.config().sol_pool_pda {
        account_metas.push(AccountMeta::readonly(light_system_program_key));
    } else {
        let account = cpi_accounts.get_account_info(current_index)?;
        account_metas.push(AccountMeta::writable(account.key()));
        current_index += 1;
    }

    if !cpi_accounts.config().sol_compression_recipient {
        account_metas.push(AccountMeta::readonly(light_system_program_key));
    } else {
        let account = cpi_accounts.get_account_info(current_index)?;
        account_metas.push(AccountMeta::writable(account.key()));
        current_index += 1;
    }

    // System program - use default (all zeros)
    account_metas.push(AccountMeta::readonly(&[0u8; 32]));
    current_index += 1;

    if !cpi_accounts.config().cpi_context {
        account_metas.push(AccountMeta::readonly(light_system_program_key));
    } else {
        let account = cpi_accounts.get_account_info(current_index)?;
        account_metas.push(AccountMeta::writable(account.key()));
        current_index += 1;
    }

    // Add remaining tree accounts
    let tree_accounts = cpi_accounts
        .account_infos()
        .get(current_index..)
        .ok_or(LightSdkError::CpiAccountsIndexOutOfBounds(current_index))?;
    tree_accounts.iter().for_each(|acc| {
        let account_meta = if acc.is_writable() {
            AccountMeta::writable(acc.key())
        } else {
            AccountMeta::readonly(acc.key())
        };
        account_metas.push(account_meta);
    });

    Ok(account_metas)
}

pub fn to_account_infos_for_invoke<'a>(
    cpi_accounts: &CpiAccounts<'a>,
) -> Result<Vec<&'a AccountInfo>> {
    let mut account_infos = Vec::with_capacity(1 + SYSTEM_ACCOUNTS_LEN);
    account_infos.push(cpi_accounts.fee_payer());
    // Skip the first account (light_system_program) and add the rest
    cpi_accounts.account_infos()[1..]
        .iter()
        .for_each(|acc| account_infos.push(acc));
    let mut current_index = 7;
    if !cpi_accounts.config().sol_pool_pda {
        account_infos.insert(current_index, cpi_accounts.light_system_program()?);
    }
    current_index += 1;

    if !cpi_accounts.config().sol_compression_recipient {
        account_infos.insert(current_index, cpi_accounts.light_system_program()?);
    }
    current_index += 1;
    // system program
    current_index += 1;

    if !cpi_accounts.config().cpi_context {
        account_infos.insert(current_index, cpi_accounts.light_system_program()?);
    }
    Ok(account_infos)
}

impl<'a> crate::cpi::CpiAccountsTrait for CpiAccounts<'a> {
    fn to_account_metas(&self) -> Result<Vec<AccountMeta<'_>>> {
        to_account_metas(self)
    }

    fn to_account_infos_for_invoke(&self) -> Result<Vec<&AccountInfo>> {
        to_account_infos_for_invoke(self)
    }

    fn bump(&self) -> u8 {
        self.config().cpi_signer.bump
    }

    fn get_mode(&self) -> u8 {
        0 // v1 mode
    }
}
