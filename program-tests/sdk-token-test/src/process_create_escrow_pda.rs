use anchor_lang::prelude::*;
use light_sdk::{
    account::LightAccount,
    cpi::{CpiAccounts, CpiInputs},
    instruction::ValidityProof as LightValidityProof,
};

use crate::process_update_deposit::CompressedEscrowPda;

pub fn process_create_escrow_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, crate::Generic<'info>>,
    proof: LightValidityProof,
    output_tree_index: u8,
    amount: u64,
    address: [u8; 32],
    new_address_params: light_sdk::address::PackedNewAddressParams,
) -> Result<()> {
    let cpi_accounts = CpiAccounts::new(
        ctx.accounts.signer.as_ref(),
        ctx.remaining_accounts,
        crate::LIGHT_CPI_SIGNER,
    );

    let mut my_compressed_account = LightAccount::<'_, CompressedEscrowPda>::new_init(
        &crate::ID,
        Some(address),
        output_tree_index,
    );

    my_compressed_account.amount = amount;
    my_compressed_account.owner = *cpi_accounts.fee_payer().key;

    let cpi_inputs = CpiInputs {
        proof,
        account_infos: Some(vec![my_compressed_account
            .to_account_info()
            .map_err(ProgramError::from)?]),
        new_addresses: Some(vec![new_address_params]),
        cpi_context: None,
        ..Default::default()
    };
    msg!("invoke");

    cpi_inputs
        .invoke_light_system_program(cpi_accounts)
        .map_err(ProgramError::from)?;

    Ok(())
}
