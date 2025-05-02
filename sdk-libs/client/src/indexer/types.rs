use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_indexed_merkle_tree::array::IndexedElement;
use num_bigint::BigUint;
use solana_pubkey::Pubkey;

use super::IndexerError;

pub struct ProofOfLeaf {
    pub leaf: [u8; 32],
    pub proof: Vec<[u8; 32]>,
}

pub type Address = [u8; 32];
pub type Hash = [u8; 32];

#[derive(Debug, Clone)]
pub struct MerkleProofWithContext {
    pub proof: Vec<[u8; 32]>,
    pub root: [u8; 32],
    pub leaf_index: u64,
    pub leaf: [u8; 32],
    pub merkle_tree: [u8; 32],
    pub root_seq: u64,
    pub tx_hash: Option<[u8; 32]>,
    pub account_hash: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub hash: String,
    pub leaf_index: u64,
    pub merkle_tree: String,
    pub proof: Vec<[u8; 32]>,
    pub root_seq: u64,
    pub root: [u8; 32],
}
#[derive(Debug, Clone, Copy)]
pub struct AddressWithTree {
    pub address: Address,
    pub tree: Pubkey,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct NewAddressProofWithContext {
    pub merkle_tree: Pubkey,
    pub root: [u8; 32],
    pub root_seq: u64,
    pub low_address_index: u64,
    pub low_address_value: [u8; 32],
    pub low_address_next_index: u64,
    pub low_address_next_value: [u8; 32],
    pub low_address_proof: [[u8; 32]; 16],
    pub new_low_element: Option<IndexedElement<usize>>,
    pub new_element: Option<IndexedElement<usize>>,
    pub new_element_next_value: Option<BigUint>,
}

#[derive(Debug, Clone, Default)]
pub struct ProofRpcResult {
    pub proof: CompressedProof,
    pub root_indices: Vec<u16>,
    pub address_root_indices: Vec<u16>,
}

impl ProofRpcResult {
    pub fn from_api_model(
        value: photon_api::models::CompressedProofWithContext,
        num_hashes: usize,
    ) -> Result<Self, IndexerError> {
        let proof = CompressedProof {
            a: value
                .compressed_proof
                .a
                .try_into()
                .map_err(|_| IndexerError::InvalidResponseData)?,
            b: value
                .compressed_proof
                .b
                .try_into()
                .map_err(|_| IndexerError::InvalidResponseData)?,
            c: value
                .compressed_proof
                .c
                .try_into()
                .map_err(|_| IndexerError::InvalidResponseData)?,
        };

        Ok(Self {
            proof,
            root_indices: value.root_indices[..num_hashes]
                .iter()
                .map(|x| {
                    (*x).try_into()
                        .map_err(|_| IndexerError::InvalidResponseData)
                })
                .collect::<Result<Vec<u16>, _>>()?,
            address_root_indices: value.root_indices[num_hashes..]
                .iter()
                .map(|x| {
                    (*x).try_into()
                        .map_err(|_| IndexerError::InvalidResponseData)
                })
                .collect::<Result<Vec<u16>, _>>()?,
        })
    }
}

#[cfg(feature = "v2")]
#[derive(Debug, Default, Clone)]
pub struct ProofRpcResultV2 {
    pub proof: Option<CompressedProof>,
    // If none -> proof by index  and not included in zkp, else included in zkp
    pub root_indices: Vec<Option<u16>>,
    pub address_root_indices: Vec<u16>,
}

#[cfg(feature = "v2")]
impl ProofRpcResultV2 {
    pub fn from_api_model(
        value: photon_api::models::CompressedProofWithContextV2,
        num_hashes: usize,
    ) -> Result<Self, IndexerError> {
        let proof = if let Some(proof) = value.compressed_proof {
            let proof = CompressedProof {
                a: proof
                    .a
                    .try_into()
                    .map_err(|_| IndexerError::InvalidResponseData)?,
                b: proof
                    .b
                    .try_into()
                    .map_err(|_| IndexerError::InvalidResponseData)?,
                c: proof
                    .c
                    .try_into()
                    .map_err(|_| IndexerError::InvalidResponseData)?,
            };
            Some(proof)
        } else {
            None
        };

        Ok(Self {
            proof,
            root_indices: value.root_indices[..num_hashes]
                .iter()
                .map(|x| {
                    if x.prove_by_index {
                        None
                    } else {
                        Some(x.root_index)
                    }
                })
                .collect::<Vec<Option<u16>>>(),
            address_root_indices: value.root_indices[num_hashes..]
                .iter()
                .map(|x| x.root_index)
                .collect::<Vec<u16>>(),
        })
    }
}
