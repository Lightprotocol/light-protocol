use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use anchor_lang::solana_program::system_instruction;
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use solana_sdk::{signature::Signer, transaction::Transaction};
use tokio::time::sleep;
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

    let indexer_slot = rpc.indexer()?.get_indexer_slot(None).await;

    let mut indexer_slot = match indexer_slot {
        Ok(slot) => slot,
        Err(e) => {
            error!("failed to get indexer slot from indexer: {:?}", e);
            return Err(ForesterUtilsError::Indexer(
                "Failed to get indexer slot".into(),
            ));
        }
    };

    let max_attempts = 100;
    let mut attempts = 0;

    while rpc_slot > indexer_slot {
        if attempts >= max_attempts {
            return Err(ForesterUtilsError::Indexer(
                "Maximum attempts reached waiting for indexer to catch up".into(),
            ));
        }

        if rpc_slot - indexer_slot > 50 {
            warn!(
                "indexer is behind {} slots (rpc_slot: {}, indexer_slot: {})",
                rpc_slot - indexer_slot,
                rpc_slot,
                indexer_slot
            );
        }

        sleep(std::time::Duration::from_millis(1000)).await;
        indexer_slot = rpc.indexer()?.get_indexer_slot(None).await.map_err(|e| {
            error!("failed to get indexer slot from indexer: {:?}", e);
            ForesterUtilsError::Indexer("Failed to get indexer slot".into())
        })?;

        attempts += 1;
    }
    Ok(())
}
