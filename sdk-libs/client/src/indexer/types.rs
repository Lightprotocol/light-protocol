use light_compressed_account::{
    compressed_account::{CompressedAccountData, CompressedAccountWithMerkleContext},
    TreeType,
};
use light_indexed_merkle_tree::array::IndexedElement;
use light_sdk::verifier::CompressedProof;
use num_bigint::BigUint;
use solana_pubkey::Pubkey;

use super::{base58::decode_base58_to_fixed_array, tree_info::QUEUE_TREE_MAPPING, IndexerError};

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

#[derive(Debug, Default, Clone)]
pub struct ProofRpcResultV2 {
    pub proof: Option<CompressedProof>,
    // If none -> proof by index and not included in zkp, else included in zkp
    pub root_indices: Vec<Option<u16>>,
    pub address_root_indices: Vec<u16>,
}

impl ProofRpcResultV2 {
    pub fn from_api_model(
        value: photon_api::models::CompressedProofWithContextV2,
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
            root_indices: value
                .accounts
                .iter()
                .map(|x| {
                    if x.root_index.prove_by_index {
                        None
                    } else {
                        Some(x.root_index.root_index as u16)
                    }
                })
                .collect::<Vec<Option<u16>>>(),
            address_root_indices: value
                .addresses
                .iter()
                .map(|x| x.root_index)
                .collect::<Vec<u16>>(),
        })
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct TreeContextInfo {
    pub cpi_context: Option<Pubkey>,
    pub queue: Pubkey,
    pub tree: Pubkey,
    pub tree_type: u16,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct MerkleContext {
    pub cpi_context: Option<Pubkey>,
    pub next_tree_context: Option<TreeContextInfo>,
    pub queue: Pubkey,
    pub tree: Pubkey,
    pub tree_type: TreeType,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Account {
    pub address: Option<[u8; 32]>,
    pub data: Option<CompressedAccountData>,
    pub hash: [u8; 32],
    pub lamports: u64,
    pub leaf_index: u32,
    pub merkle_context: MerkleContext,
    pub owner: Pubkey,
    pub prove_by_index: bool,
    pub seq: Option<u64>,
    pub slot_created: u64,
}

impl TryFrom<CompressedAccountWithMerkleContext> for Account {
    type Error = IndexerError;

    fn try_from(account: CompressedAccountWithMerkleContext) -> Result<Self, Self::Error> {
        let hash = account
            .hash()
            .map_err(|_| IndexerError::InvalidResponseData)?;

        Ok(Account {
            address: account.compressed_account.address,
            data: account.compressed_account.data,
            hash,
            lamports: account.compressed_account.lamports,
            leaf_index: account.merkle_context.leaf_index,
            merkle_context: MerkleContext {
                tree: account.merkle_context.merkle_tree_pubkey,
                queue: account.merkle_context.queue_pubkey,
                tree_type: account.merkle_context.tree_type,
                cpi_context: None,
                next_tree_context: None,
            },
            owner: account.compressed_account.owner,
            prove_by_index: account.merkle_context.prove_by_index,
            seq: None,
            slot_created: u64::MAX,
        })
    }
}

impl TryFrom<&photon_api::models::Account> for Account {
    type Error = IndexerError;

    fn try_from(account: &photon_api::models::Account) -> Result<Self, Self::Error> {
        let data = if let Some(data) = &account.data {
            Ok::<Option<CompressedAccountData>, IndexerError>(Some(CompressedAccountData {
                discriminator: data.discriminator.to_le_bytes(),
                data: base64::decode_config(&data.data, base64::STANDARD_NO_PAD)
                    .map_err(|_| IndexerError::InvalidResponseData)?,
                data_hash: decode_base58_to_fixed_array(&data.data_hash)?,
            }))
        } else {
            Ok::<Option<CompressedAccountData>, IndexerError>(None)
        }?;
        let owner = Pubkey::new_from_array(decode_base58_to_fixed_array(&account.owner)?);
        let address = account
            .address
            .as_ref()
            .map(|address| decode_base58_to_fixed_array(address))
            .transpose()?;
        let hash = decode_base58_to_fixed_array(&account.hash)?;
        let seq = account.seq;
        let slot_created = account.slot_created;
        let lamports = account.lamports;
        let leaf_index = account.leaf_index;

        let tree_info = QUEUE_TREE_MAPPING
            .get(&account.tree)
            .ok_or(IndexerError::InvalidResponseData)?;

        let merkle_context = MerkleContext {
            cpi_context: None,
            queue: tree_info.tree,
            tree_type: tree_info.tree_type,
            next_tree_context: None,
            tree: tree_info.tree,
        };

        Ok(Account {
            owner,
            address,
            data,
            hash,
            lamports,
            leaf_index,
            seq,
            slot_created,
            merkle_context,
            prove_by_index: false,
        })
    }
}
