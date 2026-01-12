use std::mem::ManuallyDrop;

use anchor_lang::solana_program::program_error::ProgramError;
use light_sdk::{cpi::CpiSigner, derive_light_cpi_signer};
use light_token_interface::LIGHT_TOKEN_PROGRAM_ID;
use pinocchio::{account_info::AccountInfo, msg};

pub mod compressed_token;
pub mod compressible;
pub mod convert_account_infos;
pub mod ctoken;
pub mod extensions;
pub mod shared;

// Reexport the wrapped anchor program.
pub use ::anchor_compressed_token::*;
use compressible::{process_claim, process_withdraw_funding_pool};
use ctoken::{
    process_close_token_account, process_create_associated_token_account,
    process_create_associated_token_account_idempotent, process_create_token_account,
    process_ctoken_approve, process_ctoken_burn, process_ctoken_burn_checked,
    process_ctoken_freeze_account, process_ctoken_mint_to, process_ctoken_mint_to_checked,
    process_ctoken_revoke, process_ctoken_thaw_account, process_ctoken_transfer,
    process_ctoken_transfer_checked,
};

use crate::{
    compressed_token::{
        mint_action::processor::process_mint_action, transfer2::processor::process_transfer2,
    },
    convert_account_infos::convert_account_infos,
};

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

pub const MAX_ACCOUNTS: usize = 30;
pub(crate) const MAX_PACKED_ACCOUNTS: usize = 40;
/// Maximum number of compression operations per instruction.
/// Used for compression_to_input lookup array sizing.
pub(crate) const MAX_COMPRESSIONS: usize = 32;

// Instruction discriminators use SPL Token values (3-18) for compatibility plus custom values (100+).
// When adding new instructions check anchor discriminators for collisions!
#[repr(u8)]
pub enum InstructionType {
    /// CToken transfer
    CTokenTransfer = 3,
    /// CToken Approve
    CTokenApprove = 4,
    /// CToken Revoke
    CTokenRevoke = 5,
    /// CToken mint_to - mint from decompressed CMint to CToken with top-ups
    CTokenMintTo = 7,
    /// CToken burn - burn from CToken, update CMint supply, with top-ups
    CTokenBurn = 8,
    /// CToken CloseAccount
    CloseTokenAccount = 9,
    /// CToken FreezeAccount
    CTokenFreezeAccount = 10,
    /// CToken ThawAccount
    CTokenThawAccount = 11,
    /// CToken TransferChecked - transfer with decimals validation (SPL compatible)
    CTokenTransferChecked = 12,
    /// CToken MintToChecked - mint with decimals validation
    CTokenMintToChecked = 14,
    /// CToken BurnChecked - burn with decimals validation
    CTokenBurnChecked = 15,
    /// Create CToken, equivalent to SPL Token InitializeAccount3
    CreateTokenAccount = 18,
    CreateAssociatedCTokenAccount = 100,
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
    ///     5. MintToCToken
    ///     6. UpdateMetadataField
    ///     7. UpdateMetadataAuthority
    ///     8. RemoveMetadataKey
    ///     9. DecompressMint
    ///     10. CompressAndCloseCMint
    MintAction = 103,
    /// Claim rent for past completed epochs from compressible token account
    Claim = 104,
    /// Withdraw funds from pool PDA
    WithdrawFundingPool = 105,
    Other,
}

impl From<u8> for InstructionType {
    #[inline(always)]
    fn from(value: u8) -> Self {
        match value {
            3 => InstructionType::CTokenTransfer,
            4 => InstructionType::CTokenApprove,
            5 => InstructionType::CTokenRevoke,
            7 => InstructionType::CTokenMintTo,
            8 => InstructionType::CTokenBurn,
            9 => InstructionType::CloseTokenAccount,
            10 => InstructionType::CTokenFreezeAccount,
            11 => InstructionType::CTokenThawAccount,
            12 => InstructionType::CTokenTransferChecked,
            14 => InstructionType::CTokenMintToChecked,
            15 => InstructionType::CTokenBurnChecked,
            18 => InstructionType::CreateTokenAccount,
            100 => InstructionType::CreateAssociatedCTokenAccount,
            101 => InstructionType::Transfer2,
            102 => InstructionType::CreateAssociatedTokenAccountIdempotent,
            103 => InstructionType::MintAction,
            104 => InstructionType::Claim,
            105 => InstructionType::WithdrawFundingPool,
            _ => InstructionType::Other, // anchor instructions
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
    if *program_id != LIGHT_TOKEN_PROGRAM_ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    match discriminator {
        InstructionType::CTokenTransfer => {
            // msg!("CTokenTransfer");
            process_ctoken_transfer(accounts, &instruction_data[1..])?;
        }
        InstructionType::CTokenApprove => {
            msg!("CTokenApprove");
            process_ctoken_approve(accounts, &instruction_data[1..])?;
        }
        InstructionType::CTokenRevoke => {
            msg!("CTokenRevoke");
            process_ctoken_revoke(accounts, &instruction_data[1..])?;
        }
        InstructionType::CTokenTransferChecked => {
            msg!("CTokenTransferChecked");
            process_ctoken_transfer_checked(accounts, &instruction_data[1..])?;
        }
        InstructionType::CTokenMintTo => {
            msg!("CTokenMintTo");
            process_ctoken_mint_to(accounts, &instruction_data[1..])?;
        }
        InstructionType::CTokenBurn => {
            msg!("CTokenBurn");
            process_ctoken_burn(accounts, &instruction_data[1..])?;
        }
        InstructionType::CTokenMintToChecked => {
            msg!("CTokenMintToChecked");
            process_ctoken_mint_to_checked(accounts, &instruction_data[1..])?;
        }
        InstructionType::CTokenBurnChecked => {
            msg!("CTokenBurnChecked");
            process_ctoken_burn_checked(accounts, &instruction_data[1..])?;
        }
        InstructionType::CloseTokenAccount => {
            msg!("CloseTokenAccount");
            process_close_token_account(accounts, &instruction_data[1..])?;
        }
        InstructionType::CTokenFreezeAccount => {
            msg!("CTokenFreezeAccount");
            process_ctoken_freeze_account(accounts)?;
        }
        InstructionType::CTokenThawAccount => {
            msg!("CTokenThawAccount");
            process_ctoken_thaw_account(accounts)?;
        }
        InstructionType::CreateTokenAccount => {
            msg!("CreateTokenAccount");
            process_create_token_account(accounts, &instruction_data[1..])?;
        }
        InstructionType::CreateAssociatedCTokenAccount => {
            msg!("CreateAssociatedCTokenAccount");
            process_create_associated_token_account(accounts, &instruction_data[1..])?;
        }
        InstructionType::Transfer2 => {
            msg!("Transfer2");
            process_transfer2(accounts, &instruction_data[1..])?;
        }
        InstructionType::CreateAssociatedTokenAccountIdempotent => {
            msg!("CreateAssociatedTokenAccountIdempotent");
            process_create_associated_token_account_idempotent(accounts, &instruction_data[1..])?;
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
