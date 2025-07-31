pub mod create_mint;
pub mod create_pda;
pub mod mint_to;
pub mod processor;

use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CreateCompressedMint<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub mint_authority: Signer<'info>,
    pub mint_seed: Signer<'info>,
    /// CHECK:
    pub ctoken_program: UncheckedAccount<'info>,
    /// CHECK:
    pub ctoken_cpi_authority: UncheckedAccount<'info>,
}
