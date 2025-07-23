use anyhow::Result;
use clap::Parser;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use std::str::FromStr;

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

    /// Maximum number of signatures to return (default: 1000, max: 1000)
    #[clap(long, default_value = "100")]
    pub limit: usize,
}

pub async fn get_signatures(options: Options) -> Result<()> {
    println!("Connecting to RPC: {}", options.rpc_url);

    let client = RpcClient::new(options.rpc_url);

    let mut config = GetConfirmedSignaturesForAddress2Config {
        before: None,
        until: None,
        limit: Some(options.limit.min(1000)), // Cap at Solana's max limit
        commitment: None,
    };

    // Parse before signature if provided
    if let Some(before_str) = options.before {
        let before_sig = Signature::from_str(&before_str)?;
        config.before = Some(before_sig);
        println!("Searching before signature: {}", before_sig);
    }

    // Parse until signature if provided
    if let Some(until_str) = options.until {
        let until_sig = Signature::from_str(&until_str)?;
        config.until = Some(until_sig);
        println!("Searching until signature: {}", until_sig);
    }

    let signatures =
        client.get_signatures_for_address_with_config(&account_compression::ID, config)?;

    println!("\nFound {} signatures:", signatures.len());
    println!(
        "{:<88} {:<10} {:<10} {}",
        "Signature", "Slot", "Block Time", "Error"
    );
    println!("{}", "-".repeat(120));

    for sig_info in signatures {
        let block_time = sig_info
            .block_time
            .map(|t| t.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let error = sig_info
            .err
            .map(|e| format!("{:?}", e))
            .unwrap_or_else(|| "none".to_string());

        println!(
            "{:<88} {:<10} {:<10} {}",
            sig_info.signature, sig_info.slot, block_time, error
        );
    }

    Ok(())
}
