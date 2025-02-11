use std::mem;

use account_compression::{
    state::queue::QueueAccount,
    utils::constants::{
        ADDRESS_MERKLE_TREE_HEIGHT, ADDRESS_QUEUE_VALUES, STATE_MERKLE_TREE_HEIGHT,
        STATE_NULLIFIER_QUEUE_VALUES,
    },
    AddressMerkleTreeAccount, StateMerkleTreeAccount, StateMerkleTreeConfig,
};
use light_merkle_tree_metadata::fee::compute_rollover_fee;
use solana_program::rent::Rent;
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct AccountFee {
    account: String,
    fee: u64,
}

pub fn fees() -> anyhow::Result<()> {
    let rent = Rent::default();

    let state_merkle_tree_config = StateMerkleTreeConfig::default();

    let fees = vec![
        AccountFee {
            account: "State Merkle tree (rollover)".to_owned(),
            fee: compute_rollover_fee(
                state_merkle_tree_config.rollover_threshold.unwrap(),
                STATE_MERKLE_TREE_HEIGHT as u32,
                rent.minimum_balance(8 + mem::size_of::<StateMerkleTreeAccount>()),
            )? + compute_rollover_fee(
                state_merkle_tree_config.rollover_threshold.unwrap(),
                STATE_MERKLE_TREE_HEIGHT as u32,
                rent.minimum_balance(QueueAccount::size(STATE_NULLIFIER_QUEUE_VALUES as usize)?),
            )?,
        },
        AccountFee {
            account: "Address queue (rollover)".to_owned(),
            fee: compute_rollover_fee(
                state_merkle_tree_config.rollover_threshold.unwrap(),
                ADDRESS_MERKLE_TREE_HEIGHT as u32,
                rent.minimum_balance(8 + mem::size_of::<AddressMerkleTreeAccount>()),
            )? + compute_rollover_fee(
                state_merkle_tree_config.rollover_threshold.unwrap(),
                ADDRESS_MERKLE_TREE_HEIGHT as u32,
                rent.minimum_balance(QueueAccount::size(ADDRESS_QUEUE_VALUES.into())?),
            )?,
        },
    ];

    let table = Table::new(fees);
    println!("{table}");

    Ok(())
}
