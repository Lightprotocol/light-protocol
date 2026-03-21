use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;

use crate::epoch::register_epoch::ForesterEpochPda;
use crate::errors::RegistryError;
use crate::protocol_config::state::ProtocolConfigPda;

#[derive(Accounts)]
pub struct ClaimFeesWrapper<'info> {
    /// CHECK: only eligible foresters can claim fees.
    #[account(mut)]
    pub registered_forester_pda: Option<Account<'info, ForesterEpochPda>>,
    pub authority: Signer<'info>,
    /// CHECK: (seed constraints) used to invoke account compression program via cpi.
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    /// CHECK: (account compression program) group access control.
    pub registered_program_pda: AccountInfo<'info>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: (account compression program) the tree/queue to claim from.
    #[account(mut)]
    pub merkle_tree_or_queue: AccountInfo<'info>,
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
    /// CHECK: must match protocol_config.protocol_fee_recipient.
    #[account(mut)]
    pub fee_recipient: AccountInfo<'info>,
}

pub fn process_claim_fees_wrapper(ctx: &Context<ClaimFeesWrapper>, bump: u8) -> Result<()> {
    // Verify fee_recipient matches protocol config.
    let expected_recipient = ctx.accounts.protocol_config_pda.config.protocol_fee_recipient;
    if expected_recipient == Pubkey::default() {
        return err!(RegistryError::InvalidFeeRecipient);
    }
    if ctx.accounts.fee_recipient.key() != expected_recipient {
        return err!(RegistryError::InvalidFeeRecipient);
    }

    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];

    let accounts = account_compression::cpi::accounts::ClaimFees {
        authority: ctx.accounts.cpi_authority.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
        merkle_tree_or_queue: ctx.accounts.merkle_tree_or_queue.to_account_info(),
        fee_recipient: ctx.accounts.fee_recipient.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::claim_fees(cpi_ctx)
}
