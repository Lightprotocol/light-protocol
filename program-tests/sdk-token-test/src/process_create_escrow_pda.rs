use anchor_lang::prelude::*;
use light_sdk::{
    cpi::CpiAccounts,
    instruction::ValidityProof as LightValidityProof,
};
use light_sdk_types::CpiAccountsConfig;

use crate::process_create_compressed_account::process_create_compressed_account;

pub fn process_create_escrow_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, crate::GenericWithAuthority<'info>>,
    proof: LightValidityProof,
    output_tree_index: u8,
    amount: u64,
    address: [u8; 32],
    new_address_params: light_sdk::address::PackedNewAddressParams,
    system_accounts_start_offset: u8,
) -> Result<()> {
    // Parse CPI accounts
    let config = CpiAccountsConfig {
        cpi_signer: crate::LIGHT_CPI_SIGNER,
        cpi_context: false, // No CPI context needed for PDA creation
        sol_pool_pda: false,
        sol_compression_recipient: false,
    };

    let (_token_account_infos, system_account_infos) = ctx
        .remaining_accounts
        .split_at(system_accounts_start_offset as usize);

    let cpi_accounts = CpiAccounts::try_new_with_config(
        ctx.accounts.signer.as_ref(),
        system_account_infos,
        config,
    )
    .unwrap();

    // Create the escrow PDA using existing function
    process_create_compressed_account(
        cpi_accounts,
        proof,
        output_tree_index,
        amount,
        address,
        new_address_params,
    )
}