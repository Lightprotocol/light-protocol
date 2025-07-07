use anchor_lang::solana_program::program_error::ProgramError;

use light_account_checks::AccountInfoTrait;
use light_sdk::{cpi::CpiSigner, derive_light_cpi_signer};
use pinocchio::account_info::AccountInfo;
use spl_token::{instruction::TokenInstruction, solana_program::log::sol_log_compute_units};

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
use pinocchio::program_entrypoint;

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
                    let account_infos = convert_pinocchio_to_solana_raw(accounts)?;
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
            process_create_compressed_mint(*program_id, accounts, &instruction_data[1..])?;
        }
        InstructionType::MintToCompressed => {
            process_mint_to_compressed(*program_id, accounts, &instruction_data[1..])?;
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
        _ => {
            // let pubkey_store = create_pubkey_store(accounts);
            // let account_infos = convert_pinocchio_to_solana(accounts, &pubkey_store);
            // let program_id_pubkey = solana_pubkey::Pubkey::new_from_array(*program_id);
            let account_infos = convert_pinocchio_to_solana_raw(accounts)?;
            let solana_program_id = solana_pubkey::Pubkey::new_from_array(*program_id);

            entry(
                &solana_program_id,
                account_infos.as_slice(),
                instruction_data,
            )?
        }
    }

    Ok(())
}

/// Convert Pinocchio AccountInfo to Solana AccountInfo with minimal safety overhead
///
/// SAFETY REQUIREMENTS:
/// - `pinocchio_accounts` must remain valid for lifetime 'a
/// - No other code may mutably borrow these accounts during 'a
/// - Pinocchio runtime must have properly deserialized the accounts
/// - Caller must ensure no concurrent access to returned AccountInfo
#[inline(always)]
pub fn convert_pinocchio_to_solana_raw<'a>(
    pinocchio_accounts: &'a [AccountInfo],
) -> Result<Vec<anchor_lang::prelude::AccountInfo<'a>>, ProgramError> {
    use std::cell::RefCell;
    use std::rc::Rc;

    // Compile-time type safety: Ensure Pubkey types are layout-compatible
    const _: () = {
        assert!(
            std::mem::size_of::<pinocchio::pubkey::Pubkey>()
                == std::mem::size_of::<solana_pubkey::Pubkey>()
        );
        assert!(
            std::mem::align_of::<pinocchio::pubkey::Pubkey>()
                == std::mem::align_of::<solana_pubkey::Pubkey>()
        );
    };

    let mut solana_accounts = Vec::with_capacity(pinocchio_accounts.len());
    unsafe {
        for pinocchio_account in pinocchio_accounts {
            sol_log_compute_units();
            // Safe pointer casting instead of transmute (fails to compile if types change)
            let key: &'a solana_pubkey::Pubkey =
                &*(pinocchio_account.key() as *const _ as *const solana_pubkey::Pubkey);

            sol_log_compute_units();
            let owner: &'a solana_pubkey::Pubkey =
                &*(pinocchio_account.owner() as *const _ as *const solana_pubkey::Pubkey);

            sol_log_compute_units();
            // Direct reference to lamports and data - no std::mem::forget neededneeded
            let lamports = pinocchio_account.borrow_mut_lamports_unchecked();
            let lamports = Rc::new(RefCell::new(lamports));

            sol_log_compute_units();
            let data = pinocchio_account.borrow_mut_data_unchecked();
            let data = Rc::new(RefCell::new(data));

            sol_log_compute_units();
            let account_info = anchor_lang::prelude::AccountInfo {
                key,
                is_signer: AccountInfoTrait::is_signer(pinocchio_account),
                is_writable: AccountInfoTrait::is_writable(pinocchio_account),
                lamports,
                data,
                owner,
                executable: AccountInfoTrait::executable(pinocchio_account),
                rent_epoch: 0, // Pinocchio doesn't track rent epoch
            };

            sol_log_compute_units();
            solana_accounts.push(account_info);
        }
    }
    Ok(solana_accounts)
}

// /// Convert to solana AccountInfo by re-deserializing from the original input buffer
// /// This preserves the original pointer relationships that the Solana runtime expects
// pub fn convert_to_solana_accounts<'a>(
//     program_id: &pinocchio::pubkey::Pubkey,
//     accounts: &'a [AccountInfo],
//     instruction_data: &[u8],
// ) -> (
//     solana_pubkey::Pubkey,
//     Vec<anchor_lang::prelude::AccountInfo<'a>>,
//     Vec<u8>,
// ) {
//     // We need to re-serialize and then deserialize to get proper Solana AccountInfo
//     // This is a workaround because Pinocchio uses zero-copy but Solana AccountInfo
//     // expects specific memory layout and pointer relationships

//     // For now, create a simple conversion that should work for basic cases
//     let program_id_solana = solana_pubkey::Pubkey::new_from_array(*program_id);
//     let mut solana_accounts = Vec::with_capacity(accounts.len());

//     for account in accounts {
//         // Create owned copies of the data to avoid pointer issues
//         let key = solana_pubkey::Pubkey::new_from_array(*account.key());
//         let owner = solana_pubkey::Pubkey::new_from_array( *account.owner() });

//         // Create the AccountInfo with owned data
//         let account_info = anchor_lang::prelude::AccountInfo {
//             key: Box::leak(Box::new(key)),
//             lamports: unsafe { account.borrow_mut_lamports_unchecked() },
//             data: unsafe { account.borrow_mut_data_unchecked() },
//             owner: Box::leak(Box::new(owner)),
//             rent_epoch: 0,
//             is_signer: AccountInfoTrait::is_signer(account),
//             is_writable: AccountInfoTrait::is_writable(account),
//             executable: AccountInfoTrait::executable(account),
//         };

//         solana_accounts.push(account_info);
//     }

//     (
//         program_id_solana,
//         solana_accounts,
//         instruction_data.to_vec(),
//     )
// }
