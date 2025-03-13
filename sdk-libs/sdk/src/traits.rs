use solana_program::{account_info::AccountInfo, instruction::AccountMeta};

pub trait InvokeAccounts<'info> {
    fn get_registered_program_pda(&self) -> &AccountInfo<'info>;
    fn get_noop_program(&self) -> &AccountInfo<'info>;
    fn get_account_compression_authority(&self) -> &AccountInfo<'info>;
    fn get_account_compression_program(&self) -> &AccountInfo<'info>;
    fn get_system_program(&self) -> AccountInfo<'info>;
    fn get_compressed_sol_pda(&self) -> Option<&AccountInfo<'info>>;
    fn get_compression_recipient(&self) -> Option<&AccountInfo<'info>>;
}

pub trait LightSystemAccount<'info> {
    fn get_light_system_program(&self) -> AccountInfo<'info>;
}

pub trait SignerAccounts<'info> {
    fn get_fee_payer(&self) -> AccountInfo<'info>;
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
    fn get_invoking_program(&self) -> AccountInfo<'info>;
}

pub trait LightTraits<'info>:
    InvokeAccounts<'info>
    + LightSystemAccount<'info>
    + SignerAccounts<'info>
    + InvokeCpiContextAccount<'info>
    + InvokeCpiAccounts<'info>
    + CpiAccounts<'info>
{
}

impl<'info, T> LightTraits<'info> for T where
    T: InvokeAccounts<'info>
        + LightSystemAccount<'info>
        + SignerAccounts<'info>
        + InvokeCpiContextAccount<'info>
        + InvokeCpiAccounts<'info>
        + CpiAccounts<'info>
{
}

pub trait CpiAccounts<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>>;
    fn to_account_metas(&self) -> Vec<AccountMeta>;
}
