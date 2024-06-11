use account_compression::initialize_address_merkle_tree::Pubkey;
use account_compression::{AddressMerkleTreeAccount, QueueAccount};
use anchor_lang::AccountDeserialize;
use light_hash_set::HashSet;
use light_registry::sdk::{
    create_update_address_merkle_tree_instruction, UpdateAddressMerkleTreeInstructionInputs,
};
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::errors::RpcError;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::info;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::mem;

use crate::errors::ForesterError;
use crate::nullifier::Config;
pub async fn empty_address_queue<T: Indexer, R: RpcConnection>(
    rpc: &mut R,
    indexer: &mut T,
    payer: &Keypair,
    config: &Config,
) -> Result<(), ForesterError> {
    let address_merkle_tree_pubkey = config.address_merkle_tree_pubkey;
    let address_queue_pubkey = config.address_merkle_tree_queue_pubkey;
    let mut update_errors: Vec<RpcError> = Vec::new();

    let client = RpcClient::new_with_commitment(&config.server_url, CommitmentConfig::confirmed());

    loop {
        let data: &[u8] = &client.get_account_data(&address_merkle_tree_pubkey)?;
        let mut data_ref = data;
        let merkle_tree_account: AddressMerkleTreeAccount =
            AddressMerkleTreeAccount::try_deserialize(&mut data_ref)?;
        let merkle_tree = merkle_tree_account.copy_merkle_tree()?;
        info!(
            "address merkle_tree root: {:?}",
            merkle_tree.indexed_merkle_tree().root()
        );
        let mut nullifier_queue_account = client.get_account(&address_queue_pubkey)?;
        let address_queue: HashSet = unsafe {
            HashSet::from_bytes_copy(
                &mut nullifier_queue_account.data[8 + mem::size_of::<QueueAccount>()..],
            )?
        };

        let address = address_queue.first_no_seq().unwrap();
        if address.is_none() {
            break;
        }
        let (address, address_hashset_index) = address.unwrap();
        info!("address: {:?}", address);
        info!("address_hashset_index: {:?}", address_hashset_index);
        let proof = indexer
            .get_address_tree_proof(address_merkle_tree_pubkey.to_bytes(), address.value)
            .unwrap();
        info!("proof: {:?}", proof);

        info!("updating merkle tree...");
        let update_successful = match update_merkle_tree(
            rpc,
            payer,
            address_queue_pubkey,
            address_merkle_tree_pubkey,
            address_hashset_index,
            proof.low_address_index,
            proof.low_address_value,
            proof.low_address_next_index,
            proof.low_address_next_value,
            proof.low_address_proof,
        )
        .await
        {
            Ok(event) => {
                info!("event: {:?}", event);
                true
            }
            Err(e) => {
                update_errors.push(e);
                break;
            }
        };

        info!("update_successful: {:?}", update_successful);
        if update_successful {
            indexer.address_tree_updated(address_merkle_tree_pubkey.to_bytes(), proof)
        }
    }

    if update_errors.is_empty() {
        Ok(())
    } else {
        panic!("Errors: {:?}", update_errors);
    }
}

pub async fn get_changelog_index<R: RpcConnection>(
    merkle_tree_pubkey: &Pubkey,
    client: &mut R,
) -> Result<usize, ForesterError> {
    let merkle_tree_account: AddressMerkleTreeAccount = client
        .get_anchor_account::<AddressMerkleTreeAccount>(merkle_tree_pubkey)
        .await;
    let merkle_tree = merkle_tree_account.copy_merkle_tree()?;
    let changelog_index = merkle_tree
        .indexed_merkle_tree()
        .merkle_tree
        .changelog_index();
    Ok(changelog_index)
}

#[allow(clippy::too_many_arguments)]
pub async fn update_merkle_tree<R: RpcConnection>(
    rpc: &mut R,
    payer: &Keypair,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    value: u16,
    low_address_index: u64,
    low_address_value: [u8; 32],
    low_address_next_index: u64,
    low_address_next_value: [u8; 32],
    low_address_proof: [[u8; 32]; 16],
) -> Result<bool, RpcError> {
    info!("update_merkle_tree");
    let changelog_index = get_changelog_index(&address_merkle_tree_pubkey, rpc)
        .await
        .unwrap();
    info!("changelog_index: {:?}", changelog_index);

    let update_ix =
        create_update_address_merkle_tree_instruction(UpdateAddressMerkleTreeInstructionInputs {
            authority: payer.pubkey(),
            address_merkle_tree: address_merkle_tree_pubkey,
            address_queue: address_queue_pubkey,
            value,
            low_address_index,
            low_address_value,
            low_address_next_index,
            low_address_next_value,
            low_address_proof,
            changelog_index: changelog_index as u16,
        });
    info!("sending transaction...");

    let transaction = Transaction::new_signed_with_payer(
        &[update_ix],
        Some(&payer.pubkey()),
        &[&payer],
        rpc.get_latest_blockhash().await.unwrap(),
    );

    let signature = rpc.process_transaction(transaction).await?;
    let confirmed = rpc.confirm_transaction(signature).await?;
    Ok(confirmed)
}
