use borsh::{BorshDeserialize, BorshSerialize};
use light_token_sdk::{
    ctoken::{CreateCMintCpi, CreateCMintParams, ExtensionInstructionData, SystemAccountInfos},
    CompressedProof,
};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::ID;

/// PDA seed for mint signer in invoke_signed variant
pub const MINT_SIGNER_SEED: &[u8] = b"mint_signer";

/// Instruction data for create compressed mint
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateCmintData {
    pub decimals: u8,
    pub address_merkle_tree_root_index: u16,
    pub mint_authority: Pubkey,
    pub proof: CompressedProof,
    pub compression_address: [u8; 32],
    pub mint: Pubkey,
    pub freeze_authority: Option<Pubkey>,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
}

/// Handler for creating a compressed mint (invoke)
///
/// Uses the CreateCMintCpi builder pattern. This demonstrates how to:
/// 1. Build the CreateCMintParams struct from instruction data
/// 2. Build the CreateCMintCpi with accounts
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
pub fn process_create_cmint(
    accounts: &[AccountInfo],
    data: CreateCmintData,
) -> Result<(), ProgramError> {
    if accounts.len() < 12 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Build the params
    let params = CreateCMintParams {
        decimals: data.decimals,
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
    // In this case, payer == authority (accounts[3])
    CreateCMintCpi {
        mint_seed: accounts[2].clone(),
        authority: accounts[3].clone(),
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

/// Handler for creating a compressed mint with PDA mint signer (invoke_signed)
///
/// Uses the CreateCMintCpi builder pattern with invoke_signed.
/// The mint_signer is a PDA derived from this program.
///
/// Account order:
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: light_system_program
/// - accounts[2]: mint_signer (PDA, not signer - program signs)
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
pub fn process_create_cmint_invoke_signed(
    accounts: &[AccountInfo],
    data: CreateCmintData,
) -> Result<(), ProgramError> {
    if accounts.len() < 12 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the mint signer
    let (pda, bump) = Pubkey::find_program_address(&[MINT_SIGNER_SEED], &ID);

    // Verify the mint_signer account is the PDA we expect
    if &pda != accounts[2].key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Build the params
    let params = CreateCMintParams {
        decimals: data.decimals,
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
    // In this case, payer == authority (accounts[3])
    let account_infos = CreateCMintCpi {
        mint_seed: accounts[2].clone(),
        authority: accounts[3].clone(),
        payer: accounts[3].clone(),
        address_tree: accounts[11].clone(),
        output_queue: accounts[10].clone(),
        system_accounts,
        cpi_context: None,
        cpi_context_account: None,
        params,
    };

    // Invoke with PDA signing
    let signer_seeds: &[&[u8]] = &[MINT_SIGNER_SEED, &[bump]];
    account_infos.invoke_signed(&[signer_seeds])?;

    Ok(())
}

/// Handler for creating a compressed mint with PDA mint signer AND PDA authority (invoke_signed)
///
/// Uses the SDK's CreateCMintCpi with separate authority and payer accounts.
/// Both mint_signer and authority are PDAs signed by this program.
///
/// Account order:
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: light_system_program
/// - accounts[2]: mint_signer (PDA from MINT_SIGNER_SEED, not signer - program signs)
/// - accounts[3]: authority (PDA from MINT_AUTHORITY_SEED, not signer - program signs)
/// - accounts[4]: fee_payer (signer)
/// - accounts[5]: cpi_authority_pda
/// - accounts[6]: registered_program_pda
/// - accounts[7]: account_compression_authority
/// - accounts[8]: account_compression_program
/// - accounts[9]: system_program
/// - accounts[10]: output_queue
/// - accounts[11]: address_tree
/// - accounts[12] (optional): cpi_context_account
pub fn process_create_cmint_with_pda_authority(
    accounts: &[AccountInfo],
    data: CreateCmintData,
) -> Result<(), ProgramError> {
    use crate::mint_to_ctoken::MINT_AUTHORITY_SEED;

    if accounts.len() < 12 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the mint signer
    let (mint_signer_pda, mint_signer_bump) =
        Pubkey::find_program_address(&[MINT_SIGNER_SEED], &ID);

    // Derive the PDA for the authority
    let (authority_pda, authority_bump) = Pubkey::find_program_address(&[MINT_AUTHORITY_SEED], &ID);

    // Verify the mint_signer account is the PDA we expect
    if &mint_signer_pda != accounts[2].key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Verify the authority account is the PDA we expect
    if &authority_pda != accounts[3].key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Build the params - authority is the PDA
    let params = CreateCMintParams {
        decimals: data.decimals,
        address_merkle_tree_root_index: data.address_merkle_tree_root_index,
        mint_authority: authority_pda, // Use the derived PDA as authority
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

    // Build the account infos struct using SDK
    let account_infos = CreateCMintCpi {
        mint_seed: accounts[2].clone(),
        authority: accounts[3].clone(),
        payer: accounts[4].clone(),
        address_tree: accounts[11].clone(),
        output_queue: accounts[10].clone(),
        system_accounts,
        cpi_context: None,
        cpi_context_account: None,
        params,
    };

    // Invoke with both PDAs signing
    let mint_signer_seeds: &[&[u8]] = &[MINT_SIGNER_SEED, &[mint_signer_bump]];
    let authority_seeds: &[&[u8]] = &[MINT_AUTHORITY_SEED, &[authority_bump]];
    account_infos.invoke_signed(&[mint_signer_seeds, authority_seeds])?;

    Ok(())
}
