// This file stores constants which do not have to be configured.
use anchor_lang::constant;
//instruction order
#[constant]
pub const IX_ORDER: [u8; 57] = [
    34, 14, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1,
    2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 241,
];
// Identitifiers for instructions
pub const MERKLE_TREE_UPDATE_START: u8 = 14;
pub const LOCK_START: u8 = 34;
pub const HASH_0: u8 = 0;
pub const HASH_1: u8 = 1;
pub const HASH_2: u8 = 2;
pub const ROOT_INSERT: u8 = 241;

// Seeds
#[constant]
pub const AUTHORITY_SEED: &[u8] = b"AUTHORITY_SEED";
#[constant]
pub const MERKLE_TREE_AUTHORITY_SEED: &[u8] = b"MERKLE_TREE_AUTHORITY";
#[constant]
pub const TREE_ROOT_SEED: &[u8] = b"TREE_ROOT_SEED";
#[constant]
pub const STORAGE_SEED: &[u8] = b"storage";
#[constant]
pub const LEAVES_SEED: &[u8] = b"leaves";
#[constant]
pub const NULLIFIER_SEED: &[u8] = b"nf";
#[constant]
pub const POOL_TYPE_SEED: &[u8] = b"pooltype";
#[constant]
pub const POOL_CONFIG_SEED: &[u8] = b"pool-config";
#[constant]
pub const POOL_SEED: &[u8] = b"pool";
#[constant]
pub const TOKEN_AUTHORITY_SEED: &[u8] = b"spl";
#[constant]
pub const EVENT_MERKLE_TREE_SEED: &[u8] = b"event_merkle_tree";
#[constant]
pub const TRANSACTION_MERKLE_TREE_SEED: &[u8] = b"transaction_merkle_tree";

// Merkle tree parameters
#[constant]
pub const EVENT_MERKLE_TREE_HEIGHT: usize = 18;
