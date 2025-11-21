#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::instruction::ValidityProof;
use light_compressed_token_sdk::{
    ctoken::{
        CompressibleParamsInfos, CreateAssociatedTokenAccountInfos, CreateCMintParams,
        CreateCompressedMintInfos, CreateCTokenAccountInfos, ExtensionInstructionData,
        MintToCTokenInfos, MintToCTokenParams, SystemAccountInfos, TransferCtokenAccountInfos,
    },
    CompressedProof,
};
use light_ctoken_types::instructions::mint_action::CompressedMintWithContext;
use solana_program::{
    account_info::AccountInfo, entrypoint, program_error::ProgramError, pubkey, pubkey::Pubkey,
};

/// Program ID - replace with actual program ID after deployment
pub const ID: Pubkey = pubkey!("CToknNtvExmp1eProgram11111111111111111111112");

entrypoint!(process_instruction);

/// Instruction discriminators for the 8 instructions
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
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

/// Instruction data for create compressed mint
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateCmintData {
    pub decimals: u8,
    pub version: u8,
    pub address_merkle_tree_root_index: u16,
    pub mint_authority: Pubkey,
    pub proof: CompressedProof,
    pub compression_address: [u8; 32],
    pub mint: Pubkey,
    pub freeze_authority: Option<Pubkey>,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
}

/// Instruction data for transfer operations
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TransferData {
    pub amount: u64,
}

/// Instruction data for mint_to_ctoken operations
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct MintToCTokenData {
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub amount: u64,
    pub mint_authority: Pubkey,
    pub proof: ValidityProof,
}

/// Instruction data for create token account
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTokenAccountData {
    pub owner: Pubkey,
    pub pre_pay_num_epochs: u8,
    pub lamports_per_write: u32,
}

/// Instruction data for create ATA
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateAtaData {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub bump: u8,
    pub pre_pay_num_epochs: u8,
    pub lamports_per_write: u32,
}

/// PDA seeds for invoke_signed instructions
pub const TOKEN_ACCOUNT_SEED: &[u8] = b"token_account";
pub const ATA_SEED: &[u8] = b"ata";

/// Main program entrypoint
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
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
    }
}

/// Handler for creating a compressed mint (invoke)
///
/// Uses the CreateCompressedMintInfos builder pattern. This demonstrates how to:
/// 1. Build the CreateCMintParams struct from instruction data
/// 2. Build the CreateCompressedMintInfos with accounts
/// 3. Call invoke() which handles instruction building and CPI
///
/// Account order:
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: light_system_program
/// - accounts[2]: mint_signer (signer)
/// - accounts[3]: payer (signer, also authority)
/// - accounts[4]: payer again (fee_payer in SDK)
/// - accounts[5]: cpi_authority_pda
/// - accounts[6]: registered_program_pda
/// - accounts[7]: account_compression_authority
/// - accounts[8]: account_compression_program
/// - accounts[9]: system_program
/// - accounts[10]: output_queue
/// - accounts[11]: address_tree
/// - accounts[12] (optional): cpi_context_account
fn process_create_cmint(
    accounts: &[AccountInfo],
    data: CreateCmintData,
) -> Result<(), ProgramError> {
    if accounts.len() < 12 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Build the params
    let params = CreateCMintParams {
        decimals: data.decimals,
        version: data.version,
        address_merkle_tree_root_index: data.address_merkle_tree_root_index,
        mint_authority: data.mint_authority,
        proof: data.proof,
        compression_address: data.compression_address,
        mint: data.mint,
        freeze_authority: data.freeze_authority,
        extensions: data.extensions,
    };

    // Build system accounts struct
    let system_accounts = SystemAccountInfos {
        light_system_program: accounts[1].clone(),
        cpi_authority_pda: accounts[5].clone(),
        registered_program_pda: accounts[6].clone(),
        account_compression_authority: accounts[7].clone(),
        account_compression_program: accounts[8].clone(),
        system_program: accounts[9].clone(),
    };

    // Build the account infos struct
    CreateCompressedMintInfos {
        mint_signer: accounts[2].clone(),
        payer: accounts[3].clone(),
        address_tree: accounts[11].clone(),
        output_queue: accounts[10].clone(),
        system_accounts,
        cpi_context: None,
        cpi_context_account: None,
        params,
    }
    .invoke()?;

    Ok(())
}

/// Handler for minting tokens to compressed token accounts
///
/// Uses the MintToCTokenInfos builder pattern. This demonstrates how to:
/// 1. Build MintToCTokenParams using the constructor
/// 2. Build MintToCTokenInfos with accounts and params
/// 3. Call invoke() which handles instruction building and CPI
///
/// Account order (all accounts from SDK-generated instruction):
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: light_system_program
/// - accounts[2]: authority (mint_authority)
/// - accounts[3]: fee_payer
/// - accounts[4]: cpi_authority_pda
/// - accounts[5]: registered_program_pda
/// - accounts[6]: account_compression_authority
/// - accounts[7]: account_compression_program
/// - accounts[8]: system_program
/// - accounts[9]: output_queue
/// - accounts[10]: state_tree
/// - accounts[11]: input_queue
/// - accounts[12..]: ctoken_accounts (variable length - destination accounts)
fn process_mint_to_ctoken(
    accounts: &[AccountInfo],
    data: MintToCTokenData,
) -> Result<(), ProgramError> {
    if accounts.len() < 13 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Build params using the constructor
    let params = MintToCTokenParams::new(
        data.compressed_mint_inputs,
        data.amount,
        data.mint_authority,
        data.proof,
    );

    // Build system accounts struct
    let system_accounts = SystemAccountInfos {
        light_system_program: accounts[1].clone(),
        cpi_authority_pda: accounts[4].clone(),
        registered_program_pda: accounts[5].clone(),
        account_compression_authority: accounts[6].clone(),
        account_compression_program: accounts[7].clone(),
        system_program: accounts[8].clone(),
    };

    // Collect ctoken accounts from remaining accounts (index 12 onwards)
    let ctoken_accounts: Vec<AccountInfo> = accounts[12..].to_vec();

    // Build the account infos struct and invoke
    // SDK account order: output_queue (9), tree (10), input_queue (11), ctoken_accounts (12+)
    MintToCTokenInfos {
        payer: accounts[3].clone(), // fee_payer from SDK accounts
        state_tree: accounts[10].clone(),  // tree at index 10
        input_queue: accounts[11].clone(), // input_queue at index 11
        output_queue: accounts[9].clone(), // output_queue at index 9
        ctoken_accounts,
        system_accounts,
        cpi_context: None,
        cpi_context_account: None,
        params,
    }
    .invoke()?;

    Ok(())
}

/// Handler for creating a compressible token account (invoke)
///
/// Uses the builder pattern from the ctoken module. This demonstrates how to:
/// 1. Build the account infos struct with compressible params
/// 2. Call the invoke() method which handles instruction building and CPI
///
/// Account order:
/// - accounts[0]: payer (signer)
/// - accounts[1]: account to create (signer)
/// - accounts[2]: mint
/// - accounts[3]: compressible_config
/// - accounts[4]: system_program
/// - accounts[5]: rent_sponsor
fn process_create_token_account_invoke(
    accounts: &[AccountInfo],
    data: CreateTokenAccountData,
) -> Result<(), ProgramError> {
    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Build the compressible params using constructor
    let compressible_params = CompressibleParamsInfos::new(
        data.pre_pay_num_epochs,
        data.lamports_per_write,
        accounts[3].clone(),
        accounts[5].clone(),
        accounts[4].clone(),
    );

    // Build the account infos struct
    CreateCTokenAccountInfos {
        payer: accounts[0].clone(),
        account: accounts[1].clone(),
        mint: accounts[2].clone(),
        owner: data.owner,
        compressible: Some(compressible_params),
    }
    .invoke()?;

    Ok(())
}

/// Handler for creating a compressible token account with PDA ownership (invoke_signed)
///
/// Account order:
/// - accounts[0]: payer (signer)
/// - accounts[1]: account to create (PDA, will be derived and verified)
/// - accounts[2]: mint
/// - accounts[3]: compressible_config
/// - accounts[4]: system_program
/// - accounts[5]: rent_sponsor
fn process_create_token_account_invoke_signed(
    accounts: &[AccountInfo],
    data: CreateTokenAccountData,
) -> Result<(), ProgramError> {
    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the token account
    let (pda, bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the account to create is the PDA
    if &pda != accounts[1].key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Build the compressible params using constructor
    let compressible_params = CompressibleParamsInfos::new(
        data.pre_pay_num_epochs,
        data.lamports_per_write,
        accounts[3].clone(),
        accounts[5].clone(),
        accounts[4].clone(),
    );

    // Build the account infos struct
    let account_infos = CreateCTokenAccountInfos {
        payer: accounts[0].clone(),
        account: accounts[1].clone(),
        mint: accounts[2].clone(),
        owner: data.owner,
        compressible: Some(compressible_params),
    };

    // Invoke with PDA signing
    let signer_seeds: &[&[u8]] = &[TOKEN_ACCOUNT_SEED, &[bump]];
    account_infos.invoke_signed(&[signer_seeds])?;

    Ok(())
}

/// Handler for creating a compressible associated token account (invoke)
///
/// Account order:
/// - accounts[0]: payer (signer)
/// - accounts[1]: associated token account (derived)
/// - accounts[2]: system_program
/// - accounts[3]: compressible_config
/// - accounts[4]: rent_sponsor
fn process_create_ata_invoke(
    accounts: &[AccountInfo],
    data: CreateAtaData,
) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Build the compressible params using constructor
    let compressible_params = CompressibleParamsInfos::new(
        data.pre_pay_num_epochs,
        data.lamports_per_write,
        accounts[3].clone(),
        accounts[4].clone(),
        accounts[2].clone(),
    );

    // Use the CreateAssociatedTokenAccountInfos constructor
    CreateAssociatedTokenAccountInfos::new(
        data.bump,
        data.owner,
        data.mint,
        accounts[0].clone(),
        accounts[1].clone(),
        accounts[2].clone(),
        compressible_params,
    )
    .invoke()?;

    Ok(())
}

/// Handler for creating a compressible ATA with PDA ownership (invoke_signed)
///
/// Account order:
/// - accounts[0]: payer (PDA, signer via invoke_signed)
/// - accounts[1]: associated token account (derived)
/// - accounts[2]: system_program
/// - accounts[3]: compressible_config
/// - accounts[4]: rent_sponsor
fn process_create_ata_invoke_signed(
    accounts: &[AccountInfo],
    data: CreateAtaData,
) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA that will act as payer/owner
    let (pda, bump) = Pubkey::find_program_address(&[ATA_SEED], &ID);

    // Verify the payer is the PDA
    if &pda != accounts[0].key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Build the compressible params using constructor
    let compressible_params = CompressibleParamsInfos::new(
        data.pre_pay_num_epochs,
        data.lamports_per_write,
        accounts[3].clone(),
        accounts[4].clone(),
        accounts[2].clone(),
    );

    // Use the CreateAssociatedTokenAccountInfos constructor
    let account_infos = CreateAssociatedTokenAccountInfos::new(
        data.bump,
        data.owner,
        data.mint,
        accounts[0].clone(),
        accounts[1].clone(),
        accounts[2].clone(),
        compressible_params,
    );

    // Invoke with PDA signing
    let signer_seeds: &[&[u8]] = &[ATA_SEED, &[bump]];
    account_infos.invoke_signed(&[signer_seeds])?;

    Ok(())
}

/// Handler for transferring compressed tokens (invoke)
///
/// Uses the builder pattern from the ctoken module. This demonstrates how to:
/// 1. Build the account infos struct
/// 2. Call the invoke() method which handles instruction building and CPI
///
/// Account order:
/// - accounts[0]: source ctoken account
/// - accounts[1]: destination ctoken account
/// - accounts[2]: authority (signer)
fn process_transfer_invoke(
    accounts: &[AccountInfo],
    data: TransferData,
) -> Result<(), ProgramError> {
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Build the account infos struct using the builder pattern
    TransferCtokenAccountInfos {
        source: accounts[0].clone(),
        destination: accounts[1].clone(),
        amount: data.amount,
        authority: accounts[2].clone(),
    }
    .invoke()?;

    Ok(())
}

/// Handler for transferring compressed tokens from PDA-owned account (invoke_signed)
///
/// Uses the builder pattern with invoke_signed. This demonstrates how to:
/// 1. Build the account infos struct
/// 2. Derive PDA seeds
/// 3. Call invoke_signed() method with the signer seeds
///
/// Account order:
/// - accounts[0]: source ctoken account (PDA-owned)
/// - accounts[1]: destination ctoken account
/// - accounts[2]: authority (PDA)
fn process_transfer_invoke_signed(
    accounts: &[AccountInfo],
    data: TransferData,
) -> Result<(), ProgramError> {
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the authority
    let (pda, bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if &pda != accounts[2].key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Build the account infos struct
    let transfer_accounts = TransferCtokenAccountInfos {
        source: accounts[0].clone(),
        destination: accounts[1].clone(),
        amount: data.amount,
        authority: accounts[2].clone(),
    };

    // Invoke with PDA signing - the builder handles instruction creation and invoke_signed CPI
    let signer_seeds: &[&[u8]] = &[TOKEN_ACCOUNT_SEED, &[bump]];
    transfer_accounts.invoke_signed(&[signer_seeds])?;

    Ok(())
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
        assert!(InstructionType::try_from(8).is_err());
    }
}
