use borsh::{BorshDeserialize, BorshSerialize};
use light_token_pinocchio::{
    instruction::{DecompressMintCpi, MintWithContext, SystemAccountInfos},
    ValidityProof,
};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
};

use crate::{ID, MINT_AUTHORITY_SEED};

/// Instruction data for DecompressMint operations
#[derive(BorshSerialize, BorshDeserialize)]
pub struct DecompressCmintData {
    pub compressed_mint_with_context: MintWithContext,
    pub proof: ValidityProof,
    pub rent_payment: u8,
    pub write_top_up: u32,
}

/// Handler for decompressing Mint with PDA authority (invoke_signed)
///
/// Account order:
/// - accounts[0]: mint_seed (readonly)
/// - accounts[1]: authority (PDA, readonly - program signs)
/// - accounts[2]: payer (signer, writable)
/// - accounts[3]: mint (writable)
/// - accounts[4]: compressible_config (readonly)
/// - accounts[5]: rent_sponsor (writable)
/// - accounts[6]: state_tree (writable)
/// - accounts[7]: input_queue (writable)
/// - accounts[8]: output_queue (writable)
/// - accounts[9]: light_system_program (readonly)
/// - accounts[10]: cpi_authority_pda (readonly)
/// - accounts[11]: registered_program_pda (readonly)
/// - accounts[12]: account_compression_authority (readonly)
/// - accounts[13]: account_compression_program (readonly)
/// - accounts[14]: system_program (readonly)
pub fn process_decompress_mint_invoke_signed(
    accounts: &[AccountInfo],
    data: DecompressCmintData,
) -> Result<(), ProgramError> {
    if accounts.len() < 15 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the mint authority
    let (pda, bump) = pinocchio::pubkey::find_program_address(&[MINT_AUTHORITY_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if pda != *accounts[0].key() {
        return Err(ProgramError::InvalidSeeds);
    }

    let system_accounts = SystemAccountInfos {
        light_system_program: &accounts[8],
        cpi_authority_pda: &accounts[9],
        registered_program_pda: &accounts[10],
        account_compression_authority: &accounts[11],
        account_compression_program: &accounts[12],
        system_program: &accounts[13],
    };

    let bump_byte = [bump];
    let seeds = [Seed::from(MINT_AUTHORITY_SEED), Seed::from(&bump_byte[..])];
    let signer = Signer::from(&seeds);

    DecompressMintCpi {
        authority: &accounts[0],
        payer: &accounts[1],
        mint: &accounts[2],
        compressible_config: &accounts[3],
        rent_sponsor: &accounts[4],
        state_tree: &accounts[5],
        input_queue: &accounts[6],
        output_queue: &accounts[7],
        system_accounts,
        compressed_mint_with_context: data.compressed_mint_with_context,
        proof: data.proof,
        rent_payment: data.rent_payment,
        write_top_up: data.write_top_up,
    }
    .invoke_signed(&[signer])?;

    Ok(())
}
