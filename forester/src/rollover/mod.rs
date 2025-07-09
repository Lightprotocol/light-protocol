mod operations;
mod state;
use std::sync::Arc;

use forester_utils::forester_epoch::TreeAccounts;
use light_client::rpc::Rpc;
pub use operations::{
    get_tree_fullness, is_tree_ready_for_rollover, perform_address_merkle_tree_rollover,
    perform_state_merkle_tree_rollover_forester,
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
pub use state::RolloverState;
use tracing::info;

use crate::{errors::ForesterError, ForesterConfig};

pub async fn rollover_state_merkle_tree<R: Rpc>(
    config: Arc<ForesterConfig>,
    rpc: &mut R,
    tree_accounts: &TreeAccounts,
    epoch: u64,
) -> Result<(), ForesterError> {
    let new_nullifier_queue_keypair = Keypair::new();
    let new_merkle_tree_keypair = Keypair::new();
    let new_cpi_signature_keypair = Keypair::new();

    let rollover_signature = perform_state_merkle_tree_rollover_forester(
        &config.payer_keypair,
        &config.derivation_pubkey,
        rpc,
        &new_nullifier_queue_keypair,
        &new_merkle_tree_keypair,
        &new_cpi_signature_keypair,
        &tree_accounts.merkle_tree,
        &tree_accounts.queue,
        &Pubkey::default(),
        epoch,
    )
    .await?;

    info!("State rollover signature: {:?}", rollover_signature);
    Ok(())
}

pub async fn rollover_address_merkle_tree<R: Rpc>(
    config: Arc<ForesterConfig>,
    rpc: &mut R,
    tree_accounts: &TreeAccounts,
    epoch: u64,
) -> Result<(), ForesterError> {
    let new_nullifier_queue_keypair = Keypair::new();
    let new_merkle_tree_keypair = Keypair::new();

    let rollover_signature = perform_address_merkle_tree_rollover(
        &config.payer_keypair,
        &config.derivation_pubkey,
        rpc,
        &new_nullifier_queue_keypair,
        &new_merkle_tree_keypair,
        &tree_accounts.merkle_tree,
        &tree_accounts.queue,
        epoch,
    )
    .await?;

    info!("Address rollover signature: {:?}", rollover_signature);

    Ok(())
}
