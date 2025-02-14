// use light_bounded_vec::{BoundedVec, BoundedVecError};
// use light_hasher::{errors::HasherError, Hasher};
// use std::marker::PhantomData;
// use thiserror::Error;
//
// #[derive(Debug, Error)]
// pub enum PhotonMerkleTreeError {
//     #[error("Leaf {0} does not exist")]
//     LeafDoesNotExist(usize),
//     #[error("Hasher error: {0}")]
//     Hasher(#[from] HasherError),
//     #[error("Invalid proof length provided: {0} required {1}")]
//     InvalidProofLength(usize, usize),
//     #[error("Node index out of bounds")]
//     NodeIndexOutOfBounds,
// }
//
// #[derive(Debug, Clone)]
// pub struct PhotonMerkleTree<H>
// where
//     H: Hasher,
// {
//     pub height: usize,
//     pub nodes: Vec<[u8; 32]>,
//     _hasher: PhantomData<H>,
// }
//
// impl<H> PhotonMerkleTree<H>
// where
//     H: Hasher,
// {
//     pub fn new(height: usize) -> Self {
//         let node_count = (1 << height) - 1;
//         let mut nodes = Vec::with_capacity(node_count);
//
//         for level in 0..height {
//             let level_nodes = 1 << level;
//             for _ in 0..level_nodes {
//                 nodes.push(H::zero_bytes()[level]);
//             }
//         }
//
//         let leaf_count = 1 << (height - 1);
//         for _ in 0..leaf_count {
//             nodes.push(H::zero_bytes()[height - 1]);
//         }
//
//         Self {
//             height,
//             nodes,
//             _hasher: PhantomData,
//         }
//     }
//
//     /// Convert leaf index to node index in the flattened array
//     fn leaf_index_to_node_index(&self, leaf_index: usize) -> usize {
//         (1 << (self.height - 1)) + leaf_index
//     }
//
//     /// Convert node index to leaf index
//     fn node_index_to_leaf_index(&self, node_index: usize) -> Option<usize> {
//         let level = self.get_level_by_node_index(node_index);
//         if level == self.height - 1 {
//             Some(node_index - (1 << (self.height - 1)))
//         } else {
//             None
//         }
//     }
//
//     /// Get the level of a node by its index
//     fn get_level_by_node_index(&self, index: usize) -> usize {
//         let mut level = 0;
//         let mut idx = index;
//         while idx > 1 {
//             idx >>= 1;
//             level += 1;
//         }
//         level
//     }
//
//     /// Get proof path indices for a given node index
//     fn get_proof_path(&self, index: usize, include_leaf: bool) -> Vec<usize> {
//         let mut indices = vec![];
//         let mut idx = index;
//
//         if include_leaf {
//             indices.push(index);
//         }
//
//         while idx > 1 {
//             if idx % 2 == 0 {
//                 indices.push(idx + 1);
//             } else {
//                 indices.push(idx - 1);
//             }
//             idx >>= 1;
//         }
//         indices
//     }
//
//     /// Get the parent index of a node
//     fn get_parent_index(&self, index: usize) -> Option<usize> {
//         if index <= 1 {
//             None
//         } else {
//             Some(index >> 1)
//         }
//     }
//
//     /// Get sibling index of a node
//     fn get_sibling_index(&self, index: usize) -> Option<usize> {
//         if index <= 1 {
//             None
//         } else {
//             Some(if index % 2 == 0 { index + 1 } else { index - 1 })
//         }
//     }
//
//     // fn count_leaves(&self) -> usize {
//     //     let leaf_start = (1 << (self.height - 1)) - 1;
//     //     let initial_leaf_value = H::zero_bytes()[self.height - 1];
//     //
//     //     self.nodes[leaf_start..]
//     //         .iter()
//     //         .take(1 << (self.height - 1))
//     //         .filter(|&leaf| leaf != &initial_leaf_value)
//     //         .count()
//     // }
//
//     fn count_leaves(&self) -> usize {
//         let leaf_start = (1 << (self.height - 1)) - 1;
//         let max_leaves = 1 << (self.height - 1);
//
//         // Only look at actual leaf positions
//         let mut count = 0;
//         for i in 0..max_leaves {
//             let leaf_idx = leaf_start + i;
//             if leaf_idx < self.nodes.len() &&
//                 self.nodes[leaf_idx] != H::zero_bytes()[self.height - 1] {
//                 count += 1;
//             }
//         }
//         count
//     }
//
//     // pub fn append(&mut self, leaf: &[u8; 32]) -> Result<(), HasherError> {
//     //     println!("nodes before append: {:?}", self.nodes);
//     //     let leaf_start = (1 << (self.height - 1)) - 1;
//     //     let leaf_count = self.count_leaves();
//     //     let max_leaves = 1 << (self.height - 1);
//     //
//     //     println!(
//     //         "leaf_start: {}, leaf_count: {}, max_leaves: {}",
//     //         leaf_start, leaf_count, max_leaves
//     //     );
//     //     if leaf_count >= max_leaves {
//     //         return Err(HasherError::IntegerOverflow);
//     //     }
//     //
//     //     let node_index = self.leaf_index_to_node_index(leaf_count);
//     //     println!("node_index: {}", node_index);
//     //     self.nodes[node_index] = *leaf;
//     //
//     //     let mut current_index = node_index;
//     //     while let Some(parent_index) = self.get_parent_index(current_index) {
//     //         let sibling_index = self.get_sibling_index(current_index).unwrap();
//     //         let left_child = if current_index % 2 == 0 {
//     //             &self.nodes[current_index]
//     //         } else {
//     //             &self.nodes[sibling_index]
//     //         };
//     //         let right_child = if current_index % 2 == 0 {
//     //             &self.nodes[sibling_index]
//     //         } else {
//     //             &self.nodes[current_index]
//     //         };
//     //
//     //         self.nodes[parent_index] = H::hashv(&[&left_child[..], &right_child[..]])?;
//     //         current_index = parent_index;
//     //     }
//     //
//     //     println!("nodes after append: {:?}", self.nodes);
//     //     Ok(())
//     // }
//
//     pub fn append(&mut self, leaf: &[u8; 32]) -> Result<(), HasherError> {
//         let leaf_start = (1 << (self.height - 1)) - 1;
//         let leaf_count = self.count_leaves();
//         let max_leaves = 1 << (self.height - 1);
//
//         if leaf_count >= max_leaves {
//             return Err(HasherError::IntegerOverflow);
//         }
//
//         // Calculate the correct leaf position
//         let leaf_index = leaf_count;
//         let node_index = leaf_start + leaf_index;
//
//         // Set the leaf value
//         self.nodes[node_index] = *leaf;
//
//         // Update the path to the root
//         let mut current_index = node_index;
//         while let Some(parent_index) = self.get_parent_index(current_index) {
//             let sibling_index = self.get_sibling_index(current_index).unwrap();
//
//             // Ensure left-right ordering is maintained
//             let (left_child, right_child) = if current_index % 2 == 0 {
//                 (&self.nodes[current_index], &self.nodes[sibling_index])
//             } else {
//                 (&self.nodes[sibling_index], &self.nodes[current_index])
//             };
//
//             // Update parent
//             self.nodes[parent_index] = H::hashv(&[left_child, right_child])?;
//             current_index = parent_index;
//         }
//
//         Ok(())
//     }
//
//     pub fn root(&self) -> [u8; 32] {
//         self.nodes[1]
//     }
//
//     pub fn get_proof_by_indices(&self, indices: &[i32]) -> Vec<Vec<[u8; 32]>> {
//         let mut proofs = Vec::new();
//
//         for &leaf_index in indices {
//             let mut proof = Vec::with_capacity(self.height - 1);
//             let leaf_start = (1 << (self.height - 1)) - 1;
//             let mut current_index = leaf_start + leaf_index as usize;
//
//             // Start from the leaf and collect sibling hashes
//             while let Some(parent_index) = self.get_parent_index(current_index) {
//                 let sibling_index = if current_index % 2 == 0 {
//                     current_index + 1
//                 } else {
//                     current_index - 1
//                 };
//
//                 // Get sibling hash or the appropriate zero value
//                 let node_level = self.get_level_by_node_index(sibling_index);
//                 let sibling_value = if sibling_index < self.nodes.len() {
//                     self.nodes[sibling_index]
//                 } else {
//                     H::zero_bytes()[node_level]
//                 };
//
//                 proof.push(sibling_value);
//                 current_index = parent_index;
//             }
//
//             // Pad proof with zero values if necessary
//             while proof.len() < self.height - 1 {
//                 let level = proof.len();
//                 proof.push(H::zero_bytes()[level]);
//             }
//
//             proofs.push(proof);
//         }
//
//         proofs
//     }
//
//     pub fn verify(
//         &self,
//         leaf: &[u8; 32],
//         proof: &BoundedVec<[u8; 32]>,
//         leaf_index: usize,
//     ) -> Result<bool, PhotonMerkleTreeError> {
//         let max_leaves = 1 << (self.height - 1);
//         if leaf_index >= max_leaves {
//             return Err(PhotonMerkleTreeError::LeafDoesNotExist(leaf_index));
//         }
//
//         if proof.len() != self.height - 1 {
//             return Err(PhotonMerkleTreeError::InvalidProofLength(
//                 proof.len(),
//                 self.height - 1,
//             ));
//         }
//
//         let mut current_hash = *leaf;
//         let mut current_index = (1 << (self.height - 1)) - 1 + leaf_index;
//
//         for sibling_hash in proof.iter() {
//             let is_left = current_index % 2 == 0;
//
//             // Ensure correct ordering of hashes
//             let (left_child, right_child) = if is_left {
//                 (&current_hash[..], &sibling_hash[..])
//             } else {
//                 (&sibling_hash[..], &current_hash[..])
//             };
//
//             // Hash the pair
//             current_hash = H::hashv(&[left_child, right_child])?;
//
//             // Move up to parent
//             current_index = current_index.checked_shr(1)
//                 .ok_or(PhotonMerkleTreeError::NodeIndexOutOfBounds)?;
//         }
//
//         Ok(current_hash == self.root())
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use light_hasher::Poseidon;
//
//     #[test]
//     fn test_count_leaves() {
//         let mut tree = PhotonMerkleTree::<Poseidon>::new(4);
//         assert_eq!(tree.count_leaves(), 0);
//
//         let mut leaf = [0u8; 32];
//         leaf[31] = 1;
//         tree.append(&leaf).unwrap();
//         assert_eq!(tree.count_leaves(), 1);
//
//         leaf[31] = 2;
//         tree.append(&leaf).unwrap();
//         assert_eq!(tree.count_leaves(), 2);
//     }
//
//     #[test]
//     fn test_append_and_verify() {
//         let mut tree = PhotonMerkleTree::<Poseidon>::new(4);
//         assert_eq!(tree.count_leaves(), 0);
//
//         let mut leaf_1 = [0u8; 32];
//         leaf_1[31] = 1;
//         let mut leaf_2 = [0u8; 32];
//         leaf_2[31] = 2;
//
//         // Append leaves
//         tree.append(&leaf_1).unwrap();
//         assert_eq!(tree.count_leaves(), 1);
//
//         tree.append(&leaf_2).unwrap();
//         assert_eq!(tree.count_leaves(), 2);
//
//         // Get and verify proof
//         let indices = [0, 1];
//         let proofs = tree.get_proof_by_indices(&indices);
//
//         // Create BoundedVec for the first proof
//         let mut bounded_proof_1 = BoundedVec::with_capacity(tree.height - 1);
//         for element in &proofs[0] {
//             bounded_proof_1.push(*element).unwrap();
//         }
//         assert!(tree.verify(&leaf_1, &bounded_proof_1, 0).unwrap());
//
//         // Create BoundedVec for the second proof
//         let mut bounded_proof_2 = BoundedVec::with_capacity(tree.height - 1);
//         for element in &proofs[1] {
//             bounded_proof_2.push(*element).unwrap();
//         }
//         assert!(tree.verify(&leaf_2, &bounded_proof_2, 1).unwrap());
//
//         // Add remaining leaves up to capacity
//         for i in 2..8 {
//             let mut leaf = [0u8; 32];
//             leaf[31] = i as u8;
//             tree.append(&leaf).unwrap();
//             assert_eq!(tree.count_leaves(), i + 1);
//         }
//
//         // Try to append one more leaf (should fail)
//         assert!(tree.append(&[0u8; 32]).is_err());
//     }
// }
