//! Accounts module for create_all instruction.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use solana_account_info::AccountInfo;

use crate::account_loader::ZeroCopyRecord;
use crate::pda::MinimalRecord;

/// Seed constants for ALL module (DIFFERENT from pda/account_loader modules)
pub const ALL_BORSH_SEED: &[u8] = b"all_borsh";
pub const ALL_ZERO_COPY_SEED: &[u8] = b"all_zero_copy";
pub const ALL_MINT_SIGNER_SEED: &[u8] = b"all_mint_signer";
pub const ALL_TOKEN_VAULT_SEED: &[u8] = b"all_vault";

/// Parameters for creating all account types in a single instruction.
#[derive(Clone, AnchorSerialize, AnchorDeserialize, Debug)]
pub struct CreateAllParams {
    /// Proof for creating PDAs and mint addresses (3 addresses: 2 PDAs + 1 Mint).
    pub create_accounts_proof: CreateAccountsProof,
    /// Bump for the mint signer PDA.
    pub mint_signer_bump: u8,
    /// Bump for the token vault PDA.
    pub token_vault_bump: u8,
    /// Owner pubkey (used as seed for both PDAs).
    pub owner: Pubkey,
    /// Value for the zero-copy record.
    pub value: u64,
}

/// Accounts struct for creating all account types in a single instruction.
///
/// CPI context indices:
/// - PDA 0: Borsh PDA (MinimalRecord) - index 0
/// - PDA 1: ZeroCopy PDA (ZeroCopyRecord) - index 1
/// - Mint 0: Compressed mint - index 2 (offset by NUM_LIGHT_PDAS=2)
#[derive(Accounts)]
#[instruction(params: CreateAllParams)]
pub struct CreateAllAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Authority for the mint (mint::authority = authority)
    pub authority: Signer<'info>,

    /// CHECK: Compression config PDA for this program (for PDAs)
    pub compression_config: AccountInfo<'info>,

    // ==================== Borsh PDA ====================
    #[account(
        init,
        payer = payer,
        space = 8 + MinimalRecord::INIT_SPACE,
        seeds = [b"all_borsh", params.owner.as_ref()],
        bump,
    )]
    pub borsh_record: Account<'info, MinimalRecord>,

    // ==================== Zero-Copy PDA ====================
    #[account(
        init,
        payer = payer,
        space = 8 + ZeroCopyRecord::INIT_SPACE,
        seeds = [b"all_zero_copy", params.owner.as_ref()],
        bump,
    )]
    pub zero_copy_record: AccountLoader<'info, ZeroCopyRecord>,

    // ==================== Mint ====================
    /// CHECK: PDA mint signer
    #[account(
        seeds = [ALL_MINT_SIGNER_SEED, authority.key().as_ref()],
        bump = params.mint_signer_bump,
    )]
    pub mint_signer: UncheckedAccount<'info>,

    /// CHECK: Mint PDA - derived from mint_signer by light-token
    #[account(mut)]
    pub mint: UncheckedAccount<'info>,

    // ==================== Token Vault ====================
    /// CHECK: Token vault PDA
    #[account(
        mut,
        seeds = [ALL_TOKEN_VAULT_SEED, mint.key().as_ref()],
        bump = params.token_vault_bump,
    )]
    pub token_vault: UncheckedAccount<'info>,

    /// CHECK: Owner of the token vault
    pub vault_owner: AccountInfo<'info>,

    // ==================== ATA ====================
    /// CHECK: Owner of the ATA
    pub ata_owner: AccountInfo<'info>,

    /// CHECK: Associated Token Account
    #[account(mut)]
    pub user_ata: UncheckedAccount<'info>,

    // ==================== Infrastructure ====================
    /// CHECK: CompressibleConfig for light-token program
    pub compressible_config: AccountInfo<'info>,

    /// CHECK: Rent sponsor PDA
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token program
    pub light_token_program: AccountInfo<'info>,

    /// CHECK: CPI authority PDA
    pub cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
