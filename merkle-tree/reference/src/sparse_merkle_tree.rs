use std::marker::PhantomData;

use light_hasher::Hasher;

#[derive(Clone, Debug)]
pub struct SparseMerkleTree<H: Hasher, const HEIGHT: usize> {
    subtrees: [[u8; 32]; HEIGHT],
    next_index: usize,
    root: [u8; 32],
    _hasher: PhantomData<H>,
}

impl<H, const HEIGHT: usize> SparseMerkleTree<H, HEIGHT>
where
    H: Hasher,
{
    pub fn new(subtrees: [[u8; 32]; HEIGHT], next_index: usize) -> Self {
        Self {
            subtrees,
            next_index,
            root: [0u8; 32],
            _hasher: PhantomData,
        }
    }

    pub fn new_empty() -> Self {
        Self {
            subtrees: H::zero_bytes()[0..HEIGHT].try_into().unwrap(),
            next_index: 0,
            root: H::zero_bytes()[HEIGHT],
            _hasher: PhantomData,
        }
    }

    pub fn append(&mut self, leaf: [u8; 32]) {
        let mut current_index = self.next_index;
        let mut current_level_hash = leaf;
        let mut left;
        let mut right;

        for i in 0..HEIGHT {
            if current_index % 2 == 0 {
                left = current_level_hash;
                right = H::zero_bytes()[i];
                self.subtrees[i] = current_level_hash;
            } else {
                left = self.subtrees[i];
                right = current_level_hash;
            }
            current_level_hash = H::hashv(&[&left, &right]).unwrap();
            current_index /= 2;
        }

        self.root = current_level_hash;
        self.next_index += 1;
    }

    pub fn root(&self) -> [u8; 32] {
        self.root
    }

    pub fn get_subtrees(&self) -> [[u8; 32]; HEIGHT] {
        self.subtrees
    }

    pub fn get_height(&self) -> usize {
        HEIGHT
    }

    pub fn get_next_index(&self) -> usize {
        self.next_index
    }
}

#[cfg(test)]
mod test {
    use crate::MerkleTree;

    use super::*;
    use light_hasher::Poseidon;

    #[test]
    fn test_sparse_merkle_tree() {
        let height = 10;
        let mut merkle_tree = SparseMerkleTree::<Poseidon, 10>::new_empty();
        let mut reference_merkle_tree = MerkleTree::<Poseidon>::new(height, 0);
        for i in 0..1 << height {
            let mut leaf = [0u8; 32];
            leaf[24..].copy_from_slice(&(i as u64).to_be_bytes());
            println!("i: {}, leaf: {:?}", i, leaf);
            merkle_tree.append(leaf);
            reference_merkle_tree.append(&leaf).unwrap();
            assert_eq!(merkle_tree.root(), reference_merkle_tree.root());
            assert_eq!(merkle_tree.get_next_index(), i + 1);
            let subtrees = merkle_tree.get_subtrees();
            let reference_subtrees = reference_merkle_tree.get_subtrees();
            assert_eq!(subtrees.to_vec(), reference_subtrees);
        }
    }
}
