use anchor_lang::solana_program::pubkey::Pubkey;

use crate::{
    address::PackedNewAddressParams,
    compressed_account::{
        OutputCompressedAccountWithPackedContext, PackedCompressedAccountWithMerkleContext,
    },
    proof::CompressedProof,
    verify::{CompressedCpiContext, InstructionDataInvokeCpi},
    PROGRAM_ID_ACCOUNT_COMPRESSION,
};

/// Get the PDA for a given program that is registered in the Light Protocol's
/// Account Compression Program. Examples include the Light System Program.
pub fn get_registered_program_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[program_id.to_bytes().as_slice()],
        &PROGRAM_ID_ACCOUNT_COMPRESSION,
    )
}

/// Get the PDA and derivation bump for a given program's CPI authority.
/// The Program signs a CPI with this PDA.
pub fn get_cpi_authority_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"cpi_authority"], program_id)
}

/// Create CPI inputs for a new compressed PDA account.
pub fn create_cpi_inputs_for_new_account(
    proof: CompressedProof,
    new_address_params: PackedNewAddressParams,
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
/// Constructs CPI inputs for updating a compressed PDA account.
///
/// # Arguments
/// * `proof` - ValidityProof for the old_compressed_pda state.
/// * `old_compressed_pda` - Existing compressed PDA account with Merkle context.
/// * `new_compressed_pda` - New compressed PDA account to be updated.
/// * `cpi_context` - Optional context for the CPI operation.
///
/// # Returns
/// An `InstructionDataInvokeCpi` struct containing the necessary inputs for the update.
pub fn create_cpi_inputs_for_account_update(
    proof: CompressedProof,
    old_compressed_pda: PackedCompressedAccountWithMerkleContext,
    new_compressed_pda: OutputCompressedAccountWithPackedContext,
    cpi_context: Option<CompressedCpiContext>,
) -> InstructionDataInvokeCpi {
    InstructionDataInvokeCpi {
        proof: Some(proof),
        new_address_params: Vec::new(),
        input_compressed_accounts_with_merkle_context: vec![old_compressed_pda],
        output_compressed_accounts: vec![new_compressed_pda],
        relay_fee: None,
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context,
    }
}

/// Create CPI inputs for deleting a compressed PDA account.
pub fn create_cpi_inputs_for_account_deletion(
    proof: CompressedProof,
    compressed_pda: PackedCompressedAccountWithMerkleContext,
    cpi_context: Option<CompressedCpiContext>,
) -> InstructionDataInvokeCpi {
    InstructionDataInvokeCpi {
        proof: Some(proof),
        new_address_params: vec![],
        input_compressed_accounts_with_merkle_context: vec![compressed_pda],
        output_compressed_accounts: vec![],
        relay_fee: None,
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context,
    }
}
