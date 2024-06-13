use anchor_lang::solana_program::pubkey::Pubkey;
use light_system_program::{
    errors::SystemProgramError, invoke::processor::CompressedProof, sdk::CompressedCpiContext, InstructionDataInvokeCpi, NewAddressParamsPacked, OutputCompressedAccountWithPackedContext
};
use light_utils::hash_to_bn254_field_size_be;

pub fn get_registered_program_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[program_id.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0
}

pub fn get_cpi_authority_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"cpi_authority"], program_id).0
}

/// Helper function to create data for creating a single PDA.
pub fn create_cpi_inputs_for_new_address(
    proof: CompressedProof,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: OutputCompressedAccountWithPackedContext,
    seeds: &[&[u8]],
    cpi_context: Option<CompressedCpiContext>,
) -> InstructionDataInvokeCpi {
    InstructionDataInvokeCpi {
        proof: Some(proof),
        new_address_params: vec![new_address_params],
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: Vec::new(),
        output_compressed_accounts: vec![compressed_pda],
        compress_or_decompress_lamports: None,
        is_compress: false,
        signer_seeds: seeds.iter().map(|x| x.to_vec()).collect::<Vec<Vec<u8>>>(),
        cpi_context,
    }
}

// TODO: move to light-sdk as helper.
pub fn derive_program_derived_address_seeds(program_id: &Pubkey, seeds: &[&[u8]]) -> Result<[u8; 32], SystemProgramError> {
    let mut concatenated_seeds = program_id.to_bytes().to_vec();
    for seed in seeds {
        concatenated_seeds.extend_from_slice(seed);
    }

    let hash = match hash_to_bn254_field_size_be(concatenated_seeds.as_slice()) {
        Some(hash) => Ok::<[u8; 32], SystemProgramError>(hash.0),
        None => return Err(SystemProgramError::DeriveAddressError.into()),
    }?;

    Ok(hash)
}
