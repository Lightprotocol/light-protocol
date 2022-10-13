use crate::config::{
    MERKLE_TREE_INIT_AUTHORITY,
    ZERO_BYTES_MERKLE_TREE_18
};
use crate::errors::ErrorCode;
use anchor_lang::prelude::*;

use anchor_lang::solana_program::{
    account_info::AccountInfo, msg, program_pack::Pack, pubkey::Pubkey,
};
use crate::state::MerkleTree;
use crate::MerkleTreeAuthority;
#[derive(Accounts)]
#[instruction(merkle_tree_nr: u64)]
pub struct InitializeNewMerkleTree<'info> {
    #[account(mut, address=merkle_tree_authority_pda.pubkey @ErrorCode::InvalidAuthority)]
    pub authority: Signer<'info>,
    /// CHECK: it should be unpacked internally
    #[account(
        init,
        seeds = [&program_id.to_bytes()[..]//, &[0u8;8][..]
        ],
        bump,
        payer = authority,
        space = 2024 //1698
    )]
    pub merkle_tree: AccountLoader<'info, MerkleTree>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    #[account(
        init,
        payer = authority,
        seeds = [&merkle_tree.key().to_bytes()],
        bump,
        space = 16,
    )]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    #[account(seeds = [&b"MERKLE_TREE_AUTHORITY"[..]], bump)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
}

// keeps track of leaves which have been queued but not inserted into the merkle tree yet
#[account]
pub struct MerkleTreePdaToken {}

// keeps track of leaves which have been queued but not inserted into the merkle tree yet
#[account]
pub struct PreInsertedLeavesIndex {
    pub next_index: u64,
}

#[allow(clippy::manual_memcpy)]
pub fn process_initialize_new_merkle_tree_18(
    ctx: Context<InitializeNewMerkleTree>,
    init_bytes: &[u8],
    merkle_tree_nr: u64
) -> Result<()> {
    let merkle_tree_state_data = &mut ctx.accounts.merkle_tree.load_init()?;

    for (i, zero) in ZERO_BYTES_MERKLE_TREE_18.chunks(32).enumerate() {
        merkle_tree_state_data.filled_subtrees[i] = zero.try_into().unwrap();
    }
    merkle_tree_state_data.levels = merkle_tree_state_data.filled_subtrees.len().try_into().unwrap();
    merkle_tree_state_data.merkle_tree_nr = merkle_tree_nr;
    merkle_tree_state_data.root_history_size = 1024;

    merkle_tree_state_data.roots[0] = merkle_tree_state_data.filled_subtrees[merkle_tree_state_data.filled_subtrees.len() - 1];
    msg!("merkle_tree_state_data.roots[0]: {:?}", merkle_tree_state_data.roots[0]);
    msg!("merkle_tree_state_data.levels {}", merkle_tree_state_data.levels);

    Ok(())
}
