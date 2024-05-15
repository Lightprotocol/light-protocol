use crate::errors::CrankError;
use crate::indexer::get_compressed_account_proof;
use account_compression::processor::initialize_nullifier_queue::NullifierQueueAccount;
use account_compression::StateMerkleTreeAccount;
use anchor_lang::AccountDeserialize;
use light_hash_set::HashSet;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::mem;
use std::sync::Arc;
use tokio::sync::Semaphore;

const CONCURRENCY_LIMIT: usize = 10;
pub async fn nullify(
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    payer_keypair: Arc<Keypair>,
    server_url: String,
) -> Result<(), CrankError> {
    let semaphore = Arc::new(Semaphore::new(CONCURRENCY_LIMIT));

    // This provides the starting point for the onchain merkle tree to start looking at changelogs
    // to patch the proof. If the changelog index is greater than the indexers Merkle tree state
    // some changelogs that might be required to fix the proof will not be used and the proof might
    // not be patched and verification fail.
    // The changelog index needs to be in sync with the indexer Merkle tree.
    // indexer Merkle tree next index with onchain Merkle tree next index -> diff
    // change_log_index - diff
    let (change_log_index, sequence_number) = {
        let temporary_client = RpcClient::new(&server_url);
        get_changelog_index(merkle_tree_pubkey, &temporary_client)?
    };
    let mut compressed_accounts_to_nullify = {
        let temporary_client = RpcClient::new(&server_url);
        get_nullifier_queue(nullifier_queue_pubkey, &temporary_client)?
    };

    let mut tasks = vec![];

    while !compressed_accounts_to_nullify.is_empty() {
        let permit = Arc::clone(&semaphore).acquire_owned().await;
        let (index_in_nullifier_queue, compressed_account) = compressed_accounts_to_nullify.remove(0);
        let c_payer_keypair = payer_keypair.clone();
        let clone_nullifier_queue_pubkey = *nullifier_queue_pubkey; // Cloning
        let clone_merkle_tree_pubkey = *merkle_tree_pubkey; // Cloning
        let clone_server_url = server_url.clone(); // Cloning
        // nullify_compressed_account being spawned in a separate task concurrently
        let task = tokio::spawn(async move {
            let _permit = permit;
            let client = RpcClient::new(&clone_server_url);
            let mut retries = 5;
            while retries > 0 {
                match nullify_compressed_account(
                    index_in_nullifier_queue,
                    &compressed_account,
                    change_log_index,
                    sequence_number as i64,
                    &clone_nullifier_queue_pubkey,
                    &clone_merkle_tree_pubkey,
                    c_payer_keypair.clone(),
                    &client,
                )
                    .await
                {
                    Ok(_) => {
                        break;
                    }
                    Err(e) => {
                        retries -= 1;
                        println!("Error: {}", e);
                        println!("Retrying nullify compressed account, retries left: {}", retries);
                    }
                }
            }
        });
        tasks.push(task);
    }

    // Wait for all tasks to complete
    for task in tasks {
        match task.await {
            Ok(_) => {}
            Err(e) => {
                println!("Task ended with error {}", e);
            }
        }
    }

    Ok(())
}

pub async fn nullify_compressed_account(
    index_in_nullifier_queue: usize,
    compressed_account: &[u8],
    change_log_index: usize,
    sequence_number: i64,
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    payer_keypair: Arc<Keypair>,
    client: &RpcClient,
) -> Result<(), CrankError> {
    println!("Nullifying compressed account");
    let nullifier_queue_pubkey = *nullifier_queue_pubkey;
    let merkle_tree_pubkey = *merkle_tree_pubkey;

    println!(
        "Nullifying account with index: {}",
        index_in_nullifier_queue
    );
    let account = bs58::encode(compressed_account).into_string();
    println!(
        "Getting compressed account proof for account: {:?}",
        account
    );

    let (proof, leaf_index, root_seq) = get_compressed_account_proof(&account).await?;

    // root sequence is current the same as sequence number
    let diff = root_seq - sequence_number;
    // let diff = if root_seq >= sequence_number {
    //     root_seq - sequence_number
    // } else {
    //     return Err(CrankError::Custom(format!("root_seq({}) is less than sequence_number({}).", root_seq, sequence_number)))
    // };

    // root_seq: 797, sequence_number: 945, diff: -148, change_log_index: 945
    println!("root_seq: {}, sequence_number: {}, diff: {}, change_log_index: {}", root_seq, sequence_number, diff, change_log_index);

    let change_log_index = change_log_index + diff as usize;
    // let change_log_index = change_log_index.checked_sub(diff as usize)
    //     .ok_or_else(|| CrankError::Custom("Underflow when updating change_log_index".to_string()))?;

    println!("Leaf index: {:?}", leaf_index);

    println!("Sending transaction with account_compression::nullify_leaves::sdk_nullify::create_nullify_instruction");
    let time = std::time::Instant::now();
    let instructions = [
        account_compression::nullify_leaves::sdk_nullify::create_nullify_instruction(
            vec![change_log_index as u64].as_slice(),
            vec![(index_in_nullifier_queue) as u16].as_slice(),
            vec![leaf_index].as_slice(),
            vec![proof].as_slice(),
            &payer_keypair.pubkey(),
            &merkle_tree_pubkey,
            &nullifier_queue_pubkey,
        ),
    ];
    let latest_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer_keypair.pubkey()),
        &[&payer_keypair],
        latest_blockhash,
    );
    let tx_result = client.send_and_confirm_transaction(&transaction)?;
    println!("Time elapsed: {:?}", time.elapsed());
    println!("Transaction signature: {:?}", tx_result);

    Ok(())
}

pub fn get_nullifier_queue(
    nullifier_queue_pubkey: &Pubkey,
    client: &RpcClient,
) -> Result<Vec<(usize, [u8; 32])>, CrankError> {
    let mut nullifier_queue_account = client.get_account(nullifier_queue_pubkey)?;
    let nullifier_queue: HashSet<u16> = unsafe {
        HashSet::from_bytes_copy(
            &mut nullifier_queue_account.data[8 + mem::size_of::<NullifierQueueAccount>()..],
        )?
    };

    let mut compressed_accounts_to_nullify = Vec::new();
    for (i, element) in nullifier_queue.iter() {
        if element.sequence_number().is_none() {
            compressed_accounts_to_nullify.push((i, element.value_bytes()));
        }
    }
    Ok(compressed_accounts_to_nullify)
}

pub fn get_changelog_index(
    merkle_tree_pubkey: &Pubkey,
    client: &RpcClient,
) -> Result<(usize, usize), CrankError> {
    let data: &[u8] = &client.get_account_data(merkle_tree_pubkey)?;
    let mut data_ref = &data[..];
    let merkle_tree_account: StateMerkleTreeAccount =
        StateMerkleTreeAccount::try_deserialize(&mut data_ref)?;
    let merkle_tree = merkle_tree_account.copy_merkle_tree()?;
    Ok((
        merkle_tree.current_changelog_index,
        merkle_tree.sequence_number,
    ))
}
