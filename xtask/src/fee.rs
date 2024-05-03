use std::mem;

use account_compression::{
    initialize_nullifier_queue::NullifierQueueAccount,
    utils::constants::{
        ADDRESS_MERKLE_TREE_HEIGHT, ADDRESS_QUEUE_INDICES, ADDRESS_QUEUE_VALUES,
        STATE_MERKLE_TREE_HEIGHT, STATE_NULLIFIER_QUEUE_INDICES, STATE_NULLIFIER_QUEUE_VALUES,
    },
    AddressMerkleTreeAccount, AddressMerkleTreeConfig, AddressQueueAccount, AddressQueueConfig,
    NullifierQueueConfig, StateMerkleTreeAccount, StateMerkleTreeConfig,
};
use light_utils::fee::compute_rollover_fee;
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
    let nullifier_queue_config = NullifierQueueConfig::default();
    let address_merkle_tree_config = AddressMerkleTreeConfig::default();
    let address_queue_config = AddressQueueConfig::default();

    let fees = vec![
        AccountFee {
            account: "State Merkle tree (rollover)".to_owned(),
            fee: compute_rollover_fee(
                state_merkle_tree_config.rollover_threshold.unwrap(),
                state_merkle_tree_config.height,
                rent.minimum_balance(StateMerkleTreeAccount::size()),
            )?,
        },
        AccountFee {
            account: "Nullifier queue (rollover)".to_owned(),
            fee: compute_rollover_fee(
                state_merkle_tree_config.rollover_threshold.unwrap(),
                state_merkle_tree_config.height,
                rent.minimum_balance(NullifierQueueAccount::size(
                    nullifier_queue_config.capacity_indices,
                    nullifier_queue_config.capacity_values,
                )?),
            )?,
        },
        AccountFee {
            account: "Address queue (rollover)".to_owned(),
            fee: compute_rollover_fee(
                address_merkle_tree_config.rollover_threshold.unwrap(),
                address_merkle_tree_config.height,
                rent.minimum_balance(8 + mem::size_of::<AddressMerkleTreeAccount>()),
            )? + compute_rollover_fee(
                state_merkle_tree_config.rollover_threshold.unwrap(),
                address_merkle_tree_config.height,
                rent.minimum_balance(AddressQueueAccount::size(
                    ADDRESS_QUEUE_INDICES.into(),
                    ADDRESS_QUEUE_VALUES.into(),
                )?),
            )?,
        },
    ];

    let table = Table::new(fees);
    println!("{table}");

    Ok(())
}
