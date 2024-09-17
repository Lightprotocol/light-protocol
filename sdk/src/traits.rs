// Ported from light-system-program, adjusted for caller programs.
use anchor_lang::prelude::*;

pub trait InvokeAccounts<'info> {
    fn get_registered_program_pda(&self) -> &AccountInfo<'info>;
    fn get_noop_program(&self) -> &AccountInfo<'info>;
    fn get_account_compression_authority(&self) -> &AccountInfo<'info>;
    fn get_account_compression_program(&self) -> &AccountInfo<'info>;
    fn get_system_program(&self) -> &Program<'info, System>;
    fn get_compressed_sol_pda(&self) -> Option<&AccountInfo<'info>>;
    fn get_compression_recipient(&self) -> Option<&AccountInfo<'info>>;
}

pub trait LightSystemAccount<'info> {
    fn get_light_system_program(&self) -> &AccountInfo<'info>;
}

pub trait SignerAccounts<'info> {
    fn get_fee_payer(&self) -> &Signer<'info>;
    fn get_authority(&self) -> &AccountInfo<'info>;
}

// Only used within the systemprogram
pub trait InvokeCpiContextAccountMut<'info> {
    fn get_cpi_context_account_mut(&mut self) -> &mut Option<AccountInfo<'info>>;
}

pub trait InvokeCpiContextAccount<'info> {
    fn get_cpi_context_account(&self) -> Option<&AccountInfo<'info>>;
}

pub trait InvokeCpiAccounts<'info> {
    fn get_invoking_program(&self) -> &AccountInfo<'info>;
}
