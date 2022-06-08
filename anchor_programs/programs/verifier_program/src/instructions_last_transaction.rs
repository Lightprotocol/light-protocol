use crate::state::VerifierState;
use anchor_lang::prelude::*;

use merkle_tree_program::{
    program::MerkleTreeProgram,
    wrapped_state::{ MerkleTree, MerkleTreeTmpPda},
    utils::config::{
        STORAGE_SEED,
        NF_SEED,
        LEAVES_SEED,
    }
};
use crate::FeeEscrowState;

use crate::utils::config:: {ESCROW_SEED};
#[derive(Accounts)]
pub struct LastTransactionDeposit<'info> {
    #[account(
        mut,
        seeds = [verifier_state.load()?.nullifier0.as_ref(), NF_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    /// CHECK:` doc comment explaining why no checks through types are necessary
    pub nullifier0_pda: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [verifier_state.load()?.nullifier1.as_ref(), NF_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    /// CHECK:` doc comment explaining why no checks through types are necessary
    pub nullifier1_pda: UncheckedAccount<'info>, //Account<'info, Nullifier>,
    #[account(
        mut,
        seeds = [merkle_tree_tmp_storage.leaf_left.as_ref(), LEAVES_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    /// CHECK:` doc comment explaining why no checks through types are necessary
    pub leaves_pda: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [verifier_state.load()?.tx_integrity_hash.as_ref(), STORAGE_SEED.as_ref()],
        bump, close = signing_address
    )]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    #[account(
        mut,
        seeds = [verifier_state.key().as_ref(), ESCROW_SEED.as_ref()],
        bump
    )]
    /// CHECK:` doc comment explaining why no checks through types are necessary
    pub escrow_pda: UncheckedAccount<'info>,
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(
        mut,
        seeds = [verifier_state.load()?.tx_integrity_hash.as_ref(), STORAGE_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    pub merkle_tree_tmp_storage: Account<'info, MerkleTreeTmpPda>,
    pub rent: Sysvar<'info, Rent>,
    // merkle tree account liquidity pool pda
    #[account(mut)]
    /// CHECK: doc comment explaining why no checks through types are necessary.
    pub merkle_tree_pda_token: AccountInfo<'info>,
    // account from which funds are transferred
    // #[account(mut)]
    // pub user_account: Signer<'info>,
    #[account(mut, close = signing_address)]
    pub fee_escrow_state: Account<'info, FeeEscrowState>,
    #[account(mut)]
    pub merkle_tree: Account<'info, MerkleTree>,

}

#[derive(Accounts)]
pub struct LastTransactionWithdrawal<'info> {
    #[account(
        mut,
        seeds = [verifier_state.load()?.nullifier0.as_ref(), NF_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub nullifier0_pda: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [verifier_state.load()?.nullifier1.as_ref(), NF_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub nullifier1_pda: UncheckedAccount<'info>, //Account<'info, Nullifier>,
    #[account(
        mut,
        seeds = [merkle_tree_tmp_storage.leaf_left.as_ref(), LEAVES_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    /// CHECK:` doc comment explaining why no checks through types are necessary
    pub leaves_pda: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [verifier_state.load()?.tx_integrity_hash.as_ref(), STORAGE_SEED.as_ref()],
        bump
    )]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    #[account(
        mut,
        seeds = [verifier_state.key().as_ref(), ESCROW_SEED.as_ref()],
        bump
    )]
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub escrow_pda: UncheckedAccount<'info>,
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: Program<'info, System>,
    #[account(
        mut,
        seeds = [verifier_state.load()?.tx_integrity_hash.as_ref(), STORAGE_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    pub merkle_tree_tmp_storage: Account<'info, MerkleTreeTmpPda>,
    pub rent: Sysvar<'info, Rent>,
    // merkle tree account liquidity pool pda
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub merkle_tree_pda_token: AccountInfo<'info>,
    // account from which funds are transferred
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub merkle_tree: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub recipient: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub relayer_recipient: AccountInfo<'info>,
}
