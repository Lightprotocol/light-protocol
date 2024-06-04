use account_compression::initialize_address_merkle_tree::{AccountMeta, Pubkey};
use account_compression::instruction::UpdateAddressMerkleTree;
use account_compression::{AddressMerkleTreeAccount, QueueAccount, ID};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use light_hash_set::HashSet;
use light_registry::sdk::get_cpi_authority_pda;
use light_system_program::utils::get_registered_program_pda;
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::errors::RpcError;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::test_env::NOOP_PROGRAM_ID;
use log::info;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use std::mem;

use crate::errors::ForesterError;
use crate::nullifier::Config;
pub async fn empty_address_queue<T: Indexer, R: RpcConnection>(
    rpc: &mut R,
    indexer: &mut T,
    config: &Config,
) -> Result<(), ForesterError> {
    let address_merkle_tree_pubkey = config.address_merkle_tree_pubkey;
    let address_queue_pubkey = config.address_merkle_tree_queue_pubkey;
    let mut update_errors: Vec<RpcError> = Vec::new();

    let client = RpcClient::new(&config.server_url);

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
            address_queue_pubkey,
            address_merkle_tree_pubkey,
            address_hashset_index,
            proof.low_address_index,
            proof.low_address_value,
            proof.low_address_next_index,
            proof.low_address_next_value,
            proof.low_address_proof,
            None,
            Some(config.registry_pubkey),
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
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    value: u16,
    low_address_index: u64,
    low_address_value: [u8; 32],
    low_address_next_index: u64,
    low_address_next_value: [u8; 32],
    low_address_proof: [[u8; 32]; 16],
    _changelog_index: Option<u16>,
    registered_program: Option<Pubkey>,
) -> Result<bool, RpcError> {
    info!("update_merkle_tree");
    let changelog_index = get_changelog_index(&address_merkle_tree_pubkey, rpc)
        .await
        .unwrap();
    info!("changelog_index: {:?}", changelog_index);

    let update_ix = match registered_program {
        Some(registered_program) => {
            let register_program_pda = get_registered_program_pda(&registered_program);
            let (cpi_authority, bump) = get_cpi_authority_pda();
            let instruction_data = light_registry::instruction::UpdateAddressMerkleTree {
                bump,
                changelog_index: changelog_index as u16,
                value,
                low_address_index,
                low_address_value,
                low_address_next_index,
                low_address_next_value,
                low_address_proof,
            };
            let accounts = light_registry::accounts::UpdateMerkleTree {
                authority: rpc.get_payer().pubkey(),
                registered_program_pda: register_program_pda,
                queue: address_queue_pubkey,
                merkle_tree: address_merkle_tree_pubkey,
                log_wrapper: NOOP_PROGRAM_ID,
                cpi_authority,
                account_compression_program: ID,
            };
            Instruction {
                program_id: registered_program,
                accounts: accounts.to_account_metas(Some(true)),
                data: instruction_data.data(),
            }
        }
        None => {
            let instruction_data = UpdateAddressMerkleTree {
                changelog_index: changelog_index as u16,
                value,
                low_address_index,
                low_address_value,
                low_address_next_index,
                low_address_next_value,
                low_address_proof,
            };
            Instruction {
                program_id: ID,
                accounts: vec![
                    AccountMeta::new(rpc.get_payer().pubkey(), true),
                    AccountMeta::new(ID, false),
                    AccountMeta::new(address_queue_pubkey, false),
                    AccountMeta::new(address_merkle_tree_pubkey, false),
                    AccountMeta::new(NOOP_PROGRAM_ID, false),
                ],
                data: instruction_data.data(),
            }
        }
    };
    info!("sending transaction...");
    let payer = rpc.get_payer().insecure_clone();

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
