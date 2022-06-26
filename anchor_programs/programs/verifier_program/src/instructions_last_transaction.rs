use crate::state::VerifierState;
use anchor_lang::prelude::*;

use merkle_tree_program::{
    program::MerkleTreeProgram,
    PreInsertedLeavesIndex,
    wrapped_state::{ MerkleTree},
    utils::config::{
        STORAGE_SEED,
        NF_SEED,
        LEAVES_SEED,
        MERKLE_TREE_ACC_BYTES_ARRAY
    },
    state::MerkleTreeTmpPda
};
use crate::FeeEscrowState;

use crate::utils::config:: {ESCROW_SEED};
#[derive(Accounts)]
pub struct LastTransactionDeposit<'info> {
    #[account(mut, address=verifier_state.load()?.signing_address)]
    pub signing_address: Signer<'info>,
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
    pub nullifier1_pda: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [verifier_state.load()?.nullifier0.as_ref(), LEAVES_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    /// CHECK:` doc comment explaining why no checks through types are necessary
    pub two_leaves_pda: UncheckedAccount<'info>,
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
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    // merkle tree account liquidity pool pda
    #[account(mut, address = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[verifier_state.load()?.merkle_tree_index as usize].1))]
    /// CHECK: doc comment explaining why no checks through types are necessary.
    pub merkle_tree_pda_token: AccountInfo<'info>,
    // account from which funds are transferred
    #[account(mut, close = signing_address)]
    pub fee_escrow_state: Account<'info, FeeEscrowState>,
    #[account(mut)]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    /// CHECK: this is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut)]
    pub authority: UncheckedAccount<'info>,
    /// CHECK: Is the same as in integrity hash.
    #[account(mut, address = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[verifier_state.load()?.merkle_tree_index as usize].0))]
    pub merkle_tree: AccountInfo<'info>

}

#[derive(Accounts)]
pub struct LastTransactionWithdrawal<'info> {
    #[account(mut, address=verifier_state.load()?.signing_address)]
    pub signing_address: Signer<'info>,
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
        seeds = [verifier_state.load()?.nullifier0.as_ref(), LEAVES_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    /// CHECK:` doc comment explaining why no checks through types are necessary
    pub two_leaves_pda: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [verifier_state.load()?.tx_integrity_hash.as_ref(), STORAGE_SEED.as_ref()],
        bump, close= signing_address
    )]
    pub verifier_state: AccountLoader<'info, VerifierState>,

    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    //
    #[account(mut, address = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[verifier_state.load()?.merkle_tree_index as usize].1))]
    /// CHECK:` Merkle tree account liquidity pool pda.Should be registered in Merkle tree corresponding to the merkle tree address.
    pub merkle_tree_pda_token: AccountInfo<'info>,
    #[account(mut, address=verifier_state.load()?.recipient)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub recipient: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK:` Is not checked the relayer has complete freedom.
    pub relayer_recipient: AccountInfo<'info>,
    #[account(mut, address = solana_program::pubkey::Pubkey::find_program_address(&[&MERKLE_TREE_ACC_BYTES_ARRAY[verifier_state.load()?.merkle_tree_index as usize].0], &MerkleTreeProgram::id()).0)]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    /// CHECK: this is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut)]
    pub authority: UncheckedAccount<'info>,
    /// CHECK: Is the same as in integrity hash.
    #[account(mut, address = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[verifier_state.load()?.merkle_tree_index as usize].0))]
    pub merkle_tree: AccountInfo<'info>
}
