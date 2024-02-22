use std::{collections::BTreeMap, marker::PhantomData};

use light_hasher::Hasher;

/// Store of Merkle tree nodes, which doesn't check the integrity of the tree.
/// It blindly believes what's being inserted in it. It can return Merkle
/// proofs, but without guarantee that they are correct.
///
/// Used for testing, to mock indexers.
pub struct Store<H>
where
    H: Hasher,
{
    map: BTreeMap<usize, [u8; 32]>,
    _hasher: PhantomData<H>,
}

impl<H> Default for Store<H>
where
    H: Hasher,
{
    fn default() -> Self {
        let map = BTreeMap::new();
        Self {
            map,
            _hasher: PhantomData,
        }
    }
}

impl<H> Store<H>
where
    H: Hasher,
{
    /// Adds node to the store.
    pub fn add_node(&mut self, node: [u8; 32], index: usize) {
        self.map.insert(index, node);
    }

    /// Gets the proof for a leaf with the given `index`.
    pub fn get_proof_for_leaf(&self, index: usize) -> Vec<[u8; 32]> {
        let mut index = index;
        let mut level = 0;
        let mut proof = Vec::new();

        while index > 1 {
            if index % 2 == 0 {
                index += 1;
            } else {
                index -= 1;
            }

            let node = self
                .map
                .get(&index)
                .cloned()
                .unwrap_or(H::zero_bytes()[level]);
            proof.push(node);

            index >>= 1;
            level += 1;
        }

        proof
    }
}
