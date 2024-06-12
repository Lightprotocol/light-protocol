use anchor_lang::solana_program::pubkey::Pubkey;
use light_system_program::{
    invoke::processor::CompressedProof, sdk::CompressedCpiContext, InstructionDataInvokeCpi,
    NewAddressParamsPacked, OutputCompressedAccountWithPackedContext,
};

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
