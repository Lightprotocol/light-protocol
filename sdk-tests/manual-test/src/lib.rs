//! Minimal test program for #[light_account(init)] PDA macro validation.
//!
//! This program tests ONLY the compressible PDA creation macro in isolation,
//! ensuring the simplest PDA-only program compiles and works correctly.

#![allow(deprecated)]

use anchor_lang::prelude::*;
use light_sdk::derive_light_cpi_signer;
use light_sdk::interface::{LightFinalize, LightPreInit};
use light_sdk_types::CpiSigner;
use solana_program_error::ProgramError;

pub mod light_config;
pub mod pda;
pub mod sdk_functions;
pub mod traits;

pub use light_config::*;
pub use pda::accounts::*;
pub use pda::{MinimalRecord, PackedMinimalRecord};
pub use traits::{AccountType, LightAccount, LightAccountVariant, PackedLightAccountVariant};

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
    pub fn initialize_config<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeConfig<'info>>,
        params: InitConfigParams,
    ) -> Result<()> {
        light_config::process_initialize_config(ctx, params)
    }

    /// Update the compression config for this program.
    pub fn update_config<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateConfig<'info>>,
        params: UpdateConfigParams,
    ) -> Result<()> {
        light_config::process_update_config(ctx, params)
    }
}
