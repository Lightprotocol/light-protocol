use anchor_lang::{InstructionData, ToAccountMetas};
use anyhow::Result;
use forester::rpc_pool::SolanaRpcPool;
use forester::send_transaction::{
    send_batched_transactions, BuildTransactionBatchConfig, RetryConfig,
    SendBatchedTransactionsConfig, TransactionBuilder,
};
use light_test_utils::rpc::{errors::RpcError, rpc_connection::RpcConnection, SolanaRpcConnection};
use solana_sdk::transaction::Transaction;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
};
use std::env;
use std::{time::Duration, vec};

use tokio::time::sleep;
// 3.69263936

// benchmarks
// 1 tx send_and_confirm
// 100 tx send_and_confirm
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_one_tx() {}
// TODO: add shutdown msg to all created threads

/// Devnet 10 tx 3.6s
/// Mainnet no priority fees no connection pool
/// batch size 100
/// Time taken: 23.272066313s
/// Batch time: 23.272788814s
///
/// Mainnet no priority fees with connection pool
/// build transaction time 2.692482ms
/// results.len 100
/// Time taken: 29.529072131s
/// Batch time: 29.669979182s
///
/// Mainnet no priority fees with custom thread
/// batch_size: 100
/// Final Batch time: 13.966347887s
///
/// Mainnet no priority fees with custom thread
/// num_batches: 2
/// batch_size: 100
/// Transactions landed: 200
/// Final Batch time: 14.211396111s
///
/// With really high (50_000) priority fees inst faster but no retries are needed
/// With really medium (2_800) priority fees inst faster but no retries are needed
///
/// separate connection for every tx makes things slower by 50ms per tx
///
/// 150/200ms retry wait time is a lot faster than 500ms (batch size 100, num_batches 4 11s vs > 15s), no priority fees (can also fail with lower retry times)
///
/// lower time between batches is a lot worse
///
/// Sunday evening after testing multiple times with 100 txs and 2 batches I get ratelimited to only 300 tx
/// 50 tx
/// Final Batch time: 18.210245647s (with 2 retries)
/// Final Batch time: 9.966347887s (with 1 retry)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_one_slot() {
    let url = env::var("SOLANA_RPC").unwrap_or_else(|_| "http://localhost:8899".to_string());
    let path = env::var("HOME").unwrap();
    let payer = read_keypair_file(format!("{}/.config/solana/id.json", path)).unwrap();

    let batch_size = 50;
    let num_batches = 15;
    // resembles slot time
    let timeout = 15;
    let priority_fee = None;
    let compute_unit_limit: Option<u32> = None; //Some(300_000);
                                                // recent blockhash is valid for 2 mins we only need one
                                                // let recent_blockhash = client.get_latest_blockhash().await.unwrap();
    let mut rpc = SolanaRpcConnection::new(url.clone(), None);
    let pool = SolanaRpcPool::<SolanaRpcConnection>::new(
        url.clone(),
        CommitmentConfig::confirmed(),
        num_batches as u32 + 2,
    )
    .await
    .unwrap();

    let pre_count = get_current_count(&mut rpc).await.unwrap();
    let time = tokio::time::Instant::now();

    send_batched_transactions::<TestTransactionBuilder>(
        &payer,
        &pool,
        SendBatchedTransactionsConfig {
            num_batches,
            batch_time: 1,
            build_transaction_batch_config: BuildTransactionBatchConfig {
                batch_size,
                compute_unit_price: priority_fee,
                compute_unit_limit,
            },
            retry_config: RetryConfig {
                max_retries: 10,
                retry_wait_time_ms: 200,
            },
        },
        0,
        &url,
    )
    .await;
    println!("Batch time: {:?}", time.elapsed());
    println!("batch_size: {}", batch_size);
    let mut rpc = SolanaRpcConnection::new(url.clone(), None);
    let mut post_count = 0;
    while post_count < pre_count + (batch_size * num_batches)
        && time.elapsed() < Duration::from_secs(timeout)
    {
        post_count = get_current_count(&mut rpc).await.unwrap();
        println!("post_count: {}", post_count);
        println!(
            "Transactions landed: {}",
            post_count.saturating_sub(pre_count)
        );
        println!("Batch time: {:?}", time.elapsed());
        if post_count < pre_count + (batch_size * num_batches) {
            sleep(Duration::from_secs(1)).await;
        }
    }
    println!(
        "Transactions landed: {}",
        post_count.saturating_sub(pre_count)
    );
    println!("Final Batch time: {:?}", time.elapsed());
}

/// Mainnet:
/// - batch size 50
/// - num_batches 10
/// Constantly stays above 100  send txs per second because of retries. This will
/// clash with the rate limit of 100 send transactions calls per second at some
/// point.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_slots() {
    let url = env::var("SOLANA_RPC").unwrap_or_else(|_| "http://localhost:8899".to_string());
    let path = env::var("HOME").unwrap();
    let payer = read_keypair_file(format!("{}/.config/solana/id.json", path)).unwrap();

    let batch_size = 50;
    let num_batches = 10;
    // resembles slot time
    let slot_time = 15;
    let num_slots = 40;
    let timeout = slot_time * num_slots + 100;
    let priority_fee = None;
    let compute_unit_limit: Option<u32> = None; //Some(300_000);
    let mut rpc = SolanaRpcConnection::new(url.clone(), None);
    let pool = SolanaRpcPool::<SolanaRpcConnection>::new(
        url.clone(),
        CommitmentConfig::confirmed(),
        num_batches as u32 + 2,
    )
    .await
    .unwrap();

    let pre_count = get_current_count(&mut rpc).await.unwrap();
    let time = tokio::time::Instant::now();

    // TODO: try whether I can send more tx with multiple threads at this level
    // TODO: try multiple api keys
    // TODO: emit msg onchain to record value and poll msg strings to check for dropped or duplicate txs

    time.into_std();
    for i in 0..num_slots {
        println!("Slot: {}", i);
        let slot = sleep(Duration::from_secs(slot_time));
        send_batched_transactions::<TestTransactionBuilder>(
            &payer,
            &pool,
            SendBatchedTransactionsConfig {
                num_batches,
                batch_time: 1,
                build_transaction_batch_config: BuildTransactionBatchConfig {
                    batch_size,
                    compute_unit_price: priority_fee,
                    compute_unit_limit,
                },
                retry_config: RetryConfig {
                    max_retries: 10,
                    retry_wait_time_ms: 1000,
                },
            },
            i * 1000,
            &url,
        )
        .await;
        let mut rpc = pool.get_connection().await.unwrap();
        let post_count = get_current_count(&mut *rpc).await.unwrap();
        println!("in slot thread: post_count: {}", post_count);
        println!(
            "in slot thread: Transactions landed: {}",
            post_count - pre_count
        );

        slot.await;
    }
    println!("Batch time: {:?}", time.elapsed());
    println!("batch_size: {}", batch_size);
    let mut rpc = SolanaRpcConnection::new(url.clone(), None);
    let mut post_count = 0;
    while post_count < pre_count + (batch_size * num_batches * num_slots)
        && time.elapsed() < Duration::from_secs(timeout)
    {
        post_count = get_current_count(&mut rpc).await.unwrap();
        println!("post_count: {}", post_count);
        println!("Transactions landed: {}", post_count - pre_count);
        println!("Batch time: {:?}", time.elapsed());
        if post_count < pre_count + (batch_size * num_batches * num_slots) {
            sleep(Duration::from_secs(1)).await;
        }
    }
    println!(
        "Transactions landed: {}",
        post_count.saturating_sub(pre_count)
    );
    println!("Final Batch time: {:?}", time.elapsed());
}

async fn get_current_count<R: RpcConnection>(client: &mut R) -> Result<u64, RpcError> {
    let account = client
        .get_anchor_account::<test_batch_transactions::TransactionCounter>(
            &get_counter_account_address(),
        )
        .await?;
    match account {
        Some(account) => Ok(account.counter),
        None => Ok(0),
    }
}

fn get_counter_account_address() -> Pubkey {
    Pubkey::find_program_address(&[b"counter"], &test_batch_transactions::id()).0
}

pub struct TestTransactionBuilder;
async fn build_signed_transaction(
    value: u64,
    payer: &Keypair,
    recent_blockhash: &Hash,
    comput_unit_price: Option<u64>,
    compute_unit_limit: Option<u32>,
) -> Transaction {
    let counter_account = get_counter_account_address();
    let accounts = test_batch_transactions::accounts::Initialize {
        signer: payer.pubkey(),
        counter: counter_account,
        system_program: solana_sdk::system_program::id(),
    };
    let mut instructions: Vec<Instruction> = if let Some(price) = comput_unit_price {
        vec![ComputeBudgetInstruction::set_compute_unit_price(price)]
    } else {
        vec![]
    };
    if let Some(limit) = compute_unit_limit {
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(limit));
    }
    instructions.push(Instruction {
        program_id: test_batch_transactions::id(),
        accounts: accounts.to_account_metas(Some(true)),
        data: test_batch_transactions::instruction::Initialize {
            num_hashes: 200,
            unique_id: value,
        }
        .data(),
    });

    let mut transaction =
        Transaction::new_with_payer(&instructions.as_slice(), Some(&payer.pubkey()));
    transaction.sign(&[payer], *recent_blockhash);
    transaction
}
impl TransactionBuilder for TestTransactionBuilder {
    async fn build_signed_transaction_batch(
        payer: &Keypair,
        recent_blockhash: &Hash,
        domain: u64,
        config: BuildTransactionBatchConfig,
    ) -> Vec<Transaction> {
        // simulate photon request time
        sleep(Duration::from_millis(500)).await;
        let mut transactions = vec![];
        for j in 0..config.batch_size {
            let transaction = build_signed_transaction(
                j + domain,
                payer,
                &recent_blockhash,
                config.compute_unit_price,
                config.compute_unit_limit,
            )
            .await;
            transactions.push(transaction);
        }
        transactions
    }
}
