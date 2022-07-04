use crate::ENCRYPTED_UTXOS_LENGTH;
// This file stores constants which do not have to be configured.
use anchor_lang::constant;

pub const ROOT_CHECK: u8 = 15;
pub const TWO_LEAVES_PDA_SIZE: u64 = 106 + ENCRYPTED_UTXOS_LENGTH as u64;
//instruction order
pub const IX_ORDER: [u8; 57] = [
    34, 14, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1,
    2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2, 241,
];
// Identitifiers for instructions
pub const MERKLE_TREE_UPDATE_START: u8 = 14;
pub const MERKLE_TREE_UPDATE_LEVEL: u8 = 25;
pub const LOCK_START: u8 = 34;
pub const HASH_0: u8 = 0;
pub const HASH_1: u8 = 1;
pub const HASH_2: u8 = 2;
pub const ROOT_INSERT: u8 = 241;
pub const MERKLE_TREE_SIZE:u64 = 16658;

#[constant]
pub const AUTHORITY_SEED: &[u8] = b"AUTHORITY_SEED";
#[constant]
pub const TREE_ROOT_SEED: &[u8] = b"TREE_ROOT_SEED";
#[constant]
pub const STORAGE_SEED: &[u8] = b"storage";
#[constant]
pub const LEAVES_SEED: &[u8] = b"leaves";
#[constant]
pub const NF_SEED: &[u8] = b"nf";

// account types
pub const TMP_STORAGE_ACCOUNT_TYPE: u8 = 1;
pub const MERKLE_TREE_ACCOUNT_TYPE: u8 = 2;
pub const NULLIFIER_ACCOUNT_TYPE: u8 = 3;
pub const LEAVES_PDA_ACCOUNT_TYPE: u8 = 4;
pub const USER_ACCOUNT_TYPE: u8 = 5;
pub const MERKLE_TREE_TMP_STORAGE_ACCOUNT_TYPE: u8 = 6;
pub const UNINSERTED_LEAVES_PDA_ACCOUNT_TYPE: u8 = 7;
