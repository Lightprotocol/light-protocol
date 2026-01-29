//! Standard Anchor accounts struct for create_token_vault instruction.

use anchor_lang::prelude::*;
use solana_account_info::AccountInfo;

/// Seed constant for token vault PDA
pub const TOKEN_VAULT_SEED: &[u8] = b"vault";

/// Minimal params for token vault creation.
#[derive(Clone, AnchorSerialize, AnchorDeserialize, Debug)]
pub struct CreateTokenVaultParams {
    pub vault_bump: u8,
}

/// Accounts struct for creating a PDA token vault.
///
/// What the macro would look like:
/// ```rust,ignore
/// #[account(mut)]
/// #[light_account(init,
///     token::mint = mint,
///     token::owner = vault_owner,
///     token::authority = [TOKEN_VAULT_SEED, mint.key().as_ref()],
///     token::bump = params.vault_bump
/// )]
/// pub token_vault: UncheckedAccount<'info>,
/// ```
#[derive(Accounts)]
#[instruction(params: CreateTokenVaultParams)]
pub struct CreateTokenVaultAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The mint for the token account
    /// CHECK: Validated by light-token program
    pub mint: AccountInfo<'info>,

    /// Owner of the token account (can be any pubkey)
    /// CHECK: Just a pubkey, no validation needed
    pub vault_owner: AccountInfo<'info>,

    /// CHECK: Token vault PDA - derived from [TOKEN_VAULT_SEED, mint.key()]
    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, mint.key().as_ref()],
        bump = params.vault_bump,
    )]
    pub token_vault: UncheckedAccount<'info>,

    // ========== Infrastructure accounts for CreateTokenAccountCpi ==========
    /// CHECK: CompressibleConfig for light-token program
    pub compressible_config: AccountInfo<'info>,

    /// CHECK: Rent sponsor PDA
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
