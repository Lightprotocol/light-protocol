use anchor_lang::solana_program::pubkey::Pubkey;

use crate::{
    address::NewAddressParamsPacked,
    compressed_account::{
        OutputCompressedAccountWithPackedContext, PackedCompressedAccountWithMerkleContext,
    },
    proof::CompressedProof,
    verify::{CompressedCpiContext, InstructionDataInvokeCpi},
    PROGRAM_ID_ACCOUNT_COMPRESSION,
};

pub fn get_registered_program_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[program_id.to_bytes().as_slice()],
        &PROGRAM_ID_ACCOUNT_COMPRESSION,
    )
    .0
}

pub fn get_cpi_authority_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"cpi_authority"], program_id).0
}

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

pub fn create_cpi_inputs_for_account_update(
    proof: CompressedProof,
    old_compressed_pda: PackedCompressedAccountWithMerkleContext,
    new_compressed_pda: OutputCompressedAccountWithPackedContext,
    cpi_context: Option<CompressedCpiContext>,
) -> InstructionDataInvokeCpi {
    InstructionDataInvokeCpi {
        proof: Some(proof),
        new_address_params: vec![],
        input_compressed_accounts_with_merkle_context: vec![old_compressed_pda],
        output_compressed_accounts: vec![new_compressed_pda],
        relay_fee: None,
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context,
    }
}

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
