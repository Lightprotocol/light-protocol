use anchor_lang::prelude::*;
use crate::state::VerifierState;
use merkle_tree_program::program::MerkleTreeProgram;
#[derive(Accounts)]
#[instruction(
    nullifier0: [u8;32],
    nullifier1: [u8;32],
)]
pub struct LastTransactionDeposit<'info> {
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub nullifier0_pda: UncheckedAccount<'info>,
    // #[account(init, seeds = [nullifier1.as_ref(), b"nf"], bump,  payer=signing_address, space=8, owner=merkle_tree.key())]
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub nullifier1_pda: UncheckedAccount<'info>,//Account<'info, Nullifier>,
    #[account(mut)]
    // #[account(init, seeds = [nullifier0.as_ref(), b"leaves"], bump,  payer=signing_address, space=8+96 + 8 + 256, owner=merkle_tree.key() )]
    /// CHECK:` doc comment explaining why no checks through types are necessary
    pub leaves_pda: UncheckedAccount<'info>,
    #[account(mut)]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    // #[account(seeds = [nullifier0.as_ref(), b"esrow"], bump)]
    #[account(mut)]
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub escrow_pda: UncheckedAccount<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
     /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub merkle_tree_tmp_storage: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    // merkle tree account liquidity pool pda
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub merkle_tree_pda_token: AccountInfo<'info>,
    // account from which funds are transferred
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub user_account: Signer<'info>,
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub merkle_tree:  AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(
    nullifier0: [u8;32],
    nullifier1: [u8;32],
)]
pub struct LastTransactionWithdrawal<'info> {
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub nullifier0_pda: UncheckedAccount<'info>,
    // #[account(init, seeds = [nullifier1.as_ref(), b"nf"], bump,  payer=signing_address, space=8, owner=merkle_tree.key())]
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub nullifier1_pda: UncheckedAccount<'info>,//Account<'info, Nullifier>,
    #[account(mut)]
    // #[account(init, seeds = [nullifier0.as_ref(), b"leaves"], bump,  payer=signing_address, space=8+96 + 8 + 256, owner=merkle_tree.key() )]
    /// CHECK:` doc comment explaining why no checks through types are necessary
    pub leaves_pda: UncheckedAccount<'info>,
    #[account(mut)]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    // #[account(seeds = [nullifier0.as_ref(), b"esrow"], bump)]
    #[account(mut)]
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub escrow_pda: UncheckedAccount<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    #[account(mut)]
    pub signing_address: Signer<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub merkle_tree_tmp_storage: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    // merkle tree account liquidity pool pda
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub merkle_tree_pda_token: AccountInfo<'info>,
    // account from which funds are transferred
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub merkle_tree:  AccountInfo<'info>,
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub recipient:  AccountInfo<'info>,
    #[account(mut)]
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    pub relayer_recipient:  AccountInfo<'info>,
}
