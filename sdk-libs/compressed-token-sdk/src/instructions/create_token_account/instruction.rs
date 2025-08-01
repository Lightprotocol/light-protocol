use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::error::Result;

/// Input parameters for creating a token account with compressible extension
#[derive(Debug, Clone)]
pub struct CreateCompressibleTokenAccount {
    /// The account to be created
    pub account_pubkey: Pubkey,
    /// The mint for the token account
    pub mint_pubkey: Pubkey,
    /// The owner of the token account
    pub owner_pubkey: Pubkey,
    /// The authority that can close this account (in addition to owner)
    pub rent_authority: Pubkey,
    /// The recipient of lamports when the account is closed by rent authority
    pub rent_recipient: Pubkey,
    /// Number of slots that must pass before compression is allowed
    pub slots_until_compression: u64,
}

pub fn create_compressible_token_account(
    inputs: CreateCompressibleTokenAccount,
) -> Result<Instruction> {
    // Format: [18, owner_pubkey_32_bytes, 0]
    // Create compressible extension data manually
    // Layout: [slots_until_compression: u64, rent_authority: 32 bytes, rent_recipient: 32 bytes]
    let mut data = Vec::with_capacity(1 + 32 + 1 + 8 + 32 + 32);
    data.push(18u8); // InitializeAccount3 opcode
    data.extend_from_slice(&inputs.owner_pubkey.to_bytes());
    data.push(1); // Some option byte extension
    data.extend_from_slice(&inputs.slots_until_compression.to_le_bytes());
    data.extend_from_slice(&inputs.rent_authority.to_bytes());
    data.extend_from_slice(&inputs.rent_recipient.to_bytes());

    Ok(Instruction {
        program_id: Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: vec![
            solana_instruction::AccountMeta::new(inputs.account_pubkey, false),
            solana_instruction::AccountMeta::new_readonly(inputs.mint_pubkey, false),
        ],
        data,
    })
}

pub fn create_token_account(
    account_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    owner_pubkey: Pubkey,
) -> Result<Instruction> {
    // Create InitializeAccount3 instruction data manually
    // Format: [18, owner_pubkey_32_bytes, 0]
    let mut data = Vec::with_capacity(1 + 32);
    data.push(18u8); // InitializeAccount3 opcode
    data.extend_from_slice(&owner_pubkey.to_bytes());

    Ok(Instruction {
        program_id: Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: vec![
            solana_instruction::AccountMeta::new(account_pubkey, false),
            solana_instruction::AccountMeta::new_readonly(mint_pubkey, false),
        ],
        data,
    })
}
