//! ATA and token account creation helpers for decompression.

use light_token_interface::instructions::{
    create_token_account::CreateTokenAccountInstructionData,
    extensions::{CompressToPubkey, CompressibleExtensionInstructionData},
};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::AnchorSerialize;

/// Build a CreateAssociatedTokenAccountIdempotent instruction for ATA decompression.
///
/// Creates a compressible ATA with compression_only mode (required for ATA decompression).
///
/// # Account order (per on-chain handler):
/// 0. owner (non-mut, non-signer) - The wallet owner
/// 1. mint (non-mut, non-signer) - The token mint
/// 2. fee_payer (signer, writable) - Pays for account creation
/// 3. associated_token_account (writable, NOT signer) - The ATA to create
/// 4. system_program (readonly) - System program
/// 5. compressible_config (readonly) - Compressible config PDA
/// 6. rent_payer (writable) - Rent sponsor account
///
/// # Arguments
/// * `wallet_owner` - The wallet owner (ATA derivation seed)
/// * `mint` - The token mint
/// * `fee_payer` - Pays for account creation
/// * `ata` - The ATA pubkey (derived from wallet_owner, program_id, mint)
/// * `bump` - The ATA derivation bump
/// * `compressible_config` - Compressible config PDA
/// * `rent_sponsor` - Rent sponsor account
/// * `write_top_up` - Lamports per write for top-up
#[allow(clippy::too_many_arguments)]
pub fn build_create_ata_instruction(
    wallet_owner: &Pubkey,
    mint: &Pubkey,
    fee_payer: &Pubkey,
    ata: &Pubkey,
    bump: u8,
    compressible_config: &Pubkey,
    rent_sponsor: &Pubkey,
    write_top_up: u32,
) -> Result<Instruction, ProgramError> {
    use light_token_interface::instructions::{
        create_associated_token_account::CreateAssociatedTokenAccountInstructionData,
        extensions::CompressibleExtensionInstructionData,
    };

    let instruction_data = CreateAssociatedTokenAccountInstructionData {
        bump,
        compressible_config: Some(CompressibleExtensionInstructionData {
            token_account_version: 3, // ShaFlat version (required)
            rent_payment: 16,         // 24h, TODO: make configurable
            compression_only: 1,      // Required for ATA
            write_top_up,
            compress_to_account_pubkey: None, // Required to be None for ATA
        }),
    };

    let mut data = Vec::new();
    data.push(102u8); // CreateAssociatedTokenAccountIdempotent discriminator
    instruction_data
        .serialize(&mut data)
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    let accounts = vec![
        AccountMeta::new_readonly(*wallet_owner, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new(*ata, false), // NOT a signer - ATA is derived
        AccountMeta::new_readonly(Pubkey::default(), false), // system_program
        AccountMeta::new_readonly(*compressible_config, false),
        AccountMeta::new(*rent_sponsor, false),
    ];

    Ok(Instruction {
        program_id: light_token_interface::LIGHT_TOKEN_PROGRAM_ID.into(),
        accounts,
        data,
    })
}

/// Build a CreateTokenAccount instruction for decompression.
///
/// Creates a compressible token account with ShaFlat version (required by light token program).
///
/// # Account order:
/// 0. token_account (signer, writable) - The token account PDA to create
/// 1. mint (readonly) - The token mint
/// 2. fee_payer (signer, writable) - Pays for account creation
/// 3. compressible_config (readonly) - Compressible config PDA
/// 4. system_program (readonly) - System program
/// 5. rent_sponsor (writable) - Rent sponsor account
///
/// # Arguments
/// * `signer_seeds` - Seeds including bump for the token account PDA
/// * `program_id` - Program ID that owns the token account PDA
#[allow(clippy::too_many_arguments)]
pub fn build_create_token_account_instruction(
    token_account: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
    fee_payer: &Pubkey,
    compressible_config: &Pubkey,
    rent_sponsor: &Pubkey,
    write_top_up: u32,
    signer_seeds: &[&[u8]],
    program_id: &Pubkey,
) -> Result<Instruction, ProgramError> {
    // Build CompressToPubkey from signer_seeds (last seed is bump)
    let bump = signer_seeds
        .last()
        .and_then(|s| s.first().copied())
        .ok_or(ProgramError::InvalidSeeds)?;
    let seeds_without_bump: Vec<Vec<u8>> = signer_seeds
        .iter()
        .take(signer_seeds.len().saturating_sub(1))
        .map(|s| s.to_vec())
        .collect();

    let compress_to_account_pubkey = CompressToPubkey {
        bump,
        program_id: program_id.to_bytes(),
        seeds: seeds_without_bump,
    };

    let instruction_data = CreateTokenAccountInstructionData {
        owner: light_compressed_account::Pubkey::from(owner.to_bytes()),
        compressible_config: Some(CompressibleExtensionInstructionData {
            token_account_version: 3, // ShaFlat version (required)
            rent_payment: 16,         // 24h, TODO: make configurable
            compression_only: 0,      // Regular tokens can be transferred, not compression-only
            write_top_up,
            compress_to_account_pubkey: Some(compress_to_account_pubkey),
        }),
    };

    let mut data = Vec::new();
    data.push(18u8); // InitializeAccount3 opcode
    instruction_data
        .serialize(&mut data)
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    let accounts = vec![
        AccountMeta::new(*token_account, true),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(*compressible_config, false),
        AccountMeta::new_readonly(Pubkey::default(), false), // system_program
        AccountMeta::new(*rent_sponsor, false),
    ];

    Ok(Instruction {
        program_id: light_token_interface::LIGHT_TOKEN_PROGRAM_ID.into(),
        accounts,
        data,
    })
}
