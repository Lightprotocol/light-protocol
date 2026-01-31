//! Standard Anchor accounts struct for create_ata instruction.

use anchor_lang::prelude::*;
use solana_account_info::AccountInfo;

/// Params for ATA creation (empty - bump is derived automatically).
#[derive(Clone, AnchorSerialize, AnchorDeserialize, Debug, Default)]
pub struct CreateAtaParams {}

/// Accounts struct for creating an Associated Token Account.
///
/// What the macro would look like:
/// ```rust,ignore
/// #[light_account(init, associated_token,
///     associated_token::authority = ata_owner,
///     associated_token::mint = mint
/// )]
/// pub user_ata: UncheckedAccount<'info>,
/// ```
#[derive(Accounts)]
pub struct CreateAtaAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The mint for the ATA
    /// CHECK: Validated by light-token program
    pub mint: AccountInfo<'info>,

    /// Owner of the ATA (authority)
    /// CHECK: Can be any pubkey - the wallet that owns this ATA
    pub ata_owner: AccountInfo<'info>,

    /// CHECK: Associated Token Account - derived from [owner, LIGHT_TOKEN_PROGRAM_ID, mint]
    #[account(mut)]
    pub user_ata: UncheckedAccount<'info>,

    // ========== Infrastructure accounts for CreateTokenAtaCpi ==========
    /// CHECK: CompressibleConfig for light-token program
    pub compressible_config: AccountInfo<'info>,

    /// CHECK: Rent sponsor PDA
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
