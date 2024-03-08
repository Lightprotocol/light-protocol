use std::{cell::RefCell, collections::VecDeque, marker::PhantomData, rc::Rc};

use light_bounded_vec::{BoundedVec, BoundedVecError};
use light_hasher::{errors::HasherError, Hasher};

pub mod store;

/// Reference implementation of Merkle tree which stores all nodes. Used for
/// testing.
#[derive(Debug)]
pub struct MerkleTree<H>
where
    H: Hasher,
{
    pub height: usize,
    pub roots_size: usize,
    pub canopy_depth: usize,
    pub leaf_nodes: Vec<Rc<RefCell<TreeNode<H>>>>,
    pub roots: Vec<[u8; 32]>,
    pub rightmost_index: usize,

    _hasher: PhantomData<H>,
}

impl<H> MerkleTree<H>
where
    H: Hasher,
{
    pub fn new(height: usize, roots_size: usize, canopy_depth: usize) -> Result<Self, HasherError> {
        let mut leaf_nodes = vec![];
        for i in 0..(1 << height) {
            let tree_node = TreeNode::new_empty(0, i);
            leaf_nodes.push(Rc::new(RefCell::new(tree_node)));
        }

        let mut tree = VecDeque::from_iter(leaf_nodes.iter().cloned());
        let mut seq_num = leaf_nodes.len();
        while tree.len() > 1 {
            let left = tree.pop_front().unwrap();
            let level = left.borrow().level;
            let right = if level != tree[0].borrow().level {
                let node = Rc::new(RefCell::new(TreeNode::new_empty(level, seq_num)));
                seq_num += 1;
                node
            } else {
                tree.pop_front().unwrap()
            };

            let parent = Rc::new(RefCell::new(TreeNode::new(
                None,
                left.clone(),
                right.clone(),
                level + 1,
                seq_num,
            )));
            left.borrow_mut().assign_parent(parent.clone());
            right.borrow_mut().assign_parent(parent.clone());
            tree.push_back(parent);
            seq_num += 1;
        }

        let root = H::zero_bytes()[height];
        let roots = vec![root];
        Ok(Self {
            height,
            roots_size,
            canopy_depth,
            leaf_nodes,
            roots,
            rightmost_index: 0,
            _hasher: PhantomData,
        })
    }

    /// Returns the Merkle proof of the leaf under the given `ìndex`.
    pub fn get_proof_of_leaf(&self, index: usize) -> Result<BoundedVec<[u8; 32]>, BoundedVecError> {
        let mut proof = BoundedVec::with_capacity(self.height);
        let mut node = self.leaf_nodes[index].clone();

        let limit = self.height - self.canopy_depth;

        for i in 0..limit {
            let ref_node = node.clone();
            if ref_node.borrow().parent.is_none() {
                // Add a zero node to the proof.
                proof.push(H::zero_bytes()[i])?;
            } else {
                // Add a non-zero node to the proof.
                let parent = ref_node.borrow().parent.as_ref().unwrap().clone();
                if parent.borrow().left.as_ref().unwrap().borrow().id == ref_node.borrow().id {
                    proof.push(
                        parent
                            .borrow()
                            .right
                            .as_ref()
                            .unwrap()
                            .borrow()
                            .node
                            .unwrap_or(H::zero_bytes()[i]),
                    )?;
                } else {
                    proof.push(
                        parent
                            .borrow()
                            .left
                            .as_ref()
                            .unwrap()
                            .borrow()
                            .node
                            .unwrap_or(H::zero_bytes()[i]),
                    )?;
                }
                node = parent;
            }
        }

        Ok(proof)
    }

    /// Updates root from an updated leaf node set at index: `idx`
    fn update_root_from_leaf(&mut self, leaf_idx: usize) -> Result<(), HasherError> {
        let mut node = self.leaf_nodes[leaf_idx].clone();
        let mut i = 0_usize;
        loop {
            let ref_node = node.clone();
            if ref_node.borrow().parent.is_none() {
                self.roots
                    .push(ref_node.borrow().node.unwrap_or(H::zero_bytes()[i]));
                break;
            }
            let parent = ref_node.borrow().parent.as_ref().unwrap().clone();
            let hash = if parent.borrow().left.as_ref().unwrap().borrow().id == ref_node.borrow().id
            {
                H::hashv(&[
                    &ref_node.borrow().node.unwrap_or(H::zero_bytes()[i]),
                    &parent
                        .borrow()
                        .right
                        .as_ref()
                        .unwrap()
                        .borrow()
                        .node
                        .unwrap_or(H::zero_bytes()[i]),
                ])?
            } else {
                H::hashv(&[
                    &parent
                        .borrow()
                        .left
                        .as_ref()
                        .unwrap()
                        .borrow()
                        .node
                        .unwrap_or(H::zero_bytes()[i]),
                    &ref_node.borrow().node.unwrap_or(H::zero_bytes()[i]),
                ])?
            };
            node = parent;
            node.borrow_mut().node = Some(hash);
            i += 1;
        }

        Ok(())
    }

    pub fn root(&self) -> Option<[u8; 32]> {
        self.roots.last().copied()
    }

    pub fn update(&mut self, leaf: &[u8; 32], leaf_idx: usize) -> Result<(), HasherError> {
        self.leaf_nodes[leaf_idx].borrow_mut().node = Some(*leaf);
        self.update_root_from_leaf(leaf_idx)
    }

    pub fn append(&mut self, leaf: &[u8; 32]) -> Result<(), HasherError> {
        self.update(leaf, self.rightmost_index)?;
        self.rightmost_index = self.rightmost_index.saturating_add(1);
        Ok(())
    }

    pub fn leaf(&self, leaf_idx: usize) -> [u8; 32] {
        self.leaf_nodes[leaf_idx]
            .borrow()
            .node
            .unwrap_or(H::zero_bytes()[0])
    }

    pub fn get_leaf_index(&self, leaf: &[u8; 32]) -> Option<usize> {
        self.leaf_nodes
            .iter()
            .position(|node| node.borrow().node == Some(*leaf))
    }
}

#[derive(Clone, Debug)]
pub struct TreeNode<H>
where
    H: Hasher,
{
    pub node: Option<[u8; 32]>,
    left: Option<Rc<RefCell<TreeNode<H>>>>,
    right: Option<Rc<RefCell<TreeNode<H>>>>,
    parent: Option<Rc<RefCell<TreeNode<H>>>>,
    level: usize,
    /// ID needed to figure out whether we came from left or right child node
    /// when hashing path upwards
    id: usize,

    _hasher: PhantomData<H>,
}

impl<H> TreeNode<H>
where
    H: Hasher,
{
    pub fn new(
        node: Option<[u8; 32]>,
        left: Rc<RefCell<TreeNode<H>>>,
        right: Rc<RefCell<TreeNode<H>>>,
        level: usize,
        id: usize,
    ) -> Self {
        Self {
            node,
            left: Some(left),
            right: Some(right),
            parent: None,
            level,
            id,
            _hasher: PhantomData,
        }
    }

    pub fn new_empty(level: usize, id: usize) -> Self {
        Self {
            node: None,
            left: None,
            right: None,
            parent: None,
            level,
            id,
            _hasher: PhantomData,
        }
    }

    /// Allows to propagate parent assignment
    pub fn assign_parent(&mut self, parent: Rc<RefCell<TreeNode<H>>>) {
        self.parent = Some(parent);
    }
}
