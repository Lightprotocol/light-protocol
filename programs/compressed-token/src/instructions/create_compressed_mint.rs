use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;
use light_system_program::program::LightSystemProgram;

use crate::program::LightCompressedToken;

/// Creates a compressed mint stored as a compressed account
#[derive(Accounts)]
pub struct CreateCompressedMintInstruction<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CPI authority for compressed account creation
    pub cpi_authority_pda: AccountInfo<'info>,

    /// Light system program for compressed account creation
    pub light_system_program: Program<'info, LightSystemProgram>,

    /// Account compression program
    pub account_compression_program: Program<'info, AccountCompression>,

    /// Registered program PDA for light system program
    pub registered_program_pda: AccountInfo<'info>,

    /// NoOp program for event emission
    pub noop_program: AccountInfo<'info>,

    /// Authority for account compression
    pub account_compression_authority: AccountInfo<'info>,

    /// Self program reference
    pub self_program: Program<'info, LightCompressedToken>,

    pub system_program: Program<'info, System>,

    /// Address merkle tree for compressed account creation
    /// CHECK: Validated by light-system-program
    #[account(mut)]
    pub address_merkle_tree: AccountInfo<'info>,

    /// Output queue account where compressed mint will be stored
    /// CHECK: Validated by light-system-program
    #[account(mut)]
    pub output_queue: AccountInfo<'info>,

    /// Signer used as seed for PDA derivation (ensures uniqueness)
    pub mint_signer: Signer<'info>,
}
