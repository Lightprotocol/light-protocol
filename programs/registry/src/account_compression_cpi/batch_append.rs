use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;
use light_merkle_tree_metadata::fee::FORESTER_REIMBURSEMENT_CAP;

use crate::fee_reimbursement::initialize::REIMBURSEMENT_PDA_SEED;
use crate::fee_reimbursement::state::ReimbursementPda;
use crate::ForesterEpochPda;

#[derive(Accounts)]
pub struct BatchAppend<'info> {
    /// CHECK: only eligible foresters can append leaves. Is checked in ix.
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
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK: (account compression program).
    #[account(mut)]
    pub output_queue: AccountInfo<'info>,
    /// CHECK: receives network fee reimbursement.
    #[account(mut)]
    pub fee_payer: UncheckedAccount<'info>,
    /// Per-tree reimbursement escrow. Funded by forester after CPI.
    #[account(
        mut,
        seeds = [REIMBURSEMENT_PDA_SEED, merkle_tree.key().as_ref()],
        bump,
    )]
    pub reimbursement_pda: Account<'info, ReimbursementPda>,
    pub system_program: Program<'info, System>,
}

pub fn process_batch_append(
    ctx: &Context<BatchAppend>,
    bump: u8,
    data: Vec<u8>,
    network_fee: u64,
) -> Result<()> {
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let accounts = account_compression::cpi::accounts::BatchAppend {
        authority: ctx.accounts.cpi_authority.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.clone()),
        log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
        output_queue: ctx.accounts.output_queue.to_account_info(),
        fee_payer: ctx.accounts.fee_payer.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::batch_append(cpi_ctx, data)?;

    // After CPI: transfer FORESTER_REIMBURSEMENT_CAP from forester to reimbursement PDA.
    // This pre-funds the PDA for batch_nullify reimbursements.
    // Uses system_program::transfer (CPI) because the forester is a signer.
    if network_fee >= FORESTER_REIMBURSEMENT_CAP {
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.authority.to_account_info(),
                    to: ctx.accounts.reimbursement_pda.to_account_info(),
                },
            ),
            FORESTER_REIMBURSEMENT_CAP,
        )?;
    }

    Ok(())
}
