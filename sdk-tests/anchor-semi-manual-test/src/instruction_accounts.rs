//! Accounts module for single-pda-derive-test.

use anchor_lang::prelude::*;
use light_account::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};
use light_sdk_macros::LightAccounts;
use light_sdk_types::{interface::CreateAccountsProof, LIGHT_TOKEN_PROGRAM_ID};

use crate::{
    state::{MinimalRecord, ZeroCopyRecord},
    MINT_SIGNER_SEED_A, MINT_SIGNER_SEED_B, RECORD_SEED, VAULT_AUTH_SEED, VAULT_SEED,
};

// =============================================================================
// 1. CreatePda
// =============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreatePdaParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Minimal accounts struct for testing single PDA creation.
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreatePdaParams)]
pub struct CreatePda<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for rent reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + MinimalRecord::INIT_SPACE,
        seeds = [b"minimal_record", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub record: Account<'info, MinimalRecord>,

    pub system_program: Program<'info, System>,
}

// =============================================================================
// 2. CreateAta
// =============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateAtaParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub ata_bump: u8,
}

/// Accounts struct for testing single ATA creation.
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateAtaParams)]
pub struct CreateAta<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint for the ATA
    pub ata_mint: AccountInfo<'info>,

    /// CHECK: Owner of the ATA
    pub ata_owner: AccountInfo<'info>,

    /// ATA account - created via LightFinalize CPI.
    #[account(mut)]
    #[light_account(init, associated_token::authority = ata_owner, associated_token::mint = ata_mint, associated_token::bump = params.ata_bump)]
    pub ata: UncheckedAccount<'info>,

    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light Token Program for CPI
    #[account(address = LIGHT_TOKEN_PROGRAM_ID.into())]
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

// =============================================================================
// 3. CreateTokenVault
// =============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateTokenVaultParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub vault_bump: u8,
}

/// Accounts struct for testing single token vault creation.
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateTokenVaultParams)]
pub struct CreateTokenVault<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint
    pub mint: AccountInfo<'info>,

    #[account(
        seeds = [VAULT_AUTH_SEED],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    /// Token vault account - created via LightFinalize CPI.
    #[account(
        mut,
        seeds = [VAULT_SEED, mint.key().as_ref()],
        bump,
    )]
    #[light_account(init, token::seeds = [VAULT_SEED, self.mint.key()], token::mint = mint, token::owner = vault_authority, token::owner_seeds = [VAULT_AUTH_SEED])]
    pub vault: UncheckedAccount<'info>,

    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

// =============================================================================
// 4. CreateZeroCopyRecord
// =============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateZeroCopyRecordParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Accounts struct for creating a zero-copy record.
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateZeroCopyRecordParams)]
pub struct CreateZeroCopyRecord<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config PDA
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for rent reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + core::mem::size_of::<ZeroCopyRecord>(),
        seeds = [RECORD_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init, zero_copy)]
    pub record: AccountLoader<'info, ZeroCopyRecord>,

    pub system_program: Program<'info, System>,
}

// =============================================================================
// 5. CreateMint
// =============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateMintParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub mint_signer_bump: u8,
}

/// Accounts struct for testing single mint creation.
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateMintParams)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub authority: Signer<'info>,

    /// CHECK: PDA derived from authority
    #[account(
        seeds = [MINT_SIGNER_SEED_A, authority.key().as_ref()],
        bump,
    )]
    pub mint_signer: UncheckedAccount<'info>,

    /// CHECK: Initialized by light_mint CPI
    #[account(mut)]
    #[light_account(init,
        mint::signer = mint_signer,
        mint::authority = fee_payer,
        mint::decimals = 9,
        mint::seeds = &[MINT_SIGNER_SEED_A, self.authority.to_account_info().key.as_ref()],
        mint::bump = params.mint_signer_bump
    )]
    pub mint: UncheckedAccount<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: CToken config
    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    /// CHECK: CToken rent sponsor
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: CToken program
    pub light_token_program: AccountInfo<'info>,

    /// CHECK: CToken CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

// =============================================================================
// 6. CreateTwoMints
// =============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateTwoMintsParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub mint_signer_bump_a: u8,
    pub mint_signer_bump_b: u8,
}

/// Accounts struct for testing two mint creation in a single instruction.
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateTwoMintsParams)]
pub struct CreateTwoMints<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub authority: Signer<'info>,

    /// CHECK: PDA for mint A
    #[account(
        seeds = [MINT_SIGNER_SEED_A, authority.key().as_ref()],
        bump,
    )]
    pub mint_signer_a: UncheckedAccount<'info>,

    /// CHECK: Mint A - initialized by light_mint CPI
    #[account(mut)]
    #[light_account(init,
        mint::signer = mint_signer_a,
        mint::authority = fee_payer,
        mint::decimals = 9,
        mint::seeds = &[MINT_SIGNER_SEED_A, self.authority.to_account_info().key.as_ref()],
        mint::bump = params.mint_signer_bump_a
    )]
    pub mint_a: UncheckedAccount<'info>,

    /// CHECK: PDA for mint B
    #[account(
        seeds = [MINT_SIGNER_SEED_B, authority.key().as_ref()],
        bump,
    )]
    pub mint_signer_b: UncheckedAccount<'info>,

    /// CHECK: Mint B - initialized by light_mint CPI
    #[account(mut)]
    #[light_account(init,
        mint::signer = mint_signer_b,
        mint::authority = fee_payer,
        mint::decimals = 6,
        mint::seeds = &[MINT_SIGNER_SEED_B, self.authority.to_account_info().key.as_ref()],
        mint::bump = params.mint_signer_bump_b
    )]
    pub mint_b: UncheckedAccount<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: CToken config
    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    /// CHECK: CToken rent sponsor
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: CToken program
    pub light_token_program: AccountInfo<'info>,

    /// CHECK: CToken CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

// =============================================================================
// 7. CreateAll
// =============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateAllParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    pub ata_bump: u8,
    pub vault_bump: u8,
    pub mint_signer_bump_a: u8,
    pub mint_signer_bump_b: u8,
}

/// Combined accounts struct exercising all variant types in one instruction.
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateAllParams)]
pub struct CreateAll<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    // -- PDA --
    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + MinimalRecord::INIT_SPACE,
        seeds = [b"minimal_record", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub record: Account<'info, MinimalRecord>,

    // -- Zero-copy --
    #[account(
        init,
        payer = fee_payer,
        space = 8 + core::mem::size_of::<ZeroCopyRecord>(),
        seeds = [RECORD_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init, zero_copy)]
    pub zero_copy_record: AccountLoader<'info, ZeroCopyRecord>,

    // -- ATA --
    /// CHECK: Token mint for the ATA
    pub ata_mint: AccountInfo<'info>,

    /// CHECK: Owner of the ATA
    pub ata_owner: AccountInfo<'info>,

    #[account(mut)]
    #[light_account(init, associated_token::authority = ata_owner, associated_token::mint = ata_mint, associated_token::bump = params.ata_bump)]
    pub ata: UncheckedAccount<'info>,

    // -- Token vault --
    /// CHECK: Token mint for the vault
    pub vault_mint: AccountInfo<'info>,

    #[account(
        seeds = [VAULT_AUTH_SEED],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, vault_mint.key().as_ref()],
        bump,
    )]
    #[light_account(init, token::seeds = [VAULT_SEED, self.vault_mint.key()], token::mint = vault_mint, token::owner = vault_authority, token::owner_seeds = [VAULT_AUTH_SEED])]
    pub vault: UncheckedAccount<'info>,

    // -- Mint A --
    pub authority: Signer<'info>,

    /// CHECK: PDA for mint A
    #[account(
        seeds = [MINT_SIGNER_SEED_A, authority.key().as_ref()],
        bump,
    )]
    pub mint_signer_a: UncheckedAccount<'info>,

    #[account(mut)]
    #[light_account(init,
        mint::signer = mint_signer_a,
        mint::authority = fee_payer,
        mint::decimals = 9,
        mint::seeds = &[MINT_SIGNER_SEED_A, self.authority.to_account_info().key.as_ref()],
        mint::bump = params.mint_signer_bump_a
    )]
    pub mint_a: UncheckedAccount<'info>,

    // -- Mint B --
    /// CHECK: PDA for mint B
    #[account(
        seeds = [MINT_SIGNER_SEED_B, authority.key().as_ref()],
        bump,
    )]
    pub mint_signer_b: UncheckedAccount<'info>,

    #[account(mut)]
    #[light_account(init,
        mint::signer = mint_signer_b,
        mint::authority = fee_payer,
        mint::decimals = 6,
        mint::seeds = &[MINT_SIGNER_SEED_B, self.authority.to_account_info().key.as_ref()],
        mint::bump = params.mint_signer_bump_b
    )]
    pub mint_b: UncheckedAccount<'info>,

    // -- Infrastructure --
    /// CHECK: CToken config
    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    /// CHECK: CToken rent sponsor
    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: CToken CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: CToken program
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
