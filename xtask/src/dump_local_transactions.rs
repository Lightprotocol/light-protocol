use std::{
    collections::BTreeMap,
    fs::{self, File},
    path::Path,
};

use anyhow::{bail, Context};
use clap::Parser;
use serde_json::json;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcBlockConfig};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::{
    EncodedTransaction, EncodedTransactionWithStatusMeta, TransactionDetails, UiTransactionEncoding,
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
    let first_available_block = client
        .get_first_available_block()
        .context("failed to fetch first available block")?;
    let latest_slot = client
        .get_slot_with_commitment(CommitmentConfig::confirmed())
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
    let transactions_dir = output_folder.join("transactions");
    fs::create_dir_all(&transactions_dir)
        .with_context(|| format!("failed to create {}", transactions_dir.display()))?;

    let slots = client
        .get_blocks_with_commitment(start_slot, Some(end_slot), CommitmentConfig::confirmed())
        .context("failed to fetch confirmed blocks")?;

    let mut slot_signatures = BTreeMap::<u64, Vec<String>>::new();
    let mut failed_signatures = Vec::<String>::new();
    let mut transactions_dumped = 0usize;
    let blocks_scanned = slots.len();

    for slot in slots {
        let block = client
            .get_block_with_config(
                slot,
                RpcBlockConfig {
                    encoding: Some(UiTransactionEncoding::Json),
                    transaction_details: Some(TransactionDetails::Full),
                    rewards: Some(false),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(0),
                },
            )
            .with_context(|| format!("failed to fetch block {}", slot))?;

        let Some(transactions) = block.transactions else {
            continue;
        };

        let mut signatures = Vec::with_capacity(transactions.len());
        for transaction in &transactions {
            let Some(signature) = extract_signature(transaction) else {
                continue;
            };
            let file_path = transactions_dir.join(format!("{signature}.json"));
            let file = File::create(&file_path)
                .with_context(|| format!("failed to create {}", file_path.display()))?;
            serde_json::to_writer_pretty(file, transaction)
                .with_context(|| format!("failed to write {}", file_path.display()))?;
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
            "first_available_block": first_available_block,
            "latest_confirmed_slot": latest_slot,
            "start_slot": start_slot,
            "end_slot": end_slot,
            "blocks_scanned": blocks_scanned,
            "slots_dumped": slot_signatures.len(),
            "transactions_dumped": transactions_dumped,
            "failed_signatures": failed_signatures,
            "slot_signatures": slot_signatures,
        }),
    )
    .with_context(|| format!("failed to write {}", summary_path.display()))?;

    println!(
        "Dumped {} transactions across {} slots into {}",
        transactions_dumped,
        slot_signatures.len(),
        output_folder.display()
    );

    Ok(())
}
