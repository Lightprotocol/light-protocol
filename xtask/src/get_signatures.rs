use anyhow::Result;
use clap::Parser;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use std::fs::File;
use std::io::Write;
use std::str::FromStr;
use std::u64;

#[derive(Parser)]
pub struct Options {
    /// RPC URL to connect to
    #[clap(long, default_value = "https://api.devnet.solana.com")]
    pub rpc_url: String,

    /// Start searching backwards from this transaction signature
    #[clap(long)]
    pub before: Option<String>,

    /// Search forward until this transaction signature
    #[clap(long)]
    pub until: Option<String>,
    /// Search forward until this transaction signature
    #[clap(long)]
    pub until_slot: Option<u64>,
    /// Search forward until this transaction signature
    #[clap(long)]
    pub before_slot: Option<u64>,
    /// Maximum number of signatures to return (default: 1000, max: 1000)
    #[clap(long, default_value = "1000")]
    pub limit: usize,
}

pub async fn get_signatures(options: Options) -> Result<()> {
    println!("Connecting to RPC: {}", options.rpc_url);

    let client = RpcClient::new(options.rpc_url);

    // Create target directory and file
    std::fs::create_dir_all("target")?;
    let mut file = File::create("target/signatures.txt")?;

    let mut latest_slot = u64::MAX;
    let mut before = None;
    let target_until_slot = options.until_slot.unwrap_or(0);
    let target_before_slot = options.before_slot.unwrap_or(u64::MAX);

    while latest_slot > target_until_slot {
        let config = GetConfirmedSignaturesForAddress2Config {
            before,
            until: None,
            limit: Some(options.limit.min(1000)), // Cap at Solana's max limit
            commitment: None,
        };
        let signatures =
            client.get_signatures_for_address_with_config(&account_compression::ID, config)?;

        println!("\nFound {} signatures:", signatures.len());
        println!(
            "{:<88} {:<10} {:<10} {}",
            "Signature", "Slot", "Block Time", "Error"
        );
        println!("{}", "-".repeat(120));

        for sig_info in signatures.iter() {
            let slot = sig_info.slot;

            if slot > target_before_slot {
                continue;
            }
            if slot < target_until_slot {
                continue;
            }
            if sig_info.err.is_some() {
                continue;
            }
            
            // Write signature to file
            writeln!(file, "{}", sig_info.signature)?;
            
            let block_time = sig_info
                .block_time
                .map(|t| t.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let error = sig_info
                .err
                .as_ref()
                .map(|e| format!("{:?}", e))
                .unwrap_or_else(|| "none".to_string());

            println!(
                "{:<88} {:<10} {:<10} {}",
                sig_info.signature, sig_info.slot, block_time, error
            );
        }
        before = signatures
            .last()
            .map(|sig| Signature::from_str(&sig.signature).unwrap());

        latest_slot = signatures.last().map(|sig| sig.slot).unwrap_or(u64::MAX);
        println!("latest slot {}", latest_slot);
        println!(
            "until target slot {}",
            latest_slot.saturating_sub(target_until_slot)
        );
        if latest_slot < options.until_slot.unwrap() {
            println!(
                "Reached the specified slot {} {}",
                latest_slot,
                options.until_slot.unwrap()
            );
        }
    }
    Ok(())
}
