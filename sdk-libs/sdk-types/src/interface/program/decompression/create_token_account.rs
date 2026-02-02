//! ATA and token account creation helpers for decompression.
//!
//! Returns `(instruction_data, account_metas)` tuples for use with `AI::invoke_cpi()`.

use light_account_checks::CpiMeta;
use light_token_interface::{
    instructions::{
        create_associated_token_account::CreateAssociatedTokenAccountInstructionData,
        create_token_account::CreateTokenAccountInstructionData,
        extensions::{CompressToPubkey, CompressibleExtensionInstructionData},
    },
    LIGHT_TOKEN_PROGRAM_ID,
};

use crate::{error::LightSdkTypesError, AnchorSerialize};

/// Build instruction data and account metas for creating a compressible ATA.
///
/// Returns `(data, account_metas, program_id)` for use with `AI::invoke_cpi()`.
///
/// # Account order (per on-chain handler):
/// 0. owner (non-mut, non-signer)
/// 1. mint (non-mut, non-signer)
/// 2. fee_payer (signer, writable)
/// 3. associated_token_account (writable, NOT signer)
/// 4. system_program (readonly)
/// 5. compressible_config (readonly)
/// 6. rent_payer (writable)
#[allow(clippy::too_many_arguments)]
pub fn build_create_ata_instruction(
    wallet_owner: &[u8; 32],
    mint: &[u8; 32],
    fee_payer: &[u8; 32],
    ata: &[u8; 32],
    bump: u8,
    compressible_config: &[u8; 32],
    rent_sponsor: &[u8; 32],
    write_top_up: u32,
) -> Result<(Vec<u8>, Vec<CpiMeta>), LightSdkTypesError> {
    let instruction_data = CreateAssociatedTokenAccountInstructionData {
        bump,
        compressible_config: Some(CompressibleExtensionInstructionData {
            token_account_version: 3, // ShaFlat version (required)
            rent_payment: 16,         // 24h
            compression_only: 1,      // Required for ATA
            write_top_up,
            compress_to_account_pubkey: None,
        }),
    };

    let mut data = Vec::new();
    data.push(102u8); // CreateAssociatedTokenAccountIdempotent discriminator
    instruction_data
        .serialize(&mut data)
        .map_err(|_| LightSdkTypesError::Borsh)?;

    let accounts = vec![
        CpiMeta {
            pubkey: *wallet_owner,
            is_signer: false,
            is_writable: false,
        },
        CpiMeta {
            pubkey: *mint,
            is_signer: false,
            is_writable: false,
        },
        CpiMeta {
            pubkey: *fee_payer,
            is_signer: true,
            is_writable: true,
        },
        CpiMeta {
            pubkey: *ata,
            is_signer: false,
            is_writable: true,
        }, // NOT a signer - ATA is derived
        CpiMeta {
            pubkey: [0u8; 32],
            is_signer: false,
            is_writable: false,
        }, // system_program
        CpiMeta {
            pubkey: *compressible_config,
            is_signer: false,
            is_writable: false,
        },
        CpiMeta {
            pubkey: *rent_sponsor,
            is_signer: false,
            is_writable: true,
        },
    ];

    Ok((data, accounts))
}

/// Build instruction data and account metas for creating a compressible token account.
///
/// Returns `(data, account_metas)` for use with `AI::invoke_cpi()`.
///
/// # Account order:
/// 0. token_account (signer, writable)
/// 1. mint (readonly)
/// 2. fee_payer (signer, writable)
/// 3. compressible_config (readonly)
/// 4. system_program (readonly)
/// 5. rent_sponsor (writable)
#[allow(clippy::too_many_arguments)]
pub fn build_create_token_account_instruction(
    token_account: &[u8; 32],
    mint: &[u8; 32],
    owner: &[u8; 32],
    fee_payer: &[u8; 32],
    compressible_config: &[u8; 32],
    rent_sponsor: &[u8; 32],
    write_top_up: u32,
    signer_seeds: &[&[u8]],
    program_id: &[u8; 32],
) -> Result<(Vec<u8>, Vec<CpiMeta>), LightSdkTypesError> {
    let bump = signer_seeds
        .last()
        .and_then(|s| s.first().copied())
        .ok_or(LightSdkTypesError::InvalidSeeds)?;
    let seeds_without_bump: Vec<Vec<u8>> = signer_seeds
        .iter()
        .take(signer_seeds.len().saturating_sub(1))
        .map(|s| s.to_vec())
        .collect();

    let compress_to_account_pubkey = CompressToPubkey {
        bump,
        program_id: *program_id,
        seeds: seeds_without_bump,
    };

    let instruction_data = CreateTokenAccountInstructionData {
        owner: light_compressed_account::Pubkey::from(*owner),
        compressible_config: Some(CompressibleExtensionInstructionData {
            token_account_version: 3, // ShaFlat version (required)
            rent_payment: 16,         // 24h
            compression_only: 0,      // Regular tokens can be transferred
            write_top_up,
            compress_to_account_pubkey: Some(compress_to_account_pubkey),
        }),
    };

    let mut data = Vec::new();
    data.push(18u8); // InitializeAccount3 opcode
    instruction_data
        .serialize(&mut data)
        .map_err(|_| LightSdkTypesError::Borsh)?;

    let accounts = vec![
        CpiMeta {
            pubkey: *token_account,
            is_signer: true,
            is_writable: true,
        },
        CpiMeta {
            pubkey: *mint,
            is_signer: false,
            is_writable: false,
        },
        CpiMeta {
            pubkey: *fee_payer,
            is_signer: true,
            is_writable: true,
        },
        CpiMeta {
            pubkey: *compressible_config,
            is_signer: false,
            is_writable: false,
        },
        CpiMeta {
            pubkey: [0u8; 32],
            is_signer: false,
            is_writable: false,
        }, // system_program
        CpiMeta {
            pubkey: *rent_sponsor,
            is_signer: false,
            is_writable: true,
        },
    ];

    Ok((data, accounts))
}

/// The Light Token Program ID for CPI calls.
pub const TOKEN_PROGRAM_ID: [u8; 32] = LIGHT_TOKEN_PROGRAM_ID;
