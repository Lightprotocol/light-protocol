use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use anchor_lang::solana_program::system_instruction;
use light_client::{
    indexer::Indexer,
    rpc::{RpcConnection, RpcError},
};
use light_compressed_account::address::derive_address;
use light_hasher::Poseidon;
use light_merkle_tree_reference::indexed::IndexedMerkleTree;
use num_bigint::BigUint;
use rand::{prelude::StdRng, Rng, SeedableRng};
use solana_sdk::{signature::Signer, transaction::Transaction};
use tokio::time::sleep;
use tracing::{debug, error};

use crate::error::ForesterUtilsError;

pub async fn airdrop_lamports<R: RpcConnection>(
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
        latest_blockhash,
    );
    rpc.process_transaction(transaction).await?;
    Ok(())
}

pub async fn wait_for_indexer<R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &I,
) -> Result<(), ForesterUtilsError> {
    let rpc_slot = rpc
        .get_slot()
        .await
        .map_err(|_| ForesterUtilsError::Rpc("Failed to get rpc slot".into()))?;

    let indexer_slot = indexer.get_indexer_slot(rpc).await;

    let mut indexer_slot = match indexer_slot {
        Ok(slot) => slot,
        Err(e) => {
            error!("failed to get indexer slot from indexer: {:?}", e);
            return Err(ForesterUtilsError::Indexer(
                "Failed to get indexer slot".into(),
            ));
        }
    };

    let max_attempts = 20;
    let mut attempts = 0;

    while rpc_slot > indexer_slot {
        if attempts >= max_attempts {
            return Err(ForesterUtilsError::Indexer(
                "Maximum attempts reached waiting for indexer to catch up".into(),
            ));
        }

        debug!(
            "waiting for indexer to catch up, rpc_slot: {}, indexer_slot: {}",
            rpc_slot, indexer_slot
        );
        sleep(std::time::Duration::from_millis(400)).await;
        indexer_slot = indexer.get_indexer_slot(rpc).await.map_err(|e| {
            error!("failed to get indexer slot from indexer: {:?}", e);
            ForesterUtilsError::Indexer("Failed to get indexer slot".into())
        })?;

        attempts += 1;
    }
    Ok(())
}

pub fn create_reference_address_tree(
    tree_pubkey: &Pubkey,
    seed: u64,
    addresses_count: u64,
) -> IndexedMerkleTree<Poseidon, usize> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut tree = IndexedMerkleTree::<Poseidon, usize>::new(40, 0).unwrap();
    for _ in 0..addresses_count {
        let seed = rng.gen();
        let address = derive_address(
            &seed,
            &tree_pubkey.to_bytes(),
            &create_address_test_program::ID.to_bytes(),
        );
        tree.append(&BigUint::from_bytes_be(&address)).unwrap();
    }
    tree
}
