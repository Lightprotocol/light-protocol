//! Standard Anchor accounts struct for create_derived_mints instruction.

use anchor_lang::prelude::*;
use light_account::CreateAccountsProof;
use solana_account_info::AccountInfo;

/// Seed constants
pub const MINT_SIGNER_0_SEED: &[u8] = b"mint_signer_0";
pub const MINT_SIGNER_1_SEED: &[u8] = b"mint_signer_1";

/// Minimal params - matches macro pattern.
#[derive(Clone, AnchorSerialize, AnchorDeserialize, Debug)]
pub struct CreateDerivedMintsParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub mint_signer_0_bump: u8,
    pub mint_signer_1_bump: u8,
}

/// Accounts struct - matches macro pattern with mint signers as PDAs.
#[derive(Accounts)]
#[instruction(params: CreateDerivedMintsParams)]
pub struct CreateDerivedMintsAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Authority for both mints (mint::authority = authority)
    pub authority: Signer<'info>,

    /// CHECK: PDA mint signer 0 (mint::signer = mint_signer_0)
    #[account(
        seeds = [MINT_SIGNER_0_SEED, authority.key().as_ref()],
        bump = params.mint_signer_0_bump,
    )]
    pub mint_signer_0: UncheckedAccount<'info>,

    /// CHECK: PDA mint signer 1 (mint::signer = mint_signer_1)
    #[account(
        seeds = [MINT_SIGNER_1_SEED, authority.key().as_ref()],
        bump = params.mint_signer_1_bump,
    )]
    pub mint_signer_1: UncheckedAccount<'info>,

    /// CHECK: Mint 0 PDA - derived from mint_signer_0 by light-token
    #[account(mut)]
    pub mint_0: UncheckedAccount<'info>,

    /// CHECK: Mint 1 PDA - derived from mint_signer_1 by light-token
    #[account(mut)]
    pub mint_1: UncheckedAccount<'info>,

    // ========== Infrastructure accounts for invoke_create_mints ==========
    /// CHECK: CompressibleConfig for light-token program
    pub compressible_config: AccountInfo<'info>,

    /// CHECK: Rent sponsor PDA
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,

    /// CHECK: CPI authority PDA
    pub cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
