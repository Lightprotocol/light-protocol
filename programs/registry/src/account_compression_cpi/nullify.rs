use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, StateMerkleTreeAccount,
};
use anchor_lang::prelude::*;

use crate::epoch::register_epoch::ForesterEpochPda;

#[derive(Accounts)]
pub struct NullifyLeaves<'info> {
    /// CHECK: only eligible foresters can nullify leaves. Is checked in ix.
    #[account(mut)]
    pub registered_forester_pda: Option<Account<'info, ForesterEpochPda>>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: (seed constraints) used to invoke account compression program via cpi.
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    /// CHECK: (account compression program) group access control.
    pub registered_program_pda: AccountInfo<'info>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: (account compression program) when emitting event.
    pub log_wrapper: UncheckedAccount<'info>,
    /// CHECK: (account compression program).
    #[account(mut)]
    pub merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
    /// CHECK: (account compression program).
    #[account(mut)]
    pub nullifier_queue: AccountInfo<'info>,
}

pub fn process_nullify(
    ctx: &Context<NullifyLeaves>,
    bump: u8,
    change_log_indices: Vec<u64>,
    leaves_queue_indices: Vec<u16>,
    indices: Vec<u64>,
    proofs: Vec<Vec<[u8; 32]>>,
) -> Result<()> {
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let accounts = account_compression::cpi::accounts::NullifyLeaves {
        authority: ctx.accounts.cpi_authority.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
        log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        nullifier_queue: ctx.accounts.nullifier_queue.to_account_info(),
        fee_payer: Some(ctx.accounts.authority.to_account_info()),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::nullify_leaves(
        cpi_ctx,
        change_log_indices,
        leaves_queue_indices,
        indices,
        proofs,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn process_nullify_2(
    ctx: &Context<NullifyLeaves>,
    change_log_index: u16,
    queue_index_0: u16,
    queue_index_1: u16,
    leaf_index_0: u32,
    leaf_index_1: u32,
    proof_0: [[u8; 32]; 15],
    proof_1: [[u8; 32]; 15],
    shared_proof_node: [u8; 32],
) -> Result<()> {
    let bump = ctx.bumps.cpi_authority;
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];

    // Reconstruct full 16-node proofs by appending the shared node (level 15).
    let mut full_proof_0: Vec<[u8; 32]> = proof_0.to_vec();
    full_proof_0.push(shared_proof_node);
    let mut full_proof_1: Vec<[u8; 32]> = proof_1.to_vec();
    full_proof_1.push(shared_proof_node);

    // First CPI: nullify leaf 0
    {
        let accounts = account_compression::cpi::accounts::NullifyLeaves {
            authority: ctx.accounts.cpi_authority.to_account_info(),
            registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
            log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
            merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
            nullifier_queue: ctx.accounts.nullifier_queue.to_account_info(),
            fee_payer: Some(ctx.accounts.authority.to_account_info()),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            signer_seeds,
        );
        account_compression::cpi::nullify_leaves(
            cpi_ctx,
            vec![change_log_index as u64],
            vec![queue_index_0],
            vec![leaf_index_0 as u64],
            vec![full_proof_0],
        )?;
    }

    // Second CPI: nullify leaf 1 (same change_log_index -- proof is patched via changelog replay)
    {
        let accounts = account_compression::cpi::accounts::NullifyLeaves {
            authority: ctx.accounts.cpi_authority.to_account_info(),
            registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
            log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
            merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
            nullifier_queue: ctx.accounts.nullifier_queue.to_account_info(),
            fee_payer: Some(ctx.accounts.authority.to_account_info()),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            signer_seeds,
        );
        account_compression::cpi::nullify_leaves(
            cpi_ctx,
            vec![change_log_index as u64],
            vec![queue_index_1],
            vec![leaf_index_1 as u64],
            vec![full_proof_1],
        )?;
    }

    Ok(())
}
