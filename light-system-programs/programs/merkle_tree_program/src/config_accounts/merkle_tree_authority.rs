use crate::config;
use crate::errors::ErrorCode;
use crate::transaction_merkle_tree::state::TransactionMerkleTree;
use crate::utils::constants::MERKLE_TREE_AUTHORITY_SEED;
use anchor_lang::prelude::*;

/// Configures the authority of the merkle tree which can:
/// - register new verifiers
/// - register new asset pools
/// - register new asset pool types
/// - set permissions for new asset pool creation
/// - keeps current highest index for assets and merkle trees to enable lookups of these
#[account]
pub struct MerkleTreeAuthority {
    pub pubkey: Pubkey,
    pub transaction_merkle_tree_index: u64,
    pub event_merkle_tree_index: u64,
    pub registered_asset_index: u64,
    pub enable_nfts: bool,
    pub enable_permissionless_spl_tokens: bool,
    pub enable_permissionless_merkle_tree_registration: bool,
}

#[derive(Accounts)]
pub struct InitializeMerkleTreeAuthority<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [MERKLE_TREE_AUTHORITY_SEED],
        bump,
        space = 8 + 32 + 8 + 8 + 8 + 8
    )]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
    /// CHECK:` Signer is merkle tree init authority.
    #[account(mut, address=anchor_lang::prelude::Pubkey::try_from(config::INITIAL_MERKLE_TREE_AUTHORITY).map_err(|_| ErrorCode::PubkeyTryFromFailed)? @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdateMerkleTreeAuthority<'info> {
    #[account(mut, seeds = [MERKLE_TREE_AUTHORITY_SEED], bump)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
    /// CHECK:` Signer is merkle tree authority.
    #[account(address=merkle_tree_authority_pda.pubkey @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    /// CHECK:` New authority no need to be checked
    pub new_authority: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct UpdateMerkleTreeAuthorityConfig<'info> {
    #[account(mut, seeds = [MERKLE_TREE_AUTHORITY_SEED], bump)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
    /// CHECK:` Signer is merkle tree authority.
    #[account( address=merkle_tree_authority_pda.pubkey @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateLockDuration<'info> {
    #[account(mut, seeds = [MERKLE_TREE_AUTHORITY_SEED], bump)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
    /// CHECK:` Signer is merkle tree authority.
    #[account( address=merkle_tree_authority_pda.pubkey @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
}
