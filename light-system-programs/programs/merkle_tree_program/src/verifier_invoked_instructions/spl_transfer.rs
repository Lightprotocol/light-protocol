use crate::utils::constants::TOKEN_AUTHORITY_SEED;
use crate::RegisteredVerifier;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer};
#[derive(Accounts)]
pub struct WithdrawSpl<'info> {
    /// CHECK:` Signer is registered verifier program.
    #[account(mut, seeds=[program_id.to_bytes().as_ref()],bump,seeds::program=registered_verifier_pda.pubkey)]
    pub authority: Signer<'info>,
    /// CHECK:` That the merkle tree token belongs to a registered Merkle tree.
    #[account(mut)]
    pub merkle_tree_token: Account<'info, TokenAccount>,
    /// CHECK:` That the merkle tree token belongs to a registered Merkle tree.
    #[account(mut)]
    pub recipient: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    /// CHECK:` that the token authority is derived in the correct way.
    #[account(mut, seeds=[TOKEN_AUTHORITY_SEED], bump)]
    pub token_authority: AccountInfo<'info>,
    #[account(seeds=[&registered_verifier_pda.pubkey.to_bytes()],  bump)]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
}

pub fn process_spl_transfer<'info>(
    ctx: Context<'_, '_, '_, 'info, WithdrawSpl<'info>>,
    amount: u64,
) -> Result<()> {
    // msg!("Withdrawing spl token {}", amount);
    let (_, bump) = anchor_lang::prelude::Pubkey::find_program_address(
        &[TOKEN_AUTHORITY_SEED],
        ctx.program_id,
    );
    let bump = &[bump][..];
    let seeds = &[&[TOKEN_AUTHORITY_SEED, bump][..]];
    let accounts = Transfer {
        from: ctx.accounts.merkle_tree_token.to_account_info(),
        to: ctx.accounts.recipient.to_account_info(),
        authority: ctx.accounts.token_authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        accounts,
        seeds,
    );
    anchor_spl::token::transfer(cpi_ctx, amount)
}
