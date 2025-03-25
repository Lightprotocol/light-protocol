use std::io::Cursor;

use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_zero_copy::{ZeroCopy, ZeroCopyEq};

use crate::{error::LightSdkError, merkle_context::PackedAddressMerkleContext};

#[derive(Debug, PartialEq, Clone, ZeroCopy, BorshDeserialize, BorshSerialize)]
pub struct LightInstructionData {
    pub proof: Option<CompressedProof>,
    // // TODO: remove accounts field
    // pub accounts: Option<Vec<LightAccountMeta>>,
    // TODO: refactor addresses in separate pr
    pub new_addresses: Option<Vec<PackedAddressMerkleContext>>,
}

// impl LightInstructionData {
//     pub fn deserialize(bytes: &[u8]) -> Result<(&[u8], Self)> {
//         let mut inputs = Cursor::new(bytes);

//         let proof = Option::<CompressedProof>::deserialize_reader(&mut inputs)
//             .map_err(|_| LightSdkError::Borsh)?;
//         // let accounts = Option::<Vec<LightAccountMeta>>::deserialize_reader(&mut inputs)
//         //     .map_err(|_| LightSdkError::Borsh)?;
//         let new_addresses =
//             Option::<Vec<PackedAddressMerkleContext>>::deserialize_reader(&mut inputs)
//                 .map_err(|_| LightSdkError::Borsh)?;
//         let (_, remaining_bytes) = bytes.split_at(inputs.position() as usize);
//         Ok((
//             remaining_bytes,
//             LightInstructionData {
//                 proof,
//                 // accounts,
//                 new_addresses,
//             },
//         ))
//     }

//     pub fn serialize(&self) -> Result<Vec<u8>> {
//         let mut bytes = Vec::new();
//         self.proof
//             .serialize(&mut bytes)
//             .map_err(|_| LightSdkError::Borsh)?;
//         // self.accounts
//         //     .serialize(&mut bytes)
//         //     .map_err(|_| LightSdkError::Borsh)?;
//         self.new_addresses
//             .serialize(&mut bytes)
//             .map_err(|_| LightSdkError::Borsh)?;
//         Ok(bytes)
//     }
// }
