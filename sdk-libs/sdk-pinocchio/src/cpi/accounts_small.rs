use light_sdk_types::{
    CpiAccountsSmall as GenericCpiAccountsSmall, ACCOUNT_COMPRESSION_AUTHORITY_PDA,
    ACCOUNT_COMPRESSION_PROGRAM_ID, REGISTERED_PROGRAM_PDA, SMALL_SYSTEM_ACCOUNTS_LEN,
};
use pinocchio::{account_info::AccountInfo, instruction::AccountMeta, pubkey::Pubkey};

use crate::error::Result;

pub type CpiAccountsSmall<'a> = GenericCpiAccountsSmall<'a, AccountInfo>;
/*
pub fn to_account_metas_small<'a>(
    cpi_accounts: &CpiAccountsSmall<'a>,
) -> Result<Vec<AccountMeta<'a>>> {
    let mut account_metas = Vec::with_capacity(1 + SMALL_SYSTEM_ACCOUNTS_LEN);

    // 1. Fee payer (signer, writable)
    account_metas.push(AccountMeta::writable_signer(cpi_accounts.fee_payer().key()));

    // 2. Authority/CPI Signer (signer, readonly) - hardcoded from config
    account_metas.push(AccountMeta::readonly_signer(
        &cpi_accounts.config().cpi_signer(),
    ));

    // 3. Registered Program PDA (readonly) - hardcoded constant
    account_metas.push(AccountMeta::readonly(&Pubkey::from(REGISTERED_PROGRAM_PDA)));

    // 4. Account Compression Authority (readonly) - hardcoded constant
    account_metas.push(AccountMeta::readonly(&Pubkey::from(
        ACCOUNT_COMPRESSION_AUTHORITY_PDA,
    )));

    // 5. Account Compression Program (readonly) - hardcoded constant
    account_metas.push(AccountMeta::readonly(&Pubkey::from(
        ACCOUNT_COMPRESSION_PROGRAM_ID,
    )));

    // 6. System Program (readonly) - always default pubkey
    account_metas.push(AccountMeta::readonly(&Pubkey::default()));

    // Optional accounts based on config
    if cpi_accounts.config().sol_pool_pda {
        account_metas.push(AccountMeta::writable(cpi_accounts.sol_pool_pda()?.key()));
    }

    if cpi_accounts.config().sol_compression_recipient {
        account_metas.push(AccountMeta::writable(
            cpi_accounts.decompression_recipient()?.key(),
        ));
    }

    if cpi_accounts.config().cpi_context {
        account_metas.push(AccountMeta::writable(cpi_accounts.cpi_context()?.key()));
    }

    // Add tree accounts
    let tree_accounts = cpi_accounts.tree_accounts()?;
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
*/
