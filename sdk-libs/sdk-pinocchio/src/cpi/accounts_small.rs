use light_sdk_types::{
    CompressionCpiAccountIndexSmall, CpiAccountsSmall as GenericCpiAccountsSmall,
    PROGRAM_ACCOUNTS_LEN,
};
use pinocchio::{account_info::AccountInfo, instruction::AccountMeta};

pub type CpiAccountsSmall<'a> = GenericCpiAccountsSmall<'a, AccountInfo>;

pub fn to_account_metas_small<'a>(cpi_accounts: &CpiAccountsSmall<'a>) -> Vec<AccountMeta<'a>> {
    let mut account_metas =
        Vec::with_capacity(1 + cpi_accounts.account_infos().len() - PROGRAM_ACCOUNTS_LEN);

    account_metas.push(AccountMeta::writable_signer(cpi_accounts.fee_payer().key()));
    account_metas.push(AccountMeta::readonly_signer(cpi_accounts.authority().key()));

    let accounts = cpi_accounts.account_infos();
    account_metas.push(AccountMeta::readonly(
        accounts[CompressionCpiAccountIndexSmall::RegisteredProgramPda as usize].key(),
    ));
    account_metas.push(AccountMeta::readonly(
        accounts[CompressionCpiAccountIndexSmall::AccountCompressionAuthority as usize].key(),
    ));

    let mut index = CompressionCpiAccountIndexSmall::SolPoolPda as usize;
    if cpi_accounts.config().sol_pool_pda {
        account_metas.push(AccountMeta::writable(accounts[index].key()));
        index += 1;
    }

    if cpi_accounts.config().sol_compression_recipient {
        account_metas.push(AccountMeta::writable(accounts[index].key()));
        index += 1;
    }

    if cpi_accounts.config().cpi_context {
        account_metas.push(AccountMeta::writable(accounts[index].key()));
        index += 1;
    }

    // Add remaining tree accounts
    accounts[index..].iter().for_each(|acc| {
        let account_meta = if acc.is_writable() {
            AccountMeta::writable(acc.key())
        } else {
            AccountMeta::readonly(acc.key())
        };
        account_metas.push(account_meta);
    });

    account_metas
}
