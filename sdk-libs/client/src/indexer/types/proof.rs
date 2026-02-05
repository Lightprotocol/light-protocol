use light_account::PackedAccounts;
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_sdk::instruction::{PackedAddressTreeInfo, PackedStateTreeInfo, ValidityProof};
use solana_pubkey::Pubkey;

use super::{
    super::{base58::decode_base58_to_fixed_array, tree_info::QUEUE_TREE_MAPPING, IndexerError},
    tree::TreeInfo,
};

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
    pub address: super::Address,
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
    pub new_low_element: Option<light_indexed_merkle_tree::array::IndexedElement<usize>>,
    pub new_element: Option<light_indexed_merkle_tree::array::IndexedElement<usize>>,
    pub new_element_next_value: Option<num_bigint::BigUint>,
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
                .map_err(|_| IndexerError::decode_error("proof.a", "invalid length"))?,
            b: value
                .compressed_proof
                .b
                .try_into()
                .map_err(|_| IndexerError::decode_error("proof.b", "invalid length"))?,
            c: value
                .compressed_proof
                .c
                .try_into()
                .map_err(|_| IndexerError::decode_error("proof.c", "invalid length"))?,
        }));

        // Convert account data from V1 flat arrays to V2 structured format
        let accounts = (0..num_hashes)
            .map(|i| {
                let tree_pubkey =
                    Pubkey::new_from_array(decode_base58_to_fixed_array(&value.merkle_trees[i])?);
                let tree_info = QUEUE_TREE_MAPPING.get(&value.merkle_trees[i]).ok_or(
                    IndexerError::MissingResult {
                        context: "conversion".into(),
                        message: format!(
                            "tree not found in QUEUE_TREE_MAPPING: {}",
                            &value.merkle_trees[i]
                        ),
                    },
                )?;

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
                    let tree_info = QUEUE_TREE_MAPPING.get(&value.merkle_trees[i]).ok_or(
                        IndexerError::MissingResult {
                            context: "conversion".into(),
                            message: "expected value was None".into(),
                        },
                    )?;

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
                    .map_err(|_| IndexerError::decode_error("proof.a", "invalid length"))?,
                b: proof
                    .b
                    .try_into()
                    .map_err(|_| IndexerError::decode_error("proof.b", "invalid length"))?,
                c: proof
                    .c
                    .try_into()
                    .map_err(|_| IndexerError::decode_error("proof.c", "invalid length"))?,
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
