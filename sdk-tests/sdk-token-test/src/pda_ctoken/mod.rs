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
    #[account(mut)]
    pub token_account: UncheckedAccount<'info>,
    /// CHECK:
    pub ctoken_program: UncheckedAccount<'info>,
    /// CHECK:
    pub ctoken_cpi_authority: UncheckedAccount<'info>,
}
