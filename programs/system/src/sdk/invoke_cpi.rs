// #![cfg(target_os = "solana")]
use anchor_lang::{prelude::*, Bumps};

use super::{accounts::{InvokeAccounts, InvokeCpiAccounts,  LightSystemAccount, SignerAccounts}, CompressedCpiContext};

//TODO: add test and update function name
// Invokes the light system program for state transitions on compressed
// accounts.
//
// This function facilitates caller programs to interact with the light system
// program, ensuring state transitions are verified and applied correctly.
//
// # Parameters
// * `remaining_accounts`             A vector of `AccountInfo`.
// * `light_system_program`           The `AccountInfo` for the light system program.
// * `inputs`                         Serialized input data for the CPI call.
// * `cpi_accounts`                   Accounts required for the CPI, structured for the light system program.
// * `seeds`                          Array of seeds used for deriving the signing PDA.
//
// # Returns
// Result indicating the success or failure of the operation.
// pub fn invoke_system_cpi<'info>(
//     remaining_accounts: Vec<AccountInfo<'info>>,
//     light_system_program: AccountInfo<'info>,
//     inputs: Vec<u8>,
//     cpi_accounts: InvokeCpiInstruction,
//     seeds: [&[&[u8]]; 1],
// ) -> Result<()> {
//     invoke_cpi(
//         CpiContext::new_with_signer(light_system_program, cpi_accounts, &seeds)
//             .with_remaining_accounts(remaining_accounts.to_vec()),
//         inputs,
//     )
// }
// TODO: properly document compressed-cpi-context
// TODO: turn into a simple check!
// TOOD: CHECK needed bc can be different from own, if called from another program.
pub fn get_compressed_cpi_context_account<'info>(
    ctx: &Context<'_, '_, '_, 'info, impl InvokeAccounts<'info> + LightSystemAccount<'info> + InvokeCpiAccounts<'info> + SignerAccounts<'info> + Bumps>, 
    compressed_cpi_context: &CompressedCpiContext,
) -> Result<AccountInfo<'info>> {
    let cpi_context_account = ctx.remaining_accounts
        .get(compressed_cpi_context.cpi_context_account_index as usize)
        .map(|account| account.to_account_info())
        .ok_or_else(|| anchor_lang::error::Error::from(crate::errors::CompressedPdaError::CpiContextAccountUndefined))?;
    Ok(cpi_context_account)
}


// pub fn verify<'info>(
//     ctx: Context<'_, '_, '_, 'info, impl InvokeAccounts<'info> + LightSystemAccount<'info> + InvokeCpiAccounts<'info> + SignerAccounts<'info> + InvokeCpiContextAccount<'info> + Bumps>, 
//     inputs: Vec<u8>, 
//     seeds: [&[&[u8]]; 1]
// ) -> Result<()> {

//     let cpi_accounts = crate::cpi::accounts::InvokeCpiInstruction {
//         fee_payer: ctx.accounts.get_fee_payer().to_account_info(),
//         authority: ctx.accounts.get_authority().to_account_info(),
//         registered_program_pda: ctx.accounts.get_registered_program_pda().to_account_info(),
//         noop_program: ctx.accounts.get_noop_program().to_account_info(),
//         account_compression_authority: ctx.accounts.get_account_compression_authority().to_account_info(),
//         account_compression_program: ctx.accounts.get_account_compression_program().to_account_info(),
//         invoking_program: ctx.accounts.get_invoking_program().to_account_info(),
//         compressed_sol_pda: None,
//         compression_recipient: None,
//         system_program: ctx.accounts.get_system_program().to_account_info(),
//         // cpi_context_account: ctx.accounts.get_cpi_context_account().map(|acc| acc.to_account_info()), // Assuming there's a method to get this optionally
//         cpi_context_account: ctx.accounts.get_cpi_context_account().map(|acc| acc.to_account_info()),
//     };

//     crate::cpi::invoke_cpi(
//         CpiContext::new_with_signer(
//             ctx.accounts.get_light_system_program().to_account_info(),
//             cpi_accounts,
//             &seeds,
//         ).with_remaining_accounts(ctx.remaining_accounts.to_vec()),
//         inputs
//     )
// }
