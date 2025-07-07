use anchor_lang::solana_program::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey,
};

use light_sdk::{cpi::CpiSigner, derive_light_cpi_signer};
use spl_token::instruction::TokenInstruction;

pub mod close_token_account;
pub mod create_associated_token_account;
pub mod create_spl_mint;
pub mod create_token_account;
pub mod mint;
pub mod mint_to_compressed;
pub mod multi_transfer;
pub mod shared;

// Reexport the wrapped anchor program.
pub use ::anchor_compressed_token::*;
use close_token_account::processor::process_close_token_account;
use create_associated_token_account::processor::process_create_associated_token_account;
use create_spl_mint::processor::process_create_spl_mint;
use create_token_account::processor::process_create_token_account;
use mint::processor::process_create_compressed_mint;
use mint_to_compressed::processor::process_mint_to_compressed;

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

// Start light token instructions at 100 to skip spl-token program instrutions.
// When adding new instructions check anchor discriminators for collisions!
#[repr(u8)]
pub enum InstructionType {
    DecompressedTransfer = 3,
    CloseTokenAccount = 9, // SPL Token CloseAccount
    CreateCompressedMint = 100,
    MintToCompressed = 101,
    CreateSplMint = 102,
    CreateAssociatedTokenAccount = 103,
    CreateTokenAccount = 18, // SPL Token InitializeAccount3
    Other,
}

impl From<u8> for InstructionType {
    fn from(value: u8) -> Self {
        match value {
            3 => InstructionType::DecompressedTransfer,
            9 => InstructionType::CloseTokenAccount,
            100 => InstructionType::CreateCompressedMint,
            101 => InstructionType::MintToCompressed,
            102 => InstructionType::CreateSplMint,
            103 => InstructionType::CreateAssociatedTokenAccount,
            18 => InstructionType::CreateTokenAccount,
            _ => InstructionType::Other,
        }
    }
}

#[cfg(not(feature = "cpi"))]
anchor_lang::solana_program::entrypoint!(process_instruction);

pub fn process_instruction<'info>(
    program_id: &Pubkey,
    accounts: &'info [AccountInfo<'info>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let discriminator = InstructionType::from(instruction_data[0]);
    match discriminator {
        InstructionType::DecompressedTransfer => {
            let instruction = TokenInstruction::unpack(instruction_data)?;
            match instruction {
                TokenInstruction::Transfer { amount } => {
                    spl_token::processor::Processor::process_transfer(
                        program_id, accounts, amount, None,
                    )?;
                }
                _ => return Err(ProgramError::InvalidInstructionData),
            }
        }
        InstructionType::CreateCompressedMint => {
            process_create_compressed_mint(program_id.into(), accounts, &instruction_data[1..])?;
        }
        InstructionType::MintToCompressed => {
            process_mint_to_compressed(program_id.into(), accounts, &instruction_data[1..])?;
        }
        InstructionType::CreateSplMint => {
            process_create_spl_mint(*program_id, accounts, &instruction_data[1..])?;
        }
        InstructionType::CreateAssociatedTokenAccount => {
            process_create_associated_token_account(accounts, &instruction_data[1..])?;
        }
        InstructionType::CreateTokenAccount => {
            process_create_token_account(accounts, &instruction_data[1..])?;
        }
        InstructionType::CloseTokenAccount => {
            process_close_token_account(accounts, &instruction_data[1..])?;
        }
        // anchor instructions have no discriminator conflicts with InstructionType
        _ => entry(program_id, accounts, instruction_data)?,
    }

    Ok(())
}
