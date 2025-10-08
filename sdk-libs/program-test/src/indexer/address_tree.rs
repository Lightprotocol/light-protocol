use std::fmt::Debug;

#[cfg(feature = "devenv")]
use light_batched_merkle_tree::constants::DEFAULT_BATCH_ROOT_HISTORY_LEN;
use light_client::{
    fee::FeeConfig,
    indexer::{AddressMerkleTreeAccounts, IndexerError},
};
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{
    array::{IndexedArray, IndexedElement, IndexedElementBundle},
    reference::IndexedMerkleTree,
};
use light_prover_client::proof_types::non_inclusion::v2::NonInclusionMerkleProofInputs;
use light_sdk::constants::STATE_MERKLE_TREE_ROOTS;
use num_bigint::{BigInt, BigUint};
use num_traits::ops::bytes::FromBytes;

#[cfg(not(feature = "devenv"))]
use super::test_indexer::DEFAULT_BATCH_ROOT_HISTORY_LEN;

#[derive(Debug, Clone)]
pub enum IndexedMerkleTreeVersion {
    V1(Box<IndexedMerkleTree<Poseidon, usize>>),
    V2(Box<light_merkle_tree_reference::indexed::IndexedMerkleTree<Poseidon, usize>>),
}

#[derive(Debug, Clone)]
pub struct AddressMerkleTreeBundle {
    pub rollover_fee: i64,
    pub merkle_tree: IndexedMerkleTreeVersion,
    indexed_array: Box<IndexedArray<Poseidon, usize>>,
    pub accounts: AddressMerkleTreeAccounts,
    pub queue_elements: Vec<[u8; 32]>,
}

impl AddressMerkleTreeBundle {
    pub fn new_v1(accounts: AddressMerkleTreeAccounts) -> Result<Self, IndexerError> {
        let height = 26;
        let canopy = 10;
        let mut merkle_tree = IndexedMerkleTree::<Poseidon, usize>::new(height, canopy)
            .map_err(|_| IndexerError::InvalidResponseData)?;
        merkle_tree.merkle_tree.root_history_array_len = Some(STATE_MERKLE_TREE_ROOTS);
        let mut merkle_tree = Box::new(merkle_tree);
        merkle_tree.init()?;
        let mut indexed_array = Box::<IndexedArray<Poseidon, usize>>::default();
        indexed_array.init()?;
        Ok(AddressMerkleTreeBundle {
            merkle_tree: IndexedMerkleTreeVersion::V1(merkle_tree),
            indexed_array,
            accounts,
            rollover_fee: FeeConfig::default().address_queue_rollover as i64,
            queue_elements: vec![],
        })
    }

    pub fn new_v2(accounts: AddressMerkleTreeAccounts) -> Result<Self, IndexerError> {
        let height = 40;
        let canopy = 0;
        let mut merkle_tree = light_merkle_tree_reference::indexed::IndexedMerkleTree::<
            Poseidon,
            usize,
        >::new(height, canopy)
        .map_err(|_| IndexerError::InvalidResponseData)?;
        merkle_tree.merkle_tree.root_history_array_len =
            Some(DEFAULT_BATCH_ROOT_HISTORY_LEN as usize);
        let merkle_tree = IndexedMerkleTreeVersion::V2(Box::new(merkle_tree));

        Ok(AddressMerkleTreeBundle {
            merkle_tree,
            indexed_array: Box::default(),
            accounts,
            rollover_fee: FeeConfig::default().address_queue_rollover as i64,
            queue_elements: vec![],
        })
    }

    pub fn get_v1_indexed_merkle_tree(&self) -> Option<&IndexedMerkleTree<Poseidon, usize>> {
        match &self.merkle_tree {
            IndexedMerkleTreeVersion::V1(tree) => Some(tree),
            _ => None,
        }
    }

    pub fn get_v1_indexed_merkle_tree_mut(
        &mut self,
    ) -> Option<&mut IndexedMerkleTree<Poseidon, usize>> {
        match &mut self.merkle_tree {
            IndexedMerkleTreeVersion::V1(tree) => Some(tree),
            _ => None,
        }
    }

    pub fn get_v2_indexed_merkle_tree(
        &self,
    ) -> Option<&light_merkle_tree_reference::indexed::IndexedMerkleTree<Poseidon, usize>> {
        match &self.merkle_tree {
            IndexedMerkleTreeVersion::V2(tree) => Some(tree),
            _ => None,
        }
    }

    pub fn get_v2_indexed_merkle_tree_mut(
        &mut self,
    ) -> Option<&mut light_merkle_tree_reference::indexed::IndexedMerkleTree<Poseidon, usize>> {
        match &mut self.merkle_tree {
            IndexedMerkleTreeVersion::V2(tree) => Some(tree),
            _ => None,
        }
    }

    pub fn get_subtrees(&self) -> Vec<[u8; 32]> {
        match &self.merkle_tree {
            IndexedMerkleTreeVersion::V1(tree) => tree.merkle_tree.get_subtrees(),
            IndexedMerkleTreeVersion::V2(tree) => tree.merkle_tree.get_subtrees(),
        }
    }

    pub fn root(&self) -> [u8; 32] {
        match &self.merkle_tree {
            IndexedMerkleTreeVersion::V1(tree) => tree.merkle_tree.root(),
            IndexedMerkleTreeVersion::V2(tree) => tree.merkle_tree.root(),
        }
    }

    pub fn find_low_element_for_nonexistent(
        &self,
        value: &BigUint,
    ) -> Result<(IndexedElement<usize>, BigUint), IndexerError> {
        match &self.merkle_tree {
            IndexedMerkleTreeVersion::V1(_) => Ok(self
                .indexed_array
                .find_low_element_for_nonexistent(value)
                .map_err(|_| IndexerError::InvalidResponseData)?),
            IndexedMerkleTreeVersion::V2(tree) => {
                let (indexed_element, next_value) = tree
                    .indexed_array
                    .find_low_element_for_nonexistent(value)
                    .map_err(|_| IndexerError::InvalidResponseData)?;
                Ok((
                    IndexedElement {
                        index: indexed_element.index,
                        value: indexed_element.value.clone(),
                        next_index: indexed_element.next_index,
                    },
                    next_value,
                ))
            }
        }
    }

    pub fn new_element_with_low_element_index(
        &self,
        index: usize,
        value: &BigUint,
    ) -> Result<IndexedElementBundle<usize>, IndexerError> {
        match &self.merkle_tree {
            IndexedMerkleTreeVersion::V1(_) => Ok(self
                .indexed_array
                .new_element_with_low_element_index(index, value)
                .map_err(|_| IndexerError::InvalidResponseData)?),
            IndexedMerkleTreeVersion::V2(tree) => {
                let res = tree
                    .indexed_array
                    .new_element_with_low_element_index(index, value)
                    .map_err(|_| IndexerError::InvalidResponseData)?;
                Ok(IndexedElementBundle {
                    new_element: IndexedElement {
                        index: res.new_element.index,
                        value: res.new_element.value.clone(),
                        next_index: res.new_element.next_index,
                    },
                    new_low_element: IndexedElement {
                        index: res.new_low_element.index,
                        value: res.new_low_element.value.clone(),
                        next_index: res.new_low_element.next_index,
                    },
                    new_element_next_value: res.new_element_next_value.clone(),
                })
            }
        }
    }

    pub fn get_proof_of_leaf(
        &self,
        index: usize,
        full: bool,
    ) -> Result<Vec<[u8; 32]>, IndexerError> {
        match &self.merkle_tree {
            IndexedMerkleTreeVersion::V1(tree) => Ok(tree
                .get_proof_of_leaf(index, full)
                .map_err(|_| IndexerError::InvalidResponseData)?
                .to_vec()),
            IndexedMerkleTreeVersion::V2(tree) => Ok(tree
                .get_proof_of_leaf(index, full)
                .map_err(|_| IndexerError::InvalidResponseData)?),
        }
    }

    pub fn append(&mut self, value: &BigUint) -> Result<(), IndexerError> {
        match &mut self.merkle_tree {
            IndexedMerkleTreeVersion::V1(tree) => {
                tree.append(value, &mut self.indexed_array)
                    .map_err(|_| IndexerError::InvalidResponseData)?;
                Ok(())
            }
            IndexedMerkleTreeVersion::V2(tree) => {
                tree.append(value)
                    .map_err(|_| IndexerError::InvalidResponseData)?;
                Ok(())
            }
        }
    }

    pub fn get_non_inclusion_proof_inputs(
        &self,
        value: &[u8; 32],
    ) -> Result<NonInclusionMerkleProofInputs, IndexerError> {
        match &self.merkle_tree {
            IndexedMerkleTreeVersion::V1(tree) => Ok(get_non_inclusion_proof_inputs(
                value,
                tree,
                &self.indexed_array,
            )),
            IndexedMerkleTreeVersion::V2(merkle_tree) => {
                let non_inclusion_proof = merkle_tree
                    .get_non_inclusion_proof(&BigUint::from_be_bytes(value))
                    .map_err(|_| IndexerError::InvalidResponseData)?;
                let proof = non_inclusion_proof
                    .merkle_proof
                    .iter()
                    .map(|x| BigInt::from_be_bytes(x))
                    .collect();
                Ok(NonInclusionMerkleProofInputs {
                    root: BigInt::from_be_bytes(merkle_tree.root().as_slice()),
                    value: BigInt::from_be_bytes(value),
                    leaf_lower_range_value: BigInt::from_be_bytes(
                        &non_inclusion_proof.leaf_lower_range_value,
                    ),
                    leaf_higher_range_value: BigInt::from_be_bytes(
                        &non_inclusion_proof.leaf_higher_range_value,
                    ),
                    merkle_proof_hashed_indexed_element_leaf: proof,
                    index_hashed_indexed_element_leaf: BigInt::from(non_inclusion_proof.leaf_index),
                    next_index: BigInt::from(non_inclusion_proof.next_index),
                })
            }
        }
    }

    pub fn right_most_index(&self) -> usize {
        match &self.merkle_tree {
            IndexedMerkleTreeVersion::V1(tree) => tree.merkle_tree.rightmost_index,
            IndexedMerkleTreeVersion::V2(tree) => tree.merkle_tree.rightmost_index,
        }
    }

    pub fn append_with_low_element_index(
        &mut self,
        index: usize,
        value: &BigUint,
    ) -> Result<IndexedElementBundle<usize>, IndexerError> {
        match &mut self.merkle_tree {
            IndexedMerkleTreeVersion::V1(_) => Ok(self
                .indexed_array
                .append_with_low_element_index(index, value)
                .map_err(|_| IndexerError::InvalidResponseData)?),
            IndexedMerkleTreeVersion::V2(_) => {
                unimplemented!("append_with_low_element_index")
            }
        }
    }

    pub fn sequence_number(&self) -> u64 {
        match &self.merkle_tree {
            IndexedMerkleTreeVersion::V1(tree) => tree.merkle_tree.sequence_number as u64,
            IndexedMerkleTreeVersion::V2(tree) => tree.merkle_tree.sequence_number as u64,
        }
    }

    pub fn height(&self) -> usize {
        match &self.merkle_tree {
            IndexedMerkleTreeVersion::V1(tree) => tree.merkle_tree.height,
            IndexedMerkleTreeVersion::V2(tree) => tree.merkle_tree.height,
        }
    }

    pub fn get_path_of_leaf(
        &self,
        index: usize,
        full: bool,
    ) -> Result<Vec<[u8; 32]>, IndexerError> {
        match &self.merkle_tree {
            IndexedMerkleTreeVersion::V1(tree) => Ok(tree
                .get_path_of_leaf(index, full)
                .map_err(|_| IndexerError::InvalidResponseData)?
                .to_vec()),
            IndexedMerkleTreeVersion::V2(tree) => Ok(tree
                .get_path_of_leaf(index, full)
                .map_err(|_| IndexerError::InvalidResponseData)?),
        }
    }

    pub fn indexed_array_v1(&self) -> Option<&IndexedArray<Poseidon, usize>> {
        println!(
            "indexed_array_v2: merkle_tree pubkey: {:?}",
            self.accounts.merkle_tree
        );
        match &self.merkle_tree {
            IndexedMerkleTreeVersion::V1(_) => Some(&self.indexed_array),
            _ => None,
        }
    }

    pub fn indexed_array_v2(
        &self,
    ) -> Option<&light_indexed_array::array::IndexedArray<Poseidon, usize>> {
        println!(
            "indexed_array_v2: merkle_tree pubkey: {:?}",
            self.accounts.merkle_tree
        );
        match &self.merkle_tree {
            IndexedMerkleTreeVersion::V2(tree) => Some(&tree.indexed_array),
            _ => None,
        }
    }

    pub fn update(
        &mut self,
        new_low_element: &IndexedElement<usize>,
        new_element: &IndexedElement<usize>,
        new_element_next_value: &BigUint,
    ) -> Result<(), IndexerError> {
        match &mut self.merkle_tree {
            IndexedMerkleTreeVersion::V1(tree) => {
                Ok(tree.update(new_low_element, new_element, new_element_next_value)?)
            }
            IndexedMerkleTreeVersion::V2(tree) => {
                let new_low_element = light_indexed_array::array::IndexedElement::<usize> {
                    index: new_low_element.index,
                    value: new_low_element.value.clone(),
                    next_index: new_low_element.next_index,
                };
                let new_element = light_indexed_array::array::IndexedElement::<usize> {
                    index: new_element.index,
                    value: new_element.value.clone(),
                    next_index: new_element.next_index,
                };
                tree.update(&new_low_element, &new_element, new_element_next_value)
                    .unwrap();
                Ok(())
            }
        }
    }
}

// TODO: eliminate use of BigInt in favor of BigUint
pub fn get_non_inclusion_proof_inputs(
    value: &[u8; 32],
    merkle_tree: &light_indexed_merkle_tree::reference::IndexedMerkleTree<
        light_hasher::Poseidon,
        usize,
    >,
    indexed_array: &IndexedArray<light_hasher::Poseidon, usize>,
) -> NonInclusionMerkleProofInputs {
    let non_inclusion_proof = merkle_tree
        .get_non_inclusion_proof(&BigUint::from_be_bytes(value), indexed_array)
        .unwrap();
    let proof = non_inclusion_proof
        .merkle_proof
        .iter()
        .map(|x| BigInt::from_be_bytes(x))
        .collect();
    NonInclusionMerkleProofInputs {
        root: BigInt::from_be_bytes(merkle_tree.root().as_slice()),
        value: BigInt::from_be_bytes(value),
        leaf_lower_range_value: BigInt::from_be_bytes(&non_inclusion_proof.leaf_lower_range_value),
        leaf_higher_range_value: BigInt::from_be_bytes(
            &non_inclusion_proof.leaf_higher_range_value,
        ),
        merkle_proof_hashed_indexed_element_leaf: proof,
        index_hashed_indexed_element_leaf: BigInt::from(non_inclusion_proof.leaf_index),
        next_index: BigInt::from(non_inclusion_proof.next_index),
    }
}
