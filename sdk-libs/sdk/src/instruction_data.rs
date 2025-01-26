use std::io::{self, Cursor};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::{account_meta::PackedLightAccountMeta, proof::ProofRpcResult};

pub struct LightInstructionData {
    /// Optional validity proof for the instruction.
    pub proof: Option<ProofRpcResult>,
    /// Optional vector of compressed account metas passed as instruction data.
    pub accounts: Option<Vec<PackedLightAccountMeta>>,
}
impl LightInstructionData {
    pub fn deserialize(bytes: &[u8]) -> Result<Self, io::Error> {
        let mut inputs = Cursor::new(bytes);

        let proof = Option::<ProofRpcResult>::deserialize_reader(&mut inputs)?;
        let accounts = Option::<Vec<PackedLightAccountMeta>>::deserialize_reader(&mut inputs)?;

        Ok(LightInstructionData { proof, accounts })
    }

    pub fn serialize(&self) -> Result<Vec<u8>, io::Error> {
        let mut bytes = Vec::new();
        self.proof.serialize(&mut bytes)?;
        self.accounts.serialize(&mut bytes)?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use solana_sdk::pubkey::Pubkey;

    use super::*;
    use crate::{account_meta::PackedLightAccountMeta, proof::CompressedProof};

    #[test]
    fn test_serialize_deserialize() {
        let proof = Some(ProofRpcResult {
            proof: CompressedProof::default(),
            root_indices: vec![Some(1), None],
            address_root_indices: vec![2, 3],
        });

        let accounts = Some(vec![
            PackedLightAccountMeta {
                lamports: Some(0),
                address: Some(Pubkey::new_unique().to_bytes()),
                data: Some(vec![]),
                merkle_context: None,
                merkle_tree_root_index: None,
                output_merkle_tree_index: None,
                address_merkle_context: None,
                address_merkle_tree_root_index: None,
                read_only: false,
            },
            PackedLightAccountMeta {
                lamports: Some(0),
                address: Some(Pubkey::new_unique().to_bytes()),
                data: Some(vec![]),
                merkle_context: None,
                merkle_tree_root_index: None,
                output_merkle_tree_index: None,
                address_merkle_context: None,
                address_merkle_tree_root_index: None,
                read_only: false,
            },
        ]);

        let instruction_data = LightInstructionData { proof, accounts };

        let serialized = instruction_data.serialize().unwrap();
        let deserialized = LightInstructionData::deserialize(&serialized).unwrap();

        assert_eq!(instruction_data.proof, deserialized.proof);
        assert_eq!(instruction_data.accounts, deserialized.accounts);
    }
}
