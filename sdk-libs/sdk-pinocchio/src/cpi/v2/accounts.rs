use light_sdk_types::cpi_accounts::v2::{
    CompressionCpiAccountIndex, CpiAccounts as GenericCpiAccounts, PROGRAM_ACCOUNTS_LEN,
};
use pinocchio::{AccountView as AccountInfo, instruction::InstructionAccount};

use crate::error::{LightSdkError, Result};

pub type CpiAccounts<'c> = GenericCpiAccounts<'c, AccountInfo>;

pub fn to_account_metas<'a>(cpi_accounts: &CpiAccounts<'a>) -> Result<Vec<InstructionAccount<'a>>> {
    let mut account_metas =
        Vec::with_capacity(1 + cpi_accounts.account_infos().len() - PROGRAM_ACCOUNTS_LEN);

    account_metas.push(InstructionAccount::writable_signer(cpi_accounts.fee_payer().address()));
    account_metas.push(InstructionAccount::readonly_signer(
        cpi_accounts.authority()?.address(),
    ));
    account_metas.push(InstructionAccount::readonly(
        cpi_accounts.registered_program_pda()?.address(),
    ));
    account_metas.push(InstructionAccount::readonly(
        cpi_accounts.account_compression_authority()?.address(),
    ));
    account_metas.push(InstructionAccount::readonly(
        cpi_accounts.account_compression_program()?.address(),
    ));
    account_metas.push(InstructionAccount::readonly(cpi_accounts.system_program()?.address()));

    let accounts = cpi_accounts.account_infos();
    let mut index = CompressionCpiAccountIndex::SolPoolPda as usize;

    if cpi_accounts.config().sol_pool_pda {
        let account = cpi_accounts.get_account_info(index)?;
        account_metas.push(InstructionAccount::writable(account.address()));
        index += 1;
    }

    if cpi_accounts.config().sol_compression_recipient {
        let account = cpi_accounts.get_account_info(index)?;
        account_metas.push(InstructionAccount::writable(account.address()));
        index += 1;
    }

    if cpi_accounts.config().cpi_context {
        let account = cpi_accounts.get_account_info(index)?;
        account_metas.push(InstructionAccount::writable(account.address()));
        index += 1;
    }

    assert_eq!(cpi_accounts.system_accounts_end_offset(), index);

    let tree_accounts = accounts
        .get(index..)
        .ok_or(LightSdkError::CpiAccountsIndexOutOfBounds(index))?;
    tree_accounts.iter().for_each(|acc| {
        let account_meta = if acc.is_writable() {
            InstructionAccount::writable(acc.address())
        } else {
            InstructionAccount::readonly(acc.address())
        };
        account_metas.push(account_meta);
    });

    Ok(account_metas)
}

pub fn to_account_infos_for_invoke<'a>(
    cpi_accounts: &CpiAccounts<'a>,
) -> Result<Vec<&'a AccountInfo>> {
    let mut account_infos = Vec::with_capacity(1 + cpi_accounts.account_infos().len());
    account_infos.push(cpi_accounts.fee_payer());
    cpi_accounts.account_infos()[1..]
        .iter()
        .for_each(|acc| account_infos.push(acc));
    Ok(account_infos)
}

impl<'a> crate::cpi::CpiAccountsTrait for CpiAccounts<'a> {
    fn to_account_metas(&self) -> Result<Vec<InstructionAccount<'_>>> {
        to_account_metas(self)
    }

    fn to_account_infos_for_invoke(&self) -> Result<Vec<&AccountInfo>> {
        to_account_infos_for_invoke(self)
    }

    fn bump(&self) -> u8 {
        self.config().cpi_signer.bump
    }

    fn get_mode(&self) -> u8 {
        1
    }
}
