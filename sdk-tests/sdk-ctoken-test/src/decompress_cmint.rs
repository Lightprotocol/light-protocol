use borsh::{BorshDeserialize, BorshSerialize};
use light_ctoken_sdk::{
    ctoken::{CompressedMintWithContext, DecompressCMintCpi, SystemAccountInfos},
    ValidityProof,
};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{mint_to_ctoken::MINT_AUTHORITY_SEED, ID};

/// Instruction data for DecompressCMint operations
#[derive(BorshSerialize, BorshDeserialize)]
pub struct DecompressCmintData {
    pub compressed_mint_with_context: CompressedMintWithContext,
    pub proof: ValidityProof,
    pub rent_payment: u8,
    pub write_top_up: u32,
}

/// Handler for decompressing CMint with PDA authority (invoke_signed)
///
/// Account order:
/// - accounts[0]: mint_seed (readonly)
/// - accounts[1]: authority (PDA, readonly - program signs)
/// - accounts[2]: payer (signer, writable)
/// - accounts[3]: cmint (writable)
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
pub fn process_decompress_cmint_invoke_signed(
    accounts: &[AccountInfo],
    data: DecompressCmintData,
) -> Result<(), ProgramError> {
    if accounts.len() < 15 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA for the mint authority
    let (pda, bump) = Pubkey::find_program_address(&[MINT_AUTHORITY_SEED], &ID);

    // Verify the authority account is the PDA we expect
    if &pda != accounts[1].key {
        return Err(ProgramError::InvalidSeeds);
    }

    let system_accounts = SystemAccountInfos {
        light_system_program: accounts[9].clone(),
        cpi_authority_pda: accounts[10].clone(),
        registered_program_pda: accounts[11].clone(),
        account_compression_authority: accounts[12].clone(),
        account_compression_program: accounts[13].clone(),
        system_program: accounts[14].clone(),
    };

    let signer_seeds: &[&[u8]] = &[MINT_AUTHORITY_SEED, &[bump]];
    DecompressCMintCpi {
        mint_seed: accounts[0].clone(),
        authority: accounts[1].clone(),
        payer: accounts[2].clone(),
        cmint: accounts[3].clone(),
        compressible_config: accounts[4].clone(),
        rent_sponsor: accounts[5].clone(),
        state_tree: accounts[6].clone(),
        input_queue: accounts[7].clone(),
        output_queue: accounts[8].clone(),
        system_accounts,
        compressed_mint_with_context: data.compressed_mint_with_context,
        proof: data.proof,
        rent_payment: data.rent_payment,
        write_top_up: data.write_top_up,
    }
    .invoke_signed(&[signer_seeds])?;

    Ok(())
}
