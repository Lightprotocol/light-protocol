#![cfg(feature = "legacy")]

//! Legacy types re-imported from programs which should be removed as soon as
//! possible.

use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof,
    cpi_context::CompressedCpiContext,
    data::{NewAddressParamsPacked, OutputCompressedAccountWithPackedContext},
    invoke_cpi::InstructionDataInvokeCpi,
};
/// Helper function to create data for creating a single PDA.
pub fn create_cpi_inputs_for_new_account(
    proof: CompressedProof,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: OutputCompressedAccountWithPackedContext,
    cpi_context: Option<CompressedCpiContext>,
) -> InstructionDataInvokeCpi {
    InstructionDataInvokeCpi {
        proof: Some(proof),
        new_address_params: vec![new_address_params],
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: vec![],
        output_compressed_accounts: vec![compressed_pda],
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context,
    }
}
