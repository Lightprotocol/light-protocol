use std::mem;

use account_compression::{
    utils::constants::{
        ADDRESS_MERKLE_TREE_CHANGELOG, ADDRESS_MERKLE_TREE_HEIGHT, ADDRESS_MERKLE_TREE_ROOTS,
        STATE_INDEXED_ARRAY_SIZE, STATE_MERKLE_TREE_CHANGELOG, STATE_MERKLE_TREE_HEIGHT,
        STATE_MERKLE_TREE_ROOTS,
    },
    AddressMerkleTreeAccount, StateMerkleTreeAccount,
};
use account_compression_state::{AddressMerkleTree, AddressQueue, StateMerkleTree};
use ark_ff::BigInteger256;
use light_concurrent_merkle_tree::changelog::{ChangelogEntry22, ChangelogEntry26};
use light_hasher::Poseidon;
use light_indexed_merkle_tree::array::IndexingArray;
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct Type {
    name: String,
    space: usize,
}

pub fn type_sizes() -> anyhow::Result<()> {
    let accounts = vec![
        Type {
            name: "StateMerkleTreeAccount (with discriminator)".to_owned(),
            space: mem::size_of::<StateMerkleTreeAccount>() + 8,
        },
        Type {
            name: "StateMerkleTree".to_owned(),
            space: mem::size_of::<StateMerkleTree>(),
        },
        Type {
            name: "StateMerkleTree->filled_subtrees".to_owned(),
            space: mem::size_of::<[u8; 32]>() * STATE_MERKLE_TREE_HEIGHT,
        },
        Type {
            name: "StateMerkleTree->changelog".to_owned(),
            space: mem::size_of::<ChangelogEntry26>() * STATE_MERKLE_TREE_CHANGELOG,
        },
        Type {
            name: "StateMerkleTree->roots".to_owned(),
            space: mem::size_of::<[u8; 32]>() * STATE_MERKLE_TREE_ROOTS,
        },
        Type {
            name: "IndexedArray".to_owned(),
            space: mem::size_of::<
                IndexingArray<Poseidon, u16, BigInteger256, STATE_INDEXED_ARRAY_SIZE>,
            >(),
        },
        Type {
            name: "AddressQueue".to_owned(),
            space: mem::size_of::<AddressQueue>(),
        },
        Type {
            name: "AddressMerkleTreeAccount (with discriminator)".to_owned(),
            space: mem::size_of::<AddressMerkleTreeAccount>() + 8,
        },
        Type {
            name: "AddressMerkleTree".to_owned(),
            space: mem::size_of::<AddressMerkleTree>(),
        },
        Type {
            name: "AddressMerkleTree->filled_subtrees".to_owned(),
            space: mem::size_of::<[u8; 32]>() * ADDRESS_MERKLE_TREE_HEIGHT,
        },
        Type {
            name: "AddressMerkleTree->changelog".to_owned(),
            space: mem::size_of::<ChangelogEntry22>() * ADDRESS_MERKLE_TREE_CHANGELOG,
        },
        Type {
            name: "AddressMerkleTree->roots".to_owned(),
            space: mem::size_of::<[u8; 32]>() * ADDRESS_MERKLE_TREE_ROOTS,
        },
    ];

    let table = Table::new(accounts);
    println!("{table}");

    Ok(())
}
