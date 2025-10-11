use anchor_lang::prelude::*;
use light_compressed_token_sdk::{CompressedCpiContext, ValidityProof};
use light_sdk::{account::LightAccount, cpi::CpiInputs};
use light_sdk_types::{cpi_context_write::CpiContextWriteAccounts, CpiAccountsSmall};

use crate::{process_update_deposit::CompressedEscrowPda, LIGHT_CPI_SIGNER};

pub fn process_create_escrow_pda_with_cpi_context(
    amount: u64,
    address: [u8; 32],
    mut new_address_params: light_sdk::address::NewAddressParamsAssignedPacked,
    cpi_accounts: &CpiAccountsSmall<'_, AccountInfo>,
) -> Result<()> {
    let mut my_compressed_account =
        LightAccount::<'_, CompressedEscrowPda>::new_init(&crate::ID, Some(address), 0);

    my_compressed_account.amount = amount;
    my_compressed_account.owner = *cpi_accounts.fee_payer().key;
    // Compressed output account order: 0. escrow account 1. mint, 2. token account
    new_address_params.assigned_account_index = 0;
    new_address_params.assigned_to_account = true;
    let cpi_inputs = CpiInputs {
        proof: ValidityProof(None),
        account_infos: Some(vec![my_compressed_account
            .to_account_info()
            .map_err(ProgramError::from)?]),
        new_assigned_addresses: Some(vec![new_address_params]),
        cpi_context: Some(CompressedCpiContext {
            set_context: false,
            first_set_context: true,
            cpi_context_account_index: 0,
        }),
        ..Default::default()
    };
    msg!("invoke");
    let cpi_context_accounts = CpiContextWriteAccounts {
        fee_payer: cpi_accounts.fee_payer(),
        authority: cpi_accounts.authority().unwrap(),
        cpi_context: cpi_accounts.cpi_context().unwrap(),
        cpi_signer: LIGHT_CPI_SIGNER,
    };

    cpi_inputs
        .invoke_light_system_program_cpi_context(cpi_context_accounts)
        .map_err(ProgramError::from)?;

    Ok(())
}
