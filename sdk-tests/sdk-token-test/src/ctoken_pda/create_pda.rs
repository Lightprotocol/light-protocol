use anchor_lang::prelude::*;
use light_compressed_token_sdk::ValidityProof;
use light_sdk::{
    account::LightAccount,
    cpi::{
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
};

use crate::process_update_deposit::CompressedEscrowPda;

pub fn process_create_escrow_pda<'a, 'info>(
    proof: ValidityProof,
    output_tree_index: u8,
    amount: u64,
    address: [u8; 32],
    mut new_address_params: light_sdk::address::NewAddressParamsAssignedPacked,
    cpi_accounts: CpiAccounts<'a, 'info>,
) -> Result<()> {
    let mut my_compressed_account =
        LightAccount::<CompressedEscrowPda>::new_init(&crate::ID, Some(address), output_tree_index);

    my_compressed_account.amount = amount;
    my_compressed_account.owner = *cpi_accounts.fee_payer().key;
    // Compressed output account order: 1. mint, 2. token account 3. escrow account
    new_address_params.assigned_account_index = 2;
    new_address_params.assigned_to_account = true;

    msg!("invoke");

    LightSystemProgramCpi::new_cpi(crate::LIGHT_CPI_SIGNER, proof)
        .with_light_account(my_compressed_account)?
        .with_new_addresses(&[new_address_params])
        .invoke_execute_cpi_context(cpi_accounts)?;

    Ok(())
}
