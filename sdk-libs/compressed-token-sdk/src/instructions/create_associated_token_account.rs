use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::error::{Result, TokenSdkError};

/// Discriminators for create ATA instructions
const CREATE_ATA_DISCRIMINATOR: u8 = 103;
const CREATE_ATA_IDEMPOTENT_DISCRIMINATOR: u8 = 105;

/// Input parameters for creating an associated token account with compressible extension
#[derive(Debug, Clone)]
pub struct CreateCompressibleAssociatedTokenAccountInputs {
    /// The payer for the account creation
    pub payer: Pubkey,
    /// The owner of the associated token account
    pub owner: Pubkey,
    /// The mint for the associated token account
    pub mint: Pubkey,
    /// The authority that can close this account (in addition to owner)
    pub rent_authority: Pubkey,
    /// The recipient of lamports when the account is closed by rent authority
    pub rent_recipient: Pubkey,
    /// Number of slots that must pass before compression is allowed
    pub slots_until_compression: u64,
}

/// Creates a compressible associated token account instruction (non-idempotent)
pub fn create_compressible_associated_token_account(
    inputs: CreateCompressibleAssociatedTokenAccountInputs,
) -> Result<Instruction> {
    create_compressible_associated_token_account_with_mode::<false>(inputs)
}

/// Creates a compressible associated token account instruction (idempotent)
pub fn create_compressible_associated_token_account_idempotent(
    inputs: CreateCompressibleAssociatedTokenAccountInputs,
) -> Result<Instruction> {
    create_compressible_associated_token_account_with_mode::<true>(inputs)
}

/// Creates a compressible associated token account instruction with compile-time idempotent mode
pub fn create_compressible_associated_token_account_with_mode<const IDEMPOTENT: bool>(
    inputs: CreateCompressibleAssociatedTokenAccountInputs,
) -> Result<Instruction> {
    let (ata_pubkey, bump) = derive_ctoken_ata(&inputs.owner, &inputs.mint);
    create_compressible_associated_token_account_with_bump_and_mode::<IDEMPOTENT>(
        inputs, ata_pubkey, bump,
    )
}

/// Creates a compressible associated token account instruction with a specified bump (non-idempotent)
pub fn create_compressible_associated_token_account_with_bump(
    inputs: CreateCompressibleAssociatedTokenAccountInputs,
    ata_pubkey: Pubkey,
    bump: u8,
) -> Result<Instruction> {
    create_compressible_associated_token_account_with_bump_and_mode::<false>(
        inputs, ata_pubkey, bump,
    )
}

/// Creates a compressible associated token account instruction with a specified bump and mode
pub fn create_compressible_associated_token_account_with_bump_and_mode<const IDEMPOTENT: bool>(
    inputs: CreateCompressibleAssociatedTokenAccountInputs,
    ata_pubkey: Pubkey,
    bump: u8,
) -> Result<Instruction> {
    create_ata_instruction_unified::<IDEMPOTENT, true>(
        inputs.payer,
        inputs.owner,
        inputs.mint,
        ata_pubkey,
        bump,
        Some((
            inputs.slots_until_compression,
            inputs.rent_authority,
            inputs.rent_recipient,
        )),
    )
}

/// Creates a basic associated token account instruction (non-idempotent)
pub fn create_associated_token_account(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
) -> Result<Instruction> {
    create_associated_token_account_with_mode::<false>(payer, owner, mint)
}

/// Creates a basic associated token account instruction (idempotent)
pub fn create_associated_token_account_idempotent(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
) -> Result<Instruction> {
    create_associated_token_account_with_mode::<true>(payer, owner, mint)
}

/// Creates a basic associated token account instruction with compile-time idempotent mode
pub fn create_associated_token_account_with_mode<const IDEMPOTENT: bool>(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
) -> Result<Instruction> {
    let (ata_pubkey, bump) = derive_ctoken_ata(&owner, &mint);
    create_associated_token_account_with_bump_and_mode::<IDEMPOTENT>(
        payer, owner, mint, ata_pubkey, bump,
    )
}

/// Creates a basic associated token account instruction with a specified bump (non-idempotent)
pub fn create_associated_token_account_with_bump(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
    ata_pubkey: Pubkey,
    bump: u8,
) -> Result<Instruction> {
    create_associated_token_account_with_bump_and_mode::<false>(
        payer, owner, mint, ata_pubkey, bump,
    )
}

/// Creates a basic associated token account instruction with specified bump and mode
pub fn create_associated_token_account_with_bump_and_mode<const IDEMPOTENT: bool>(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
    ata_pubkey: Pubkey,
    bump: u8,
) -> Result<Instruction> {
    create_ata_instruction_unified::<IDEMPOTENT, false>(payer, owner, mint, ata_pubkey, bump, None)
}

/// Unified function to create ATA instructions with compile-time configuration
fn create_ata_instruction_unified<const IDEMPOTENT: bool, const COMPRESSIBLE: bool>(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
    ata_pubkey: Pubkey,
    bump: u8,
    compressible_config: Option<(u64, Pubkey, Pubkey)>, // (slots_until_compression, rent_authority, rent_recipient)
) -> Result<Instruction> {
    // Select discriminator based on idempotent mode
    let discriminator = if IDEMPOTENT {
        CREATE_ATA_IDEMPOTENT_DISCRIMINATOR
    } else {
        CREATE_ATA_DISCRIMINATOR
    };

    // Calculate data size based on whether it's compressible
    let data_size = if COMPRESSIBLE {
        1 + 32 + 32 + 1 + 1 + 8 + 32 + 32 // With compressible config
    } else {
        1 + 32 + 32 + 1 + 1 // Without compressible config
    };

    // Manual serialization: [discriminator, owner, mint, bump, compressible_config]
    let mut data = Vec::with_capacity(data_size);
    data.push(discriminator);
    data.extend_from_slice(&owner.to_bytes()); // owner: 32 bytes
    data.extend_from_slice(&mint.to_bytes()); // mint: 32 bytes
    data.push(bump); // bump: 1 byte

    if COMPRESSIBLE {
        if let Some((slots_until_compression, rent_authority, rent_recipient)) = compressible_config
        {
            data.push(1u8); // Some option byte for compressible_config
            data.extend_from_slice(&slots_until_compression.to_le_bytes()); // slots_until_compression: 8 bytes
            data.extend_from_slice(&rent_authority.to_bytes()); // rent_authority: 32 bytes
            data.extend_from_slice(&rent_recipient.to_bytes()); // rent_recipient: 32 bytes
        } else {
            // This should never happen if the const generic is used correctly
            return Err(TokenSdkError::InvalidAccountData);
        }
    } else {
        data.push(0u8); // None option byte for compressible_config
    }

    Ok(Instruction {
        program_id: Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: vec![
            solana_instruction::AccountMeta::new(payer, true), // fee_payer (signer)
            solana_instruction::AccountMeta::new(ata_pubkey, false), // associated_token_account
            solana_instruction::AccountMeta::new_readonly(Pubkey::new_from_array([0; 32]), false), // system_program
        ],
        data,
    })
}

pub fn derive_ctoken_ata(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            owner.as_ref(),
            light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID.as_ref(),
            mint.as_ref(),
        ],
        &Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
    )
}
