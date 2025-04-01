use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;

pub trait InvokeAccounts<'info> {
    fn get_registered_program_pda(&self) -> &AccountInfo<'info>;
    fn get_noop_program(&self) -> &UncheckedAccount<'info>;
    fn get_account_compression_authority(&self) -> &UncheckedAccount<'info>;
    fn get_account_compression_program(&self) -> &Program<'info, AccountCompression>;
    fn get_system_program(&self) -> &Program<'info, System>;
    fn get_sol_pool_pda(&self) -> Option<&AccountInfo<'info>>;
    fn get_decompression_recipient(&self) -> Option<&AccountInfo<'info>>;
}

pub trait SignerAccounts<'info> {
    fn get_fee_payer(&self) -> &Signer<'info>;
    fn get_authority(&self) -> &Signer<'info>;
}
