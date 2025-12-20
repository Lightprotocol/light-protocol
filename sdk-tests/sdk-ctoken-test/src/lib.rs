#![allow(unexpected_cfgs)]

mod approve;
mod burn;
mod close;
mod create_ata;
mod create_cmint;
mod create_token_account;
mod ctoken_mint_to;
mod decompress_cmint;
mod freeze;
mod mint_to_ctoken;
mod revoke;
mod thaw;
mod transfer;
mod transfer_interface;
mod transfer_spl_ctoken;

// Re-export all instruction data types
pub use approve::{process_approve_invoke, process_approve_invoke_signed, ApproveData};
pub use burn::{process_burn_invoke, process_burn_invoke_signed, BurnData};
pub use close::{process_close_account_invoke, process_close_account_invoke_signed};
pub use create_ata::{process_create_ata_invoke, process_create_ata_invoke_signed, CreateAtaData};
pub use create_cmint::{
    process_create_cmint, process_create_cmint_invoke_signed,
    process_create_cmint_with_pda_authority, CreateCmintData, MINT_SIGNER_SEED,
};
pub use create_token_account::{
    process_create_token_account_invoke, process_create_token_account_invoke_signed,
    CreateTokenAccountData,
};
pub use ctoken_mint_to::{
    process_ctoken_mint_to_invoke, process_ctoken_mint_to_invoke_signed, MintToData,
};
pub use decompress_cmint::{process_decompress_cmint_invoke_signed, DecompressCmintData};
pub use freeze::{process_freeze_invoke, process_freeze_invoke_signed};
pub use mint_to_ctoken::{
    process_mint_to_ctoken, process_mint_to_ctoken_invoke_signed, MintToCTokenData,
    MINT_AUTHORITY_SEED,
};
pub use revoke::{process_revoke_invoke, process_revoke_invoke_signed};
use solana_program::{
    account_info::AccountInfo, entrypoint, program_error::ProgramError, pubkey, pubkey::Pubkey,
};
pub use thaw::{process_thaw_invoke, process_thaw_invoke_signed};
pub use transfer::{process_transfer_invoke, process_transfer_invoke_signed, TransferData};
pub use transfer_interface::{
    process_transfer_interface_invoke, process_transfer_interface_invoke_signed,
    TransferInterfaceData, TRANSFER_INTERFACE_AUTHORITY_SEED,
};
pub use transfer_spl_ctoken::{
    process_ctoken_to_spl_invoke, process_ctoken_to_spl_invoke_signed,
    process_spl_to_ctoken_invoke, process_spl_to_ctoken_invoke_signed, TransferCTokenToSplData,
    TransferSplToCtokenData, TRANSFER_AUTHORITY_SEED,
};

/// Program ID - replace with actual program ID after deployment
pub const ID: Pubkey = pubkey!("CToknNtvExmp1eProgram11111111111111111111112");

/// PDA seeds for invoke_signed instructions
pub const TOKEN_ACCOUNT_SEED: &[u8] = b"token_account";
pub const ATA_SEED: &[u8] = b"ata";
pub const FREEZE_AUTHORITY_SEED: &[u8] = b"freeze_authority";

entrypoint!(process_instruction);

/// Instruction discriminators
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InstructionType {
    /// Create a compressed mint
    CreateCmint = 0,
    /// Mint tokens to compressed accounts
    MintToCtoken = 1,
    /// Create compressible token account (invoke)
    CreateTokenAccountInvoke = 2,
    /// Create compressible token account with PDA ownership (invoke_signed)
    CreateTokenAccountInvokeSigned = 3,
    /// Create compressible associated token account (invoke)
    CreateAtaInvoke = 4,
    /// Create compressible associated token account with PDA ownership (invoke_signed)
    CreateAtaInvokeSigned = 5,
    /// Transfer compressed tokens CToken->CToken (invoke)
    CTokenTransferInvoke = 6,
    /// Transfer compressed tokens CToken->CToken from PDA-owned account (invoke_signed)
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
    /// Mint tokens with PDA mint authority (invoke_signed)
    MintToCtokenInvokeSigned = 13,
    /// Create a compressed mint with PDA mint signer AND PDA authority (invoke_signed)
    CreateCmintWithPdaAuthority = 14,
    /// Transfer SPL tokens to CToken account (invoke)
    SplToCtokenInvoke = 15,
    /// Transfer SPL tokens to CToken account with PDA authority (invoke_signed)
    SplToCtokenInvokeSigned = 16,
    /// Transfer CToken to SPL token account (invoke)
    CtokenToSplInvoke = 17,
    /// Transfer CToken to SPL token account with PDA authority (invoke_signed)
    CtokenToSplInvokeSigned = 18,
    /// Unified transfer interface - auto-detects account types (invoke)
    TransferInterfaceInvoke = 19,
    /// Unified transfer interface with PDA authority (invoke_signed)
    TransferInterfaceInvokeSigned = 20,
    /// Approve delegate for CToken account (invoke)
    ApproveInvoke = 21,
    /// Approve delegate for PDA-owned CToken account (invoke_signed)
    ApproveInvokeSigned = 22,
    /// Revoke delegation for CToken account (invoke)
    RevokeInvoke = 23,
    /// Revoke delegation for PDA-owned CToken account (invoke_signed)
    RevokeInvokeSigned = 24,
    /// Freeze CToken account (invoke)
    FreezeInvoke = 25,
    /// Freeze CToken account with PDA freeze authority (invoke_signed)
    FreezeInvokeSigned = 26,
    /// Thaw frozen CToken account (invoke)
    ThawInvoke = 27,
    /// Thaw frozen CToken account with PDA freeze authority (invoke_signed)
    ThawInvokeSigned = 28,
    /// Burn CTokens (invoke)
    BurnInvoke = 29,
    /// Burn CTokens with PDA authority (invoke_signed)
    BurnInvokeSigned = 30,
    /// Mint to CToken from decompressed CMint (invoke)
    CTokenMintToInvoke = 31,
    /// Mint to CToken from decompressed CMint with PDA authority (invoke_signed)
    CTokenMintToInvokeSigned = 32,
    /// Decompress CMint with PDA authority (invoke_signed)
    DecompressCmintInvokeSigned = 33,
}

impl TryFrom<u8> for InstructionType {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InstructionType::CreateCmint),
            1 => Ok(InstructionType::MintToCtoken),
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
            13 => Ok(InstructionType::MintToCtokenInvokeSigned),
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
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

/// Main program entrypoint
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    use borsh::BorshDeserialize;

    if program_id != &ID {
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
            process_create_cmint(accounts, data)
        }
        InstructionType::MintToCtoken => {
            let data = MintToCTokenData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_mint_to_ctoken(accounts, data)
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
            process_create_cmint_invoke_signed(accounts, data)
        }
        InstructionType::MintToCtokenInvokeSigned => {
            let data = MintToCTokenData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_mint_to_ctoken_invoke_signed(accounts, data)
        }
        InstructionType::CreateCmintWithPdaAuthority => {
            let data = CreateCmintData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_create_cmint_with_pda_authority(accounts, data)
        }
        InstructionType::SplToCtokenInvoke => {
            let data = TransferSplToCtokenData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_spl_to_ctoken_invoke(accounts, data)
        }
        InstructionType::SplToCtokenInvokeSigned => {
            let data = TransferSplToCtokenData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_spl_to_ctoken_invoke_signed(accounts, data)
        }
        InstructionType::CtokenToSplInvoke => {
            let data = TransferCTokenToSplData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_ctoken_to_spl_invoke(accounts, data)
        }
        InstructionType::CtokenToSplInvokeSigned => {
            let data = TransferCTokenToSplData::try_from_slice(&instruction_data[1..])
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
            process_ctoken_mint_to_invoke(accounts, data.amount)
        }
        InstructionType::CTokenMintToInvokeSigned => {
            let data = MintToData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_ctoken_mint_to_invoke_signed(accounts, data.amount)
        }
        InstructionType::DecompressCmintInvokeSigned => {
            let data = DecompressCmintData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_decompress_cmint_invoke_signed(accounts, data)
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_discriminators() {
        assert_eq!(InstructionType::CreateCmint as u8, 0);
        assert_eq!(InstructionType::MintToCtoken as u8, 1);
        assert_eq!(InstructionType::CreateTokenAccountInvoke as u8, 2);
        assert_eq!(InstructionType::CreateTokenAccountInvokeSigned as u8, 3);
        assert_eq!(InstructionType::CreateAtaInvoke as u8, 4);
        assert_eq!(InstructionType::CreateAtaInvokeSigned as u8, 5);
        assert_eq!(InstructionType::CTokenTransferInvoke as u8, 6);
        assert_eq!(InstructionType::CTokenTransferInvokeSigned as u8, 7);
        assert_eq!(InstructionType::CloseAccountInvoke as u8, 8);
        assert_eq!(InstructionType::CloseAccountInvokeSigned as u8, 9);
        assert_eq!(InstructionType::CreateAta2Invoke as u8, 10);
        assert_eq!(InstructionType::CreateAta2InvokeSigned as u8, 11);
        assert_eq!(InstructionType::CreateCmintInvokeSigned as u8, 12);
        assert_eq!(InstructionType::MintToCtokenInvokeSigned as u8, 13);
        assert_eq!(InstructionType::CreateCmintWithPdaAuthority as u8, 14);
        assert_eq!(InstructionType::SplToCtokenInvoke as u8, 15);
        assert_eq!(InstructionType::SplToCtokenInvokeSigned as u8, 16);
        assert_eq!(InstructionType::CtokenToSplInvoke as u8, 17);
        assert_eq!(InstructionType::CtokenToSplInvokeSigned as u8, 18);
        assert_eq!(InstructionType::TransferInterfaceInvoke as u8, 19);
        assert_eq!(InstructionType::TransferInterfaceInvokeSigned as u8, 20);
        assert_eq!(InstructionType::ApproveInvoke as u8, 21);
        assert_eq!(InstructionType::ApproveInvokeSigned as u8, 22);
        assert_eq!(InstructionType::RevokeInvoke as u8, 23);
        assert_eq!(InstructionType::RevokeInvokeSigned as u8, 24);
        assert_eq!(InstructionType::FreezeInvoke as u8, 25);
        assert_eq!(InstructionType::FreezeInvokeSigned as u8, 26);
        assert_eq!(InstructionType::ThawInvoke as u8, 27);
        assert_eq!(InstructionType::ThawInvokeSigned as u8, 28);
        assert_eq!(InstructionType::BurnInvoke as u8, 29);
        assert_eq!(InstructionType::BurnInvokeSigned as u8, 30);
        assert_eq!(InstructionType::CTokenMintToInvoke as u8, 31);
        assert_eq!(InstructionType::CTokenMintToInvokeSigned as u8, 32);
        assert_eq!(InstructionType::DecompressCmintInvokeSigned as u8, 33);
    }

    #[test]
    fn test_instruction_type_conversion() {
        assert_eq!(
            InstructionType::try_from(0).unwrap(),
            InstructionType::CreateCmint
        );
        assert_eq!(
            InstructionType::try_from(1).unwrap(),
            InstructionType::MintToCtoken
        );
        assert_eq!(
            InstructionType::try_from(2).unwrap(),
            InstructionType::CreateTokenAccountInvoke
        );
        assert_eq!(
            InstructionType::try_from(3).unwrap(),
            InstructionType::CreateTokenAccountInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(4).unwrap(),
            InstructionType::CreateAtaInvoke
        );
        assert_eq!(
            InstructionType::try_from(5).unwrap(),
            InstructionType::CreateAtaInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(6).unwrap(),
            InstructionType::CTokenTransferInvoke
        );
        assert_eq!(
            InstructionType::try_from(7).unwrap(),
            InstructionType::CTokenTransferInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(8).unwrap(),
            InstructionType::CloseAccountInvoke
        );
        assert_eq!(
            InstructionType::try_from(9).unwrap(),
            InstructionType::CloseAccountInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(10).unwrap(),
            InstructionType::CreateAta2Invoke
        );
        assert_eq!(
            InstructionType::try_from(11).unwrap(),
            InstructionType::CreateAta2InvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(12).unwrap(),
            InstructionType::CreateCmintInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(13).unwrap(),
            InstructionType::MintToCtokenInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(14).unwrap(),
            InstructionType::CreateCmintWithPdaAuthority
        );
        assert_eq!(
            InstructionType::try_from(15).unwrap(),
            InstructionType::SplToCtokenInvoke
        );
        assert_eq!(
            InstructionType::try_from(16).unwrap(),
            InstructionType::SplToCtokenInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(17).unwrap(),
            InstructionType::CtokenToSplInvoke
        );
        assert_eq!(
            InstructionType::try_from(18).unwrap(),
            InstructionType::CtokenToSplInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(19).unwrap(),
            InstructionType::TransferInterfaceInvoke
        );
        assert_eq!(
            InstructionType::try_from(20).unwrap(),
            InstructionType::TransferInterfaceInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(21).unwrap(),
            InstructionType::ApproveInvoke
        );
        assert_eq!(
            InstructionType::try_from(22).unwrap(),
            InstructionType::ApproveInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(23).unwrap(),
            InstructionType::RevokeInvoke
        );
        assert_eq!(
            InstructionType::try_from(24).unwrap(),
            InstructionType::RevokeInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(25).unwrap(),
            InstructionType::FreezeInvoke
        );
        assert_eq!(
            InstructionType::try_from(26).unwrap(),
            InstructionType::FreezeInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(27).unwrap(),
            InstructionType::ThawInvoke
        );
        assert_eq!(
            InstructionType::try_from(28).unwrap(),
            InstructionType::ThawInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(29).unwrap(),
            InstructionType::BurnInvoke
        );
        assert_eq!(
            InstructionType::try_from(30).unwrap(),
            InstructionType::BurnInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(31).unwrap(),
            InstructionType::CTokenMintToInvoke
        );
        assert_eq!(
            InstructionType::try_from(32).unwrap(),
            InstructionType::CTokenMintToInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(33).unwrap(),
            InstructionType::DecompressCmintInvokeSigned
        );
        assert!(InstructionType::try_from(34).is_err());
    }
}
