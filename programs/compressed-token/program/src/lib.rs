use std::mem::ManuallyDrop;

use anchor_lang::solana_program::program_error::ProgramError;
use light_sdk::{cpi::CpiSigner, derive_light_cpi_signer};
use pinocchio::account_info::AccountInfo;
use spl_token::instruction::TokenInstruction;

pub mod close_token_account;
pub mod convert_account_infos;
pub mod create_associated_token_account;
pub mod create_spl_mint;
pub mod create_token_account;
pub mod extensions;
pub mod mint;
pub mod mint_action;
pub mod mint_to_compressed;
pub mod shared;
pub mod transfer2;
// pub mod update_metadata;
pub mod update_mint;

// Reexport the wrapped anchor program.
pub use ::anchor_compressed_token::*;
use close_token_account::processor::process_close_token_account;
use create_associated_token_account::processor::process_create_associated_token_account;
use create_spl_mint::processor::process_create_spl_mint;
use create_token_account::processor::process_create_token_account;
use mint::processor::process_create_compressed_mint;
use mint_to_compressed::processor::process_mint_to_compressed;
use update_mint::processor::process_update_compressed_mint;

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

pub const MAX_ACCOUNTS: usize = 30;

// Start light token instructions at 100 to skip spl-token program instrutions.
// When adding new instructions check anchor discriminators for collisions!
#[repr(u8)]
pub enum InstructionType {
    DecompressedTransfer = 3, // SPL Token transfer
    CloseTokenAccount = 9,    // SPL Token CloseAccount
    CreateCompressedMint = 100,
    MintToCompressed = 101,
    CreateSplMint = 102,
    CreateAssociatedTokenAccount = 103,
    Transfer2 = 104,
    UpdateCompressedMint = 105,
    CreateTokenAccount = 18, // equivalen to SPL Token InitializeAccount3
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
            103 => InstructionType::CreateAssociatedTokenAccount, // TODO: double check compatibility
            104 => InstructionType::Transfer2,
            105 => InstructionType::UpdateCompressedMint,
            18 => InstructionType::CreateTokenAccount,
            _ => InstructionType::Other,
        }
    }
}

#[cfg(not(feature = "cpi"))]
use pinocchio::program_entrypoint;

use crate::{
    convert_account_infos::convert_account_infos, transfer2::processor::process_transfer2,
};

#[cfg(not(feature = "cpi"))]
program_entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &pinocchio::pubkey::Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let discriminator = InstructionType::from(instruction_data[0]);
    match discriminator {
        InstructionType::DecompressedTransfer => {
            let instruction = TokenInstruction::unpack(instruction_data)?;
            match instruction {
                TokenInstruction::Transfer { amount } => {
                    let account_infos = unsafe { convert_account_infos::<MAX_ACCOUNTS>(accounts)? };
                    let program_id_pubkey = solana_pubkey::Pubkey::new_from_array(*program_id);
                    spl_token::processor::Processor::process_transfer(
                        &program_id_pubkey,
                        &account_infos,
                        amount,
                        None,
                    )?;
                }
                _ => return Err(ProgramError::InvalidInstructionData),
            }
        }
        InstructionType::CreateCompressedMint => {
            anchor_lang::solana_program::msg!("CreateCompressedMint");
            process_create_compressed_mint(accounts, &instruction_data[1..])?;
        }
        InstructionType::MintToCompressed => {
            anchor_lang::solana_program::msg!("MintToCompressed");
            process_mint_to_compressed(accounts, &instruction_data[1..])?;
        }
        InstructionType::CreateSplMint => {
            anchor_lang::solana_program::msg!("CreateSplMint");
            process_create_spl_mint(accounts, &instruction_data[1..])?;
        }
        InstructionType::CreateAssociatedTokenAccount => {
            anchor_lang::solana_program::msg!("CreateAssociatedTokenAccount");
            process_create_associated_token_account(accounts, &instruction_data[1..])?;
        }
        InstructionType::CreateTokenAccount => {
            anchor_lang::solana_program::msg!("CreateTokenAccount");
            process_create_token_account(accounts, &instruction_data[1..])?;
        }
        InstructionType::CloseTokenAccount => {
            anchor_lang::solana_program::msg!("CloseTokenAccount");
            process_close_token_account(accounts, &instruction_data[1..])?;
        }
        InstructionType::Transfer2 => {
            anchor_lang::solana_program::msg!("Transfer2");
            process_transfer2(accounts, &instruction_data[1..])?;
        }
        InstructionType::UpdateCompressedMint => {
            anchor_lang::solana_program::msg!("UpdateCompressedMint");
            process_update_compressed_mint(accounts, &instruction_data[1..])?;
        }
        // anchor instructions have no discriminator conflicts with InstructionType
        _ => {
            let account_infos = unsafe { convert_account_infos::<MAX_ACCOUNTS>(accounts)? };
            let account_infos = ManuallyDrop::new(account_infos);
            let solana_program_id = solana_pubkey::Pubkey::new_from_array(*program_id);

            entry(
                &solana_program_id,
                account_infos.as_slice(),
                instruction_data,
            )?;
        }
    }
    Ok(())
}
