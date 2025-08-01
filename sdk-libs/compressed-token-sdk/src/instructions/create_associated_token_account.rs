use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::error::Result;

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

/// Creates a compressible associated token account instruction
pub fn create_compressible_associated_token_account(
    inputs: CreateCompressibleAssociatedTokenAccountInputs,
) -> Result<Instruction> {
    let (ata_pubkey, bump) = derive_ctoken_ata(&inputs.owner, &inputs.mint);
    create_compressible_associated_token_account_with_bump(inputs, ata_pubkey, bump)
}

/// Creates a compressible associated token account instruction with a specified bump
pub fn create_compressible_associated_token_account_with_bump(
    inputs: CreateCompressibleAssociatedTokenAccountInputs,
    ata_pubkey: Pubkey,
    bump: u8,
) -> Result<Instruction> {
    // Manual serialization: [discriminator, owner, mint, bump, compressible_config]
    let mut data = Vec::with_capacity(103 + 32 + 32 + 1 + 1 + 8 + 32 + 32);
    data.push(103u8); // CreateAssociatedTokenAccount discriminator
    data.extend_from_slice(&inputs.owner.to_bytes()); // owner: 32 bytes
    data.extend_from_slice(&inputs.mint.to_bytes()); // mint: 32 bytes
    data.push(bump); // bump: 1 byte
    data.push(1u8); // Some option byte for compressible_config
    data.extend_from_slice(&inputs.slots_until_compression.to_le_bytes()); // slots_until_compression: 8 bytes
    data.extend_from_slice(&inputs.rent_authority.to_bytes()); // rent_authority: 32 bytes
    data.extend_from_slice(&inputs.rent_recipient.to_bytes()); // rent_recipient: 32 bytes

    Ok(Instruction {
        program_id: Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: vec![
            solana_instruction::AccountMeta::new(inputs.payer, true), // fee_payer (signer)
            solana_instruction::AccountMeta::new(ata_pubkey, false),  // associated_token_account
            solana_instruction::AccountMeta::new_readonly(inputs.mint, false), // mint
            solana_instruction::AccountMeta::new_readonly(inputs.owner, false), // owner
            solana_instruction::AccountMeta::new_readonly(Pubkey::new_from_array([0; 32]), false), // system_program
        ],
        data,
    })
}

/// Creates a basic associated token account instruction
pub fn create_associated_token_account(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
) -> Result<Instruction> {
    let (ata_pubkey, bump) = derive_ctoken_ata(&owner, &mint);
    create_associated_token_account_with_bump(payer, owner, mint, ata_pubkey, bump)
}

pub fn create_associated_token_account_with_bump(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
    ata_pubkey: Pubkey,
    bump: u8,
) -> Result<Instruction> {
    // Manual serialization: [discriminator, owner, mint, bump, compressible_config]
    let mut data = Vec::with_capacity(1 + 32 + 32 + 1 + 1);
    data.push(103u8); // CreateAssociatedTokenAccount discriminator
    data.extend_from_slice(&owner.to_bytes()); // owner: 32 bytes
    data.extend_from_slice(&mint.to_bytes()); // mint: 32 bytes
    data.push(bump); // bump: 1 byte
    data.push(0u8); // None option byte for compressible_config

    Ok(Instruction {
        program_id: Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: vec![
            solana_instruction::AccountMeta::new(payer, true), // fee_payer (signer)
            solana_instruction::AccountMeta::new(ata_pubkey, false), // associated_token_account
            solana_instruction::AccountMeta::new_readonly(mint, false), // mint
            solana_instruction::AccountMeta::new_readonly(owner, false), // owner
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
