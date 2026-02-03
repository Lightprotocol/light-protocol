use alloc::vec::Vec;

use borsh::{BorshDeserialize, BorshSerialize};
use light_token_pinocchio::{
    instruction::{CreateMintCpi, CreateMintParams, ExtensionInstructionData, SystemAccountInfos},
    CompressedProof,
};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
};

use crate::ID;

/// PDA seed for mint signer in invoke_signed variant
pub const MINT_SIGNER_SEED: &[u8] = b"mint_signer";

/// Instruction data for create compressed mint
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateCmintData {
    pub decimals: u8,
    pub address_merkle_tree_root_index: u16,
    pub mint_authority: [u8; 32],
    pub proof: CompressedProof,
    pub compression_address: [u8; 32],
    pub mint: [u8; 32],
    pub bump: u8,
    pub freeze_authority: Option<[u8; 32]>,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
    pub rent_payment: u8,
    pub write_top_up: u32,
}

/// Handler for creating a compressed mint (invoke)
///
/// Uses the CreateMintCpi builder pattern. This demonstrates how to:
/// 1. Build the CreateMintParams struct from instruction data
/// 2. Build the CreateMintCpi with accounts
/// 3. Call invoke() which handles instruction building and CPI
///
/// Account order (matches MintActionMetaConfig::to_account_metas()):
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: light_system_program
/// - accounts[2]: mint_signer (signer)
/// - accounts[3]: authority (signer)
/// - accounts[4]: compressible_config
/// - accounts[5]: mint (PDA, writable)
/// - accounts[6]: rent_sponsor (PDA, writable)
/// - accounts[7]: fee_payer (signer)
/// - accounts[8]: cpi_authority_pda
/// - accounts[9]: registered_program_pda
/// - accounts[10]: account_compression_authority
/// - accounts[11]: account_compression_program
/// - accounts[12]: system_program
/// - accounts[13]: output_queue
/// - accounts[14]: address_tree
/// - accounts[15] (optional): cpi_context_account
pub fn process_create_mint(
    accounts: &[AccountInfo],
    data: CreateCmintData,
) -> Result<(), ProgramError> {
    if accounts.len() < 15 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Build the params
    let params = CreateMintParams {
        decimals: data.decimals,
        address_merkle_tree_root_index: data.address_merkle_tree_root_index,
        mint_authority: data.mint_authority,
        proof: data.proof,
        compression_address: data.compression_address,
        mint: data.mint,
        bump: data.bump,
        freeze_authority: data.freeze_authority,
        extensions: data.extensions,
        rent_payment: data.rent_payment,
        write_top_up: data.write_top_up,
    };

    // Build system accounts struct
    let system_accounts = SystemAccountInfos {
        light_system_program: &accounts[1],
        cpi_authority_pda: &accounts[8],
        registered_program_pda: &accounts[9],
        account_compression_authority: &accounts[10],
        account_compression_program: &accounts[11],
        system_program: &accounts[12],
    };

    // Build the account infos struct
    CreateMintCpi {
        mint_seed: &accounts[2],
        authority: &accounts[3],
        payer: &accounts[7],
        address_tree: &accounts[14],
        output_queue: &accounts[13],
        compressible_config: &accounts[4],
        mint: &accounts[5],
        rent_sponsor: &accounts[6],
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
/// Uses the CreateMintCpi builder pattern with invoke_signed.
/// The mint_signer is a PDA derived from this program.
///
/// Account order (matches MintActionMetaConfig::to_account_metas()):
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: light_system_program
/// - accounts[2]: mint_signer (PDA, not signer - program signs)
/// - accounts[3]: authority (signer)
/// - accounts[4]: compressible_config
/// - accounts[5]: mint (PDA, writable)
/// - accounts[6]: rent_sponsor (PDA, writable)
/// - accounts[7]: fee_payer (signer)
/// - accounts[8]: cpi_authority_pda
/// - accounts[9]: registered_program_pda
/// - accounts[10]: account_compression_authority
/// - accounts[11]: account_compression_program
/// - accounts[12]: system_program
/// - accounts[13]: output_queue
/// - accounts[14]: address_tree
/// - accounts[15] (optional): cpi_context_account
pub fn process_create_mint_invoke_signed(
    accounts: &[AccountInfo],
    data: CreateCmintData,
) -> Result<(), ProgramError> {
    if accounts.len() < 15 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the mint signer
    let (pda, bump) = pinocchio::pubkey::find_program_address(&[MINT_SIGNER_SEED], &ID);

    // Verify the mint_signer account is the PDA we expect
    if pda != *accounts[2].key() {
        return Err(ProgramError::InvalidSeeds);
    }

    // Build the params
    let params = CreateMintParams {
        decimals: data.decimals,
        address_merkle_tree_root_index: data.address_merkle_tree_root_index,
        mint_authority: data.mint_authority,
        proof: data.proof,
        compression_address: data.compression_address,
        mint: data.mint,
        bump: data.bump,
        freeze_authority: data.freeze_authority,
        extensions: data.extensions,
        rent_payment: data.rent_payment,
        write_top_up: data.write_top_up,
    };

    // Build system accounts struct
    let system_accounts = SystemAccountInfos {
        light_system_program: &accounts[1],
        cpi_authority_pda: &accounts[8],
        registered_program_pda: &accounts[9],
        account_compression_authority: &accounts[10],
        account_compression_program: &accounts[11],
        system_program: &accounts[12],
    };

    // Build the account infos struct
    let account_infos = CreateMintCpi {
        mint_seed: &accounts[2],
        authority: &accounts[3],
        payer: &accounts[7],
        address_tree: &accounts[14],
        output_queue: &accounts[13],
        compressible_config: &accounts[4],
        mint: &accounts[5],
        rent_sponsor: &accounts[6],
        system_accounts,
        cpi_context: None,
        cpi_context_account: None,
        params,
    };

    // Invoke with PDA signing
    let bump_byte = [bump];
    let seeds = [Seed::from(MINT_SIGNER_SEED), Seed::from(&bump_byte[..])];
    let signer = Signer::from(&seeds);
    account_infos.invoke_signed(&[signer])?;

    Ok(())
}

/// Handler for creating a compressed mint with PDA mint signer AND PDA authority (invoke_signed)
///
/// Uses the SDK's CreateMintCpi with separate authority and payer accounts.
/// Both mint_signer and authority are PDAs signed by this program.
///
/// Account order (matches MintActionMetaConfig::to_account_metas()):
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: light_system_program
/// - accounts[2]: mint_signer (PDA from MINT_SIGNER_SEED, not signer - program signs)
/// - accounts[3]: authority (PDA from MINT_AUTHORITY_SEED, not signer - program signs)
/// - accounts[4]: compressible_config
/// - accounts[5]: mint (PDA, writable)
/// - accounts[6]: rent_sponsor (PDA, writable)
/// - accounts[7]: fee_payer (signer)
/// - accounts[8]: cpi_authority_pda
/// - accounts[9]: registered_program_pda
/// - accounts[10]: account_compression_authority
/// - accounts[11]: account_compression_program
/// - accounts[12]: system_program
/// - accounts[13]: output_queue
/// - accounts[14]: address_tree
/// - accounts[15] (optional): cpi_context_account
pub fn process_create_mint_with_pda_authority(
    accounts: &[AccountInfo],
    data: CreateCmintData,
) -> Result<(), ProgramError> {
    use crate::MINT_AUTHORITY_SEED;

    if accounts.len() < 15 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the mint signer
    let (mint_signer_pda, mint_signer_bump) =
        pinocchio::pubkey::find_program_address(&[MINT_SIGNER_SEED], &ID);

    // Derive the PDA for the authority
    let (authority_pda, authority_bump) =
        pinocchio::pubkey::find_program_address(&[MINT_AUTHORITY_SEED], &ID);

    // Verify the mint_signer account is the PDA we expect
    if mint_signer_pda != *accounts[2].key() {
        return Err(ProgramError::InvalidSeeds);
    }

    // Verify the authority account is the PDA we expect
    if authority_pda != *accounts[3].key() {
        return Err(ProgramError::InvalidSeeds);
    }

    // Build the params - authority is the PDA
    let params = CreateMintParams {
        decimals: data.decimals,
        address_merkle_tree_root_index: data.address_merkle_tree_root_index,
        mint_authority: authority_pda,
        proof: data.proof,
        compression_address: data.compression_address,
        mint: data.mint,
        bump: data.bump,
        freeze_authority: data.freeze_authority,
        extensions: data.extensions,
        rent_payment: data.rent_payment,
        write_top_up: data.write_top_up,
    };

    // Build system accounts struct
    let system_accounts = SystemAccountInfos {
        light_system_program: &accounts[1],
        cpi_authority_pda: &accounts[8],
        registered_program_pda: &accounts[9],
        account_compression_authority: &accounts[10],
        account_compression_program: &accounts[11],
        system_program: &accounts[12],
    };

    // Build the account infos struct using SDK
    let account_infos = CreateMintCpi {
        mint_seed: &accounts[2],
        authority: &accounts[3],
        payer: &accounts[7],
        address_tree: &accounts[14],
        output_queue: &accounts[13],
        compressible_config: &accounts[4],
        mint: &accounts[5],
        rent_sponsor: &accounts[6],
        system_accounts,
        cpi_context: None,
        cpi_context_account: None,
        params,
    };

    // Invoke with both PDAs signing
    let mint_signer_bump_byte = [mint_signer_bump];
    let mint_signer_seeds = [
        Seed::from(MINT_SIGNER_SEED),
        Seed::from(&mint_signer_bump_byte[..]),
    ];
    let mint_signer = Signer::from(&mint_signer_seeds);

    let authority_bump_byte = [authority_bump];
    let authority_seeds = [
        Seed::from(MINT_AUTHORITY_SEED),
        Seed::from(&authority_bump_byte[..]),
    ];
    let authority_signer = Signer::from(&authority_seeds);

    account_infos.invoke_signed(&[mint_signer, authority_signer])?;

    Ok(())
}
