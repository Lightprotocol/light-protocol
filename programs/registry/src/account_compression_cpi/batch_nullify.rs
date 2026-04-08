use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;
use light_merkle_tree_metadata::fee::FORESTER_REIMBURSEMENT_CAP;

use crate::{
    fee_reimbursement::{initialize::REIMBURSEMENT_PDA_SEED, state::ReimbursementPda},
    ForesterEpochPda,
};

#[derive(Accounts)]
pub struct BatchNullify<'info> {
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
    pub merkle_tree: AccountInfo<'info>,
    /// Per-tree reimbursement escrow. Disburses to forester after CPI.
    #[account(
        mut,
        seeds = [REIMBURSEMENT_PDA_SEED, merkle_tree.key().as_ref()],
        bump,
    )]
    pub reimbursement_pda: Account<'info, ReimbursementPda>,
}

pub fn process_batch_nullify(
    ctx: &Context<BatchNullify>,
    bump: u8,
    data: Vec<u8>,
    network_fee: u64,
) -> Result<()> {
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let accounts = account_compression::cpi::accounts::BatchNullify {
        authority: ctx.accounts.cpi_authority.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.clone()),
        log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::batch_nullify(cpi_ctx, data)?;

    // After CPI: reimburse forester from PDA (5000 lamports) if funded.
    // Uses direct lamport manipulation (not CPI) because the registry program owns the PDA.
    if network_fee >= FORESTER_REIMBURSEMENT_CAP {
        let pda_info = ctx.accounts.reimbursement_pda.to_account_info();
        let rent_exempt = Rent::get()?.minimum_balance(pda_info.data_len());
        let available = pda_info.lamports().saturating_sub(rent_exempt);
        if available >= FORESTER_REIMBURSEMENT_CAP {
            // Direct lamport transfer: registry owns the PDA.
            **pda_info.try_borrow_mut_lamports()? -= FORESTER_REIMBURSEMENT_CAP;
            **ctx
                .accounts
                .authority
                .to_account_info()
                .try_borrow_mut_lamports()? += FORESTER_REIMBURSEMENT_CAP;
        }
    }

    Ok(())
}
