pub mod create_pda;
mod mint;
mod processor;
use anchor_lang::prelude::*;
pub use processor::process_ctoken_pda;

#[derive(Accounts)]
pub struct CTokenPda<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub mint_authority: Signer<'info>,
    pub mint_seed: Signer<'info>,
    /// CHECK:
    pub light_token_program: UncheckedAccount<'info>,
    /// CHECK:
    pub light_token_cpi_authority: UncheckedAccount<'info>,
}
