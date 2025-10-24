use anchor_lang::prelude::*;
use light_sdk::{
    account::LightAccount,
    cpi::{v2::LightSystemProgramCpi, InvokeLightSystemProgram, LightCpiInstruction},
    instruction::ValidityProof as LightValidityProof,
};
use light_sdk_types::cpi_accounts::v2::CpiAccounts;

use crate::process_update_deposit::CompressedEscrowPda;

pub fn process_create_escrow_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, crate::Generic<'info>>,
    proof: LightValidityProof,
    output_tree_index: u8,
    amount: u64,
    address: [u8; 32],
    new_address_params: light_sdk::address::NewAddressParamsAssignedPacked,
) -> Result<()> {
    let cpi_accounts = CpiAccounts::new(
        ctx.accounts.signer.as_ref(),
        ctx.remaining_accounts,
        crate::LIGHT_CPI_SIGNER,
    );

    let mut my_compressed_account =
        LightAccount::<CompressedEscrowPda>::new_init(&crate::ID, Some(address), output_tree_index);

    my_compressed_account.amount = amount;
    my_compressed_account.owner = *cpi_accounts.fee_payer().key;

    msg!("invoke");

    LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, proof)
        .with_light_account(my_compressed_account)?
        .with_new_addresses(&[new_address_params])
        .invoke(cpi_accounts)?;

    Ok(())
}
