use borsh::BorshSerialize;
use light_ctoken_types::{
    instructions::{
        create_associated_token_account::CreateAssociatedTokenAccountInstructionData,
        create_associated_token_account2::CreateAssociatedTokenAccount2InstructionData,
        extensions::compressible::CompressibleExtensionInstructionData,
    },
    state::TokenDataVersion,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::error::{Result, TokenSdkError};

/// Discriminators for create ATA instructions
const CREATE_ATA_DISCRIMINATOR: u8 = 100;
const CREATE_ATA_IDEMPOTENT_DISCRIMINATOR: u8 = 102;
const CREATE_ATA2_DISCRIMINATOR: u8 = 106;
const CREATE_ATA2_IDEMPOTENT_DISCRIMINATOR: u8 = 107;

/// Input parameters for creating an associated token account with compressible extension
#[derive(Debug, Clone)]
pub struct CreateCompressibleAssociatedTokenAccountInputs {
    /// The payer for the account creation
    pub payer: Pubkey,
    /// The owner of the associated token account
    pub owner: Pubkey,
    /// The mint for the associated token account
    pub mint: Pubkey,
    /// The CompressibleConfig account
    pub compressible_config: Pubkey,
    /// The recipient of lamports when the account is closed by rent authority (fee_payer_pda)
    pub rent_sponsor: Pubkey,
    /// Number of epochs of rent to prepay
    pub pre_pay_num_epochs: u8,
    /// Initial lamports to top up for rent payments (optional)
    pub lamports_per_write: Option<u32>,
    /// Version of the compressed token account when ctoken account is
    /// compressed and closed. (The version specifies the hashing scheme.)
    pub token_account_version: TokenDataVersion,
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
            inputs.pre_pay_num_epochs,
            inputs.lamports_per_write,
            inputs.rent_sponsor,
            inputs.compressible_config,
            inputs.token_account_version,
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
    compressible_config: Option<(u8, Option<u32>, Pubkey, Pubkey, TokenDataVersion)>, // (pre_pay_num_epochs, lamports_per_write, rent_sponsor, compressible_config_account, token_account_version)
) -> Result<Instruction> {
    // Select discriminator based on idempotent mode
    let discriminator = if IDEMPOTENT {
        CREATE_ATA_IDEMPOTENT_DISCRIMINATOR
    } else {
        CREATE_ATA_DISCRIMINATOR
    };

    // Create the instruction data struct
    let compressible_extension = if COMPRESSIBLE {
        if let Some((pre_pay_num_epochs, lamports_per_write, _, _, token_account_version)) =
            compressible_config
        {
            Some(CompressibleExtensionInstructionData {
                token_account_version: token_account_version as u8,
                rent_payment: pre_pay_num_epochs,
                has_top_up: if lamports_per_write.is_some() { 1 } else { 0 },
                write_top_up: lamports_per_write.unwrap_or(0),
                compress_to_account_pubkey: None, // Not used for ATA creation
            })
        } else {
            return Err(TokenSdkError::InvalidAccountData);
        }
    } else {
        None
    };

    let instruction_data = CreateAssociatedTokenAccountInstructionData {
        owner: light_compressed_account::Pubkey::from(owner.to_bytes()),
        mint: light_compressed_account::Pubkey::from(mint.to_bytes()),
        bump,
        compressible_config: compressible_extension,
    };

    // Serialize with Borsh
    let mut data = Vec::new();
    data.push(discriminator);
    instruction_data
        .serialize(&mut data)
        .map_err(|_| TokenSdkError::SerializationError)?;

    // Build accounts list based on whether it's compressible
    let mut accounts = vec![
        solana_instruction::AccountMeta::new(payer, true), // fee_payer (signer)
        solana_instruction::AccountMeta::new(ata_pubkey, false), // associated_token_account
        solana_instruction::AccountMeta::new_readonly(Pubkey::new_from_array([0; 32]), false), // system_program
    ];

    // Add compressible-specific accounts
    if COMPRESSIBLE {
        if let Some((_, _, rent_sponsor, compressible_config_account, _)) = compressible_config {
            accounts.push(solana_instruction::AccountMeta::new_readonly(
                compressible_config_account,
                false,
            )); // compressible_config
            accounts.push(solana_instruction::AccountMeta::new(rent_sponsor, false));
            // fee_payer_pda (rent_sponsor)
        }
    }

    Ok(Instruction {
        program_id: Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts,
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

// ============================================================================
// CreateAssociatedTokenAccount2 - Owner and mint as accounts
// ============================================================================

/// Creates a compressible associated token account instruction v2 (non-idempotent)
/// Owner and mint are passed as account infos instead of instruction data
pub fn create_compressible_associated_token_account2(
    inputs: CreateCompressibleAssociatedTokenAccountInputs,
) -> Result<Instruction> {
    create_compressible_associated_token_account2_with_mode::<false>(inputs)
}

/// Creates a compressible associated token account instruction v2 (idempotent)
/// Owner and mint are passed as account infos instead of instruction data
pub fn create_compressible_associated_token_account2_idempotent(
    inputs: CreateCompressibleAssociatedTokenAccountInputs,
) -> Result<Instruction> {
    create_compressible_associated_token_account2_with_mode::<true>(inputs)
}

/// Creates a compressible associated token account instruction v2 with compile-time idempotent mode
fn create_compressible_associated_token_account2_with_mode<const IDEMPOTENT: bool>(
    inputs: CreateCompressibleAssociatedTokenAccountInputs,
) -> Result<Instruction> {
    let (ata_pubkey, bump) = derive_ctoken_ata(&inputs.owner, &inputs.mint);
    create_compressible_associated_token_account2_with_bump_and_mode::<IDEMPOTENT>(
        inputs, ata_pubkey, bump,
    )
}

/// Creates a compressible associated token account instruction v2 with specified bump and mode
fn create_compressible_associated_token_account2_with_bump_and_mode<const IDEMPOTENT: bool>(
    inputs: CreateCompressibleAssociatedTokenAccountInputs,
    ata_pubkey: Pubkey,
    bump: u8,
) -> Result<Instruction> {
    create_ata2_instruction_unified::<IDEMPOTENT, true>(
        inputs.payer,
        inputs.owner,
        inputs.mint,
        ata_pubkey,
        bump,
        Some((
            inputs.pre_pay_num_epochs,
            inputs.lamports_per_write,
            inputs.rent_sponsor,
            inputs.compressible_config,
            inputs.token_account_version,
        )),
    )
}

/// Creates a basic associated token account instruction v2 (non-idempotent)
/// Owner and mint are passed as account infos instead of instruction data
pub fn create_associated_token_account2(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
) -> Result<Instruction> {
    create_associated_token_account2_with_mode::<false>(payer, owner, mint)
}

/// Creates a basic associated token account instruction v2 (idempotent)
/// Owner and mint are passed as account infos instead of instruction data
pub fn create_associated_token_account2_idempotent(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
) -> Result<Instruction> {
    create_associated_token_account2_with_mode::<true>(payer, owner, mint)
}

/// Creates a basic associated token account instruction v2 with compile-time idempotent mode
fn create_associated_token_account2_with_mode<const IDEMPOTENT: bool>(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
) -> Result<Instruction> {
    let (ata_pubkey, bump) = derive_ctoken_ata(&owner, &mint);
    create_associated_token_account2_with_bump_and_mode::<IDEMPOTENT>(
        payer, owner, mint, ata_pubkey, bump,
    )
}

/// Creates a basic associated token account instruction v2 with specified bump and mode
fn create_associated_token_account2_with_bump_and_mode<const IDEMPOTENT: bool>(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
    ata_pubkey: Pubkey,
    bump: u8,
) -> Result<Instruction> {
    create_ata2_instruction_unified::<IDEMPOTENT, false>(payer, owner, mint, ata_pubkey, bump, None)
}

/// Unified function to create ATA2 instructions with compile-time configuration
/// Account order: [owner, mint, fee_payer, ata, system_program, ...]
fn create_ata2_instruction_unified<const IDEMPOTENT: bool, const COMPRESSIBLE: bool>(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
    ata_pubkey: Pubkey,
    bump: u8,
    compressible_config: Option<(u8, Option<u32>, Pubkey, Pubkey, TokenDataVersion)>,
) -> Result<Instruction> {
    let discriminator = if IDEMPOTENT {
        CREATE_ATA2_IDEMPOTENT_DISCRIMINATOR
    } else {
        CREATE_ATA2_DISCRIMINATOR
    };

    let compressible_extension = if COMPRESSIBLE {
        if let Some((pre_pay_num_epochs, lamports_per_write, _, _, token_account_version)) =
            compressible_config
        {
            Some(CompressibleExtensionInstructionData {
                token_account_version: token_account_version as u8,
                rent_payment: pre_pay_num_epochs,
                has_top_up: if lamports_per_write.is_some() { 1 } else { 0 },
                write_top_up: lamports_per_write.unwrap_or(0),
                compress_to_account_pubkey: None,
            })
        } else {
            return Err(TokenSdkError::InvalidAccountData);
        }
    } else {
        None
    };

    let instruction_data = CreateAssociatedTokenAccount2InstructionData {
        bump,
        compressible_config: compressible_extension,
    };

    let mut data = Vec::new();
    data.push(discriminator);
    instruction_data
        .serialize(&mut data)
        .map_err(|_| TokenSdkError::SerializationError)?;

    let mut accounts = vec![
        solana_instruction::AccountMeta::new_readonly(owner, false),
        solana_instruction::AccountMeta::new_readonly(mint, false),
        solana_instruction::AccountMeta::new(payer, true),
        solana_instruction::AccountMeta::new(ata_pubkey, false),
        solana_instruction::AccountMeta::new_readonly(Pubkey::new_from_array([0; 32]), false),
    ];

    if COMPRESSIBLE {
        if let Some((_, _, rent_sponsor, compressible_config_account, _)) = compressible_config {
            accounts.push(solana_instruction::AccountMeta::new_readonly(
                compressible_config_account,
                false,
            ));
            accounts.push(solana_instruction::AccountMeta::new(rent_sponsor, false));
        }
    }

    Ok(Instruction {
        program_id: Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts,
        data,
    })
}
