use borsh::{BorshDeserialize, BorshSerialize};
use light_token_interface::instructions::mint_action::CompressedMintWithContext;
use light_ctoken_sdk::ctoken::{MintToCTokenCpi, MintToCTokenParams, SystemAccountInfos};
use light_sdk::instruction::ValidityProof;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::ID;

/// PDA seed for mint authority in invoke_signed variant
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";

/// Instruction data for mint_to_ctoken operations
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct MintToCTokenData {
    pub compressed_mint_inputs: CompressedMintWithContext,
    pub amount: u64,
    pub mint_authority: Pubkey,
    pub proof: ValidityProof,
}

/// Handler for minting tokens to compressed token accounts
///
/// Uses the MintToCTokenCpi builder pattern. This demonstrates how to:
/// 1. Build MintToCTokenParams using the constructor
/// 2. Build MintToCTokenCpi with accounts and params
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
pub fn process_mint_to_ctoken(
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
    // In this case, payer == authority (accounts[3])
    MintToCTokenCpi {
        authority: accounts[2].clone(),    // authority from SDK accounts
        payer: accounts[3].clone(),        // fee_payer from SDK accounts
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

/// Handler for minting tokens with PDA mint authority (invoke_signed)
///
/// Uses the MintToCTokenCpi builder pattern with invoke_signed.
/// The mint authority is a PDA derived from this program.
///
/// Account order (all accounts from SDK-generated instruction):
/// - accounts[0]: compressed_token_program (for CPI)
/// - accounts[1]: light_system_program
/// - accounts[2]: authority (PDA mint_authority, not signer - program signs)
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
pub fn process_mint_to_ctoken_invoke_signed(
    accounts: &[AccountInfo],
    data: MintToCTokenData,
) -> Result<(), ProgramError> {
    if accounts.len() < 13 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the mint authority
    let (pda, bump) = Pubkey::find_program_address(&[MINT_AUTHORITY_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if &pda != accounts[2].key {
        return Err(ProgramError::InvalidSeeds);
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

    // Build the account infos struct
    // authority is the PDA (accounts[2])
    let account_infos = MintToCTokenCpi {
        authority: accounts[2].clone(),    // authority PDA
        payer: accounts[3].clone(),        // fee_payer from SDK accounts
        state_tree: accounts[10].clone(),  // tree at index 10
        input_queue: accounts[11].clone(), // input_queue at index 11
        output_queue: accounts[9].clone(), // output_queue at index 9
        ctoken_accounts,
        system_accounts,
        cpi_context: None,
        cpi_context_account: None,
        params,
    };

    // Invoke with PDA signing
    let signer_seeds: &[&[u8]] = &[MINT_AUTHORITY_SEED, &[bump]];
    account_infos.invoke_signed(&[signer_seeds])?;

    Ok(())
}
