//! Legacy types re-imported from programs which should be removed as soon as
//! possible.

use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof,
    cpi_context::CompressedCpiContext,
    data::{NewAddressParamsPacked, OutputCompressedAccountWithPackedContext},
    invoke_cpi::InstructionDataInvokeCpi,
};

use crate::AccountInfo;

/// Helper function to create data for creating a single PDA.
pub fn create_cpi_inputs_for_new_account(
    proof: CompressedProof,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: OutputCompressedAccountWithPackedContext,
    cpi_context: Option<CompressedCpiContext>,
) -> InstructionDataInvokeCpi {
    InstructionDataInvokeCpi {
        proof: Some(proof),
        new_address_params: vec![new_address_params],
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: vec![],
        output_compressed_accounts: vec![compressed_pda],
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context,
    }
}

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
{
}

impl<'info, T> LightTraits<'info> for T where
    T: InvokeAccounts<'info>
        + LightSystemAccount<'info>
        + SignerAccounts<'info>
        + InvokeCpiContextAccount<'info>
        + InvokeCpiAccounts<'info>
{
}
