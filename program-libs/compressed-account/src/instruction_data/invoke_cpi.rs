use light_zero_copy::ZeroCopyMut;

use super::{
    cpi_context::CompressedCpiContext,
    data::{NewAddressParamsPacked, OutputCompressedAccountWithPackedContext},
};
use crate::{
    compressed_account::PackedCompressedAccountWithMerkleContext,
    instruction_data::compressed_proof::CompressedProof, AnchorDeserialize, AnchorSerialize,
};

#[repr(C)]
#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize, ZeroCopyMut)]
pub struct InstructionDataInvokeCpi {
    pub proof: Option<CompressedProof>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<u64>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
    pub cpi_context: Option<CompressedCpiContext>,
}
