use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
    vec,
};

use account_compression::utils::constants::{
    ADDRESS_MERKLE_TREE_CHANGELOG, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG, ADDRESS_QUEUE_VALUES,
    STATE_MERKLE_TREE_CHANGELOG, STATE_NULLIFIER_QUEUE_VALUES,
};
use async_trait::async_trait;
use forester_utils::forester_epoch::TreeAccounts;
use light_client::{
    indexer::Indexer,
    rpc::{RetryConfig, RpcConnection, RpcError},
    rpc_pool::SolanaRpcPool,
};
use light_compressed_account::TreeType;
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
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use tokio::{join, sync::Mutex, time::Instant};
use tracing::{info, warn};
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
    smart_transaction::{create_smart_transaction, CreateSmartTransactionConfig},
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
pub async fn send_batched_transactions<T: TransactionBuilder, R: RpcConnection>(
    payer: &Keypair,
    derivation: &Pubkey,
    pool: Arc<SolanaRpcPool<R>>,
    config: &SendBatchedTransactionsConfig,
    tree_accounts: TreeAccounts,
    transaction_builder: &T,
) -> Result<usize> {
    let start_time = Instant::now();

    let num_sent_transactions = Arc::new(AtomicUsize::new(0));
    let cancel_signal = Arc::new(AtomicBool::new(false));
    let (tx_sender, mut tx_receiver) =
        tokio::sync::mpsc::channel::<std::result::Result<Signature, RpcError>>(120);

    let processor_pool = pool.clone();
    let queue_length = if tree_accounts.tree_type == TreeType::StateV1 {
        STATE_NULLIFIER_QUEUE_VALUES
    } else {
        ADDRESS_QUEUE_VALUES
    };
    let start_index = if tree_accounts.tree_type == TreeType::StateV1 {
        config.queue_config.state_queue_start_index
    } else {
        config.queue_config.address_queue_start_index
    };
    let length = if tree_accounts.tree_type == TreeType::StateV1 {
        config.queue_config.state_queue_length
    } else {
        config.queue_config.address_queue_length
    };

    let queue_item_data = {
        let mut rpc = pool.get_connection().await?;
        fetch_queue_item_data(
            &mut *rpc,
            &tree_accounts.queue,
            start_index,
            length,
            queue_length,
        )
        .await?
    };

    if queue_item_data.is_empty() {
        return Ok(0);
    }

    let (recent_blockhash, current_block_height) = {
        let mut rpc = pool.get_connection().await?;
        (
            rpc.get_latest_blockhash().await?,
            rpc.get_block_height().await?,
        )
    };
    let last_valid_block_height = current_block_height + 150;
    let forester_epoch_pda_pubkey =
        get_forester_epoch_pda_from_authority(derivation, transaction_builder.epoch()).0;
    let priority_fee = if config.build_transaction_batch_config.enable_priority_fees {
        let rpc = pool.get_connection().await?;
        let account_keys = vec![
            payer.pubkey(),
            forester_epoch_pda_pubkey,
            tree_accounts.queue,
            tree_accounts.merkle_tree,
        ];
        let url = Url::parse(&rpc.get_url()).expect("Failed to parse URL");
        request_priority_fee_estimate(&url, account_keys).await?
    } else {
        10_000 // Minimum priority fee when disabled
    };

    let num_sent_clone = Arc::clone(&num_sent_transactions);
    let cancel_clone = Arc::clone(&cancel_signal);

    let processor_handle = tokio::spawn(async move {
        while let Some(result) = tx_receiver.recv().await {
            match result {
                Ok(signature) => {
                    num_sent_clone.fetch_add(1, Ordering::SeqCst);
                    info!(
                        "tree {} / queue {} / tx {:?}",
                        tree_accounts.merkle_tree.to_string(),
                        tree_accounts.queue.to_string(),
                        signature
                    );
                }
                Err(e) => {
                    warn!("Transaction failed: {:?}", e);

                    let mut rpc = processor_pool.get_connection().await;
                    if let Err(e) = &rpc {
                        warn!("Failed to get RPC connection: {}", e);
                    }
                    if let Ok(rpc) = rpc.as_mut() {
                        if !rpc.should_retry(&e) {
                            cancel_clone.store(true, Ordering::SeqCst);
                            break;
                        }
                    }
                }
            }
        }
    });

    let work_items: Vec<WorkItem> = queue_item_data
        .into_iter()
        .map(|data| WorkItem {
            tree_account: tree_accounts,
            queue_item_data: data,
        })
        .collect();

    let buffer_duration = Duration::from_secs(2);
    let adjusted_timeout = if config.retry_config.timeout > buffer_duration {
        config.retry_config.timeout - buffer_duration
    } else {
        return Ok(0);
    };
    let timeout_deadline = start_time + adjusted_timeout;

    for work_chunk in work_items.chunks(config.build_transaction_batch_config.batch_size as usize) {
        if cancel_signal.load(Ordering::SeqCst) || Instant::now() >= timeout_deadline {
            break;
        }

        let (transactions, _) = transaction_builder
            .build_signed_transaction_batch(
                payer,
                derivation,
                &recent_blockhash,
                last_valid_block_height,
                priority_fee,
                work_chunk,
                config.build_transaction_batch_config,
            )
            .await?;

        let now = Instant::now();
        if now >= timeout_deadline {
            break;
        }

        for tx in transactions {
            if cancel_signal.load(Ordering::SeqCst) {
                break;
            }

            let now = Instant::now();
            if now >= timeout_deadline {
                warn!("Reached timeout deadline, stopping batch processing");
                break;
            }

            let tx_sender = tx_sender.clone();
            let pool_clone = pool.clone();
            let config = RpcSendTransactionConfig {
                skip_preflight: true,
                max_retries: Some(0),
                preflight_commitment: Some(CommitmentLevel::Confirmed),
                ..Default::default()
            };

            let cancel_signal_clone = cancel_signal.clone();
            let deadline = timeout_deadline;

            tokio::spawn(async move {
                if cancel_signal_clone.load(Ordering::SeqCst) || Instant::now() >= deadline {
                    return;
                }

                if let Ok(mut rpc) = pool_clone.get_connection().await {
                    let result = rpc.process_transaction_with_config(tx, config).await;
                    if !cancel_signal_clone.load(Ordering::SeqCst) {
                        let _ = tx_sender.send(result).await;
                    }
                }
            });
        }
    }
    drop(tx_sender);
    processor_handle.await?;
    Ok(num_sent_transactions.load(Ordering::SeqCst))
}

#[derive(Debug, Clone, Copy)]
pub struct CapConfig {
    pub rec_fee_microlamports_per_cu: u64,
    pub min_fee_lamports: u64,
    pub max_fee_lamports: u64,
    pub compute_unit_limit: u64,
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
    pub enable_priority_fees: bool,
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
        .partition(|item| matches!(item.tree_account.tree_type, TreeType::AddressV1));

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
    if url.host_str() != Some("mainnet") {
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
