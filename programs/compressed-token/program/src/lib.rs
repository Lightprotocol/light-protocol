use std::mem::ManuallyDrop;

use anchor_lang::solana_program::program_error::ProgramError;
use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use light_sdk::{cpi::CpiSigner, derive_light_cpi_signer};
use pinocchio::{account_info::AccountInfo, msg};

pub mod claim;
pub mod close_token_account;
pub mod convert_account_infos;
pub mod create_associated_token_account;
pub mod create_associated_token_account2;
pub mod create_token_account;
pub mod ctoken_transfer;
pub mod extensions;
pub mod mint_action;
pub mod shared;
pub mod transfer2;
pub mod withdraw_funding_pool;

// Reexport the wrapped anchor program.
pub use ::anchor_compressed_token::*;
use claim::process_claim;
use close_token_account::processor::process_close_token_account;
use create_associated_token_account::{
    process_create_associated_token_account, process_create_associated_token_account_idempotent,
};
use create_associated_token_account2::{
    process_create_associated_token_account2, process_create_associated_token_account2_idempotent,
};
use create_token_account::process_create_token_account;
use ctoken_transfer::process_ctoken_transfer;
use withdraw_funding_pool::process_withdraw_funding_pool;

use crate::{
    convert_account_infos::convert_account_infos, mint_action::processor::process_mint_action,
};

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

pub const MAX_ACCOUNTS: usize = 30;

// Custom ctoken instructions start at 100 to skip spl-token program instrutions.
// When adding new instructions check anchor discriminators for collisions!
#[repr(u8)]
pub enum InstructionType {
    /// CToken transfer
    CTokenTransfer = 3,
    /// CToken CloseAccount
    CloseTokenAccount = 9,
    /// Create CToken, equivalent to SPL Token InitializeAccount3
    CreateTokenAccount = 18,
    CreateAssociatedTokenAccount = 100,
    /// Batch instruction for ctoken transfers:
    ///     1. transfer compressed tokens
    ///     2. compress ctokens/spl tokens
    ///     3. decompress ctokens/spl tokens
    ///     4. compress and close ctokens/spl tokens
    Transfer2 = 101,
    CreateAssociatedTokenAccountIdempotent = 102,
    /// Batch instruction for operation on one compressed Mint account:
    ///     1. CreateMint
    ///     2. MintTo
    ///     3. UpdateMintAuthority
    ///     4. UpdateFreezeAuthority
    ///     5. CreateSplMint
    ///     6. MintToCToken
    ///     7. UpdateMetadataField
    ///     8. UpdateMetadataAuthority
    ///     9. RemoveMetadataKey
    MintAction = 103,
    /// Claim rent for past completed epochs from compressible token account
    Claim = 104,
    /// Withdraw funds from pool PDA
    WithdrawFundingPool = 105,
    /// Create associated token account with owner and mint as accounts (non-idempotent)
    CreateAssociatedTokenAccount2 = 106,
    /// Create associated token account with owner and mint as accounts (idempotent)
    CreateAssociatedTokenAccount2Idempotent = 107,
    Other,
}

impl From<u8> for InstructionType {
    #[inline(always)]
    fn from(value: u8) -> Self {
        match value {
            3 => InstructionType::CTokenTransfer,
            9 => InstructionType::CloseTokenAccount,
            18 => InstructionType::CreateTokenAccount,
            100 => InstructionType::CreateAssociatedTokenAccount,
            101 => InstructionType::Transfer2,
            102 => InstructionType::CreateAssociatedTokenAccountIdempotent,
            103 => InstructionType::MintAction,
            104 => InstructionType::Claim,
            105 => InstructionType::WithdrawFundingPool,
            106 => InstructionType::CreateAssociatedTokenAccount2,
            107 => InstructionType::CreateAssociatedTokenAccount2Idempotent,
            _ => InstructionType::Other, // anchor instructions
        }
    }
}

#[cfg(not(feature = "cpi"))]
use pinocchio::program_entrypoint;

use crate::transfer2::processor::process_transfer2;

#[cfg(not(feature = "cpi"))]
program_entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &pinocchio::pubkey::Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let discriminator = InstructionType::from(instruction_data[0]);
    if *program_id != COMPRESSED_TOKEN_PROGRAM_ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    match discriminator {
        InstructionType::CTokenTransfer => {
            // msg!("CTokenTransfer");
            process_ctoken_transfer(accounts, &instruction_data[1..])?;
        }
        InstructionType::CreateAssociatedTokenAccount => {
            msg!("CreateAssociatedTokenAccount");
            process_create_associated_token_account(accounts, &instruction_data[1..])?;
        }
        InstructionType::CreateAssociatedTokenAccountIdempotent => {
            msg!("CreateAssociatedTokenAccountIdempotent");
            process_create_associated_token_account_idempotent(accounts, &instruction_data[1..])?;
        }
        InstructionType::CreateTokenAccount => {
            msg!("CreateTokenAccount");
            process_create_token_account(accounts, &instruction_data[1..])?;
        }
        InstructionType::CloseTokenAccount => {
            msg!("CloseTokenAccount");
            process_close_token_account(accounts, &instruction_data[1..])?;
        }
        InstructionType::Transfer2 => {
            msg!("Transfer2");
            process_transfer2(accounts, &instruction_data[1..])?;
        }
        InstructionType::MintAction => {
            msg!("MintAction");
            process_mint_action(accounts, &instruction_data[1..])?;
        }
        InstructionType::Claim => {
            msg!("Claim");
            process_claim(accounts, &instruction_data[1..])?;
        }
        InstructionType::WithdrawFundingPool => {
            msg!("WithdrawFundingPool");
            process_withdraw_funding_pool(accounts, &instruction_data[1..])?;
        }
        InstructionType::CreateAssociatedTokenAccount2 => {
            msg!("CreateAssociatedTokenAccount2");
            process_create_associated_token_account2(accounts, &instruction_data[1..])?;
        }
        InstructionType::CreateAssociatedTokenAccount2Idempotent => {
            msg!("CreateAssociatedTokenAccount2Idempotent");
            process_create_associated_token_account2_idempotent(accounts, &instruction_data[1..])?;
        }
        // anchor instructions have no discriminator conflicts with InstructionType
        // TODO: add test for discriminator conflict
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
