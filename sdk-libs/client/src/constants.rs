use solana_pubkey::{pubkey, Pubkey};

/// Address lookup table containing state Merkle tree pubkeys for mainnet-beta.
/// Used to reduce transaction size by referencing trees via lookup table indices.
pub const STATE_TREE_LOOKUP_TABLE_MAINNET: Pubkey =
    pubkey!("7i86eQs3GSqHjN47WdWLTCGMW6gde1q96G2EVnUyK2st");

/// Address lookup table containing nullifier queue pubkeys for mainnet-beta.
/// Used to reduce transaction size by referencing queues via lookup table indices.
pub const NULLIFIED_STATE_TREE_LOOKUP_TABLE_MAINNET: Pubkey =
    pubkey!("H9QD4u1fG7KmkAzn2tDXhheushxFe1EcrjGGyEFXeMqT");

/// Address lookup table containing state Merkle tree pubkeys for devnet.
/// Used to reduce transaction size by referencing trees via lookup table indices.
pub const STATE_TREE_LOOKUP_TABLE_DEVNET: Pubkey =
    pubkey!("8n8rH2bFRVA6cSGNDpgqcKHCndbFCT1bXxAQG89ejVsh");

/// Address lookup table containing nullifier queue pubkeys for devnet.
/// Used to reduce transaction size by referencing queues via lookup table indices.
pub const NULLIFIED_STATE_TREE_LOOKUP_TABLE_DEVNET: Pubkey =
    pubkey!("5dhaJLBjnVBQFErr8oiCJmcVsx3Zj6xDekGB2zULPsnP");
