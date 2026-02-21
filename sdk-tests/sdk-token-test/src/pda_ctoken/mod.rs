mod create_pda;
pub mod mint;
mod processor;

use anchor_lang::prelude::*;
pub use create_pda::*;
pub use processor::{process_pda_ctoken, ChainedCtokenInstructionData, PdaCreationData};

#[derive(Accounts)]
pub struct PdaCToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub mint_authority: Signer<'info>,
    pub mint_seed: Signer<'info>,
    /// CHECK:
    pub light_token_program: UncheckedAccount<'info>,
    /// CHECK:
    pub light_token_cpi_authority: UncheckedAccount<'info>,
    /// CHECK: Rent sponsor PDA that receives the mint creation fee.
    #[account(mut)]
    pub rent_sponsor: UncheckedAccount<'info>,
}
