use crate::groth16_verifier::VerifierState;

use anchor_lang::prelude::*;

use crate::escrow::escrow_state::FeeEscrowState;
use merkle_tree_program::{
    program::MerkleTreeProgram,
    utils::config::MERKLE_TREE_ACC_BYTES_ARRAY,
    utils::constants::{LEAVES_SEED, NF_SEED, STORAGE_SEED},
    PreInsertedLeavesIndex,
};

use crate::utils::config::ESCROW_SEED;

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
    /// CHECK:` Nullifier account which will be initialized via cpi.
    pub nullifier0_pda: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [verifier_state.load()?.nullifier1.as_ref(), NF_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    /// CHECK:` Nullifier account which will be initialized via cpi.
    pub nullifier1_pda: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [verifier_state.load()?.nullifier0.as_ref(), LEAVES_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    /// CHECK:` Leaves account which will be initialized via cpi.
    pub two_leaves_pda: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [verifier_state.load()?.tx_integrity_hash.as_ref(), STORAGE_SEED.as_ref()],
        bump, close = signing_address
    )]
    pub verifier_state: AccountLoader<'info, VerifierState>,

    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    // merkle tree account liquidity pool pda
    #[account(mut, address = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[verifier_state.load()?.merkle_tree_index as usize].1))]
    /// CHECK: The pda which serves as liquidity pool for a registered merkle tree.
    pub merkle_tree_pda_token: AccountInfo<'info>,
    /// CHECK: Is the same as in integrity hash.
    #[account(mut, address = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[verifier_state.load()?.merkle_tree_index as usize].0))]
    pub merkle_tree: AccountInfo<'info>,
    /// account from which funds are transferred
    #[account(
        mut,
        seeds = [verifier_state.load()?.tx_integrity_hash.as_ref(), ESCROW_SEED.as_ref()], bump,
        constraint= verifier_state.key() == fee_escrow_state.verifier_state_pubkey,
        close = signing_address
    )]
    pub fee_escrow_state: Account<'info, FeeEscrowState>,
    #[account(
        mut,
        address = solana_program::pubkey::Pubkey::find_program_address(&[merkle_tree.key().to_bytes().as_ref()], &MerkleTreeProgram::id()).0)]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    /// CHECK: this is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut, seeds= [MerkleTreeProgram::id().to_bytes().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,

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
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    #[account(mut, address = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[verifier_state.load()?.merkle_tree_index as usize].1))]
    /// CHECK:` Merkle tree account liquidity pool pda. Should be registered in Merkle tree corresponding to the merkle tree address.
    pub merkle_tree_pda_token: AccountInfo<'info>,
    #[account(mut, address=verifier_state.load()?.recipient)]
    /// CHECK:` that it is the same recipient as in tx integrity hash
    pub recipient: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK:` Is not checked the relayer has complete freedom.
    pub relayer_recipient: AccountInfo<'info>,
    #[account(mut, address = solana_program::pubkey::Pubkey::find_program_address(&[merkle_tree.key().to_bytes().as_ref()], &MerkleTreeProgram::id()).0)]
    pub pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
    /// CHECK: this is the cpi authority and will be enforced in the Merkle tree program.
    #[account(mut, seeds= [MerkleTreeProgram::id().to_bytes().as_ref()], bump)]
    pub authority: UncheckedAccount<'info>,
    /// CHECK: Is the same as in integrity hash.
    #[account(mut, address = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[verifier_state.load()?.merkle_tree_index as usize].0))]
    pub merkle_tree: AccountInfo<'info>,
}
