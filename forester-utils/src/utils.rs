use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use anchor_lang::solana_program::system_instruction;
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use solana_sdk::{signature::Signer, transaction::Transaction};
use tracing::{error, warn};

use crate::error::ForesterUtilsError;

pub async fn airdrop_lamports<R: Rpc>(
    rpc: &mut R,
    destination_pubkey: &Pubkey,
    lamports: u64,
) -> Result<(), RpcError> {
    let transfer_instruction =
        system_instruction::transfer(&rpc.get_payer().pubkey(), destination_pubkey, lamports);
    let latest_blockhash = rpc.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&rpc.get_payer().pubkey()),
        &vec![&rpc.get_payer()],
        latest_blockhash.0,
    );
    rpc.process_transaction_with_context(transaction).await?;
    Ok(())
}

pub async fn wait_for_indexer<R: Rpc>(rpc: &R) -> Result<(), ForesterUtilsError> {
    let rpc_slot = rpc.get_slot().await?;
    let indexer_slot = match rpc.indexer()?.get_indexer_slot(None).await {
        Ok(slot) => slot,
        Err(e) => {
            error!("failed to get indexer slot from indexer: {:?}", e);
            return Err(ForesterUtilsError::Indexer(
                "Failed to get indexer slot".into(),
            ));
        }
    };

    let max_lag_slots = std::env::var("INDEXER_MAX_LAG_SLOTS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(100);
    let lag = rpc_slot.saturating_sub(indexer_slot);
    if lag > max_lag_slots {
        warn!(
            lag,
            max_lag_slots, rpc_slot, indexer_slot, "indexer freshness gate rejected proof work"
        );
        return Err(ForesterUtilsError::Indexer(format!(
            "Indexer is behind {lag} slots (maximum allowed: {max_lag_slots})"
        )));
    }
    Ok(())
}
