#![allow(unexpected_cfgs)]

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
pub use approve::{
    process_approve_invoke, process_approve_invoke_signed, process_approve_invoke_with_fee_payer,
    ApproveData,
};
pub use burn::{
    process_burn_invoke, process_burn_invoke_signed, process_burn_invoke_with_fee_payer, BurnData,
};
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
pub use ctoken_mint_to::{
    process_mint_to_invoke, process_mint_to_invoke_signed, process_mint_to_invoke_with_fee_payer,
    MintToData,
};
pub use decompress_mint::{process_decompress_mint_invoke_signed, DecompressCmintData};
pub use freeze::{process_freeze_invoke, process_freeze_invoke_signed};
pub use revoke::{
    process_revoke_invoke, process_revoke_invoke_signed, process_revoke_invoke_with_fee_payer,
};
use solana_program::{
    account_info::AccountInfo, entrypoint, program_error::ProgramError, pubkey, pubkey::Pubkey,
};
pub use thaw::{process_thaw_invoke, process_thaw_invoke_signed};
pub use transfer::{
    process_transfer_invoke, process_transfer_invoke_signed, process_transfer_invoke_with_fee_payer,
    TransferData,
};
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
pub const ID: Pubkey = pubkey!("CToknNtvExmp1eProgram11111111111111111111112");

/// PDA seeds for invoke_signed instructions
pub const TOKEN_ACCOUNT_SEED: &[u8] = b"token_account";
pub const ATA_SEED: &[u8] = b"ata";
pub const FREEZE_AUTHORITY_SEED: &[u8] = b"freeze_authority";
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";

entrypoint!(process_instruction);

/// Instruction discriminators
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// Transfer compressed tokens with separate fee_payer (invoke)
    CTokenTransferInvokeWithFeePayer = 36,
    /// Burn CTokens with separate fee_payer (invoke)
    BurnInvokeWithFeePayer = 37,
    /// Mint to Light Token with separate fee_payer (invoke)
    CTokenMintToInvokeWithFeePayer = 38,
    /// Approve delegate with separate fee_payer (invoke)
    ApproveInvokeWithFeePayer = 39,
    /// Revoke delegation with separate fee_payer (invoke)
    RevokeInvokeWithFeePayer = 40,
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
            36 => Ok(InstructionType::CTokenTransferInvokeWithFeePayer),
            37 => Ok(InstructionType::BurnInvokeWithFeePayer),
            38 => Ok(InstructionType::CTokenMintToInvokeWithFeePayer),
            39 => Ok(InstructionType::ApproveInvokeWithFeePayer),
            40 => Ok(InstructionType::RevokeInvokeWithFeePayer),
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
        InstructionType::CTokenTransferInvokeWithFeePayer => {
            let data = TransferData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_transfer_invoke_with_fee_payer(accounts, data)
        }
        InstructionType::BurnInvokeWithFeePayer => {
            let data = BurnData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_burn_invoke_with_fee_payer(accounts, data.amount)
        }
        InstructionType::CTokenMintToInvokeWithFeePayer => {
            let data = MintToData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_mint_to_invoke_with_fee_payer(accounts, data.amount)
        }
        InstructionType::ApproveInvokeWithFeePayer => {
            let data = ApproveData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_approve_invoke_with_fee_payer(accounts, data)
        }
        InstructionType::RevokeInvokeWithFeePayer => {
            process_revoke_invoke_with_fee_payer(accounts)
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
        assert_eq!(InstructionType::CTokenTransferCheckedInvoke as u8, 34);
        assert_eq!(InstructionType::CTokenTransferCheckedInvokeSigned as u8, 35);
        assert_eq!(InstructionType::CTokenTransferInvokeWithFeePayer as u8, 36);
        assert_eq!(InstructionType::BurnInvokeWithFeePayer as u8, 37);
        assert_eq!(InstructionType::CTokenMintToInvokeWithFeePayer as u8, 38);
        assert_eq!(InstructionType::ApproveInvokeWithFeePayer as u8, 39);
        assert_eq!(InstructionType::RevokeInvokeWithFeePayer as u8, 40);
    }

    #[test]
    fn test_instruction_type_conversion() {
        assert_eq!(
            InstructionType::try_from(0).unwrap(),
            InstructionType::CreateCmint
        );
        assert!(InstructionType::try_from(1).is_err());
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
        assert!(InstructionType::try_from(13).is_err());
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
        assert_eq!(
            InstructionType::try_from(34).unwrap(),
            InstructionType::CTokenTransferCheckedInvoke
        );
        assert_eq!(
            InstructionType::try_from(35).unwrap(),
            InstructionType::CTokenTransferCheckedInvokeSigned
        );
        assert_eq!(
            InstructionType::try_from(36).unwrap(),
            InstructionType::CTokenTransferInvokeWithFeePayer
        );
        assert_eq!(
            InstructionType::try_from(37).unwrap(),
            InstructionType::BurnInvokeWithFeePayer
        );
        assert_eq!(
            InstructionType::try_from(38).unwrap(),
            InstructionType::CTokenMintToInvokeWithFeePayer
        );
        assert_eq!(
            InstructionType::try_from(39).unwrap(),
            InstructionType::ApproveInvokeWithFeePayer
        );
        assert_eq!(
            InstructionType::try_from(40).unwrap(),
            InstructionType::RevokeInvokeWithFeePayer
        );
        assert!(InstructionType::try_from(41).is_err());
    }
}
