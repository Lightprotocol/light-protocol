use light_compressed_account::{
    compressed_account::{CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext},
    TreeType,
};
use light_indexed_merkle_tree::array::IndexedElement;
use light_sdk::{verifier::CompressedProof, ValidityProof};
use num_bigint::BigUint;
use solana_pubkey::Pubkey;

use super::{base58::{decode_base58_to_fixed_array, decode_base58_option_to_pubkey}, tree_info::QUEUE_TREE_MAPPING, IndexerError};

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
    pub hash: [u8; 32],
    pub leaf_index: u64,
    pub merkle_tree: Pubkey,
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
    pub low_address_proof: Vec<[u8; 32]>,
    pub new_low_element: Option<IndexedElement<usize>>,
    pub new_element: Option<IndexedElement<usize>>,
    pub new_element_next_value: Option<BigUint>,
}


#[derive(Debug, Default, Clone)]
pub struct ProofRpcResult {
    pub compressed_proof: ValidityProof,
    pub accounts: Vec<AccountProofInputs>,
    pub addresses: Vec<AddressProofInputs>,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct AccountProofInputs {
    pub hash: [u8; 32],
    pub root: [u8; 32],
    pub root_index: Option<u16>,
    pub leaf_index: u64,
    pub merkle_context: MerkleContext,
}

impl AccountProofInputs {
    pub fn from_api_model(
        value: &photon_api::models::compressed_proof_with_context_v2::AccountProofInputs,
    ) -> Result<Self, IndexerError> {
        let root_index = {
            if value.root_index.prove_by_index {
                None
            } else {
                Some(
                    value
                        .root_index
                        .root_index
                        .try_into()
                        .map_err(|_| IndexerError::InvalidResponseData)?,
                )
            }
        };
        Ok(Self {
            hash: decode_base58_to_fixed_array(&value.hash)?,
            root: decode_base58_to_fixed_array(&value.root)?,
            root_index,
            leaf_index: value.leaf_index,
            merkle_context: MerkleContext::from_api_model(&value.merkle_context)?,
        })
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct AddressProofInputs {
    pub address: [u8; 32],
    pub root: [u8; 32],
    pub root_index: u16,
    pub merkle_context: MerkleContext,
}

impl AddressProofInputs {
    pub fn from_api_model(
        value: &photon_api::models::compressed_proof_with_context_v2::AddressProofInputs,
    ) -> Result<Self, IndexerError> {
        Ok(Self {
            address: decode_base58_to_fixed_array(&value.address)?,
            root: decode_base58_to_fixed_array(&value.root)?,
            root_index: value.root_index,
            merkle_context: MerkleContext::from_api_model(&value.merkle_context)?,
        })
    }
}

impl ProofRpcResult {
    pub fn from_api_model(
        value: photon_api::models::CompressedProofWithContext,
        num_hashes: usize,
    ) -> Result<Self, IndexerError> {
        let compressed_proof = ValidityProof::new(Some(CompressedProof {
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
        }));

        // Convert account data from V1 flat arrays to V2 structured format
        let accounts = (0..num_hashes)
            .map(|i| {
                let tree_pubkey = Pubkey::new_from_array(decode_base58_to_fixed_array(&value.merkle_trees[i])?);
                let tree_info = super::tree_info::QUEUE_TREE_MAPPING
                    .get(&value.merkle_trees[i])
                    .ok_or(IndexerError::InvalidResponseData)?;
                
                Ok(AccountProofInputs {
                    hash: decode_base58_to_fixed_array(&value.leaves[i])?,
                    root: decode_base58_to_fixed_array(&value.roots[i])?,
                    root_index: Some(value.root_indices[i] as u16),
                    leaf_index: value.leaf_indices[i] as u64,
                    merkle_context: MerkleContext {
                        tree_type: tree_info.tree_type,
                        tree: tree_pubkey,
                        queue: tree_info.queue,
                        cpi_context: None,
                        next_tree_context: None,
                    },
                })
            })
            .collect::<Result<Vec<_>, IndexerError>>()?;

        // Convert address data from remaining indices (if any)
        let addresses = if value.root_indices.len() > num_hashes {
            (num_hashes..value.root_indices.len())
                .map(|i| {
                    let tree_pubkey = Pubkey::new_from_array(decode_base58_to_fixed_array(&value.merkle_trees[i])?);
                    let tree_info = super::tree_info::QUEUE_TREE_MAPPING
                        .get(&value.merkle_trees[i])
                        .ok_or(IndexerError::InvalidResponseData)?;
                    
                    Ok(AddressProofInputs {
                        address: decode_base58_to_fixed_array(&value.leaves[i])?, // Address is in leaves
                        root: decode_base58_to_fixed_array(&value.roots[i])?,
                        root_index: value.root_indices[i] as u16,
                        merkle_context: MerkleContext {
                            tree_type: tree_info.tree_type,
                            tree: tree_pubkey,
                            queue: tree_info.queue,
                            cpi_context: None,
                            next_tree_context: None,
                        },
                    })
                })
                .collect::<Result<Vec<_>, IndexerError>>()?
        } else {
            Vec::new()
        };

        Ok(Self {
            compressed_proof,
            accounts,
            addresses,
        })
    }

    pub fn from_api_model_v2(
        value: photon_api::models::CompressedProofWithContextV2,
    ) -> Result<Self, IndexerError> {
        let compressed_proof = if let Some(proof) = value.compressed_proof {
            ValidityProof::new(Some(CompressedProof {
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
            }))
        } else {
            ValidityProof::new(None)
        };

        let accounts = value
            .accounts
            .iter()
            .map(|account| AccountProofInputs::from_api_model(account))
            .collect::<Result<Vec<_>, IndexerError>>()?;

        let addresses = value
            .addresses
            .iter()
            .map(|address| AddressProofInputs::from_api_model(address))
            .collect::<Result<Vec<_>, IndexerError>>()?;

        Ok(Self {
            compressed_proof,
            accounts,
            addresses,
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

impl TreeContextInfo {
    pub fn from_api_model(
        value: &photon_api::models::compressed_proof_with_context_v2::TreeContextInfo,
    ) -> Result<Self, IndexerError> {
        Ok(Self {
            tree_type: value.tree_type,
            tree: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.tree)?),
            queue: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.queue)?),
            cpi_context: decode_base58_option_to_pubkey(&value.cpi_context)?,
        })
    }
}

impl TryFrom<&photon_api::models::TreeContextInfo> for TreeContextInfo {
    type Error = IndexerError;

    fn try_from(value: &photon_api::models::TreeContextInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            tree_type: value.tree_type,
            tree: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.tree)?),
            queue: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.queue)?),
            cpi_context: decode_base58_option_to_pubkey(&value.cpi_context)?,
        })
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct MerkleContext {
    pub cpi_context: Option<Pubkey>,
    pub next_tree_context: Option<TreeContextInfo>,
    pub queue: Pubkey,
    pub tree: Pubkey,
    pub tree_type: TreeType,
}

impl MerkleContext {
    pub fn from_api_model(
        value: &photon_api::models::compressed_proof_with_context_v2::MerkleContextV2,
    ) -> Result<Self, IndexerError> {
        Ok(Self {
            tree_type: TreeType::from(value.tree_type as u64),
            tree: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.tree)?),
            queue: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.queue)?),
            cpi_context: decode_base58_option_to_pubkey(&value.cpi_context)?,
            next_tree_context: value
                .next_tree_context
                .as_ref()
                .map(|ctx| TreeContextInfo::from_api_model(ctx))
                .transpose()?,
        })
    }

    pub fn to_light_merkle_context(&self, leaf_index: u32, prove_by_index: bool) -> light_compressed_account::compressed_account::MerkleContext {
        light_compressed_account::compressed_account::MerkleContext {
            merkle_tree_pubkey: self.tree,
            queue_pubkey: self.queue,
            leaf_index,
            tree_type: self.tree_type,
            prove_by_index,
        }
    }
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

impl From<Account> for CompressedAccountWithMerkleContext {
    fn from(account: Account) -> Self {
        let compressed_account = CompressedAccount {
            owner: account.owner,
            lamports: account.lamports,
            address: account.address,
            data: account.data,
        };
        
        let merkle_context = account.merkle_context.to_light_merkle_context(
            account.leaf_index,
            account.prove_by_index,
        );

        CompressedAccountWithMerkleContext {
            compressed_account,
            merkle_context,
        }
    }
}

impl TryFrom<&photon_api::models::AccountV2> for Account {
    type Error = IndexerError;

    fn try_from(account: &photon_api::models::AccountV2) -> Result<Self, Self::Error> {
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

        let merkle_context = MerkleContext {
            tree: Pubkey::new_from_array(decode_base58_to_fixed_array(&account.merkle_context.tree)?),
            queue: Pubkey::new_from_array(decode_base58_to_fixed_array(&account.merkle_context.queue)?),
            tree_type: TreeType::from(account.merkle_context.tree_type as u64),
            cpi_context: decode_base58_option_to_pubkey(&account.merkle_context.cpi_context)?,
            next_tree_context: account.merkle_context.next_tree_context
                .as_ref()
                .map(|ctx| TreeContextInfo::try_from(ctx.as_ref()))
                .transpose()?,
        };

        Ok(Account {
            owner,
            address,
            data,
            hash,
            lamports: account.lamports,
            leaf_index: account.leaf_index,
            seq: account.seq,
            slot_created: account.slot_created,
            merkle_context,
            prove_by_index: account.prove_by_index,
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

#[derive(Debug, Clone)]
pub struct AddressQueueIndex {
    pub address: [u8; 32],
    pub queue_index: u64,
}

#[derive(Debug, Clone)]
pub struct BatchAddressUpdateIndexerResponse {
    pub batch_start_index: u64,
    pub addresses: Vec<AddressQueueIndex>,
    pub non_inclusion_proofs: Vec<NewAddressProofWithContext>,
    pub subtrees: Vec<[u8; 32]>,
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct StateMerkleTreeAccounts {
    pub merkle_tree: Pubkey,
    pub nullifier_queue: Pubkey,
    pub cpi_context: Pubkey,
}

#[derive(Debug, Clone, Copy)]
pub struct AddressMerkleTreeAccounts {
    pub merkle_tree: Pubkey,
    pub queue: Pubkey,
}
