use anchor_lang::prelude::*;
use light_compressed_token_sdk::ValidityProof;
use light_sdk::{
    account::LightAccount,
    cpi::{
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
};
use light_sdk_types::cpi_context_write::CpiContextWriteAccounts;

use crate::{process_update_deposit::CompressedEscrowPda, LIGHT_CPI_SIGNER};

pub fn process_create_escrow_pda_with_cpi_context<'a, 'info>(
    amount: u64,
    address: [u8; 32],
    mut new_address_params: light_sdk::address::NewAddressParamsAssignedPacked,
    cpi_accounts: &CpiAccounts<'a, 'info>,
) -> Result<()> {
    let mut my_compressed_account =
        LightAccount::<CompressedEscrowPda>::new_init(&crate::ID, Some(address), 0);

    my_compressed_account.amount = amount;
    my_compressed_account.owner = *cpi_accounts.fee_payer().key;
    // Compressed output account order: 0. escrow account 1. mint, 2. token account
    new_address_params.assigned_account_index = 0;
    new_address_params.assigned_to_account = true;

    msg!("invoke");
    let cpi_context_accounts = CpiContextWriteAccounts {
        fee_payer: cpi_accounts.fee_payer(),
        authority: cpi_accounts.authority().unwrap(),
        cpi_context: cpi_accounts.cpi_context().unwrap(),
        cpi_signer: LIGHT_CPI_SIGNER,
    };

    LightSystemProgramCpi::new_cpi(LIGHT_CPI_SIGNER, ValidityProof(None))
        .with_light_account(my_compressed_account)?
        .with_new_addresses(&[new_address_params])
        .invoke_write_to_cpi_context_first(cpi_context_accounts)?;

    Ok(())
}
