use std::path::PathBuf;

use clap::Parser;
use dirs::home_dir;
use light_client::rpc::{LightClient, LightClientConfig, Rpc};
use light_token_sdk::token::{CompressibleParams, CreateTokenAccount};
use solana_sdk::{
    signature::{read_keypair_file, Keypair, Signer},
    transaction::Transaction,
};

#[derive(Debug, Parser)]
pub struct Options {
    /// Path to payer keypair file (defaults to ~/.config/solana/id.json)
    #[clap(long)]
    payer: Option<PathBuf>,

    /// Network: devnet, mainnet, local, or custom RPC URL (default: devnet)
    #[clap(long, default_value = "devnet")]
    network: String,

    /// Number of accounts to create (default: 1)
    #[clap(long, default_value = "1")]
    count: u32,
}

fn get_rpc_url(network: &str) -> String {
    match network {
        "local" => String::from("http://127.0.0.1:8899"),
        "devnet" => String::from("https://api.devnet.solana.com"),
        "mainnet" => String::from("https://api.mainnet-beta.solana.com"),
        other => other.to_string(),
    }
}

pub async fn create_ctoken_account(options: Options) -> anyhow::Result<()> {
    let rpc_url = get_rpc_url(&options.network);
    println!("Connecting to: {}", rpc_url);

    let mut rpc = LightClient::new(LightClientConfig {
        url: rpc_url,
        photon_url: None,
        commitment_config: None,
        fetch_active_tree: false,
        api_key: None,
    })
    .await?;

    // Load payer keypair
    let payer = if let Some(payer_path) = options.payer.as_ref() {
        read_keypair_file(payer_path).unwrap_or_else(|_| panic!("Failed to read {:?}", payer_path))
    } else {
        let keypair_path: PathBuf = home_dir()
            .expect("Could not find home directory")
            .join(".config/solana/id.json");
        read_keypair_file(&keypair_path)
            .unwrap_or_else(|_| panic!("Keypair not found in default path {:?}", keypair_path))
    };
    println!("Payer: {}", payer.pubkey());

    let balance = rpc.get_balance(&payer.pubkey()).await?;
    println!(
        "Payer balance: {} lamports ({} SOL)",
        balance,
        balance as f64 / 1e9
    );

    let owner = payer.pubkey();
    println!("Owner: {}", owner);
    println!("Creating {} account(s)...\n", options.count);

    for i in 0..options.count {
        let account_keypair = Keypair::new();
        let mint = Keypair::new().pubkey();

        // Create compressible params with 0 prepaid epochs
        let compressible_params = CompressibleParams {
            pre_pay_num_epochs: 0,
            lamports_per_write: None,
            ..CompressibleParams::default()
        };

        let create_ix =
            CreateTokenAccount::new(payer.pubkey(), account_keypair.pubkey(), mint, owner)
                .with_compressible(compressible_params)
                .instruction()?;

        let transaction = Transaction::new_signed_with_payer(
            &[create_ix],
            Some(&payer.pubkey()),
            &[&payer, &account_keypair],
            rpc.get_latest_blockhash().await?.0,
        );

        let signature = rpc.send_transaction(&transaction).await?;

        println!(
            "[{}/{}] Account: {} | Mint: {} | Sig: {:?}",
            i + 1,
            options.count,
            account_keypair.pubkey(),
            mint,
            signature
        );
    }

    println!("\n=== Summary ===");
    println!("Network: {}", options.network);
    println!("Accounts created: {}", options.count);
    println!("Owner: {}", owner);
    println!("Prepaid epochs: 0");

    Ok(())
}
