use std::{sync::Arc, time::Duration, vec};

use account_compression::utils::constants::{
    ADDRESS_MERKLE_TREE_CHANGELOG, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG, ADDRESS_QUEUE_VALUES,
    STATE_MERKLE_TREE_CHANGELOG, STATE_NULLIFIER_QUEUE_VALUES,
};
use async_trait::async_trait;
use forester_utils::{forester_epoch::{TreeAccounts, TreeType}, rpc_pool::RpcPool};
use futures::future::join_all;
use light_client::{
    indexer::Indexer,
    rpc::RpcConnection,
};
use forester_utils::solana_rpc::RetryConfig;
use light_registry::{
    account_compression_cpi::sdk::{
        create_nullify_instruction, create_update_address_merkle_tree_instruction,
        CreateNullifyInstructionInputs, UpdateAddressMerkleTreeInstructionInputs,
    },
    utils::get_forester_epoch_pda_from_authority,
};
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::{
    bs58,
    commitment_config::CommitmentLevel,
    hash::Hash,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use tokio::{
    join,
    sync::Mutex,
    time::{sleep, Instant},
};
use tracing::{debug, warn};
use url::Url;

use crate::{
    config::QueueConfig,
    epoch_manager::{MerkleProofType, WorkItem},
    errors::ForesterError,
    helius_priority_fee_types::{
        GetPriorityFeeEstimateOptions, GetPriorityFeeEstimateRequest,
        GetPriorityFeeEstimateResponse, RpcRequest, RpcResponse,
    },
    queue_helpers::fetch_queue_item_data,
    smart_transaction::{
        create_smart_transaction, send_and_confirm_transaction, CreateSmartTransactionConfig,
    },
    Result,
};
#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait TransactionBuilder {
    fn epoch(&self) -> u64;
    async fn build_signed_transaction_batch(
        &self,
        payer: &Keypair,
        derivation: &Pubkey,
        recent_blockhash: &Hash,
        last_valid_block_height: u64,
        priority_fee: u64,
        work_items: &[WorkItem],
        config: BuildTransactionBatchConfig,
    ) -> Result<(Vec<Transaction>, u64)>;
}

// We're assuming that:
// 1. Helius slot latency is ~ 3 slots.
// See also: https://p.us5.datadoghq.com/sb/339e0590-c5d4-11ed-9c7b-da7ad0900005-231a672007c47d70f38e8fa321bc8407?fromUser=false&refresh_mode=sliding&tpl_var_leader_name%5B0%5D=%2A&from_ts=1725348612900&to_ts=1725953412900&live=true
// 2. Latency between forester server and helius is ~ 1 slot.
// 3. Slot duration is 500ms.
const LATENCY: Duration = Duration::from_millis(4 * 500);

const TIMEOUT_CHECK_ENABLED: bool = true;

/// Calculate the compute unit price in microLamports based on the target lamports and compute units
pub fn calculate_compute_unit_price(target_lamports: u64, compute_units: u64) -> u64 {
    ((target_lamports * 1_000_000) as f64 / compute_units as f64).ceil() as u64
}

/// Setting:
/// 1. We have 1 light slot (n solana slots), and elements in thequeue
/// 2. we want to send as many elements from the queue as possible
///
/// Strategy:
/// 1. Execute transaction batches until max number of batches is
///    reached or light slot ended (global timeout).
/// 2. Fetch queue items.
/// 3. If work items is empty, await minimum batch time.
/// 4. Fetch recent blockhash.
/// 5. Iterate over work items in chunks of batch size.
/// 6. Check if we reached the end of the light slot.
/// 7. Asynchronously send all transactions in the batch
/// 8. Await minimum batch time.
/// 9. Check if we reached max number of batches.
///
/// Questions:
/// - How do we make sure that we have send all the transactions?
///
/// TODO:
/// - return number of sent transactions
/// - test timeout for any action of this function or subfunctions, timeout is
///   end of slot
/// - consider dynamic batch size based on the number of transactions in the
///   queue
pub async fn send_batched_transactions<T: TransactionBuilder, R: RpcConnection, P: RpcPool<R>>(
    payer: &Keypair,
    derivation: &Pubkey,
    pool: Arc<P>,
    config: &SendBatchedTransactionsConfig,
    tree_accounts: TreeAccounts,
    transaction_builder: &T,
) -> Result<usize> {
    let start_time = Instant::now();

    let mut rpc = pool.get_connection().await?;

    let mut num_batches = 0;
    let mut num_sent_transactions: usize = 0;
    // 1. Execute batches until max number of batches is reached or light slot
    //    ended (light_slot_duration)
    while num_batches < config.num_batches && (start_time.elapsed() < config.retry_config.timeout) {
        debug!("Sending batch: {}", num_batches);
        // 2. Fetch queue items.
        let queue_length = if tree_accounts.tree_type == TreeType::State {
            STATE_NULLIFIER_QUEUE_VALUES
        } else {
            ADDRESS_QUEUE_VALUES
        };
        let start_index = if tree_accounts.tree_type == TreeType::State {
            config.queue_config.state_queue_start_index
        } else {
            config.queue_config.address_queue_start_index
        };
        let length = if tree_accounts.tree_type == TreeType::State {
            config.queue_config.state_queue_length
        } else {
            config.queue_config.address_queue_length
        };
        let queue_item_data = fetch_queue_item_data(
            &mut *rpc,
            &tree_accounts.queue,
            start_index,
            length,
            queue_length,
        )
        .await?;
        let work_items: Vec<WorkItem> = queue_item_data
            .into_iter()
            .map(|data| WorkItem {
                tree_account: tree_accounts,
                queue_item_data: data,
            })
            .collect();

        // 3. If work items is empty, await minimum batch time.
        // If this is triggered we could switch to subscribing to the queue
        if work_items.is_empty() {
            debug!("No work items found, waiting for next batch");
            sleep(config.retry_config.retry_delay).await;
            continue;
        }

        // 4. Fetch recent confirmed blockhash.
        // A blockhash is valid for 150 blocks.
        let recent_blockhash = rpc.get_latest_blockhash().await?;
        let current_block_height = rpc.get_block_height().await?;
        let last_valid_block_height = current_block_height + 150;

        let forester_epoch_pda_pubkey =
            get_forester_epoch_pda_from_authority(derivation, transaction_builder.epoch()).0;
        // Get the priority fee estimate based on write-locked accounts
        let account_keys = vec![
            payer.pubkey(),
            forester_epoch_pda_pubkey,
            tree_accounts.queue,
            tree_accounts.merkle_tree,
        ];
        let url = Url::parse(&rpc.get_url()).expect("Failed to parse URL");
        let priority_fee_recommendation: u64 =
            request_priority_fee_estimate(&url, account_keys).await?;

        // Cap the priority fee and CU usage with buffer.
        let cap_config = CapConfig {
            rec_fee_microlamports_per_cu: priority_fee_recommendation,
            min_fee_lamports: config
                .build_transaction_batch_config
                .compute_unit_price
                .unwrap_or(10_000),
            max_fee_lamports: config
                .build_transaction_batch_config
                .compute_unit_price
                .unwrap_or(100_000),
            compute_unit_limit: config
                .build_transaction_batch_config
                .compute_unit_limit
                .unwrap_or(200_000) as u64,
        };
        let priority_fee = get_capped_priority_fee(cap_config);

        // 5. Iterate over work items in chunks of batch size.
        for work_items in
            work_items.chunks(config.build_transaction_batch_config.batch_size as usize)
        {
            // 6. Check if we reached the end of the light slot.
            if TIMEOUT_CHECK_ENABLED {
                let remaining_time =
                    get_remaining_time_in_light_slot(start_time, config.retry_config.timeout);
                if remaining_time < LATENCY {
                    debug!("Reached end of light slot");
                    break;
                }
            }

            // Minimum time to wait for the next batch of transactions. Can be
            // used to avoid rate limits. TODO(swen): check max feasible batch
            // size and latency for large tx batches. TODO: add global rate
            // limit across our instances and queues: max 100 RPS global.
            let transaction_build_time_start = Instant::now();
            let (transactions, _block_height) = transaction_builder
                .build_signed_transaction_batch(
                    payer,
                    derivation,
                    &recent_blockhash,
                    last_valid_block_height,
                    priority_fee,
                    work_items,
                    config.build_transaction_batch_config,
                )
                .await?;
            debug!(
                "build transaction time {:?}",
                transaction_build_time_start.elapsed()
            );

            let batch_start = Instant::now();
            if TIMEOUT_CHECK_ENABLED {
                let remaining_time =
                    get_remaining_time_in_light_slot(start_time, config.retry_config.timeout);
                if remaining_time < LATENCY {
                    debug!("Reached end of light slot");
                    break;
                }
            }

            let send_transaction_config = RpcSendTransactionConfig {
                // Use required settings for routing through staked connection:
                // https://docs.helius.dev/guides/sending-transactions-on-solana
                skip_preflight: true,
                max_retries: Some(0),
                preflight_commitment: Some(CommitmentLevel::Confirmed),
                ..Default::default()
            };
            // Send and confirm all transactions in the batch non-blocking.
            let send_futures: Vec<_> = transactions
                .into_iter()
                .map(|tx| {
                    let pool_clone = Arc::clone(&pool);
                    async move {
                        match pool_clone.get_connection().await {
                            Ok(mut rpc) => {
                                send_and_confirm_transaction(
                                    &mut rpc,
                                    &tx,
                                    send_transaction_config,
                                    last_valid_block_height,
                                    config.retry_config.timeout,
                                )
                                .await
                            }
                            Err(e) => Err(light_client::rpc::RpcError::CustomError(format!(
                                "Failed to get RPC connection: {}",
                                e
                            ))),
                        }
                    }
                })
                .collect();

            let results = join_all(send_futures).await;

            // Evaluate results
            for result in results {
                match result {
                    Ok(signature) => {
                        num_sent_transactions += 1;
                        println!("Transaction sent: {:?}", signature);
                    }
                    Err(e) => warn!("Transaction failed: {:?}", e),
                }
            }

            num_batches += 1;
            let batch_duration = batch_start.elapsed();
            debug!("Batch duration: {:?}", batch_duration);

            // 8. Await minimum batch time.
            if start_time.elapsed() + config.retry_config.retry_delay < config.retry_config.timeout
            {
                sleep(config.retry_config.retry_delay).await;
            } else {
                break;
            }

            // 9. Check if we reached max number of batches.
            if num_batches >= config.num_batches {
                debug!("Reached max number of batches");
                break;
            }
        }
    }

    debug!("Sent {} transactions", num_sent_transactions);
    Ok(num_sent_transactions)
}

#[derive(Debug, Clone, Copy)]
pub struct CapConfig {
    pub rec_fee_microlamports_per_cu: u64,
    pub min_fee_lamports: u64,
    pub max_fee_lamports: u64,
    pub compute_unit_limit: u64,
}

fn get_remaining_time_in_light_slot(start_time: Instant, timeout: Duration) -> Duration {
    timeout.saturating_sub(start_time.elapsed())
}

#[derive(Debug, Clone, Copy)]
pub struct SendBatchedTransactionsConfig {
    pub num_batches: u64,
    pub build_transaction_batch_config: BuildTransactionBatchConfig,
    pub queue_config: QueueConfig,
    pub retry_config: RetryConfig,
    pub light_slot_length: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct BuildTransactionBatchConfig {
    pub batch_size: u64,
    pub compute_unit_price: Option<u64>,
    pub compute_unit_limit: Option<u32>,
}

pub struct EpochManagerTransactions<R: RpcConnection, I: Indexer<R>> {
    pub indexer: Arc<Mutex<I>>,
    pub epoch: u64,
    pub phantom: std::marker::PhantomData<R>,
}

#[async_trait]
impl<R: RpcConnection, I: Indexer<R>> TransactionBuilder for EpochManagerTransactions<R, I> {
    fn epoch(&self) -> u64 {
        self.epoch
    }

    async fn build_signed_transaction_batch(
        &self,
        payer: &Keypair,
        derivation: &Pubkey,
        recent_blockhash: &Hash,
        last_valid_block_height: u64,
        priority_fee: u64,
        work_items: &[WorkItem],
        config: BuildTransactionBatchConfig,
    ) -> Result<(Vec<Transaction>, u64)> {
        let mut transactions = vec![];
        let (_, all_instructions) = fetch_proofs_and_create_instructions(
            payer.pubkey(),
            *derivation,
            self.indexer.clone(),
            self.epoch,
            work_items,
        )
        .await?;

        for instruction in all_instructions {
            let (transaction, _) = create_smart_transaction(CreateSmartTransactionConfig {
                payer: payer.insecure_clone(),
                instructions: vec![instruction],
                recent_blockhash: *recent_blockhash,
                compute_unit_price: Some(priority_fee),
                compute_unit_limit: config.compute_unit_limit,
                last_valid_block_hash: last_valid_block_height,
            })
            .await?;
            transactions.push(transaction);
        }
        Ok((transactions, last_valid_block_height))
    }
}

/// Work items should be of only one type and tree
pub async fn fetch_proofs_and_create_instructions<R: RpcConnection, I: Indexer<R>>(
    authority: Pubkey,
    derivation: Pubkey,
    indexer: Arc<Mutex<I>>,
    epoch: u64,
    work_items: &[WorkItem],
) -> Result<(Vec<MerkleProofType>, Vec<Instruction>)> {
    let mut proofs = Vec::new();
    let mut instructions = vec![];

    let (address_items, state_items): (Vec<_>, Vec<_>) = work_items
        .iter()
        .partition(|item| matches!(item.tree_account.tree_type, TreeType::Address));

    // Prepare data for batch fetching
    let address_data = if !address_items.is_empty() {
        let merkle_tree = address_items
            .first()
            .ok_or_else(|| ForesterError::General {
                error: "No address items found".to_string(),
            })?
            .tree_account
            .merkle_tree
            .to_bytes();
        let addresses: Vec<[u8; 32]> = address_items
            .iter()
            .map(|item| item.queue_item_data.hash)
            .collect();
        Some((merkle_tree, addresses))
    } else {
        None
    };

    let state_data = if !state_items.is_empty() {
        let states: Vec<String> = state_items
            .iter()
            .map(|item| bs58::encode(&item.queue_item_data.hash).into_string())
            .collect();
        Some(states)
    } else {
        None
    };

    // Fetch all proofs in parallel
    let (address_proofs, state_proofs) = {
        let indexer = indexer.lock().await;

        let address_future = async {
            if let Some((merkle_tree, addresses)) = address_data {
                indexer
                    .get_multiple_new_address_proofs(merkle_tree, addresses)
                    .await
            } else {
                Ok(vec![])
            }
        };

        let state_future = async {
            if let Some(states) = state_data {
                indexer.get_multiple_compressed_account_proofs(states).await
            } else {
                Ok(vec![])
            }
        };

        join!(address_future, state_future)
    };

    let address_proofs = address_proofs?;
    let state_proofs = state_proofs?;

    // Process address proofs and create instructions
    for (item, proof) in address_items.iter().zip(address_proofs.into_iter()) {
        proofs.push(MerkleProofType::AddressProof(proof.clone()));
        let instruction = create_update_address_merkle_tree_instruction(
            UpdateAddressMerkleTreeInstructionInputs {
                authority,
                derivation,
                address_merkle_tree: item.tree_account.merkle_tree,
                address_queue: item.tree_account.queue,
                value: item.queue_item_data.index as u16,
                low_address_index: proof.low_address_index,
                low_address_value: proof.low_address_value,
                low_address_next_index: proof.low_address_next_index,
                low_address_next_value: proof.low_address_next_value,
                low_address_proof: proof.low_address_proof,
                changelog_index: (proof.root_seq % ADDRESS_MERKLE_TREE_CHANGELOG) as u16,
                indexed_changelog_index: (proof.root_seq % ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG)
                    as u16,
                is_metadata_forester: false,
            },
            epoch,
        );
        instructions.push(instruction);
    }

    // Process state proofs and create instructions
    for (item, proof) in state_items.iter().zip(state_proofs.into_iter()) {
        proofs.push(MerkleProofType::StateProof(proof.clone()));
        let instruction = create_nullify_instruction(
            CreateNullifyInstructionInputs {
                nullifier_queue: item.tree_account.queue,
                merkle_tree: item.tree_account.merkle_tree,
                change_log_indices: vec![proof.root_seq % STATE_MERKLE_TREE_CHANGELOG],
                leaves_queue_indices: vec![item.queue_item_data.index as u16],
                indices: vec![proof.leaf_index],
                proofs: vec![proof.proof.clone()],
                authority,
                derivation,
                is_metadata_forester: false,
            },
            epoch,
        );
        instructions.push(instruction);
    }

    Ok((proofs, instructions))
}

/// Request priority fee estimate from Helius RPC endpoint
pub async fn request_priority_fee_estimate(url: &Url, account_keys: Vec<Pubkey>) -> Result<u64> {
    if url.host_str() == Some("localhost") {
        return Ok(10_000);
    }

    let priority_fee_request = GetPriorityFeeEstimateRequest {
        transaction: None,
        account_keys: Some(
            account_keys
                .iter()
                .map(|pubkey| bs58::encode(pubkey).into_string())
                .collect(),
        ),
        options: Some(GetPriorityFeeEstimateOptions {
            include_all_priority_fee_levels: None,
            recommended: Some(true),
            include_vote: None,
            lookback_slots: None,
            priority_level: None,
            transaction_encoding: None,
        }),
    };

    let rpc_request = RpcRequest::new(
        "getPriorityFeeEstimate".to_string(),
        serde_json::json!({
            "get_priority_fee_estimate_request": priority_fee_request
        }),
    );

    let client = reqwest::Client::new();
    let response = client
        .post(url.clone())
        .header("Content-Type", "application/json")
        .json(&rpc_request)
        .send()
        .await?;

    let response_text = response.text().await?;

    let response: RpcResponse<GetPriorityFeeEstimateResponse> =
        serde_json::from_str(&response_text)?;

    response
        .result
        .priority_fee_estimate
        .map(|estimate| estimate as u64)
        .ok_or(
            ForesterError::General {
                error: "Priority fee estimate not available".to_string(),
            }
            .into(),
        )
}

/// Get capped priority fee for transaction between min and max.
pub fn get_capped_priority_fee(cap_config: CapConfig) -> u64 {
    if cap_config.max_fee_lamports < cap_config.min_fee_lamports {
        panic!("Max fee is less than min fee");
    }

    let priority_fee_max =
        calculate_compute_unit_price(cap_config.max_fee_lamports, cap_config.compute_unit_limit);
    let priority_fee_min =
        calculate_compute_unit_price(cap_config.min_fee_lamports, cap_config.compute_unit_limit);
    let capped_fee = std::cmp::min(cap_config.rec_fee_microlamports_per_cu, priority_fee_max);
    std::cmp::max(capped_fee, priority_fee_min)
}
