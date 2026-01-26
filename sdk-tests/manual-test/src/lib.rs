//! Minimal test program for #[light_account(init)] PDA macro validation.
//!
//! This program tests ONLY the compressible PDA creation macro in isolation,
//! ensuring the simplest PDA-only program compiles and works correctly.
//!
//! Supports both Borsh-serialized accounts (Account<T>) and zero-copy accounts
//! (AccountLoader<T>) for demonstrating different compressible PDA patterns.

#![allow(deprecated)]

use anchor_lang::prelude::*;
use light_sdk::derive_light_cpi_signer;
use light_sdk::interface::{LightFinalize, LightPreInit};
use light_sdk_types::CpiSigner;
use solana_program_error::ProgramError;

pub mod account_loader;
pub mod all;
pub mod ata;
pub mod derived_compress;
pub mod derived_decompress;
pub mod derived_light_config;
pub mod derived_variants;
pub mod pda;
pub mod token_account;
pub mod two_mints;

// Re-export account_loader accounts at crate root (required for Anchor's #[program] macro)
pub use account_loader::accounts::*;
pub use account_loader::{
    PackedZeroCopyRecord, PackedZeroCopyRecordSeeds, PackedZeroCopyRecordVariant, ZeroCopyRecord,
    ZeroCopyRecordSeeds, ZeroCopyRecordVariant,
};
pub use derived_compress::*;
pub use derived_decompress::*;
pub use derived_light_config::*;
pub use derived_variants::{PackedProgramAccountVariant, ProgramAccountVariant};
pub use pda::accounts::*;
pub use pda::{
    MinimalRecord, MinimalRecordSeeds, MinimalRecordVariant, PackedMinimalRecord,
    PackedMinimalRecordSeeds, PackedMinimalRecordVariant,
};
pub use light_sdk::interface::{
    AccountType, CompressAndCloseParams, DecompressIdempotentParams, DecompressVariant,
    LightAccount, LightAccountVariant, PackedLightAccountVariant,
};
pub use token_account::accounts::*;
pub use two_mints::accounts::*;
pub use ata::accounts::*;
pub use all::accounts::*;
pub use all::{
    AllBorshSeeds, AllBorshVariant, AllZeroCopySeeds, AllZeroCopyVariant, PackedAllBorshSeeds,
    PackedAllBorshVariant, PackedAllZeroCopySeeds, PackedAllZeroCopyVariant,
};

declare_id!("PdaT111111111111111111111111111111111111111");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("PdaT111111111111111111111111111111111111111");

// #[light_program]
#[program]
pub mod manual_test {
    use super::*;

    /// Create a single compressible PDA.
    /// The account is created by Anchor and made compressible by the
    /// manual LightPreInit/LightFinalize trait implementations.
    pub fn create_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePda<'info>>,
        params: CreatePdaParams,
    ) -> Result<()> {
        // 1. Pre-init: creates compressed address via Light System Program CPI
        //    and sets compression_info on the account
        let has_pre_init = ctx
            .accounts
            .light_pre_init(ctx.remaining_accounts, &params)
            .map_err(|e| anchor_lang::error::Error::from(ProgramError::from(e)))?;

        // 2. Business logic: set account data
        ctx.accounts.record.owner = params.owner;

        // 3. Finalize: no-op for PDA-only flow
        ctx.accounts
            .light_finalize(ctx.remaining_accounts, &params, has_pre_init)
            .map_err(|e| anchor_lang::error::Error::from(ProgramError::from(e)))?;

        Ok(())
    }

    /// Initialize the compression config for this program.
    /// Named to match SDK's InitializeRentFreeConfig discriminator.
    pub fn initialize_compression_config<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeConfig<'info>>,
        params: InitConfigParams,
    ) -> Result<()> {
        derived_light_config::process_initialize_config(ctx, params)
    }

    /// Update the compression config for this program.
    /// Named to match SDK's expected discriminator.
    pub fn update_compression_config<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateConfig<'info>>,
        params: UpdateConfigParams,
    ) -> Result<()> {
        derived_light_config::process_update_config(ctx, params)
    }

    /// Compress and close PDA accounts, returning rent to the sponsor.
    /// Named to match SDK/forester expected discriminator.
    ///
    /// NOTE: Empty Accounts struct - everything in remaining_accounts.
    /// Deserialization happens inside process_compress_pda_accounts_idempotent.
    pub fn compress_accounts_idempotent<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressAndClose>,
        instruction_data: Vec<u8>,
    ) -> Result<()> {
        derived_compress::process_compress_and_close(ctx.remaining_accounts, &instruction_data)
    }

    /// Decompress compressed accounts back into PDAs idempotently.
    /// Named to match SDK expected discriminator.
    ///
    /// NOTE: Empty Accounts struct - everything in remaining_accounts.
    /// Deserialization happens inside process_decompress_pda_accounts_idempotent.
    pub fn decompress_accounts_idempotent<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressIdempotent>,
        instruction_data: Vec<u8>,
    ) -> Result<()> {
        derived_decompress::process_decompress_idempotent(ctx.remaining_accounts, &instruction_data)
    }

    /// Create a single zero-copy compressible PDA using AccountLoader.
    /// Demonstrates zero-copy access pattern with `load_init()`.
    pub fn create_zero_copy<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateZeroCopy<'info>>,
        params: CreateZeroCopyParams,
    ) -> Result<()> {
        // 1. Pre-init: creates compressed address via Light System Program CPI
        //    and sets compression_info on the account
        let has_pre_init = ctx
            .accounts
            .light_pre_init(ctx.remaining_accounts, &params)
            .map_err(|e| anchor_lang::error::Error::from(ProgramError::from(e)))?;

        // 2. Business logic: set account data using load_init() pattern
        {
            let mut record = ctx.accounts.record.load_init()?;
            record.owner = params.owner.to_bytes();
            record.value = params.value;
        }

        // 3. Finalize: no-op for PDA-only flow
        ctx.accounts
            .light_finalize(ctx.remaining_accounts, &params, has_pre_init)
            .map_err(|e| anchor_lang::error::Error::from(ProgramError::from(e)))?;

        Ok(())
    }

    /// Create two compressed mints using derived PDAs as mint signers.
    /// Manual implementation of what #[light_account(init, mint::...)] generates.
    /// Demonstrates minimal params pattern where program derives everything from accounts.
    pub fn create_derived_mints<'a, 'info>(
        ctx: Context<'a, '_, 'info, 'info, CreateDerivedMintsAccounts<'info>>,
        params: CreateDerivedMintsParams,
    ) -> Result<()> {
        // 1. Pre-init: creates mints via Light Token Program CPI
        let has_pre_init = ctx
            .accounts
            .light_pre_init(ctx.remaining_accounts, &params)
            .map_err(|e| anchor_lang::error::Error::from(ProgramError::from(e)))?;

        // 2. No business logic for mint-only creation

        // 3. Finalize: no-op for mint-only flow
        ctx.accounts
            .light_finalize(ctx.remaining_accounts, &params, has_pre_init)
            .map_err(|e| anchor_lang::error::Error::from(ProgramError::from(e)))?;

        Ok(())
    }

    /// Create a PDA token vault using CreateTokenAccountCpi.
    /// Manual implementation of what #[light_account(init, token::...)] generates.
    /// Demonstrates rent-free token account creation for program-owned vaults.
    pub fn create_token_vault<'a, 'info>(
        ctx: Context<'a, '_, 'info, 'info, CreateTokenVaultAccounts<'info>>,
        params: CreateTokenVaultParams,
    ) -> Result<()> {
        // 1. Pre-init: creates token vault via Light Token Program CPI
        let has_pre_init = ctx
            .accounts
            .light_pre_init(ctx.remaining_accounts, &params)
            .map_err(|e| anchor_lang::error::Error::from(ProgramError::from(e)))?;

        // 2. No business logic for token vault-only creation

        // 3. Finalize: no-op for token vault-only flow
        ctx.accounts
            .light_finalize(ctx.remaining_accounts, &params, has_pre_init)
            .map_err(|e| anchor_lang::error::Error::from(ProgramError::from(e)))?;

        Ok(())
    }

    /// Create an Associated Token Account using CreateTokenAtaCpi.
    /// Manual implementation of what #[light_account(init, associated_token::...)] generates.
    /// Demonstrates rent-free ATA creation for user wallets.
    pub fn create_ata<'a, 'info>(
        ctx: Context<'a, '_, 'info, 'info, CreateAtaAccounts<'info>>,
        params: CreateAtaParams,
    ) -> Result<()> {
        // 1. Pre-init: creates ATA via Light Token Program CPI
        let has_pre_init = ctx
            .accounts
            .light_pre_init(ctx.remaining_accounts, &params)
            .map_err(|e| anchor_lang::error::Error::from(ProgramError::from(e)))?;

        // 2. No business logic for ATA-only creation

        // 3. Finalize: no-op for ATA-only flow
        ctx.accounts
            .light_finalize(ctx.remaining_accounts, &params, has_pre_init)
            .map_err(|e| anchor_lang::error::Error::from(ProgramError::from(e)))?;

        Ok(())
    }

    /// Create all account types in a single instruction.
    /// Demonstrates combining PDAs + Mints + Token vault + ATA in one transaction.
    ///
    /// Account types created:
    /// - Borsh PDA (MinimalRecord)
    /// - ZeroCopy PDA (ZeroCopyRecord)
    /// - Compressed Mint
    /// - Token Vault
    /// - Associated Token Account (ATA)
    pub fn create_all<'a, 'info>(
        ctx: Context<'a, '_, 'info, 'info, CreateAllAccounts<'info>>,
        params: CreateAllParams,
    ) -> Result<()> {
        // 1. Pre-init: creates all accounts via CPIs
        let has_pre_init = ctx
            .accounts
            .light_pre_init(ctx.remaining_accounts, &params)
            .map_err(|e| anchor_lang::error::Error::from(ProgramError::from(e)))?;

        // 2. Business logic: set PDA data
        ctx.accounts.borsh_record.owner = params.owner;
        {
            let mut record = ctx.accounts.zero_copy_record.load_init()?;
            record.owner = params.owner.to_bytes();
            record.value = params.value;
        }

        // 3. Finalize: no-op for this flow
        ctx.accounts
            .light_finalize(ctx.remaining_accounts, &params, has_pre_init)
            .map_err(|e| anchor_lang::error::Error::from(ProgramError::from(e)))?;

        Ok(())
    }
}
