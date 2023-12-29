use std::{cell::RefCell, collections::VecDeque, marker::PhantomData, rc::Rc};

use light_hasher::{errors::HasherError, Hasher};

pub fn build_root<H>(leaves: &[Rc<RefCell<TreeNode<H>>>]) -> Result<[u8; 32], HasherError>
where
    H: Hasher,
{
    let mut tree = VecDeque::from_iter(leaves.iter().map(Rc::clone));
    let mut seq_num = leaves.len();
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
        let mut hashed_parent = [0u8; 32];

        hashed_parent
            .copy_from_slice(H::hashv(&[&left.borrow().node, &right.borrow().node])?.as_ref());
        let parent = Rc::new(RefCell::new(TreeNode::new(
            hashed_parent,
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

    let root = tree[0].borrow().node;
    Ok(root)
}

/// Reference implementation of Merkle tree used for testing.
pub struct MerkleTree<H, const MAX_ROOTS: usize>
where
    H: Hasher,
{
    pub leaf_nodes: Vec<Rc<RefCell<TreeNode<H>>>>,
    pub roots: Vec<[u8; 32]>,

    _hasher: PhantomData<H>,
}

impl<H, const MAX_ROOTS: usize> Default for MerkleTree<H, MAX_ROOTS>
where
    H: Hasher,
{
    fn default() -> Self {
        Self {
            leaf_nodes: Vec::new(),
            roots: Vec::new(),
            _hasher: PhantomData,
        }
    }
}

impl<H, const MAX_ROOTS: usize> MerkleTree<H, MAX_ROOTS>
where
    H: Hasher,
{
    pub fn new(height: usize) -> Result<Self, HasherError> {
        let mut leaf_nodes = vec![];
        for i in 0..(1 << height) {
            let tree_node = TreeNode::new_empty(0, i);
            leaf_nodes.push(Rc::new(RefCell::new(tree_node)));
        }
        let root = build_root(leaf_nodes.as_slice())?;
        let roots = vec![root];
        Ok(Self {
            leaf_nodes,
            roots,
            _hasher: PhantomData,
        })
    }

    /// Getch the Merkle proof of the leaf under the given `ìndex`.
    pub fn get_proof_of_leaf(&self, index: usize) -> Vec<[u8; 32]> {
        let mut proof = vec![];
        let mut node = self.leaf_nodes[index].clone();
        loop {
            let ref_node = node.clone();
            if ref_node.borrow().parent.is_none() {
                break;
            }
            let parent = ref_node.borrow().parent.as_ref().unwrap().clone();
            if parent.borrow().left.as_ref().unwrap().borrow().id == ref_node.borrow().id {
                proof.push(parent.borrow().right.as_ref().unwrap().borrow().node);
            } else {
                proof.push(parent.borrow().left.as_ref().unwrap().borrow().node);
            }
            node = parent;
        }
        proof
    }

    /// Updates root from an updated leaf node set at index: `idx`
    fn update_root_from_leaf(&mut self, leaf_idx: usize) -> Result<(), HasherError> {
        let mut node = self.leaf_nodes[leaf_idx].clone();
        loop {
            let ref_node = node.clone();
            if ref_node.borrow().parent.is_none() {
                self.roots.push(ref_node.borrow().node);
                break;
            }
            let parent = ref_node.borrow().parent.as_ref().unwrap().clone();
            let hash = if parent.borrow().left.as_ref().unwrap().borrow().id == ref_node.borrow().id
            {
                H::hashv(&[
                    &ref_node.borrow().node,
                    &parent.borrow().right.as_ref().unwrap().borrow().node,
                ])?
            } else {
                H::hashv(&[
                    &parent.borrow().left.as_ref().unwrap().borrow().node,
                    &ref_node.borrow().node,
                ])?
            };
            node = parent;
            node.borrow_mut().node.copy_from_slice(hash.as_ref());
        }

        Ok(())
    }

    pub fn node(&self, idx: usize) -> [u8; 32] {
        self.leaf_nodes[idx].borrow().node
    }

    pub fn root(&self) -> Option<[u8; 32]> {
        self.roots.last().copied()
    }

    pub fn update(&mut self, leaf: &[u8; 32], leaf_idx: usize) -> Result<(), HasherError> {
        self.leaf_nodes[leaf_idx].borrow_mut().node = *leaf;
        self.update_root_from_leaf(leaf_idx)
    }

    pub fn leaf(&self, leaf_idx: usize) -> [u8; 32] {
        self.leaf_nodes[leaf_idx].borrow().node
    }
}

#[derive(Clone, Debug)]
pub struct TreeNode<H>
where
    H: Hasher,
{
    pub node: [u8; 32],
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
        node: [u8; 32],
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
            node: H::zero_bytes()[level],
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
