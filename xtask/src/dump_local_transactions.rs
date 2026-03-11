use std::{
    collections::BTreeMap,
    fs::{self, File},
    path::Path,
};

use anyhow::{bail, Context};
use clap::Parser;
use serde_json::{json, Value};
use solana_client::{rpc_client::RpcClient, rpc_request::RpcRequest};
use solana_sdk::commitment_config::CommitmentConfig;

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

fn extract_signature(transaction: &Value) -> Option<&str> {
    transaction
        .get("transaction")?
        .get("signatures")?
        .as_array()?
        .first()?
        .as_str()
}

fn transaction_failed(transaction: &Value) -> bool {
    transaction
        .get("meta")
        .and_then(|meta| meta.get("err"))
        .is_some_and(|err| !err.is_null())
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

    let slots_value: Value = client.send(
        RpcRequest::Custom {
            method: "getBlocks",
        },
        json!([start_slot, end_slot]),
    )?;
    let slots: Vec<u64> =
        serde_json::from_value(slots_value).context("failed to decode getBlocks response")?;

    let mut slot_signatures = BTreeMap::<u64, Vec<String>>::new();
    let mut failed_signatures = Vec::<String>::new();
    let mut transactions_dumped = 0usize;

    for slot in slots {
        let block: Value = client.send(
            RpcRequest::Custom { method: "getBlock" },
            json!([
                slot,
                {
                    "encoding": "base64",
                    "transactionDetails": "full",
                    "rewards": false,
                    "commitment": "confirmed",
                    "maxSupportedTransactionVersion": 0
                }
            ]),
        )?;

        let Some(transactions) = block.get("transactions").and_then(Value::as_array) else {
            continue;
        };

        let mut signatures = Vec::with_capacity(transactions.len());
        for transaction in transactions {
            let Some(signature) = extract_signature(transaction) else {
                continue;
            };
            let signature = signature.to_string();
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

        slot_signatures.insert(slot, signatures);
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
