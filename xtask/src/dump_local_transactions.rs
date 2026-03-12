use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs::{self, File},
    path::Path,
};

use anyhow::{bail, Context};
use clap::{Parser, ValueEnum};
use serde_json::json;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcBlockConfig};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransaction,
    EncodedTransactionWithStatusMeta, TransactionDetails, UiTransactionEncoding,
};

#[derive(Debug, Parser)]
pub struct Options {
    #[clap(long, default_value = "http://localhost:8899")]
    rpc_url: String,
    #[clap(long, default_value = "./target/localnet-transactions")]
    output_folder: String,
    #[clap(long)]
    start_slot: Option<u64>,
    #[clap(long)]
    end_slot: Option<u64>,
    #[clap(long, value_enum)]
    commitment: Option<RpcCommitment>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum RpcCommitment {
    Confirmed,
    Finalized,
}

impl RpcCommitment {
    fn as_config(self) -> CommitmentConfig {
        match self {
            Self::Confirmed => CommitmentConfig::confirmed(),
            Self::Finalized => CommitmentConfig::finalized(),
        }
    }
}

fn commitment_from_env() -> Option<RpcCommitment> {
    match env::var("PHOTON_INDEXING_COMMITMENT")
        .ok()?
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "confirmed" => Some(RpcCommitment::Confirmed),
        "finalized" => Some(RpcCommitment::Finalized),
        _ => None,
    }
}

fn extract_signature(transaction: &EncodedTransactionWithStatusMeta) -> Option<String> {
    match &transaction.transaction {
        EncodedTransaction::Json(ui_transaction) => ui_transaction.signatures.first().cloned(),
        _ => transaction
            .transaction
            .decode()
            .and_then(|decoded| decoded.signatures.first().map(ToString::to_string)),
    }
}

fn transaction_failed(transaction: &EncodedTransactionWithStatusMeta) -> bool {
    transaction
        .meta
        .as_ref()
        .is_some_and(|meta| meta.err.is_some())
}

pub async fn dump_local_transactions(opt: Options) -> anyhow::Result<()> {
    let client = RpcClient::new(opt.rpc_url.clone());
    let commitment = opt
        .commitment
        .or_else(commitment_from_env)
        .unwrap_or(RpcCommitment::Confirmed)
        .as_config();
    let first_available_block = client
        .get_first_available_block()
        .context("failed to fetch first available block")?;
    let latest_slot = client
        .get_slot_with_commitment(commitment)
        .context("failed to fetch latest confirmed slot")?;

    let start_slot = opt.start_slot.unwrap_or(first_available_block);
    let end_slot = opt.end_slot.unwrap_or(latest_slot);
    if start_slot > end_slot {
        bail!(
            "start_slot {} is greater than end_slot {}",
            start_slot,
            end_slot
        );
    }

    let output_folder = Path::new(&opt.output_folder);
    let blocks_dir = output_folder.join("blocks");
    let transactions_dir = output_folder.join("transactions");
    fs::create_dir_all(&blocks_dir)
        .with_context(|| format!("failed to create {}", blocks_dir.display()))?;
    fs::create_dir_all(&transactions_dir)
        .with_context(|| format!("failed to create {}", transactions_dir.display()))?;

    let slots = client
        .get_blocks_with_commitment(start_slot, Some(end_slot), commitment)
        .with_context(|| format!("failed to fetch {:?} blocks", commitment.commitment))?;

    let mut slot_signatures = BTreeMap::<u64, Vec<String>>::new();
    let mut duplicate_signatures = BTreeMap::<String, Vec<u64>>::new();
    let mut failed_signatures = Vec::<String>::new();
    let mut written_transactions = BTreeSet::<String>::new();
    let mut transactions_dumped = 0usize;
    let blocks_scanned = slots.len();

    for slot in slots {
        let block = client
            .get_block_with_config(
                slot,
                RpcBlockConfig {
                    encoding: Some(UiTransactionEncoding::Base64),
                    transaction_details: Some(TransactionDetails::Full),
                    rewards: Some(false),
                    commitment: Some(commitment),
                    max_supported_transaction_version: Some(0),
                },
            )
            .with_context(|| format!("failed to fetch block {}", slot))?;

        let block_path = blocks_dir.join(slot.to_string());
        let block_file = File::create(&block_path)
            .with_context(|| format!("failed to create {}", block_path.display()))?;
        serde_json::to_writer_pretty(block_file, &block)
            .with_context(|| format!("failed to write {}", block_path.display()))?;

        let Some(transactions) = block.transactions.clone() else {
            continue;
        };

        let mut signatures = Vec::with_capacity(transactions.len());
        for transaction in &transactions {
            let Some(signature) = extract_signature(transaction) else {
                continue;
            };
            if written_transactions.insert(signature.clone()) {
                let file_path = transactions_dir.join(&signature);
                let file = File::create(&file_path)
                    .with_context(|| format!("failed to create {}", file_path.display()))?;
                let confirmed_transaction = EncodedConfirmedTransactionWithStatusMeta {
                    slot,
                    transaction: transaction.clone(),
                    block_time: block.block_time,
                };
                serde_json::to_writer_pretty(file, &confirmed_transaction)
                    .with_context(|| format!("failed to write {}", file_path.display()))?;
            } else {
                duplicate_signatures
                    .entry(signature.clone())
                    .or_default()
                    .push(slot);
            }
            if transaction_failed(transaction) {
                failed_signatures.push(signature.clone());
            }
            signatures.push(signature);
            transactions_dumped += 1;
        }

        if !signatures.is_empty() {
            slot_signatures.insert(slot, signatures);
        }
    }

    let summary_path = output_folder.join("summary.json");
    let summary_file = File::create(&summary_path)
        .with_context(|| format!("failed to create {}", summary_path.display()))?;
    serde_json::to_writer_pretty(
        summary_file,
        &json!({
            "rpc_url": opt.rpc_url,
            "commitment": format!("{:?}", commitment.commitment).to_ascii_lowercase(),
            "first_available_block": first_available_block,
            "latest_slot_at_commitment": latest_slot,
            "start_slot": start_slot,
            "end_slot": end_slot,
            "blocks_scanned": blocks_scanned,
            "blocks_dumped": slot_signatures.len(),
            "slots_dumped": slot_signatures.len(),
            "transactions_dumped": transactions_dumped,
            "unique_transactions_dumped": written_transactions.len(),
            "failed_signatures": failed_signatures,
            "duplicate_signatures": duplicate_signatures,
            "slot_signatures": slot_signatures,
        }),
    )
    .with_context(|| format!("failed to write {}", summary_path.display()))?;

    println!(
        "Dumped {} transaction occurrences, {} unique transaction files, and {} block files into {}",
        transactions_dumped,
        written_transactions.len(),
        slot_signatures.len(),
        output_folder.display()
    );

    Ok(())
}
