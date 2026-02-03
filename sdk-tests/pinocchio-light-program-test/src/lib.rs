//! Test program for #[derive(LightProgramPinocchio)] macro validation.
//!
//! Uses #[derive(LightProgramPinocchio)] to generate compress/decompress dispatch,
//! config handlers, and variant types. No Anchor dependency.

#![allow(deprecated)]

use light_account_pinocchio::{derive_light_cpi_signer, CpiSigner, LightFinalize, LightPreInit};
use light_macros::pubkey_array;
use light_sdk_macros::LightProgramPinocchio;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

pub mod account_loader;
pub mod all;
pub mod ata;
pub mod derived_state;
pub mod mint;
pub mod pda;
pub mod state;
pub mod token_account;
pub mod two_mints;

pub use derived_state::*;
pub use state::*;

pub const ID: Pubkey = pubkey_array!("DrvPda11111111111111111111111111111111111111");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("DrvPda11111111111111111111111111111111111111");

pub const VAULT_AUTH_SEED: &[u8] = b"vault_auth";
pub const VAULT_SEED: &[u8] = b"vault";
pub const RECORD_SEED: &[u8] = b"zero_copy_record";
pub const MINT_SIGNER_SEED_A: &[u8] = b"mint_signer_a";
pub const MINT_SIGNER_SEED_B: &[u8] = b"mint_signer_b";

/// This generates: variant enums, compress/decompress dispatch, config handlers,
/// per-variant Seeds/Variant/Packed types, LightAccountVariantTrait impls,
/// size validation, seed providers, and client functions.
#[derive(LightProgramPinocchio)]
pub enum ProgramAccounts {
    #[light_account(pda::seeds = [b"minimal_record", ctx.owner])]
    MinimalRecord(MinimalRecord),

    #[light_account(associated_token)]
    Ata,

    #[light_account(token::seeds = [VAULT_SEED, ctx.mint], token::owner_seeds = [VAULT_AUTH_SEED])]
    Vault,

    #[light_account(pda::seeds = [RECORD_SEED, ctx.owner], pda::zero_copy)]
    ZeroCopyRecord(ZeroCopyRecord),
}

// ============================================================================
// Instruction Discriminators (Anchor-compatible: sha256("global:{name}")[..8])
// ============================================================================
pub mod discriminators {
    pub const CREATE_PDA: [u8; 8] = [220, 10, 244, 120, 183, 4, 64, 232];
    pub const CREATE_ATA: [u8; 8] = [26, 102, 168, 62, 117, 72, 168, 17];
    pub const CREATE_TOKEN_VAULT: [u8; 8] = [161, 29, 12, 45, 127, 88, 61, 49];
    pub const CREATE_ZERO_COPY_RECORD: [u8; 8] = [6, 252, 72, 240, 45, 91, 28, 6];
    pub const CREATE_MINT: [u8; 8] = [69, 44, 215, 132, 253, 214, 41, 45];
    pub const CREATE_TWO_MINTS: [u8; 8] = [222, 41, 188, 84, 174, 115, 236, 105];
    pub const CREATE_ALL: [u8; 8] = [149, 49, 144, 45, 208, 155, 177, 43];
    // SDK-standard discriminators (must match light-client)
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
/// Pinocchio programs must strip it manually.
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
        discriminators::CREATE_ATA => process_create_ata(accounts, data),
        discriminators::CREATE_TOKEN_VAULT => process_create_token_vault(accounts, data),
        discriminators::CREATE_ZERO_COPY_RECORD => process_create_zero_copy_record(accounts, data),
        discriminators::CREATE_MINT => process_create_mint(accounts, data),
        discriminators::CREATE_TWO_MINTS => process_create_two_mints(accounts, data),
        discriminators::CREATE_ALL => process_create_all(accounts, data),
        discriminators::INITIALIZE_COMPRESSION_CONFIG => {
            ProgramAccounts::process_initialize_config(accounts, data)
        }
        discriminators::UPDATE_COMPRESSION_CONFIG => {
            ProgramAccounts::process_update_config(accounts, data)
        }
        discriminators::COMPRESS_ACCOUNTS_IDEMPOTENT => {
            let inner = strip_vec_wrapper(data)?;
            ProgramAccounts::process_compress(accounts, inner)
        }
        discriminators::DECOMPRESS_ACCOUNTS_IDEMPOTENT => {
            let inner = strip_vec_wrapper(data)?;
            ProgramAccounts::process_decompress(accounts, inner)
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
        let mut record = state::MinimalRecord::try_from_slice(&account_data[8..])
            .map_err(|_| ProgramError::BorshIoError)?;
        record.owner = params.owner;
        let serialized = borsh::to_vec(&record).map_err(|_| ProgramError::BorshIoError)?;
        account_data[8..8 + serialized.len()].copy_from_slice(&serialized);
    }

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

fn process_create_zero_copy_record(
    accounts: &[AccountInfo],
    data: &[u8],
) -> Result<(), ProgramError> {
    use account_loader::accounts::{CreateZeroCopyRecord, CreateZeroCopyRecordParams};
    use borsh::BorshDeserialize;

    let params = CreateZeroCopyRecordParams::deserialize(&mut &data[..])
        .map_err(|_| ProgramError::BorshIoError)?;

    let remaining_start = CreateZeroCopyRecord::FIXED_LEN;
    let (fixed_accounts, remaining_accounts) = accounts.split_at(remaining_start);
    let mut ctx = CreateZeroCopyRecord::parse(fixed_accounts, &params)?;

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
            &mut account_data[8..8 + core::mem::size_of::<state::ZeroCopyRecord>()];
        let record: &mut state::ZeroCopyRecord = bytemuck::from_bytes_mut(record_bytes);
        record.owner = params.owner;
    }

    ctx.light_finalize(remaining_accounts, &params, has_pre_init)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    Ok(())
}

fn process_create_mint(accounts: &[AccountInfo], data: &[u8]) -> Result<(), ProgramError> {
    use borsh::BorshDeserialize;
    use mint::accounts::{CreateMintAccounts, CreateMintParams};

    let params =
        CreateMintParams::deserialize(&mut &data[..]).map_err(|_| ProgramError::BorshIoError)?;

    let remaining_start = CreateMintAccounts::FIXED_LEN;
    let (fixed_accounts, remaining_accounts) = accounts.split_at(remaining_start);
    let mut ctx = CreateMintAccounts::parse(fixed_accounts, &params)?;

    let has_pre_init = ctx
        .light_pre_init(remaining_accounts, &params)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    ctx.light_finalize(remaining_accounts, &params, has_pre_init)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    Ok(())
}

fn process_create_two_mints(accounts: &[AccountInfo], data: &[u8]) -> Result<(), ProgramError> {
    use borsh::BorshDeserialize;
    use two_mints::accounts::{CreateTwoMintsAccounts, CreateTwoMintsParams};

    let params = CreateTwoMintsParams::deserialize(&mut &data[..])
        .map_err(|_| ProgramError::BorshIoError)?;

    let remaining_start = CreateTwoMintsAccounts::FIXED_LEN;
    let (fixed_accounts, remaining_accounts) = accounts.split_at(remaining_start);
    let mut ctx = CreateTwoMintsAccounts::parse(fixed_accounts, &params)?;

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
        let mut borsh_record = state::MinimalRecord::try_from_slice(&borsh_data[8..])
            .map_err(|_| ProgramError::BorshIoError)?;
        borsh_record.owner = params.owner;
        let serialized =
            borsh::to_vec(&borsh_record).map_err(|_| ProgramError::BorshIoError)?;
        borsh_data[8..8 + serialized.len()].copy_from_slice(&serialized);
    }
    {
        let mut zc_data = ctx
            .zero_copy_record
            .try_borrow_mut_data()
            .map_err(|_| ProgramError::AccountBorrowFailed)?;
        let record_bytes =
            &mut zc_data[8..8 + core::mem::size_of::<state::ZeroCopyRecord>()];
        let record: &mut state::ZeroCopyRecord = bytemuck::from_bytes_mut(record_bytes);
        record.owner = params.owner;
    }

    ctx.light_finalize(remaining_accounts, &params, has_pre_init)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    Ok(())
}
