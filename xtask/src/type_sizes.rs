use std::mem;

use light_merkle_tree_program::state::{EventMerkleTree, MerkleTreeSet, StateMerkleTree};
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct Type {
    name: String,
    space: usize,
}

pub fn type_sizes() -> anyhow::Result<()> {
    let accounts = vec![
        Type {
            name: "MerkleTreeSet".to_owned(),
            space: MerkleTreeSet::LEN,
        },
        Type {
            name: "StateMerkleTree".to_owned(),
            space: mem::size_of::<StateMerkleTree>(),
        },
        Type {
            name: "EventMerkleTree".to_owned(),
            space: mem::size_of::<EventMerkleTree>(),
        },
    ];

    let table = Table::new(accounts);
    println!("{table}");

    Ok(())
}
