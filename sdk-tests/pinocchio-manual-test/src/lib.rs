//! Pinocchio-based test program for compressible PDA creation.
//!
//! This is a pinocchio port of the manual-test program.
//! Same instructions, same behavior, no Anchor dependency.

#![allow(deprecated)]

use light_account_pinocchio::{derive_light_cpi_signer, CpiSigner, LightFinalize, LightPreInit};
use light_macros::pubkey_array;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

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

// Re-exports for tests and other consumers
pub use account_loader::{
    PackedZeroCopyRecord, PackedZeroCopyRecordSeeds, PackedZeroCopyRecordVariant, ZeroCopyRecord,
    ZeroCopyRecordSeeds, ZeroCopyRecordVariant,
};
pub use all::{
    AllBorshSeeds, AllBorshVariant, AllZeroCopySeeds, AllZeroCopyVariant, PackedAllBorshSeeds,
    PackedAllBorshVariant, PackedAllZeroCopySeeds, PackedAllZeroCopyVariant,
};
pub use ata::accounts::*;
pub use derived_variants::{LightAccountVariant, PackedLightAccountVariant};
pub use light_account_pinocchio::{
    AccountType, CompressAndCloseParams, DecompressIdempotentParams, DecompressVariant,
    LightAccount,
};
pub use pda::{
    MinimalRecord, MinimalRecordSeeds, MinimalRecordVariant, PackedMinimalRecord,
    PackedMinimalRecordSeeds, PackedMinimalRecordVariant,
};
pub use token_account::accounts::*;
pub use two_mints::accounts::*;

pub const ID: Pubkey = pubkey_array!("7TWLq8Kmj1Cc3bGaEqsdNKMAiJSA7XN1JeKCN5nQeg2R");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("7TWLq8Kmj1Cc3bGaEqsdNKMAiJSA7XN1JeKCN5nQeg2R");

// ============================================================================
// Instruction Discriminators (8-byte, Anchor-compatible via sha256("global:{name}")[..8])
// ============================================================================
pub mod discriminators {
    pub const CREATE_PDA: [u8; 8] = [220, 10, 244, 120, 183, 4, 64, 232];
    pub const CREATE_ZERO_COPY: [u8; 8] = [172, 231, 175, 212, 64, 240, 20, 209];
    pub const CREATE_DERIVED_MINTS: [u8; 8] = [91, 123, 65, 133, 194, 45, 243, 75];
    pub const CREATE_TOKEN_VAULT: [u8; 8] = [161, 29, 12, 45, 127, 88, 61, 49];
    pub const CREATE_ATA: [u8; 8] = [26, 102, 168, 62, 117, 72, 168, 17];
    pub const CREATE_ALL: [u8; 8] = [149, 49, 144, 45, 208, 155, 177, 43];
    // These match the hardcoded discriminators in light_client::interface::instructions
    pub const INITIALIZE_COMPRESSION_CONFIG: [u8; 8] = [133, 228, 12, 169, 56, 76, 222, 61];
    pub const UPDATE_COMPRESSION_CONFIG: [u8; 8] = [135, 215, 243, 81, 163, 146, 33, 70];
    pub const COMPRESS_ACCOUNTS_IDEMPOTENT: [u8; 8] = [70, 236, 171, 120, 164, 93, 113, 181];
    pub const DECOMPRESS_ACCOUNTS_IDEMPOTENT: [u8; 8] = [114, 67, 61, 123, 234, 31, 1, 112];
}

// ============================================================================
// Entrypoint
// ============================================================================

/// Strip the 4-byte Vec<u8> borsh length prefix from instruction data.
///
/// The SDK client wraps serialized instruction data in Vec<u8> format
/// (4-byte little-endian length prefix + payload) for Anchor compatibility.
/// Anchor strips this automatically via its `Vec<u8>` parameter deserialization;
/// pinocchio programs must strip it manually.
#[inline]
fn strip_vec_wrapper(data: &[u8]) -> Result<&[u8], ProgramError> {
    if data.len() < 4 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let len = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
    if data.len() < 4 + len {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(&data[4..4 + len])
}

pinocchio::entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let (disc, data) = instruction_data.split_at(8);
    let disc: [u8; 8] = disc.try_into().unwrap();

    match disc {
        discriminators::CREATE_PDA => process_create_pda(accounts, data),
        discriminators::CREATE_ZERO_COPY => process_create_zero_copy(accounts, data),
        discriminators::CREATE_DERIVED_MINTS => process_create_derived_mints(accounts, data),
        discriminators::CREATE_TOKEN_VAULT => process_create_token_vault(accounts, data),
        discriminators::CREATE_ATA => process_create_ata(accounts, data),
        discriminators::CREATE_ALL => process_create_all(accounts, data),
        discriminators::INITIALIZE_COMPRESSION_CONFIG => {
            derived_light_config::process_initialize_config(accounts, data)
        }
        discriminators::UPDATE_COMPRESSION_CONFIG => {
            derived_light_config::process_update_config(accounts, data)
        }
        // The SDK client wraps compress/decompress instruction data in Vec<u8> format
        // (4-byte length prefix) for Anchor compatibility. Anchor strips this
        // automatically via its Vec<u8> parameter deserialization; pinocchio programs
        // must strip it manually.
        discriminators::COMPRESS_ACCOUNTS_IDEMPOTENT => {
            let inner = strip_vec_wrapper(data)?;
            derived_compress::process_compress_and_close(accounts, inner)
        }
        discriminators::DECOMPRESS_ACCOUNTS_IDEMPOTENT => {
            let inner = strip_vec_wrapper(data)?;
            derived_decompress::process_decompress_idempotent(accounts, inner)
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

// ============================================================================
// Instruction Handlers
// ============================================================================

fn process_create_pda(accounts: &[AccountInfo], data: &[u8]) -> Result<(), ProgramError> {
    use borsh::BorshDeserialize;
    use pda::accounts::{CreatePda, CreatePdaParams};

    let params =
        CreatePdaParams::deserialize(&mut &data[..]).map_err(|_| ProgramError::BorshIoError)?;

    let remaining_start = CreatePda::FIXED_LEN;
    let (fixed_accounts, remaining_accounts) = accounts.split_at(remaining_start);
    let mut ctx = CreatePda::parse(fixed_accounts, &params)?;

    let has_pre_init = ctx
        .light_pre_init(remaining_accounts, &params)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    // Business logic: set account data
    {
        let mut account_data = ctx
            .record
            .try_borrow_mut_data()
            .map_err(|_| ProgramError::AccountBorrowFailed)?;
        let record = pda::state::MinimalRecord::mut_from_account_data(&mut account_data);
        record.owner = params.owner;
    }

    ctx.light_finalize(remaining_accounts, &params, has_pre_init)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    Ok(())
}

fn process_create_zero_copy(accounts: &[AccountInfo], data: &[u8]) -> Result<(), ProgramError> {
    use account_loader::accounts::{CreateZeroCopy, CreateZeroCopyParams};
    use borsh::BorshDeserialize;

    let params = CreateZeroCopyParams::deserialize(&mut &data[..])
        .map_err(|_| ProgramError::BorshIoError)?;

    let remaining_start = CreateZeroCopy::FIXED_LEN;
    let (fixed_accounts, remaining_accounts) = accounts.split_at(remaining_start);
    let mut ctx = CreateZeroCopy::parse(fixed_accounts, &params)?;

    let has_pre_init = ctx
        .light_pre_init(remaining_accounts, &params)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    // Business logic: set zero-copy account data
    {
        let mut account_data = ctx
            .record
            .try_borrow_mut_data()
            .map_err(|_| ProgramError::AccountBorrowFailed)?;
        let record_bytes =
            &mut account_data[8..8 + core::mem::size_of::<account_loader::ZeroCopyRecord>()];
        let record: &mut account_loader::ZeroCopyRecord = bytemuck::from_bytes_mut(record_bytes);
        record.owner = params.owner;
        record.value = params.value;
    }

    ctx.light_finalize(remaining_accounts, &params, has_pre_init)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    Ok(())
}

fn process_create_derived_mints(accounts: &[AccountInfo], data: &[u8]) -> Result<(), ProgramError> {
    use borsh::BorshDeserialize;
    use two_mints::accounts::{CreateDerivedMintsAccounts, CreateDerivedMintsParams};

    let params = CreateDerivedMintsParams::deserialize(&mut &data[..])
        .map_err(|_| ProgramError::BorshIoError)?;

    let remaining_start = CreateDerivedMintsAccounts::FIXED_LEN;
    let (fixed_accounts, remaining_accounts) = accounts.split_at(remaining_start);
    let mut ctx = CreateDerivedMintsAccounts::parse(fixed_accounts, &params)?;

    let has_pre_init = ctx
        .light_pre_init(remaining_accounts, &params)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    ctx.light_finalize(remaining_accounts, &params, has_pre_init)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    Ok(())
}

fn process_create_token_vault(accounts: &[AccountInfo], data: &[u8]) -> Result<(), ProgramError> {
    use borsh::BorshDeserialize;
    use token_account::accounts::{CreateTokenVaultAccounts, CreateTokenVaultParams};

    let params = CreateTokenVaultParams::deserialize(&mut &data[..])
        .map_err(|_| ProgramError::BorshIoError)?;

    let remaining_start = CreateTokenVaultAccounts::FIXED_LEN;
    let (fixed_accounts, remaining_accounts) = accounts.split_at(remaining_start);
    let mut ctx = CreateTokenVaultAccounts::parse(fixed_accounts)?;

    let has_pre_init = ctx
        .light_pre_init(remaining_accounts, &params)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    ctx.light_finalize(remaining_accounts, &params, has_pre_init)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    Ok(())
}

fn process_create_ata(accounts: &[AccountInfo], data: &[u8]) -> Result<(), ProgramError> {
    use ata::accounts::{CreateAtaAccounts, CreateAtaParams};
    use borsh::BorshDeserialize;

    let params =
        CreateAtaParams::deserialize(&mut &data[..]).map_err(|_| ProgramError::BorshIoError)?;

    let remaining_start = CreateAtaAccounts::FIXED_LEN;
    let (fixed_accounts, remaining_accounts) = accounts.split_at(remaining_start);
    let mut ctx = CreateAtaAccounts::parse(fixed_accounts)?;

    let has_pre_init = ctx
        .light_pre_init(remaining_accounts, &params)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    ctx.light_finalize(remaining_accounts, &params, has_pre_init)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    Ok(())
}

fn process_create_all(accounts: &[AccountInfo], data: &[u8]) -> Result<(), ProgramError> {
    use all::accounts::{CreateAllAccounts, CreateAllParams};
    use borsh::BorshDeserialize;

    let params =
        CreateAllParams::deserialize(&mut &data[..]).map_err(|_| ProgramError::BorshIoError)?;

    let remaining_start = CreateAllAccounts::FIXED_LEN;
    let (fixed_accounts, remaining_accounts) = accounts.split_at(remaining_start);
    let mut ctx = CreateAllAccounts::parse(fixed_accounts, &params)?;

    let has_pre_init = ctx
        .light_pre_init(remaining_accounts, &params)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    // Business logic: set PDA data
    {
        let mut borsh_data = ctx
            .borsh_record
            .try_borrow_mut_data()
            .map_err(|_| ProgramError::AccountBorrowFailed)?;
        let borsh_record = pda::state::MinimalRecord::mut_from_account_data(&mut borsh_data);
        borsh_record.owner = params.owner;
    }
    {
        let mut zc_data = ctx
            .zero_copy_record
            .try_borrow_mut_data()
            .map_err(|_| ProgramError::AccountBorrowFailed)?;
        let record_bytes =
            &mut zc_data[8..8 + core::mem::size_of::<account_loader::ZeroCopyRecord>()];
        let record: &mut account_loader::ZeroCopyRecord = bytemuck::from_bytes_mut(record_bytes);
        record.owner = params.owner;
        record.value = params.value;
    }

    ctx.light_finalize(remaining_accounts, &params, has_pre_init)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    Ok(())
}
