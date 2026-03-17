use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, StateMerkleTreeAccount,
};
use anchor_lang::prelude::*;

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
    if leaf_indices[0] == u32::MAX || leaf_indices[1] == u32::MAX {
        return err!(RegistryError::InvalidProofEncoding);
    }
    Ok(if leaf_indices[2] == u32::MAX {
        2
    } else if leaf_indices[3] == u32::MAX {
        3
    } else {
        4
    })
}

#[allow(clippy::too_many_arguments)]
pub fn process_nullify_dedup(
    ctx: &Context<NullifyLeaves>,
    count: usize,
    change_log_index: u16,
    queue_indices: [u16; 4],
    leaf_indices: [u32; 4],
    proof_2_shared: u16,
    proof_3_source: u32,
    proof_4_source: u32,
    shared_top_node: [u8; 32],
    nodes: Vec<[u8; 32]>,
) -> Result<()> {
    let bump = ctx.bumps.cpi_authority;
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];

    // Reconstruct proofs from dedup encoding.
    let mut cursor: usize = 0;

    // proof_1: levels 0..14 from nodes[0..15]
    if nodes.len() < 15 {
        return err!(RegistryError::InvalidProofEncoding);
    }
    let mut proof_1 = [[0u8; 32]; 16];
    proof_1[..15].copy_from_slice(&nodes[cursor..cursor + 15]);
    proof_1[15] = shared_top_node;
    cursor += 15;

    // proof_2: bitvec proof_2_shared, bit i=1 means reuse proof_1[i], bit=0 means take next node
    let mut proof_2 = [[0u8; 32]; 16];
    for i in 0..15 {
        if (proof_2_shared >> i) & 1 == 1 {
            proof_2[i] = proof_1[i];
        } else {
            if cursor >= nodes.len() {
                return err!(RegistryError::InvalidProofEncoding);
            }
            proof_2[i] = nodes[cursor];
            cursor += 1;
        }
    }
    proof_2[15] = shared_top_node;

    // Issue CPIs for proof_1 and proof_2 immediately to free stack space
    // before reconstructing proof_3/proof_4.
    let change_log_index_u64 = change_log_index as u64;
    nullify_single_leaf_cpi(
        ctx,
        signer_seeds,
        change_log_index_u64,
        queue_indices[0],
        leaf_indices[0] as u64,
        proof_1.to_vec(),
    )?;
    nullify_single_leaf_cpi(
        ctx,
        signer_seeds,
        change_log_index_u64,
        queue_indices[1],
        leaf_indices[1] as u64,
        proof_2.to_vec(),
    )?;

    // proof_3: 2 bits per level from proof_3_source
    if count >= 3 {
        let mut proof_3 = [[0u8; 32]; 16];
        for i in 0..15 {
            let src = (proof_3_source >> (i * 2)) & 0b11;
            match src {
                0b00 => proof_3[i] = proof_1[i],
                0b01 => proof_3[i] = proof_2[i],
                0b10 => {
                    if cursor >= nodes.len() {
                        return err!(RegistryError::InvalidProofEncoding);
                    }
                    proof_3[i] = nodes[cursor];
                    cursor += 1;
                }
                _ => return err!(RegistryError::InvalidProofEncoding),
            }
        }
        proof_3[15] = shared_top_node;

        nullify_single_leaf_cpi(
            ctx,
            signer_seeds,
            change_log_index_u64,
            queue_indices[2],
            leaf_indices[2] as u64,
            proof_3.to_vec(),
        )?;

        // proof_4: 2 bits per level from proof_4_source
        if count == 4 {
            let mut proof_4 = [[0u8; 32]; 16];
            for i in 0..15 {
                let src = (proof_4_source >> (i * 2)) & 0b11;
                match src {
                    0b00 => proof_4[i] = proof_1[i],
                    0b01 => proof_4[i] = proof_2[i],
                    0b10 => proof_4[i] = proof_3[i],
                    0b11 => {
                        if cursor >= nodes.len() {
                            return err!(RegistryError::InvalidProofEncoding);
                        }
                        proof_4[i] = nodes[cursor];
                        cursor += 1;
                    }
                    _ => unreachable!(),
                }
            }
            proof_4[15] = shared_top_node;

            nullify_single_leaf_cpi(
                ctx,
                signer_seeds,
                change_log_index_u64,
                queue_indices[3],
                leaf_indices[3] as u64,
                proof_4.to_vec(),
            )?;
        }
    }

    // Validate all nodes consumed
    if cursor != nodes.len() {
        return err!(RegistryError::InvalidProofEncoding);
    }

    Ok(())
}
