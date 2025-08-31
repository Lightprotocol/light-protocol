use pinocchio::account_info::AccountInfo;

use crate::Result;

pub trait InvokeAccounts<'info> {
    fn get_registered_program_pda(&self) -> Result<&'info AccountInfo>;
    fn get_account_compression_authority(&self) -> Result<&'info AccountInfo>;
    fn get_sol_pool_pda(&self) -> Result<Option<&'info AccountInfo>>;
    fn get_decompression_recipient(&self) -> Result<Option<&'info AccountInfo>>;
}

pub trait CpiContextAccountTrait<'info> {
    fn get_cpi_context_account(&self) -> Option<&'info AccountInfo>;
}

pub trait SignerAccounts<'info> {
    fn get_fee_payer(&self) -> &'info AccountInfo;
    fn get_authority(&self) -> &'info AccountInfo;
}
