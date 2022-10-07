use crate::config;
use anchor_lang::prelude::*;
use crate::MerkleTreeAuthority;
use anchor_spl::token::{Mint, TokenAccount, Token};


/// Nullfier pdas are derived from the nullifier
/// existence of a nullifier is the check to prevent double spends.
#[account]
pub struct RegisteredAssetPool {
    pub asset_pool_pubkey: Pubkey,
    pub pool_type: [u8;32]
}


/// Pool
#[account]
pub struct RegisteredPoolType {
    pub pool_type: [u8;32]
}


#[derive(Accounts)]
#[instruction(verifier_pubkey: Pubkey, pool_type: [u8;32])]
pub struct RegisterPoolType<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&pool_type, &b"pooltype"[..]],
        bump,
        space = 8 + 32
    )]
    pub registered_pool_type_pda: Account<'info, RegisteredPoolType>,
    /// CHECK:` Signer is checked according to authority pda in instruction
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK:` New authority no need to be checked
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,

}


#[derive(Accounts)]
pub struct RegisterSplPool<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&mint.key().to_bytes(), &registered_pool_type_pda.pool_type, &b"pool"[..]],
        bump,
        space = 8 + 32 + 32
    )]
    pub registered_asset_pool_pda: Account<'info, RegisteredAssetPool>,
    #[account(init,
              seeds = [
                  &mint.key().to_bytes(), &registered_pool_type_pda.pool_type,
                  &b"token"[..]
              ],
              bump,
              payer = authority,
              token::mint = mint,
              token::authority = token_authority
    )]
    pub merkle_tree_pda_token: Account<'info, TokenAccount>,
    /// CHECK:` Signer is checked according to authority pda in instruction
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK:
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    /// CHECK:
    #[account(mut, seeds=[b"spl"], bump)]
    pub token_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub registered_pool_type_pda: Account<'info, RegisteredPoolType>,
    /// CHECK:` New authority no need to be checked
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>
}

#[derive(Accounts)]
pub struct RegisterSolPool<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [&[0u8;32], &registered_pool_type_pda.pool_type, &b"pool"[..]],
        bump,
        space = 8 + 32 + 32
    )]
    pub registered_asset_pool_pda: Account<'info, RegisteredAssetPool>,
    /// CHECK:` Signer is checked according to authority pda in instruction
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub registered_pool_type_pda: Account<'info, RegisteredPoolType>,
    /// CHECK:` New authority no need to be checked
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,

}
