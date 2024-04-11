use std::collections::HashMap;

use anchor_lang::prelude::{AccountInfo, Pubkey};

/// Mapping of address queue public keys to a bundle containing:
///
/// * The queue.
/// * Associated Merkle tree.
/// * Addresses to insert.
pub type QueueMap<'info> = HashMap<Pubkey, QueueBundle<'info>>;

/// A bundle containing:
///
/// * Address queue.
/// * Merkle tree associated with that queue.
/// * Addresses to insert to that queue.
pub struct QueueBundle<'info> {
    pub queue: &'info AccountInfo<'info>,
    pub merkle_tree: &'info AccountInfo<'info>,
    pub elements: Vec<[u8; 32]>,
}

impl<'info> QueueBundle<'info> {
    pub fn new(queue: &'info AccountInfo<'info>, merkle_tree: &'info AccountInfo<'info>) -> Self {
        Self {
            queue,
            merkle_tree,
            elements: Vec::new(),
        }
    }
}
