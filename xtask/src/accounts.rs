use light_merkle_tree_program::state::MerkleTreeSet;
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct Account {
    name: String,
    space: usize,
}

pub fn accounts() -> anyhow::Result<()> {
    let accounts = vec![Account {
        name: "MerkleTreeSet".to_owned(),
        space: MerkleTreeSet::LEN,
    }];

    let table = Table::new(accounts);
    println!("{table}");

    Ok(())
}
