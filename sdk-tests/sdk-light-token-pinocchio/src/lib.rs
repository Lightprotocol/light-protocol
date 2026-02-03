#![allow(unexpected_cfgs)]
#![no_std]

extern crate alloc;

mod approve;
mod burn;
mod close;
mod create_ata;
mod create_mint;
mod create_token_account;
mod ctoken_mint_to;
mod decompress_mint;
mod freeze;
mod revoke;
mod thaw;
mod transfer;
mod transfer_checked;
mod transfer_interface;
mod transfer_spl_ctoken;

// Re-export all instruction data types
pub use approve::{process_approve_invoke, process_approve_invoke_signed, ApproveData};
pub use burn::{process_burn_invoke, process_burn_invoke_signed, BurnData};
pub use close::{process_close_account_invoke, process_close_account_invoke_signed};
pub use create_ata::{process_create_ata_invoke, process_create_ata_invoke_signed, CreateAtaData};
pub use create_mint::{
    process_create_mint, process_create_mint_invoke_signed, process_create_mint_with_pda_authority,
    CreateCmintData, MINT_SIGNER_SEED,
};
pub use create_token_account::{
    process_create_token_account_invoke, process_create_token_account_invoke_signed,
    CreateTokenAccountData,
};
pub use ctoken_mint_to::{process_mint_to_invoke, process_mint_to_invoke_signed, MintToData};
pub use decompress_mint::{process_decompress_mint_invoke_signed, DecompressCmintData};
pub use freeze::{process_freeze_invoke, process_freeze_invoke_signed};
use light_macros::pubkey_array;
use pinocchio::{
    account_info::AccountInfo, entrypoint, program_error::ProgramError, ProgramResult,
};
pub use revoke::{process_revoke_invoke, process_revoke_invoke_signed};
pub use thaw::{process_thaw_invoke, process_thaw_invoke_signed};
pub use transfer::{process_transfer_invoke, process_transfer_invoke_signed, TransferData};
pub use transfer_checked::{
    process_transfer_checked_invoke, process_transfer_checked_invoke_signed, TransferCheckedData,
};
pub use transfer_interface::{
    process_transfer_interface_invoke, process_transfer_interface_invoke_signed,
    TransferInterfaceData, TRANSFER_INTERFACE_AUTHORITY_SEED,
};
pub use transfer_spl_ctoken::{
    process_ctoken_to_spl_invoke, process_ctoken_to_spl_invoke_signed,
    process_spl_to_ctoken_invoke, process_spl_to_ctoken_invoke_signed, TransferFromSplData,
    TransferTokenToSplData, TRANSFER_AUTHORITY_SEED,
};

/// Program ID - replace with actual program ID after deployment
pub const ID: [u8; 32] = pubkey_array!("CToknNtvExmp1eProgram11111111111111111111112");

/// PDA seeds for invoke_signed instructions
pub const TOKEN_ACCOUNT_SEED: &[u8] = b"token_account";
pub const ATA_SEED: &[u8] = b"ata";
pub const FREEZE_AUTHORITY_SEED: &[u8] = b"freeze_authority";
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";

entrypoint!(process_instruction);

/// Instruction discriminators
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionType {
    /// Create a compressed mint
    CreateCmint = 0,
    /// Create compressible token account (invoke)
    CreateTokenAccountInvoke = 2,
    /// Create compressible token account with PDA ownership (invoke_signed)
    CreateTokenAccountInvokeSigned = 3,
    /// Create compressible associated token account (invoke)
    CreateAtaInvoke = 4,
    /// Create compressible associated token account with PDA ownership (invoke_signed)
    CreateAtaInvokeSigned = 5,
    /// Transfer compressed tokens Light Token->Light Token (invoke)
    CTokenTransferInvoke = 6,
    /// Transfer compressed tokens Light Token->Light Token from PDA-owned account (invoke_signed)
    CTokenTransferInvokeSigned = 7,
    /// Close compressed token account (invoke)
    CloseAccountInvoke = 8,
    /// Close PDA-owned compressed token account (invoke_signed)
    CloseAccountInvokeSigned = 9,
    /// Create ATA using V2 variant (invoke)
    CreateAta2Invoke = 10,
    /// Create ATA using V2 variant with PDA ownership (invoke_signed)
    CreateAta2InvokeSigned = 11,
    /// Create a compressed mint with PDA mint signer (invoke_signed)
    CreateCmintInvokeSigned = 12,
    /// Create a compressed mint with PDA mint signer AND PDA authority (invoke_signed)
    CreateCmintWithPdaAuthority = 14,
    /// Transfer SPL tokens to Light Token account (invoke)
    SplToCtokenInvoke = 15,
    /// Transfer SPL tokens to Light Token account with PDA authority (invoke_signed)
    SplToCtokenInvokeSigned = 16,
    /// Transfer Light Token to SPL token account (invoke)
    CtokenToSplInvoke = 17,
    /// Transfer Light Token to SPL token account with PDA authority (invoke_signed)
    CtokenToSplInvokeSigned = 18,
    /// Unified transfer interface - auto-detects account types (invoke)
    TransferInterfaceInvoke = 19,
    /// Unified transfer interface with PDA authority (invoke_signed)
    TransferInterfaceInvokeSigned = 20,
    /// Approve delegate for Light Token account (invoke)
    ApproveInvoke = 21,
    /// Approve delegate for PDA-owned Light Token account (invoke_signed)
    ApproveInvokeSigned = 22,
    /// Revoke delegation for Light Token account (invoke)
    RevokeInvoke = 23,
    /// Revoke delegation for PDA-owned Light Token account (invoke_signed)
    RevokeInvokeSigned = 24,
    /// Freeze Light Token account (invoke)
    FreezeInvoke = 25,
    /// Freeze Light Token account with PDA freeze authority (invoke_signed)
    FreezeInvokeSigned = 26,
    /// Thaw frozen Light Token account (invoke)
    ThawInvoke = 27,
    /// Thaw frozen Light Token account with PDA freeze authority (invoke_signed)
    ThawInvokeSigned = 28,
    /// Burn CTokens (invoke)
    BurnInvoke = 29,
    /// Burn CTokens with PDA authority (invoke_signed)
    BurnInvokeSigned = 30,
    /// Mint to Light Token from decompressed Mint (invoke)
    CTokenMintToInvoke = 31,
    /// Mint to Light Token from decompressed Mint with PDA authority (invoke_signed)
    CTokenMintToInvokeSigned = 32,
    /// Decompress Mint with PDA authority (invoke_signed)
    DecompressCmintInvokeSigned = 33,
    /// Transfer cTokens with checked decimals (invoke)
    CTokenTransferCheckedInvoke = 34,
    /// Transfer cTokens with checked decimals from PDA-owned account (invoke_signed)
    CTokenTransferCheckedInvokeSigned = 35,
}

impl TryFrom<u8> for InstructionType {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InstructionType::CreateCmint),
            2 => Ok(InstructionType::CreateTokenAccountInvoke),
            3 => Ok(InstructionType::CreateTokenAccountInvokeSigned),
            4 => Ok(InstructionType::CreateAtaInvoke),
            5 => Ok(InstructionType::CreateAtaInvokeSigned),
            6 => Ok(InstructionType::CTokenTransferInvoke),
            7 => Ok(InstructionType::CTokenTransferInvokeSigned),
            8 => Ok(InstructionType::CloseAccountInvoke),
            9 => Ok(InstructionType::CloseAccountInvokeSigned),
            10 => Ok(InstructionType::CreateAta2Invoke),
            11 => Ok(InstructionType::CreateAta2InvokeSigned),
            12 => Ok(InstructionType::CreateCmintInvokeSigned),
            14 => Ok(InstructionType::CreateCmintWithPdaAuthority),
            15 => Ok(InstructionType::SplToCtokenInvoke),
            16 => Ok(InstructionType::SplToCtokenInvokeSigned),
            17 => Ok(InstructionType::CtokenToSplInvoke),
            18 => Ok(InstructionType::CtokenToSplInvokeSigned),
            19 => Ok(InstructionType::TransferInterfaceInvoke),
            20 => Ok(InstructionType::TransferInterfaceInvokeSigned),
            21 => Ok(InstructionType::ApproveInvoke),
            22 => Ok(InstructionType::ApproveInvokeSigned),
            23 => Ok(InstructionType::RevokeInvoke),
            24 => Ok(InstructionType::RevokeInvokeSigned),
            25 => Ok(InstructionType::FreezeInvoke),
            26 => Ok(InstructionType::FreezeInvokeSigned),
            27 => Ok(InstructionType::ThawInvoke),
            28 => Ok(InstructionType::ThawInvokeSigned),
            29 => Ok(InstructionType::BurnInvoke),
            30 => Ok(InstructionType::BurnInvokeSigned),
            31 => Ok(InstructionType::CTokenMintToInvoke),
            32 => Ok(InstructionType::CTokenMintToInvokeSigned),
            33 => Ok(InstructionType::DecompressCmintInvokeSigned),
            34 => Ok(InstructionType::CTokenTransferCheckedInvoke),
            35 => Ok(InstructionType::CTokenTransferCheckedInvokeSigned),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

/// Main program entrypoint
pub fn process_instruction(
    program_id: &[u8; 32],
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    use borsh::BorshDeserialize;

    if *program_id != ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    if instruction_data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }

    let discriminator = InstructionType::try_from(instruction_data[0])?;

    match discriminator {
        InstructionType::CreateCmint => {
            let data = CreateCmintData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_create_mint(accounts, data)
        }
        InstructionType::CreateTokenAccountInvoke => {
            let data = CreateTokenAccountData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_create_token_account_invoke(accounts, data)
        }
        InstructionType::CreateTokenAccountInvokeSigned => {
            let data = CreateTokenAccountData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_create_token_account_invoke_signed(accounts, data)
        }
        InstructionType::CreateAtaInvoke => {
            let data = CreateAtaData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_create_ata_invoke(accounts, data)
        }
        InstructionType::CreateAtaInvokeSigned => {
            let data = CreateAtaData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_create_ata_invoke_signed(accounts, data)
        }
        InstructionType::CTokenTransferInvoke => {
            let data = TransferData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_transfer_invoke(accounts, data)
        }
        InstructionType::CTokenTransferInvokeSigned => {
            let data = TransferData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_transfer_invoke_signed(accounts, data)
        }
        InstructionType::CloseAccountInvoke => process_close_account_invoke(accounts),
        InstructionType::CloseAccountInvokeSigned => process_close_account_invoke_signed(accounts),
        InstructionType::CreateCmintInvokeSigned => {
            let data = CreateCmintData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_create_mint_invoke_signed(accounts, data)
        }
        InstructionType::CreateCmintWithPdaAuthority => {
            let data = CreateCmintData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_create_mint_with_pda_authority(accounts, data)
        }
        InstructionType::SplToCtokenInvoke => {
            let data = TransferFromSplData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_spl_to_ctoken_invoke(accounts, data)
        }
        InstructionType::SplToCtokenInvokeSigned => {
            let data = TransferFromSplData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_spl_to_ctoken_invoke_signed(accounts, data)
        }
        InstructionType::CtokenToSplInvoke => {
            let data = TransferTokenToSplData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_ctoken_to_spl_invoke(accounts, data)
        }
        InstructionType::CtokenToSplInvokeSigned => {
            let data = TransferTokenToSplData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_ctoken_to_spl_invoke_signed(accounts, data)
        }
        InstructionType::TransferInterfaceInvoke => {
            let data = TransferInterfaceData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_transfer_interface_invoke(accounts, data)
        }
        InstructionType::TransferInterfaceInvokeSigned => {
            let data = TransferInterfaceData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_transfer_interface_invoke_signed(accounts, data)
        }
        InstructionType::ApproveInvoke => {
            let data = ApproveData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_approve_invoke(accounts, data)
        }
        InstructionType::ApproveInvokeSigned => {
            let data = ApproveData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_approve_invoke_signed(accounts, data)
        }
        InstructionType::RevokeInvoke => process_revoke_invoke(accounts),
        InstructionType::RevokeInvokeSigned => process_revoke_invoke_signed(accounts),
        InstructionType::FreezeInvoke => process_freeze_invoke(accounts),
        InstructionType::FreezeInvokeSigned => process_freeze_invoke_signed(accounts),
        InstructionType::ThawInvoke => process_thaw_invoke(accounts),
        InstructionType::ThawInvokeSigned => process_thaw_invoke_signed(accounts),
        InstructionType::BurnInvoke => {
            let data = BurnData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_burn_invoke(accounts, data.amount)
        }
        InstructionType::BurnInvokeSigned => {
            let data = BurnData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_burn_invoke_signed(accounts, data.amount)
        }
        InstructionType::CTokenMintToInvoke => {
            let data = MintToData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_mint_to_invoke(accounts, data.amount)
        }
        InstructionType::CTokenMintToInvokeSigned => {
            let data = MintToData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_mint_to_invoke_signed(accounts, data.amount)
        }
        InstructionType::DecompressCmintInvokeSigned => {
            let data = DecompressCmintData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_decompress_mint_invoke_signed(accounts, data)
        }
        InstructionType::CTokenTransferCheckedInvoke => {
            let data = TransferCheckedData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_transfer_checked_invoke(accounts, data)
        }
        InstructionType::CTokenTransferCheckedInvokeSigned => {
            let data = TransferCheckedData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_transfer_checked_invoke_signed(accounts, data)
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
