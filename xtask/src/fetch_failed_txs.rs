use std::str::FromStr;

use anyhow::Result;
use chrono::{TimeZone, Utc};
use clap::Parser;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{
    option_serializer::OptionSerializer, EncodedTransaction, UiMessage, UiTransactionEncoding,
};
use tabled::{settings::Style, Table, Tabled};

/// Light Registry program ID
const REGISTRY_PROGRAM_ID: &str = "Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX";

#[derive(Debug, Parser)]
pub struct Options {
    /// Time window in minutes to fetch failed transactions (default: 10)
    #[clap(long, default_value_t = 10)]
    minutes: u64,

    /// Network to use (mainnet, devnet, local)
    #[clap(long, default_value = "mainnet")]
    network: String,

    /// Custom RPC URL (overrides network param)
    #[clap(long)]
    rpc_url: Option<String>,

    /// Show detailed logs for each failed transaction
    #[clap(long, short, default_value_t = false)]
    verbose: bool,
}

#[derive(Tabled)]
struct FailedTxRow {
    #[tabled(rename = "Signature")]
    signature: String,
    #[tabled(rename = "Time")]
    time: String,
    #[tabled(rename = "Error")]
    error: String,
}

fn network_to_url(network: &str) -> String {
    match network {
        "mainnet" => "https://api.mainnet-beta.solana.com".to_string(),
        "devnet" => "https://api.devnet.solana.com".to_string(),
        "testnet" => "https://api.testnet.solana.com".to_string(),
        "local" | "localnet" => "http://localhost:8899".to_string(),
        custom => custom.to_string(),
    }
}

fn format_timestamp(block_time: Option<i64>) -> String {
    match block_time {
        Some(ts) => Utc
            .timestamp_opt(ts, 0)
            .single()
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Invalid timestamp".to_string()),
        None => "N/A".to_string(),
    }
}

fn shorten_signature(sig: &str) -> String {
    if sig.len() > 12 {
        format!("{}...{}", &sig[..5], &sig[sig.len() - 4..])
    } else {
        sig.to_string()
    }
}

/// Extract error message from logs by looking for "Error Message:" pattern
fn extract_error_from_logs(logs: &[String]) -> Option<String> {
    for log in logs {
        // Look for AnchorError pattern: "Error Message: <message>."
        if let Some(pos) = log.find("Error Message:") {
            let msg_start = pos + "Error Message:".len();
            let msg = log[msg_start..].trim();
            // Remove trailing period if present
            let msg = msg.trim_end_matches('.');
            return Some(msg.to_string());
        }
    }
    None
}

pub async fn fetch_failed_txs(opts: Options) -> Result<()> {
    let rpc_url = opts
        .rpc_url
        .unwrap_or_else(|| network_to_url(&opts.network));

    println!("Fetching failed transactions from Light Registry program");
    println!("RPC: {}", rpc_url);
    println!("Time window: {} minutes", opts.minutes);
    println!();

    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
    let program_id = Pubkey::from_str(REGISTRY_PROGRAM_ID)?;

    let cutoff_timestamp = Utc::now().timestamp() - (opts.minutes as i64 * 60);

    // Fetch signatures for the program
    let signatures = client.get_signatures_for_address(&program_id)?;

    // Filter to failed transactions within the time window
    let failed_txs: Vec<_> = signatures
        .into_iter()
        .filter(|sig_info| {
            // Must have an error
            if sig_info.err.is_none() {
                return false;
            }
            // Must be within time window
            if let Some(block_time) = sig_info.block_time {
                block_time >= cutoff_timestamp
            } else {
                false
            }
        })
        .collect();

    if failed_txs.is_empty() {
        println!(
            "No failed transactions found in the last {} minutes.",
            opts.minutes
        );
        return Ok(());
    }

    println!(
        "Found {} failed transaction(s) in the last {} minutes:\n",
        failed_txs.len(),
        opts.minutes
    );

    // Collect detailed info for each failed transaction
    struct TxDetails {
        signature: String,
        block_time: Option<i64>,
        error_code: String,
        error_message: String,
        logs: Vec<String>,
        accounts: Vec<String>,
    }

    let mut tx_details: Vec<TxDetails> = Vec::new();

    for sig_info in &failed_txs {
        let signature = Signature::from_str(&sig_info.signature)?;
        let error_code = sig_info
            .err
            .as_ref()
            .map(|e| format!("{:?}", e))
            .unwrap_or_else(|| "Unknown".to_string());

        let (error_message, logs, accounts) = match client.get_transaction_with_config(
            &signature,
            RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Json),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        ) {
            Ok(tx) => {
                // Extract account keys from transaction
                let accounts = match &tx.transaction.transaction {
                    EncodedTransaction::Json(ui_tx) => match &ui_tx.message {
                        UiMessage::Parsed(msg) => {
                            msg.account_keys.iter().map(|k| k.pubkey.clone()).collect()
                        }
                        UiMessage::Raw(msg) => msg.account_keys.clone(),
                    },
                    _ => vec![],
                };

                if let Some(meta) = tx.transaction.meta {
                    if let OptionSerializer::Some(logs) = meta.log_messages {
                        let error_msg =
                            extract_error_from_logs(&logs).unwrap_or_else(|| error_code.clone());
                        (error_msg, logs, accounts)
                    } else {
                        (error_code.clone(), vec![], accounts)
                    }
                } else {
                    (error_code.clone(), vec![], accounts)
                }
            }
            Err(_) => (error_code.clone(), vec![], vec![]),
        };

        tx_details.push(TxDetails {
            signature: sig_info.signature.clone(),
            block_time: sig_info.block_time,
            error_code,
            error_message,
            logs,
            accounts,
        });
    }

    // Build table rows
    let rows: Vec<FailedTxRow> = tx_details
        .iter()
        .map(|tx| FailedTxRow {
            signature: shorten_signature(&tx.signature),
            time: format_timestamp(tx.block_time),
            error: tx.error_message.clone(),
        })
        .collect();

    let table = Table::new(&rows).with(Style::rounded()).to_string();
    println!("{}", table);

    // Print one full tx log for each unique error code
    println!("\n--- Sample Logs per Error Type ---\n");
    let mut seen_errors: std::collections::HashSet<String> = std::collections::HashSet::new();
    for tx in &tx_details {
        if !seen_errors.contains(&tx.error_code) && !tx.logs.is_empty() {
            seen_errors.insert(tx.error_code.clone());
            println!("Error Code: {}", tx.error_code);
            println!("Error Message: {}", tx.error_message);
            println!("Sample Signature: {}", tx.signature);
            if !tx.accounts.is_empty() {
                println!("Accounts ({}):", tx.accounts.len());
                for (i, account) in tx.accounts.iter().enumerate() {
                    println!("  [{}] {}", i, account);
                }
            }
            println!("Logs:");
            for log in &tx.logs {
                println!("  {}", log);
            }
            println!("{}", "-".repeat(80));
        }
    }

    // Print full details for each failed transaction (only if verbose)
    if opts.verbose {
        println!("\n--- All Failed Transactions (Verbose) ---\n");

        for tx in &tx_details {
            println!("Signature: {}", tx.signature);
            println!("Time: {}", format_timestamp(tx.block_time));
            println!("Error Code: {}", tx.error_code);
            println!("Error Message: {}", tx.error_message);
            if !tx.logs.is_empty() {
                println!("Logs:");
                for log in &tx.logs {
                    println!("  {}", log);
                }
            }
            println!("{}", "-".repeat(80));
        }
    }

    println!("\nTotal: {} failed transaction(s)", tx_details.len());

    Ok(())
}
