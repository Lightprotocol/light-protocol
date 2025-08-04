use light_compressed_token_sdk::{CompressedCpiContext, ValidityProof};
use light_sdk::{account::LightAccount, cpi::CpiInputs};
use light_sdk_types::CpiAccountsSmall;

use crate::process_update_deposit::CompressedEscrowPda;

use anchor_lang::prelude::*;

pub fn process_create_escrow_pda<'a>(
    proof: ValidityProof,
    output_tree_index: u8,
    amount: u64,
    address: [u8; 32],
    mut new_address_params: light_sdk::address::NewAddressParamsAssignedPacked,
    cpi_accounts: CpiAccountsSmall<'a, AccountInfo>,
) -> Result<()> {
    let mut my_compressed_account = LightAccount::<'_, CompressedEscrowPda>::new_init(
        &crate::ID,
        Some(address),
        output_tree_index,
    );

    my_compressed_account.amount = amount;
    my_compressed_account.owner = *cpi_accounts.fee_payer().key;
    // Compressed output account order: 1. mint, 2. token account 3. escrow account
    new_address_params.assigned_account_index = 2;
    new_address_params.assigned_to_account = true;
    let cpi_inputs = CpiInputs {
        proof,
        account_infos: Some(vec![my_compressed_account
            .to_account_info()
            .map_err(ProgramError::from)?]),
        new_assigned_addresses: Some(vec![new_address_params]),
        cpi_context: Some(CompressedCpiContext {
            set_context: false,
            first_set_context: false,
            cpi_context_account_index: 0,
        }),
        ..Default::default()
    };
    msg!("invoke");

    cpi_inputs
        .invoke_light_system_program_small(cpi_accounts)
        .map_err(ProgramError::from)?;

    Ok(())
}
