#![allow(unexpected_cfgs)]

mod close;
mod create_ata;
mod create_ata2;
mod create_cmint;
mod create_token_account;
mod mint_to_ctoken;
mod transfer;

// Re-export all instruction data types
pub use close::{process_close_account_invoke, process_close_account_invoke_signed};
pub use create_ata::{process_create_ata_invoke, process_create_ata_invoke_signed, CreateAtaData};
pub use create_ata2::{
    process_create_ata2_invoke, process_create_ata2_invoke_signed, CreateAta2Data,
};
pub use create_cmint::{
    process_create_cmint, process_create_cmint_invoke_signed, process_create_cmint_with_pda_authority,
    CreateCmintData, MINT_SIGNER_SEED,
};
pub use create_token_account::{
    process_create_token_account_invoke, process_create_token_account_invoke_signed,
    CreateTokenAccountData,
};
pub use mint_to_ctoken::{
    process_mint_to_ctoken, process_mint_to_ctoken_invoke_signed, MintToCTokenData,
    MINT_AUTHORITY_SEED,
};
pub use transfer::{process_transfer_invoke, process_transfer_invoke_signed, TransferData};

use solana_program::{
    account_info::AccountInfo, entrypoint, program_error::ProgramError, pubkey, pubkey::Pubkey,
};

/// Program ID - replace with actual program ID after deployment
pub const ID: Pubkey = pubkey!("CToknNtvExmp1eProgram11111111111111111111112");

/// PDA seeds for invoke_signed instructions
pub const TOKEN_ACCOUNT_SEED: &[u8] = b"token_account";
pub const ATA_SEED: &[u8] = b"ata";

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
    /// Transfer compressed tokens (invoke)
    TransferInterfaceInvoke = 6,
    /// Transfer compressed tokens from PDA-owned account (invoke_signed)
    TransferInterfaceInvokeSigned = 7,
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
            6 => Ok(InstructionType::TransferInterfaceInvoke),
            7 => Ok(InstructionType::TransferInterfaceInvokeSigned),
            8 => Ok(InstructionType::CloseAccountInvoke),
            9 => Ok(InstructionType::CloseAccountInvokeSigned),
            10 => Ok(InstructionType::CreateAta2Invoke),
            11 => Ok(InstructionType::CreateAta2InvokeSigned),
            12 => Ok(InstructionType::CreateCmintInvokeSigned),
            13 => Ok(InstructionType::MintToCtokenInvokeSigned),
            14 => Ok(InstructionType::CreateCmintWithPdaAuthority),
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
        InstructionType::TransferInterfaceInvoke => {
            let data = TransferData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_transfer_invoke(accounts, data)
        }
        InstructionType::TransferInterfaceInvokeSigned => {
            let data = TransferData::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_transfer_invoke_signed(accounts, data)
        }
        InstructionType::CloseAccountInvoke => process_close_account_invoke(accounts),
        InstructionType::CloseAccountInvokeSigned => process_close_account_invoke_signed(accounts),
        InstructionType::CreateAta2Invoke => {
            let data = CreateAta2Data::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_create_ata2_invoke(accounts, data)
        }
        InstructionType::CreateAta2InvokeSigned => {
            let data = CreateAta2Data::try_from_slice(&instruction_data[1..])
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            process_create_ata2_invoke_signed(accounts, data)
        }
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
        assert_eq!(InstructionType::TransferInterfaceInvoke as u8, 6);
        assert_eq!(InstructionType::TransferInterfaceInvokeSigned as u8, 7);
        assert_eq!(InstructionType::CloseAccountInvoke as u8, 8);
        assert_eq!(InstructionType::CloseAccountInvokeSigned as u8, 9);
        assert_eq!(InstructionType::CreateAta2Invoke as u8, 10);
        assert_eq!(InstructionType::CreateAta2InvokeSigned as u8, 11);
        assert_eq!(InstructionType::CreateCmintInvokeSigned as u8, 12);
        assert_eq!(InstructionType::MintToCtokenInvokeSigned as u8, 13);
        assert_eq!(InstructionType::CreateCmintWithPdaAuthority as u8, 14);
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
            InstructionType::TransferInterfaceInvoke
        );
        assert_eq!(
            InstructionType::try_from(7).unwrap(),
            InstructionType::TransferInterfaceInvokeSigned
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
        assert!(InstructionType::try_from(15).is_err());
    }
}
