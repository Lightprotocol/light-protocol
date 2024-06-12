use anchor_lang::{error::Error, prelude::*, Bumps};

use crate::traits::{
    InvokeAccounts, InvokeCpiAccounts, InvokeCpiContextAccount, LightSystemAccount, SignerAccounts,
};
use light_system_program::{
    cpi::accounts::InvokeCpiInstruction, errors::SystemProgramError::CpiContextAccountUndefined,
    sdk::CompressedCpiContext, InstructionDataInvokeCpi,
};

// TODO: properly document compressed-cpi-context
// TODO: turn into a simple check!
// TOOD: CHECK needed bc can be different from own, if called from another program.
pub fn get_compressed_cpi_context_account<'info>(
    ctx: &Context<
        '_,
        '_,
        '_,
        'info,
        impl InvokeAccounts<'info>
            + LightSystemAccount<'info>
            + InvokeCpiAccounts<'info>
            + SignerAccounts<'info>
            + Bumps,
    >,
    compressed_cpi_context: &CompressedCpiContext,
) -> Result<AccountInfo<'info>> {
    let cpi_context_account = ctx
        .remaining_accounts
        .get(compressed_cpi_context.cpi_context_account_index as usize)
        .map(|account| account.to_account_info())
        .ok_or_else(|| Error::from(CpiContextAccountUndefined))?;
    Ok(cpi_context_account)
}

#[inline(always)]
pub fn setup_cpi_accounts<'info>(
    ctx: &Context<
        '_,
        '_,
        '_,
        'info,
        impl InvokeAccounts<'info>
            + LightSystemAccount<'info>
            + InvokeCpiAccounts<'info>
            + SignerAccounts<'info>
            + InvokeCpiContextAccount<'info>
            + Bumps,
    >,
) -> InvokeCpiInstruction<'info> {
    InvokeCpiInstruction {
        fee_payer: ctx.accounts.get_fee_payer().to_account_info(),
        authority: ctx.accounts.get_authority().to_account_info(),
        registered_program_pda: ctx.accounts.get_registered_program_pda().to_account_info(),
        noop_program: ctx.accounts.get_noop_program().to_account_info(),
        account_compression_authority: ctx
            .accounts
            .get_account_compression_authority()
            .to_account_info(),
        account_compression_program: ctx
            .accounts
            .get_account_compression_program()
            .to_account_info(),
        invoking_program: ctx.accounts.get_invoking_program().to_account_info(),
        sol_pool_pda: None,
        decompression_recipient: None,
        system_program: ctx.accounts.get_system_program().to_account_info(),
        cpi_context_account: ctx
            .accounts
            .get_cpi_context_account()
            .map(|acc| acc.to_account_info()),
    }
}

#[inline(always)]
pub fn invoke_cpi<'info, 'a, 'b, 'c>(
    ctx: &Context<
        '_,
        '_,
        '_,
        'info,
        impl InvokeAccounts<'info>
            + LightSystemAccount<'info>
            + InvokeCpiAccounts<'info>
            + SignerAccounts<'info>
            + InvokeCpiContextAccount<'info>
            + Bumps,
    >,
    cpi_accounts: light_system_program::cpi::accounts::InvokeCpiInstruction<'info>,
    inputs: Vec<u8>,
    signer_seeds: &'a [&'b [&'c [u8]]],
) -> Result<()> {
    light_system_program::cpi::invoke_cpi(
        CpiContext::new_with_signer(
            ctx.accounts.get_light_system_program().to_account_info(),
            cpi_accounts,
            signer_seeds,
        )
        .with_remaining_accounts(ctx.remaining_accounts.to_vec()),
        inputs,
    )
}

/// Invokes the light system program to verify and apply a zk-compressed state
/// transition. Serializes CPI instruction data, configures necessary accounts,
/// and executes the CPI.
pub fn verify<'info, 'a, 'b, 'c>(
    ctx: Context<
        '_,
        '_,
        '_,
        'info,
        impl InvokeAccounts<'info>
            + LightSystemAccount<'info>
            + InvokeCpiAccounts<'info>
            + SignerAccounts<'info>
            + InvokeCpiContextAccount<'info>
            + Bumps,
    >,
    inputs_struct: &InstructionDataInvokeCpi,
    signer_seeds: &'a [&'b [&'c [u8]]],
) -> Result<()> {
    let mut inputs: Vec<u8> = Vec::new();
    InstructionDataInvokeCpi::serialize(inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = setup_cpi_accounts(&ctx);
    invoke_cpi(&ctx, cpi_accounts, inputs, signer_seeds)
}
