use light_sdk_types::{
    CompressionCpiAccountIndexSmall, CpiAccountsSmall as GenericCpiAccountsSmall,
    PROGRAM_ACCOUNTS_LEN,
};
use pinocchio::{account_info::AccountInfo, instruction::AccountMeta};

use crate::error::Result;

pub type CpiAccountsSmall<'a> = GenericCpiAccountsSmall<'a, AccountInfo>;

pub fn to_account_metas_small<'a>(
    cpi_accounts: &CpiAccountsSmall<'a>,
) -> Result<Vec<AccountMeta<'a>>> {
    let mut account_metas =
        Vec::with_capacity(1 + cpi_accounts.account_infos().len() - PROGRAM_ACCOUNTS_LEN);

    account_metas.push(AccountMeta::writable_signer(cpi_accounts.fee_payer().key()));
    account_metas.push(AccountMeta::readonly_signer(
        cpi_accounts.authority()?.key(),
    ));

    account_metas.push(AccountMeta::readonly(
        cpi_accounts.registered_program_pda()?.key(),
    ));
    account_metas.push(AccountMeta::readonly(
        cpi_accounts.account_compression_authority()?.key(),
    ));

    let accounts = cpi_accounts.account_infos();
    let mut index = CompressionCpiAccountIndexSmall::SolPoolPda as usize;

    if cpi_accounts.config().sol_pool_pda {
        let account = cpi_accounts.get_account_info(index)?;
        account_metas.push(AccountMeta::writable(account.key()));
        index += 1;
    }

    if cpi_accounts.config().sol_compression_recipient {
        let account = cpi_accounts.get_account_info(index)?;
        account_metas.push(AccountMeta::writable(account.key()));
        index += 1;
    }

    if cpi_accounts.config().cpi_context {
        let account = cpi_accounts.get_account_info(index)?;
        account_metas.push(AccountMeta::writable(account.key()));
        index += 1;
    }

    // Add remaining tree accounts
    let tree_accounts =
        accounts
            .get(index..)
            .ok_or(crate::error::LightSdkError::CpiAccountsIndexOutOfBounds(
                index,
            ))?;
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
