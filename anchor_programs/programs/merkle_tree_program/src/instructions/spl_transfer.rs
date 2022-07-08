use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer};
use crate::utils::config;

#[derive(Accounts)]
#[instruction(data: Vec<u8>,_verifier_index: u64, _merkle_tree_index: u64)]
pub struct WithdrawSpl<'info> {
    /// CHECK:` Signer is registered verifier program.
    #[account(mut, address=solana_program::pubkey::Pubkey::new(&config::REGISTERED_VERIFIER_KEY_ARRAY[usize::try_from(_verifier_index).unwrap()]))]
    pub authority: Signer<'info>,
    /// CHECK:` That the merkle tree token belongs to a registered Merkle tree.
    #[account(mut, constraint = merkle_tree_token.key() == Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[usize::try_from(_merkle_tree_index).unwrap()].1))]
    pub merkle_tree_token: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    /// CHECK:` that the token authority is derived in the correct way.
    #[account(mut, seeds=[b"spl"], bump)]
    pub token_authority: AccountInfo<'info>,
    // Recipients are specified in remaining accounts and checked in the verifier program.
}

/// Transferring sol from the merkle_tree_token_pda to recipients which are passed-in
/// as remaining accounts.
pub fn process_spl_transfer<'info> (ctx: Context<'_, '_, '_, 'info, WithdrawSpl<'info>>, instruction_data: &[u8]) -> Result<()> {
    let account = &mut ctx.remaining_accounts.iter();
    // withdraws amounts to accounts

    for amount_u8 in instruction_data.chunks(8) {
        let amount = u64::from_le_bytes(amount_u8.try_into().unwrap());
        let to = next_account_info(account)?.clone();
        msg!("Withdrawing {}", amount);
        let (_, bump) = solana_program::pubkey::Pubkey::find_program_address(
            &[&b"spl".as_ref()], ctx.program_id );
        let bump = &[bump][..];
        let seeds = &[&[&b"spl".as_ref(), bump][..]];
        let accounts = Transfer {
            from:       ctx.accounts.merkle_tree_token.to_account_info(),
            to:         to,
            authority:  ctx.accounts.token_authority.to_account_info()
        };
        let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), accounts, seeds);
        anchor_spl::token::transfer(cpi_ctx, amount)?;
    }
    Ok(())
}
