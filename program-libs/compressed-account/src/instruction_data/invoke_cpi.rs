#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};

use super::{
    cpi_context::CompressedCpiContext,
    data::{
        NewAddressParamsPacked, OutputCompressedAccountWithPackedContext, PackedReadOnlyAddress,
    },
    zero_copy::ZInstructionDataInvokeCpi,
};
use crate::{
    compressed_account::{
        PackedCompressedAccountWithMerkleContext, PackedReadOnlyCompressedAccount,
    },
    instruction_data::compressed_proof::CompressedProof,
};

#[repr(C)]
#[derive(Debug, PartialEq, Default, Clone, AnchorDeserialize, AnchorSerialize)]
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

impl<'a, 'info: 'a> ZInstructionDataInvokeCpi<'a> {
    pub fn combine(&mut self, other: Vec<ZInstructionDataInvokeCpi<'info>>) {
        for other in other {
            // TODO: support address creation with cpi context
            // issue is that we deserialize address creation params as zero copy slice we cannot push into it
            // could reenable by passing it as extra argument without losing performance.
            // self.new_address_params
            //     .extend_from_slice(&other.new_address_params);
            for i in other.input_compressed_accounts_with_merkle_context.iter() {
                self.input_compressed_accounts_with_merkle_context
                    .push((*i).clone());
            }
            for i in other.output_compressed_accounts.iter() {
                self.output_compressed_accounts.push((*i).clone());
            }
        }
    }
}

#[derive(Debug, PartialEq, Default, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InstructionDataInvokeCpiWithReadOnly {
    pub invoke_cpi: InstructionDataInvokeCpi,
    pub read_only_addresses: Option<Vec<PackedReadOnlyAddress>>,
    pub read_only_accounts: Option<Vec<PackedReadOnlyCompressedAccount>>,
}

#[cfg(test)]
mod tests {
    use std::vec;

    use light_zero_copy::borsh::Deserialize;

    use super::*;
    use crate::{
        compressed_account::PackedCompressedAccountWithMerkleContext,
        instruction_data::{
            data::{NewAddressParamsPacked, OutputCompressedAccountWithPackedContext},
            invoke_cpi::InstructionDataInvokeCpi,
            zero_copy::ZInstructionDataInvokeCpi,
        },
    };

    // test combine instruction data transfer
    #[test]
    fn test_combine_instruction_data_transfer() {
        let mut instruction_data_transfer = InstructionDataInvokeCpi {
            proof: Some(CompressedProof {
                a: [0; 32],
                b: [0; 64],
                c: [0; 32],
            }),
            new_address_params: vec![NewAddressParamsPacked::default()],
            input_compressed_accounts_with_merkle_context: vec![
                PackedCompressedAccountWithMerkleContext::default(),
            ],
            output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext::default()],
            relay_fee: None,
            compress_or_decompress_lamports: Some(1),
            is_compress: true,
            cpi_context: None,
        };
        instruction_data_transfer.input_compressed_accounts_with_merkle_context[0]
            .merkle_context
            .leaf_index = 1;
        instruction_data_transfer.output_compressed_accounts[0].merkle_tree_index = 1;
        let other = InstructionDataInvokeCpi {
            proof: Some(CompressedProof {
                a: [0; 32],
                b: [0; 64],
                c: [0; 32],
            }),
            input_compressed_accounts_with_merkle_context: vec![
                PackedCompressedAccountWithMerkleContext::default(),
            ],
            output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext::default()],
            relay_fee: None,
            compress_or_decompress_lamports: Some(1),
            is_compress: true,
            new_address_params: vec![NewAddressParamsPacked::default()],
            cpi_context: None,
        };
        let mut vec = Vec::new();
        instruction_data_transfer.serialize(&mut vec).unwrap();
        let mut other_vec = Vec::new();
        other.serialize(&mut other_vec).unwrap();
        let (mut instruction_data_transfer, _) =
            ZInstructionDataInvokeCpi::zero_copy_at(&vec).unwrap();
        let (other, _) = ZInstructionDataInvokeCpi::zero_copy_at(&other_vec).unwrap();
        instruction_data_transfer.combine(vec![other]);
        assert_eq!(instruction_data_transfer.new_address_params.len(), 1);
        assert_eq!(
            instruction_data_transfer
                .input_compressed_accounts_with_merkle_context
                .len(),
            2
        );
        assert_eq!(
            instruction_data_transfer.output_compressed_accounts.len(),
            2
        );
    }
}
