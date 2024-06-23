use crate::{CreateMintInstruction, POOL_SEED};
use anchor_lang::prelude::*;

// TODO: remove this once the anchor-lang issue is fixed
pub fn create_token_account<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateMintInstruction<'info>>,
) -> Result<()> {
    let (_, bump) = anchor_lang::solana_program::pubkey::Pubkey::find_program_address(
        &[POOL_SEED, ctx.accounts.mint.key().to_bytes().as_ref()],
        &crate::ID,
    );
    let size = crate::anchor_spl::TokenAccount::LEN;
    let rent = Rent::get()?.minimum_balance(size);

    let create_account_instruction =
        anchor_lang::solana_program::system_instruction::create_account(
            &ctx.accounts.fee_payer.key(),
            &ctx.accounts.token_pool_pda.key(),
            rent,
            size as u64,
            &ctx.accounts.token_program.key(),
        );
    anchor_lang::solana_program::program::invoke_signed(
        &create_account_instruction,
        &[
            ctx.accounts.fee_payer.to_account_info(),
            ctx.accounts.token_pool_pda.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[&[
            POOL_SEED,
            ctx.accounts.mint.key().to_bytes().as_ref(),
            &[bump],
        ]],
    )?;

    let initialize_account_instruction = spl_token::instruction::initialize_account3(
        ctx.accounts.token_program.key,
        &ctx.accounts.token_pool_pda.key(),
        &ctx.accounts.mint.key(),
        &ctx.accounts.cpi_authority_pda.key(),
    )?;

    anchor_lang::solana_program::program::invoke(
        &initialize_account_instruction,
        &[
            ctx.accounts.token_pool_pda.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.cpi_authority_pda.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
    )?;

    Ok(())
}
