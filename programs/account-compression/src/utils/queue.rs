use std::collections::HashMap;

use anchor_lang::prelude::{AccountInfo, Pubkey};
use light_merkle_tree_metadata::QueueType;

/// Mapping of address queue public keys to a bundle containing:
///
/// * The queue.
/// * Associated Merkle tree.
/// * Addresses to insert.
pub type QueueMap<'a, 'info> = HashMap<Pubkey, QueueBundle<'a, 'info>>;

/// A bundle containing:
///
/// * Address queue.
/// * Merkle tree associated with that queue.
/// * Addresses to insert to that queue.
pub struct QueueBundle<'a, 'info> {
    pub queue_type: QueueType,
    pub accounts: Vec<&'info AccountInfo<'info>>,
    pub elements: Vec<&'a [u8; 32]>,
    pub indices: Vec<u32>,
    pub prove_by_index: Vec<bool>,
}

impl<'info> QueueBundle<'_, 'info> {
    pub fn new(queue_type: QueueType, accounts: Vec<&'info AccountInfo<'info>>) -> Self {
        Self {
            queue_type,
            accounts,
            elements: Vec::new(),
            indices: Vec::new(),
            prove_by_index: Vec::new(),
        }
    }
}
