use light_sdk_types::{
    CompressionCpiAccountIndex, CpiAccounts as GenericCpiAccounts, SYSTEM_ACCOUNTS_LEN,
};
pub use light_sdk_types::{CpiAccountsConfig, CpiSigner};
use pinocchio::{account_info::AccountInfo, instruction::AccountMeta};

pub type CpiAccounts<'a> = GenericCpiAccounts<'a, AccountInfo>;

pub fn to_account_metas<'a>(cpi_accounts: &CpiAccounts<'a>) -> Vec<AccountMeta<'a>> {
    let mut account_metas = Vec::with_capacity(1 + SYSTEM_ACCOUNTS_LEN);
    account_metas.push(AccountMeta::writable_signer(cpi_accounts.fee_payer().key()));
    account_metas.push(AccountMeta::readonly_signer(cpi_accounts.authority().key()));

    account_metas.push(AccountMeta::readonly(
        cpi_accounts.account_infos()[CompressionCpiAccountIndex::RegisteredProgramPda as usize]
            .key(),
    ));
    account_metas.push(AccountMeta::readonly(
        cpi_accounts.account_infos()[CompressionCpiAccountIndex::NoopProgram as usize].key(),
    ));
    account_metas.push(AccountMeta::readonly(
        cpi_accounts.account_infos()
            [CompressionCpiAccountIndex::AccountCompressionAuthority as usize]
            .key(),
    ));
    account_metas.push(AccountMeta::readonly(
        cpi_accounts.account_infos()
            [CompressionCpiAccountIndex::AccountCompressionProgram as usize]
            .key(),
    ));
    account_metas.push(AccountMeta::readonly(
        cpi_accounts.account_infos()[CompressionCpiAccountIndex::InvokingProgram as usize].key(),
    ));
    let mut current_index = 7;
    if !cpi_accounts.config().sol_pool_pda {
        account_metas.push(AccountMeta::readonly(
            cpi_accounts.light_system_program().key(),
        ));
    } else {
        account_metas.push(AccountMeta::writable(
            cpi_accounts.account_infos()[current_index].key(),
        ));
        current_index += 1;
    }

    if !cpi_accounts.config().sol_compression_recipient {
        account_metas.push(AccountMeta::readonly(
            cpi_accounts.light_system_program().key(),
        ));
    } else {
        account_metas.push(AccountMeta::writable(
            cpi_accounts.account_infos()[current_index].key(),
        ));
        current_index += 1;
    }

    // System program - use default (all zeros)
    account_metas.push(AccountMeta::readonly(&[0u8; 32]));
    current_index += 1;

    if !cpi_accounts.config().cpi_context {
        account_metas.push(AccountMeta::readonly(
            cpi_accounts.light_system_program().key(),
        ));
    } else {
        account_metas.push(AccountMeta::writable(
            cpi_accounts.account_infos()[current_index].key(),
        ));
        current_index += 1;
    }

    // Add remaining tree accounts
    cpi_accounts.account_infos()[current_index..]
        .iter()
        .for_each(|acc| {
            let account_meta = if acc.is_writable() {
                AccountMeta::writable(acc.key())
            } else {
                AccountMeta::readonly(acc.key())
            };
            account_metas.push(account_meta);
        });

    account_metas
}

pub fn to_account_infos_for_invoke<'a>(cpi_accounts: &CpiAccounts<'a>) -> Vec<&'a AccountInfo> {
    let mut account_infos = Vec::with_capacity(1 + SYSTEM_ACCOUNTS_LEN);
    account_infos.push(cpi_accounts.fee_payer());
    // Skip the first account (light_system_program) and add the rest
    cpi_accounts.account_infos()[1..]
        .iter()
        .for_each(|acc| account_infos.push(acc));
    let mut current_index = 7;
    if !cpi_accounts.config().sol_pool_pda {
        account_infos.insert(current_index, cpi_accounts.light_system_program());
    }
    current_index += 1;

    if !cpi_accounts.config().sol_compression_recipient {
        account_infos.insert(current_index, cpi_accounts.light_system_program());
    }
    current_index += 1;
    // system program
    current_index += 1;

    if !cpi_accounts.config().cpi_context {
        account_infos.insert(current_index, cpi_accounts.light_system_program());
    }
    account_infos
}
