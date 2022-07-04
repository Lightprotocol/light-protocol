use crate::config::MERKLE_TREE_INIT_AUTHORITY;
use crate::errors::ErrorCode;
use crate::state::InitMerkleTreeBytes;
use anchor_lang::prelude::*;

use anchor_lang::solana_program::{
    account_info::AccountInfo, msg, program_pack::Pack, pubkey::Pubkey,
};
use anchor_spl::token::{self,Mint,Token, TokenAccount};

#[derive(Accounts)]
pub struct InitializeNewMerkleTreeSpl<'info> {
    #[account(mut,address = Pubkey::new(&MERKLE_TREE_INIT_AUTHORITY))]
    pub authority: Signer<'info>,
    /// CHECK: it should be unpacked internally
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    #[account(
        init,
        seeds = [merkle_tree.key().to_bytes().as_ref()],
        bump,
        payer = authority,
        space = 8 + 8
    )]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    #[account(init,
              seeds = [
                  merkle_tree.key().as_ref(),
                  b"merkle_tree_pda_token"
              ],
              bump,
              payer = authority,
              token::mint = mint,
              token::authority = token_authority
    )]
    pub merkle_tree_pda_token: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK:
    #[account(mut, seeds=[b"spl"], bump)]
    pub token_authority: AccountInfo<'info>
}

/// keeps track of leaves which have been queued but not inserted into the merkle tree yet
#[account]
pub struct PreInsertedLeavesIndex {
    pub next_index: u64,
}

#[allow(clippy::manual_memcpy)]
pub fn process_initialize_new_merkle_tree_spl(
    merkle_tree_pda: AccountInfo,
    init_bytes: &[u8]
) -> Result<()> {

    let mut unpacked_init_merkle_tree =
        InitMerkleTreeBytes::unpack(&merkle_tree_pda.data.borrow())?;

    for i in 0..unpacked_init_merkle_tree.bytes.len() {
        unpacked_init_merkle_tree.bytes[i] = init_bytes[i];
    }

    InitMerkleTreeBytes::pack_into_slice(
        &unpacked_init_merkle_tree,
        &mut merkle_tree_pda.data.borrow_mut(),
    );
    if unpacked_init_merkle_tree.bytes[0..init_bytes.len()] != init_bytes[..] {
        msg!("merkle tree init failed");
        return err!(ErrorCode::MerkleTreeInitFailed);
    }
    Ok(())
}
