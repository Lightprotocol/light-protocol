use std::mem;

use account_compression::{
    initialize_nullifier_queue::IndexedArrayAccount,
    utils::constants::{
        ADDRESS_MERKLE_TREE_CANOPY_DEPTH, ADDRESS_MERKLE_TREE_CHANGELOG,
        ADDRESS_MERKLE_TREE_HEIGHT, ADDRESS_MERKLE_TREE_ROOTS, ADDRESS_QUEUE_INDICES,
        ADDRESS_QUEUE_VALUES, STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_CHANGELOG,
        STATE_MERKLE_TREE_HEIGHT, STATE_MERKLE_TREE_ROOTS, STATE_NULLIFIER_QUEUE_INDICES,
        STATE_NULLIFIER_QUEUE_VALUES,
    },
    AddressMerkleTreeAccount, AddressQueueAccount, NullifierQueueAccount, StateMerkleTreeAccount,
};
use light_concurrent_merkle_tree::{changelog::ChangelogEntry26, ConcurrentMerkleTree26};
use light_hasher::Poseidon;
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
            space: mem::size_of::<ConcurrentMerkleTree26<Poseidon>>(),
        },
        Type {
            name: "StateMerkleTree->filled_subtrees".to_owned(),
            space: mem::size_of::<[u8; 32]>() * STATE_MERKLE_TREE_HEIGHT as usize,
        },
        Type {
            name: "StateMerkleTree->changelog".to_owned(),
            space: mem::size_of::<ChangelogEntry26>() * STATE_MERKLE_TREE_CHANGELOG as usize,
        },
        Type {
            name: "StateMerkleTree->roots".to_owned(),
            space: mem::size_of::<[u8; 32]>() * STATE_MERKLE_TREE_ROOTS as usize,
        },
        Type {
            name: "StateMerkleTree->canopy".to_owned(),
            space: mem::size_of::<[u8; 32]>()
                * ConcurrentMerkleTree26::<Poseidon>::canopy_size(
                    STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
                ),
        },
        Type {
            name: "IndexedArrayAccount".to_owned(),
            space: NullifierQueueAccount::size(
                STATE_NULLIFIER_QUEUE_INDICES as usize,
                STATE_NULLIFIER_QUEUE_VALUES as usize,
            )
            .unwrap(),
        },
        Type {
            name: "AddressQueue".to_owned(),
            space: AddressQueueAccount::size(
                ADDRESS_QUEUE_INDICES as usize,
                ADDRESS_QUEUE_VALUES as usize,
            )
            .unwrap(),
        },
        Type {
            name: "AddressMerkleTreeAccount (with discriminator)".to_owned(),
            space: mem::size_of::<AddressMerkleTreeAccount>() + 8,
        },
        Type {
            name: "AddressMerkleTree".to_owned(),
            space: mem::size_of::<ConcurrentMerkleTree26<Poseidon>>(),
        },
        Type {
            name: "AddressMerkleTree->filled_subtrees".to_owned(),
            space: mem::size_of::<[u8; 32]>() * ADDRESS_MERKLE_TREE_HEIGHT as usize,
        },
        Type {
            name: "AddressMerkleTree->changelog".to_owned(),
            space: mem::size_of::<ChangelogEntry26>() * ADDRESS_MERKLE_TREE_CHANGELOG as usize,
        },
        Type {
            name: "AddressMerkleTree->roots".to_owned(),
            space: mem::size_of::<[u8; 32]>() * ADDRESS_MERKLE_TREE_ROOTS as usize,
        },
        Type {
            name: "AddressMerkleTree->canopy".to_owned(),
            space: mem::size_of::<[u8; 32]>()
                * ConcurrentMerkleTree26::<Poseidon>::canopy_size(
                    ADDRESS_MERKLE_TREE_CANOPY_DEPTH as usize,
                ),
        },
    ];

    let table = Table::new(accounts);
    println!("{table}");

    Ok(())
}
