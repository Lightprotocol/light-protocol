use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use light_system_program::program::LightSystemProgram;

/// Creates a Token-2022 mint account that corresponds to a compressed mint,
/// creates a token pool, and mints existing supply to the pool
#[derive(Accounts)]
pub struct CreateSplMintInstruction<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Authority for the compressed mint (must match mint_authority in compressed mint)
    pub authority: Signer<'info>,
    /// CHECK: created in instruction.
    #[account(mut)]
    pub mint: UncheckedAccount<'info>,

    pub mint_signer: UncheckedAccount<'info>,

    /// Token pool PDA account (will be created manually in process function)
    /// CHECK: created in instruction
    #[account(mut)]
    pub token_pool_pda: UncheckedAccount<'info>,

    /// Token-2022 program
    pub token_program: Program<'info, Token2022>,

    /// CPI authority for compressed account operations
    pub cpi_authority_pda: UncheckedAccount<'info>,

    /// Light system program for compressed account updates
    pub light_system_program: Program<'info, LightSystemProgram>,

    /// Registered program PDA for light system program
    pub registered_program_pda: UncheckedAccount<'info>,

    /// NoOp program for event emission
    pub noop_program: UncheckedAccount<'info>,

    /// Authority for account compression
    pub account_compression_authority: UncheckedAccount<'info>,

    /// Account compression program
    pub account_compression_program: Program<'info, AccountCompression>,

    pub system_program: Program<'info, System>,
    pub self_program: Program<'info, crate::program::LightCompressedToken>,
    // TODO: pack these accounts.
    /// Output queue account where compressed mint will be stored
    /// CHECK: Validated by light-system-program
    #[account(mut)]
    pub in_output_queue: AccountInfo<'info>,
    /// Output queue account where compressed mint will be stored
    /// CHECK: Validated by light-system-program
    #[account(mut)]
    pub in_merkle_tree: AccountInfo<'info>,
    /// Output queue account where compressed mint will be stored
    /// CHECK: Validated by light-system-program
    #[account(mut)]
    pub out_output_queue: AccountInfo<'info>,
}
