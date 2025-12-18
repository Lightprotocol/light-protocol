use borsh::BorshDeserialize;
use light_compressed_account::{
    compressed_account::{
        CompressedAccount as ProgramCompressedAccount, CompressedAccountData,
        CompressedAccountWithMerkleContext,
    },
    instruction_data::compressed_proof::CompressedProof,
    TreeType,
};
use light_ctoken_interface::state::ExtensionStruct;
use light_ctoken_sdk::compat::{AccountState, TokenData};
use light_indexed_merkle_tree::array::IndexedElement;
use light_sdk::instruction::{
    PackedAccounts, PackedAddressTreeInfo, PackedStateTreeInfo, ValidityProof,
};
use num_bigint::BigUint;
use solana_pubkey::Pubkey;
use tracing::warn;

use super::{
    base58::{decode_base58_option_to_pubkey, decode_base58_to_fixed_array},
    tree_info::QUEUE_TREE_MAPPING,
    IndexerError,
};

pub struct ProofOfLeaf {
    pub leaf: [u8; 32],
    pub proof: Vec<[u8; 32]>,
}

pub type Address = [u8; 32];
pub type Hash = [u8; 32];

#[derive(Debug, Clone, PartialEq)]
pub struct QueueInfo {
    pub tree: Pubkey,
    pub queue: Pubkey,
    pub queue_type: u8,
    pub queue_size: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct QueueInfoResult {
    pub queues: Vec<QueueInfo>,
    pub slot: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OutputQueueData {
    pub leaf_indices: Vec<u64>,
    pub account_hashes: Vec<[u8; 32]>,
    pub old_leaves: Vec<[u8; 32]>,
    pub first_queue_index: u64,
    /// The tree's next_index - where new leaves will be appended
    pub next_index: u64,
    /// Pre-computed hash chains per ZKP batch (from on-chain)
    pub leaves_hash_chains: Vec<[u8; 32]>,
}

/// V2 Input Queue Data
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InputQueueData {
    pub leaf_indices: Vec<u64>,
    pub account_hashes: Vec<[u8; 32]>,
    pub current_leaves: Vec<[u8; 32]>,
    pub tx_hashes: Vec<[u8; 32]>,
    /// Pre-computed nullifiers from indexer
    pub nullifiers: Vec<[u8; 32]>,
    pub first_queue_index: u64,
    /// Pre-computed hash chains per ZKP batch (from on-chain)
    pub leaves_hash_chains: Vec<[u8; 32]>,
}

/// State queue data with shared tree nodes for output and input queues
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StateQueueData {
    /// Shared deduplicated tree nodes for state queues (output + input)
    /// node_index encoding: (level << 56) | position
    pub nodes: Vec<u64>,
    pub node_hashes: Vec<[u8; 32]>,
    /// Initial root for the state tree (shared by output and input queues)
    pub initial_root: [u8; 32],
    /// Sequence number of the root
    pub root_seq: u64,
    /// Output queue data (if requested)
    pub output_queue: Option<OutputQueueData>,
    /// Input queue data (if requested)
    pub input_queue: Option<InputQueueData>,
}

/// V2 Address Queue Data with deduplicated nodes
/// Proofs are reconstructed from `nodes`/`node_hashes` using `low_element_indices`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AddressQueueData {
    pub addresses: Vec<[u8; 32]>,
    pub low_element_values: Vec<[u8; 32]>,
    pub low_element_next_values: Vec<[u8; 32]>,
    pub low_element_indices: Vec<u64>,
    pub low_element_next_indices: Vec<u64>,
    /// Deduplicated node indices - encoding: (level << 56) | position
    pub nodes: Vec<u64>,
    /// Hashes corresponding to each node index
    pub node_hashes: Vec<[u8; 32]>,
    pub initial_root: [u8; 32],
    pub leaves_hash_chains: Vec<[u8; 32]>,
    pub subtrees: Vec<[u8; 32]>,
    pub start_index: u64,
    pub root_seq: u64,
}

impl AddressQueueData {
    /// Reconstruct a merkle proof for a given low_element_index from the deduplicated nodes.
    /// The tree_height is needed to know how many levels to traverse.
    pub fn reconstruct_proof(
        &self,
        address_idx: usize,
        tree_height: u8,
    ) -> Result<Vec<[u8; 32]>, IndexerError> {
        let leaf_index = self.low_element_indices[address_idx];
        let mut proof = Vec::with_capacity(tree_height as usize);
        let mut pos = leaf_index;

        for level in 0..tree_height {
            let sibling_pos = if pos.is_multiple_of(2) {
                pos + 1
            } else {
                pos - 1
            };
            let sibling_idx = Self::encode_node_index(level, sibling_pos);

            if let Some(hash_idx) = self.nodes.iter().position(|&n| n == sibling_idx) {
                proof.push(self.node_hashes[hash_idx]);
            } else {
                return Err(IndexerError::MissingResult {
                    context: "reconstruct_proof".to_string(),
                    message: format!(
                        "Missing proof node at level {} position {} (encoded: {})",
                        level, sibling_pos, sibling_idx
                    ),
                });
            }
            pos /= 2;
        }

        Ok(proof)
    }

    /// Reconstruct all proofs for all addresses
    pub fn reconstruct_all_proofs(
        &self,
        tree_height: u8,
    ) -> Result<Vec<Vec<[u8; 32]>>, IndexerError> {
        (0..self.addresses.len())
            .map(|i| self.reconstruct_proof(i, tree_height))
            .collect()
    }

    /// Encode node index: (level << 56) | position
    #[inline]
    fn encode_node_index(level: u8, position: u64) -> u64 {
        ((level as u64) << 56) | position
    }
}

/// V2 Queue Elements Result with deduplicated node data
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QueueElementsResult {
    pub state_queue: Option<StateQueueData>,
    pub address_queue: Option<AddressQueueData>,
}

#[derive(Debug, Clone, PartialEq, Default)]
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MerkleProof {
    pub hash: [u8; 32],
    pub leaf_index: u64,
    pub merkle_tree: Pubkey,
    pub proof: Vec<[u8; 32]>,
    pub root_seq: u64,
    pub root: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ValidityProofWithContext {
    pub proof: ValidityProof,
    pub accounts: Vec<AccountProofInputs>,
    pub addresses: Vec<AddressProofInputs>,
}

// TODO: add get_public_inputs
// -> to make it easier to use light-verifier with get_validity_proof()
impl ValidityProofWithContext {
    pub fn get_root_indices(&self) -> Vec<Option<u16>> {
        self.accounts
            .iter()
            .map(|account| account.root_index.root_index())
            .collect()
    }

    pub fn get_address_root_indices(&self) -> Vec<u16> {
        self.addresses
            .iter()
            .map(|address| address.root_index)
            .collect()
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct AccountProofInputs {
    pub hash: [u8; 32],
    pub root: [u8; 32],
    pub root_index: RootIndex,
    pub leaf_index: u64,
    pub tree_info: TreeInfo,
}

#[derive(Clone, Default, Copy, Debug, PartialEq)]
pub struct RootIndex {
    proof_by_index: bool,
    root_index: u16,
}

impl RootIndex {
    pub fn new_none() -> Self {
        Self {
            proof_by_index: true,
            root_index: 0,
        }
    }

    pub fn new_some(root_index: u16) -> Self {
        Self {
            proof_by_index: false,
            root_index,
        }
    }

    pub fn proof_by_index(&self) -> bool {
        self.proof_by_index
    }

    pub fn root_index(&self) -> Option<u16> {
        if !self.proof_by_index {
            Some(self.root_index)
        } else {
            None
        }
    }
}

impl AccountProofInputs {
    pub fn from_api_model(
        value: &photon_api::models::AccountProofInputs,
    ) -> Result<Self, IndexerError> {
        let root_index = {
            if value.root_index.prove_by_index {
                RootIndex::new_none()
            } else {
                RootIndex::new_some(value.root_index.root_index)
            }
        };
        Ok(Self {
            hash: decode_base58_to_fixed_array(&value.hash)?,
            root: decode_base58_to_fixed_array(&value.root)?,
            root_index,
            leaf_index: value.leaf_index,
            tree_info: TreeInfo::from_api_model(&value.merkle_context)?,
        })
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct AddressProofInputs {
    pub address: [u8; 32],
    pub root: [u8; 32],
    pub root_index: u16,
    pub tree_info: TreeInfo,
}

impl AddressProofInputs {
    pub fn from_api_model(
        value: &photon_api::models::AddressProofInputs,
    ) -> Result<Self, IndexerError> {
        Ok(Self {
            address: decode_base58_to_fixed_array(&value.address)?,
            root: decode_base58_to_fixed_array(&value.root)?,
            root_index: value.root_index,
            tree_info: TreeInfo::from_api_model(&value.merkle_context)?,
        })
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct PackedStateTreeInfos {
    pub packed_tree_infos: Vec<PackedStateTreeInfo>,
    pub output_tree_index: u8,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct PackedTreeInfos {
    pub state_trees: Option<PackedStateTreeInfos>,
    pub address_trees: Vec<PackedAddressTreeInfo>,
}

impl ValidityProofWithContext {
    pub fn pack_tree_infos(&self, packed_accounts: &mut PackedAccounts) -> PackedTreeInfos {
        let mut packed_tree_infos = Vec::new();
        let mut address_trees = Vec::new();
        let mut output_tree_index = None;
        for account in self.accounts.iter() {
            // Pack TreeInfo
            let merkle_tree_pubkey_index = packed_accounts.insert_or_get(account.tree_info.tree);
            let queue_pubkey_index = packed_accounts.insert_or_get(account.tree_info.queue);
            let tree_info_packed = PackedStateTreeInfo {
                root_index: account.root_index.root_index,
                merkle_tree_pubkey_index,
                queue_pubkey_index,
                leaf_index: account.leaf_index as u32,
                prove_by_index: account.root_index.proof_by_index(),
            };
            packed_tree_infos.push(tree_info_packed);

            // If a next Merkle tree exists the Merkle tree is full -> use the next Merkle tree for new state.
            // Else use the current Merkle tree for new state.
            if let Some(next) = account.tree_info.next_tree_info {
                // SAFETY: account will always have a state Merkle tree context.
                // pack_output_tree_index only panics on an address Merkle tree context.
                let index = next.pack_output_tree_index(packed_accounts).unwrap();
                if output_tree_index.is_none() {
                    output_tree_index = Some(index);
                }
            } else {
                // SAFETY: account will always have a state Merkle tree context.
                // pack_output_tree_index only panics on an address Merkle tree context.
                let index = account
                    .tree_info
                    .pack_output_tree_index(packed_accounts)
                    .unwrap();
                if output_tree_index.is_none() {
                    output_tree_index = Some(index);
                }
            }
        }

        for address in self.addresses.iter() {
            // Pack AddressTreeInfo
            let address_merkle_tree_pubkey_index =
                packed_accounts.insert_or_get(address.tree_info.tree);
            let address_queue_pubkey_index = packed_accounts.insert_or_get(address.tree_info.queue);
            address_trees.push(PackedAddressTreeInfo {
                address_merkle_tree_pubkey_index,
                address_queue_pubkey_index,
                root_index: address.root_index,
            });
        }
        let packed_tree_infos = if packed_tree_infos.is_empty() {
            None
        } else {
            Some(PackedStateTreeInfos {
                packed_tree_infos,
                output_tree_index: output_tree_index.unwrap(),
            })
        };
        PackedTreeInfos {
            state_trees: packed_tree_infos,
            address_trees,
        }
    }

    pub fn from_api_model(
        value: photon_api::models::CompressedProofWithContext,
        num_hashes: usize,
    ) -> Result<Self, IndexerError> {
        let proof = ValidityProof::new(Some(CompressedProof {
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
                let tree_pubkey =
                    Pubkey::new_from_array(decode_base58_to_fixed_array(&value.merkle_trees[i])?);
                let tree_info = super::tree_info::QUEUE_TREE_MAPPING
                    .get(&value.merkle_trees[i])
                    .ok_or(IndexerError::InvalidResponseData)?;

                Ok(AccountProofInputs {
                    hash: decode_base58_to_fixed_array(&value.leaves[i])?,
                    root: decode_base58_to_fixed_array(&value.roots[i])?,
                    root_index: RootIndex::new_some(value.root_indices[i] as u16),
                    leaf_index: value.leaf_indices[i] as u64,
                    tree_info: TreeInfo {
                        tree_type: tree_info.tree_type,
                        tree: tree_pubkey,
                        queue: tree_info.queue,
                        cpi_context: tree_info.cpi_context,
                        next_tree_info: None,
                    },
                })
            })
            .collect::<Result<Vec<_>, IndexerError>>()?;

        // Convert address data from remaining indices (if any)
        let addresses = if value.root_indices.len() > num_hashes {
            (num_hashes..value.root_indices.len())
                .map(|i| {
                    let tree_pubkey = Pubkey::new_from_array(decode_base58_to_fixed_array(
                        &value.merkle_trees[i],
                    )?);
                    let tree_info = super::tree_info::QUEUE_TREE_MAPPING
                        .get(&value.merkle_trees[i])
                        .ok_or(IndexerError::InvalidResponseData)?;

                    Ok(AddressProofInputs {
                        address: decode_base58_to_fixed_array(&value.leaves[i])?, // Address is in leaves
                        root: decode_base58_to_fixed_array(&value.roots[i])?,
                        root_index: value.root_indices[i] as u16,
                        tree_info: TreeInfo {
                            tree_type: tree_info.tree_type,
                            tree: tree_pubkey,
                            queue: tree_info.queue,
                            cpi_context: tree_info.cpi_context,
                            next_tree_info: None,
                        },
                    })
                })
                .collect::<Result<Vec<_>, IndexerError>>()?
        } else {
            Vec::new()
        };

        Ok(Self {
            proof,
            accounts,
            addresses,
        })
    }

    pub fn from_api_model_v2(
        value: photon_api::models::CompressedProofWithContextV2,
    ) -> Result<Self, IndexerError> {
        let proof = if let Some(proof) = value.compressed_proof {
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
            .map(AccountProofInputs::from_api_model)
            .collect::<Result<Vec<_>, IndexerError>>()?;

        let addresses = value
            .addresses
            .iter()
            .map(AddressProofInputs::from_api_model)
            .collect::<Result<Vec<_>, IndexerError>>()?;

        Ok(Self {
            proof,
            accounts,
            addresses,
        })
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct NextTreeInfo {
    pub cpi_context: Option<Pubkey>,
    pub queue: Pubkey,
    pub tree: Pubkey,
    pub tree_type: TreeType,
}

impl NextTreeInfo {
    /// Get the index of the output tree in the packed accounts.
    /// For StateV1, it returns the index of the tree account.
    /// For StateV2, it returns the index of the queue account.
    /// (For V2 trees new state is inserted into the output queue.
    /// The forester updates the tree from the queue asynchronously.)
    pub fn pack_output_tree_index(
        &self,
        packed_accounts: &mut PackedAccounts,
    ) -> Result<u8, IndexerError> {
        match self.tree_type {
            TreeType::StateV1 => Ok(packed_accounts.insert_or_get(self.tree)),
            TreeType::StateV2 => Ok(packed_accounts.insert_or_get(self.queue)),
            _ => Err(IndexerError::InvalidPackTreeType),
        }
    }
    pub fn from_api_model(
        value: &photon_api::models::TreeContextInfo,
    ) -> Result<Self, IndexerError> {
        Ok(Self {
            tree_type: TreeType::from(value.tree_type as u64),
            tree: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.tree)?),
            queue: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.queue)?),
            cpi_context: decode_base58_option_to_pubkey(&value.cpi_context)?,
        })
    }
}

impl TryFrom<&photon_api::models::TreeContextInfo> for NextTreeInfo {
    type Error = IndexerError;

    fn try_from(value: &photon_api::models::TreeContextInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            tree_type: TreeType::from(value.tree_type as u64),
            tree: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.tree)?),
            queue: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.queue)?),
            cpi_context: decode_base58_option_to_pubkey(&value.cpi_context)?,
        })
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct TreeInfo {
    pub cpi_context: Option<Pubkey>,
    pub next_tree_info: Option<NextTreeInfo>,
    pub queue: Pubkey,
    pub tree: Pubkey,
    pub tree_type: TreeType,
}

impl TreeInfo {
    /// Get the index of the output tree in the packed accounts.
    /// For StateV1, it returns the index of the tree account.
    /// For StateV2, it returns the index of the queue account.
    /// (For V2 trees new state is inserted into the output queue.
    /// The forester updates the tree from the queue asynchronously.)
    pub fn pack_output_tree_index(
        &self,
        packed_accounts: &mut PackedAccounts,
    ) -> Result<u8, IndexerError> {
        match self.tree_type {
            TreeType::StateV1 => Ok(packed_accounts.insert_or_get(self.tree)),
            TreeType::StateV2 => Ok(packed_accounts.insert_or_get(self.queue)),
            _ => Err(IndexerError::InvalidPackTreeType),
        }
    }

    pub fn get_output_pubkey(&self) -> Result<Pubkey, IndexerError> {
        match self.tree_type {
            TreeType::StateV1 => Ok(self.tree),
            TreeType::StateV2 => Ok(self.queue),
            _ => Err(IndexerError::InvalidPackTreeType),
        }
    }

    pub fn from_api_model(
        value: &photon_api::models::MerkleContextV2,
    ) -> Result<Self, IndexerError> {
        Ok(Self {
            tree_type: TreeType::from(value.tree_type as u64),
            tree: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.tree)?),
            queue: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.queue)?),
            cpi_context: decode_base58_option_to_pubkey(&value.cpi_context)?,
            next_tree_info: value
                .next_tree_context
                .as_ref()
                .map(|tree_info| NextTreeInfo::from_api_model(tree_info.as_ref()))
                .transpose()?,
        })
    }

    pub fn to_light_merkle_context(
        &self,
        leaf_index: u32,
        prove_by_index: bool,
    ) -> light_compressed_account::compressed_account::MerkleContext {
        use light_compressed_account::Pubkey;
        light_compressed_account::compressed_account::MerkleContext {
            merkle_tree_pubkey: Pubkey::new_from_array(self.tree.to_bytes()),
            queue_pubkey: Pubkey::new_from_array(self.queue.to_bytes()),
            leaf_index,
            tree_type: self.tree_type,
            prove_by_index,
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct CompressedAccount {
    pub address: Option<[u8; 32]>,
    pub data: Option<CompressedAccountData>,
    pub hash: [u8; 32],
    pub lamports: u64,
    pub leaf_index: u32,
    pub owner: Pubkey,
    pub prove_by_index: bool,
    pub seq: Option<u64>,
    pub slot_created: u64,
    pub tree_info: TreeInfo,
}

impl TryFrom<CompressedAccountWithMerkleContext> for CompressedAccount {
    type Error = IndexerError;

    fn try_from(account: CompressedAccountWithMerkleContext) -> Result<Self, Self::Error> {
        let hash = account
            .hash()
            .map_err(|_| IndexerError::InvalidResponseData)?;
        // Breaks light-program-test
        let tree_info = QUEUE_TREE_MAPPING.get(
            &Pubkey::new_from_array(account.merkle_context.merkle_tree_pubkey.to_bytes())
                .to_string(),
        );
        let cpi_context = if let Some(tree_info) = tree_info {
            tree_info.cpi_context
        } else {
            warn!("Cpi context not found in queue tree mapping");
            None
        };
        Ok(CompressedAccount {
            address: account.compressed_account.address,
            data: account.compressed_account.data,
            hash,
            lamports: account.compressed_account.lamports,
            leaf_index: account.merkle_context.leaf_index,
            tree_info: TreeInfo {
                tree: Pubkey::new_from_array(account.merkle_context.merkle_tree_pubkey.to_bytes()),
                queue: Pubkey::new_from_array(account.merkle_context.queue_pubkey.to_bytes()),
                tree_type: account.merkle_context.tree_type,
                cpi_context,
                next_tree_info: None,
            },
            owner: Pubkey::new_from_array(account.compressed_account.owner.to_bytes()),
            prove_by_index: account.merkle_context.prove_by_index,
            seq: None,
            slot_created: u64::MAX,
        })
    }
}

impl From<CompressedAccount> for CompressedAccountWithMerkleContext {
    fn from(account: CompressedAccount) -> Self {
        use light_compressed_account::Pubkey;
        let compressed_account = ProgramCompressedAccount {
            owner: Pubkey::new_from_array(account.owner.to_bytes()),
            lamports: account.lamports,
            address: account.address,
            data: account.data,
        };

        let merkle_context = account
            .tree_info
            .to_light_merkle_context(account.leaf_index, account.prove_by_index);

        CompressedAccountWithMerkleContext {
            compressed_account,
            merkle_context,
        }
    }
}

impl TryFrom<&photon_api::models::AccountV2> for CompressedAccount {
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

        let tree_info = TreeInfo {
            tree: Pubkey::new_from_array(decode_base58_to_fixed_array(
                &account.merkle_context.tree,
            )?),
            queue: Pubkey::new_from_array(decode_base58_to_fixed_array(
                &account.merkle_context.queue,
            )?),
            tree_type: TreeType::from(account.merkle_context.tree_type as u64),
            cpi_context: decode_base58_option_to_pubkey(&account.merkle_context.cpi_context)?,
            next_tree_info: account
                .merkle_context
                .next_tree_context
                .as_ref()
                .map(|ctx| NextTreeInfo::try_from(ctx.as_ref()))
                .transpose()?,
        };

        Ok(CompressedAccount {
            owner,
            address,
            data,
            hash,
            lamports: account.lamports,
            leaf_index: account.leaf_index,
            seq: account.seq,
            slot_created: account.slot_created,
            tree_info,
            prove_by_index: account.prove_by_index,
        })
    }
}

impl TryFrom<&photon_api::models::Account> for CompressedAccount {
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

        let tree_info = TreeInfo {
            cpi_context: tree_info.cpi_context,
            queue: tree_info.queue,
            tree_type: tree_info.tree_type,
            next_tree_info: None,
            tree: tree_info.tree,
        };

        Ok(CompressedAccount {
            owner,
            address,
            data,
            hash,
            lamports,
            leaf_index,
            seq,
            slot_created,
            tree_info,
            prove_by_index: false,
        })
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct StateMerkleTreeAccounts {
    pub merkle_tree: Pubkey,
    pub nullifier_queue: Pubkey,
    pub cpi_context: Pubkey,
    pub tree_type: TreeType,
}

#[allow(clippy::from_over_into)]
impl Into<TreeInfo> for StateMerkleTreeAccounts {
    fn into(self) -> TreeInfo {
        TreeInfo {
            tree: self.merkle_tree,
            queue: self.nullifier_queue,
            cpi_context: Some(self.cpi_context),
            tree_type: self.tree_type,
            next_tree_info: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AddressMerkleTreeAccounts {
    pub merkle_tree: Pubkey,
    pub queue: Pubkey,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct CompressedTokenAccount {
    /// Token-specific data (mint, owner, amount, delegate, state, tlv)
    pub token: TokenData,
    /// General account information (address, hash, lamports, merkle context, etc.)
    pub account: CompressedAccount,
}

impl TryFrom<&photon_api::models::TokenAccount> for CompressedTokenAccount {
    type Error = IndexerError;

    fn try_from(token_account: &photon_api::models::TokenAccount) -> Result<Self, Self::Error> {
        let account = CompressedAccount::try_from(token_account.account.as_ref())?;

        let token = TokenData {
            mint: Pubkey::new_from_array(decode_base58_to_fixed_array(
                &token_account.token_data.mint,
            )?),
            owner: Pubkey::new_from_array(decode_base58_to_fixed_array(
                &token_account.token_data.owner,
            )?),
            amount: token_account.token_data.amount,
            delegate: token_account
                .token_data
                .delegate
                .as_ref()
                .map(|d| decode_base58_to_fixed_array(d).map(Pubkey::new_from_array))
                .transpose()?,
            state: match token_account.token_data.state {
                photon_api::models::AccountState::Initialized => AccountState::Initialized,
                photon_api::models::AccountState::Frozen => AccountState::Frozen,
            },
            tlv: token_account
                .token_data
                .tlv
                .as_ref()
                .map(|tlv| {
                    let bytes = base64::decode_config(tlv, base64::STANDARD_NO_PAD)
                        .map_err(|_| IndexerError::InvalidResponseData)?;
                    Vec::<ExtensionStruct>::deserialize(&mut bytes.as_slice())
                        .map_err(|_| IndexerError::InvalidResponseData)
                })
                .transpose()?,
        };

        Ok(CompressedTokenAccount { token, account })
    }
}

impl TryFrom<&photon_api::models::TokenAccountV2> for CompressedTokenAccount {
    type Error = IndexerError;

    fn try_from(token_account: &photon_api::models::TokenAccountV2) -> Result<Self, Self::Error> {
        let account = CompressedAccount::try_from(token_account.account.as_ref())?;

        let token = TokenData {
            mint: Pubkey::new_from_array(decode_base58_to_fixed_array(
                &token_account.token_data.mint,
            )?),
            owner: Pubkey::new_from_array(decode_base58_to_fixed_array(
                &token_account.token_data.owner,
            )?),
            amount: token_account.token_data.amount,
            delegate: token_account
                .token_data
                .delegate
                .as_ref()
                .map(|d| decode_base58_to_fixed_array(d).map(Pubkey::new_from_array))
                .transpose()?,
            state: match token_account.token_data.state {
                photon_api::models::AccountState::Initialized => AccountState::Initialized,
                photon_api::models::AccountState::Frozen => AccountState::Frozen,
            },
            tlv: token_account
                .token_data
                .tlv
                .as_ref()
                .map(|tlv| {
                    let bytes = base64::decode_config(tlv, base64::STANDARD_NO_PAD)
                        .map_err(|_| IndexerError::InvalidResponseData)?;
                    Vec::<ExtensionStruct>::deserialize(&mut bytes.as_slice())
                        .map_err(|_| IndexerError::InvalidResponseData)
                })
                .transpose()?,
        };

        Ok(CompressedTokenAccount { token, account })
    }
}

#[allow(clippy::from_over_into)]
impl Into<light_ctoken_sdk::compat::TokenDataWithMerkleContext> for CompressedTokenAccount {
    fn into(self) -> light_ctoken_sdk::compat::TokenDataWithMerkleContext {
        let compressed_account = CompressedAccountWithMerkleContext::from(self.account);

        light_ctoken_sdk::compat::TokenDataWithMerkleContext {
            token_data: self.token,
            compressed_account,
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<Vec<light_ctoken_sdk::compat::TokenDataWithMerkleContext>>
    for super::response::Response<super::response::ItemsWithCursor<CompressedTokenAccount>>
{
    fn into(self) -> Vec<light_ctoken_sdk::compat::TokenDataWithMerkleContext> {
        self.value
            .items
            .into_iter()
            .map(
                |token_account| light_ctoken_sdk::compat::TokenDataWithMerkleContext {
                    token_data: token_account.token,
                    compressed_account: CompressedAccountWithMerkleContext::from(
                        token_account.account.clone(),
                    ),
                },
            )
            .collect::<Vec<light_ctoken_sdk::compat::TokenDataWithMerkleContext>>()
    }
}

impl TryFrom<light_ctoken_sdk::compat::TokenDataWithMerkleContext> for CompressedTokenAccount {
    type Error = IndexerError;

    fn try_from(
        token_data_with_context: light_ctoken_sdk::compat::TokenDataWithMerkleContext,
    ) -> Result<Self, Self::Error> {
        let account = CompressedAccount::try_from(token_data_with_context.compressed_account)?;

        Ok(CompressedTokenAccount {
            token: token_data_with_context.token_data,
            account,
        })
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct TokenBalance {
    pub balance: u64,
    pub mint: Pubkey,
}

impl TryFrom<&photon_api::models::TokenBalance> for TokenBalance {
    type Error = IndexerError;

    fn try_from(token_balance: &photon_api::models::TokenBalance) -> Result<Self, Self::Error> {
        Ok(TokenBalance {
            balance: token_balance.balance,
            mint: Pubkey::new_from_array(decode_base58_to_fixed_array(&token_balance.mint)?),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SignatureWithMetadata {
    pub block_time: u64,
    pub signature: String,
    pub slot: u64,
}

impl TryFrom<&photon_api::models::SignatureInfo> for SignatureWithMetadata {
    type Error = IndexerError;

    fn try_from(sig_info: &photon_api::models::SignatureInfo) -> Result<Self, Self::Error> {
        Ok(SignatureWithMetadata {
            block_time: sig_info.block_time,
            signature: sig_info.signature.clone(),
            slot: sig_info.slot,
        })
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct OwnerBalance {
    pub balance: u64,
    pub owner: Pubkey,
}

impl TryFrom<&photon_api::models::OwnerBalance> for OwnerBalance {
    type Error = IndexerError;

    fn try_from(owner_balance: &photon_api::models::OwnerBalance) -> Result<Self, Self::Error> {
        Ok(OwnerBalance {
            balance: owner_balance.balance,
            owner: Pubkey::new_from_array(decode_base58_to_fixed_array(&owner_balance.owner)?),
        })
    }
}
