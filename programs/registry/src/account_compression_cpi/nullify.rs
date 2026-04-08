use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, StateMerkleTreeAccount,
};
use anchor_lang::prelude::*;
use light_merkle_tree_metadata::fee::FORESTER_REIMBURSEMENT_CAP;

use crate::{
    epoch::register_epoch::ForesterEpochPda,
    fee_reimbursement::{initialize::REIMBURSEMENT_PDA_SEED, state::ReimbursementPda},
};

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
    /// Per-tree reimbursement escrow. Receives excess clawback after CPI.
    #[account(
        mut,
        seeds = [REIMBURSEMENT_PDA_SEED, merkle_tree.key().as_ref()],
        bump,
    )]
    pub reimbursement_pda: Account<'info, ReimbursementPda>,
    pub system_program: Program<'info, System>,
}

pub fn process_nullify(
    ctx: &Context<NullifyLeaves>,
    bump: u8,
    change_log_indices: Vec<u64>,
    leaves_queue_indices: Vec<u16>,
    indices: Vec<u64>,
    proofs: Vec<Vec<[u8; 32]>>,
    network_fee: u64,
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
    )?;

    // After CPI: claw back excess above FORESTER_REIMBURSEMENT_CAP to PDA.
    if network_fee > FORESTER_REIMBURSEMENT_CAP {
        let excess = network_fee - FORESTER_REIMBURSEMENT_CAP;
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.authority.to_account_info(),
                    to: ctx.accounts.reimbursement_pda.to_account_info(),
                },
            ),
            excess,
        )?;
    }

    Ok(())
}
