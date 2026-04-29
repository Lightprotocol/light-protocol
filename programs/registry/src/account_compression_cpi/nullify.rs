use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, StateMerkleTreeAccount,
};
use anchor_lang::prelude::*;
use bitvec::prelude::*;

use crate::{epoch::register_epoch::ForesterEpochPda, errors::RegistryError};

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

/// Issues a single nullify_leaves CPI for one leaf.
#[inline(always)]
fn nullify_single_leaf_cpi(
    ctx: &Context<NullifyLeaves>,
    signer_seeds: &[&[&[u8]]],
    change_log_index: u64,
    queue_index: u16,
    leaf_index: u64,
    proof: Vec<[u8; 32]>,
) -> Result<()> {
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
        vec![change_log_index],
        vec![queue_index],
        vec![leaf_index],
        vec![proof],
    )
}

/// Determines proof count from leaf_indices sentinel values.
/// Returns Err(InvalidProofEncoding) if fewer than 2 leaves are specified.
pub fn count_from_leaf_indices(leaf_indices: &[u32; 4]) -> Result<usize> {
    match *leaf_indices {
        [a, b, u32::MAX, u32::MAX] if a != u32::MAX && b != u32::MAX => Ok(2),
        [a, b, c, u32::MAX] if a != u32::MAX && b != u32::MAX && c != u32::MAX => Ok(3),
        [a, b, c, d] if a != u32::MAX && b != u32::MAX && c != u32::MAX && d != u32::MAX => Ok(4),
        _ => err!(RegistryError::InvalidProofEncoding),
    }
}

/// Reconstructs a 16-node Merkle proof by selecting nodes from a
/// deduplicated pool. The bitvec selects which pool nodes belong to
/// this proof (exactly 16 bits must be set).
fn reconstruct_proof(nodes: &[[u8; 32]], bits: u32) -> Result<[[u8; 32]; 16]> {
    let bv = bits.view_bits::<Lsb0>();
    let mut proof = [[0u8; 32]; 16];
    let mut proof_idx = 0;
    for i in 0..nodes.len() {
        if bv[i] {
            if proof_idx >= 16 {
                return err!(RegistryError::InvalidProofEncoding);
            }
            proof[proof_idx] = nodes[i];
            proof_idx += 1;
        }
    }
    if proof_idx != 16 {
        return err!(RegistryError::InvalidProofEncoding);
    }
    Ok(proof)
}

pub fn process_nullify_state_v1_multi(
    ctx: &Context<NullifyLeaves>,
    count: usize,
    change_log_index: u16,
    queue_indices: [u16; 4],
    leaf_indices: [u32; 4],
    proof_bitvecs: [u32; 4],
    nodes: Vec<[u8; 32]>,
) -> Result<()> {
    if nodes.len() > 32 {
        return err!(RegistryError::InvalidProofEncoding);
    }

    let bump = ctx.bumps.cpi_authority;
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let change_log_index_u64 = change_log_index as u64;

    for i in 0..count {
        let proof = reconstruct_proof(&nodes, proof_bitvecs[i])?;
        nullify_single_leaf_cpi(
            ctx,
            signer_seeds,
            change_log_index_u64,
            queue_indices[i],
            leaf_indices[i] as u64,
            proof.to_vec(),
        )?;
    }

    Ok(())
}
