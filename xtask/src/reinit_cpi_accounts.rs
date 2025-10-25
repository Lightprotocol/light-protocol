use std::path::PathBuf;

use clap::Parser;
use dirs::home_dir;
use light_client::rpc::{LightClient, LightClientConfig, Rpc};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};

const LIGHT_SYSTEM_PROGRAM_ID: &str = "SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7";
// Legacy CPI context account discriminator (before reinit)
const CPI_CONTEXT_ACCOUNT_1_DISCRIMINATOR: [u8; 8] = [22, 20, 149, 218, 74, 204, 128, 166];

// ReInitCpiContextAccount instruction discriminator
const REINIT_CPI_CONTEXT_DISCRIMINATOR: [u8; 8] = [187, 147, 22, 142, 104, 180, 136, 190];

#[derive(Debug, Parser)]
pub struct Options {
    /// Path to payer keypair (defaults to ~/.config/solana/id.json)
    #[clap(long)]
    payer: Option<PathBuf>,

    /// Network: local, devnet, mainnet, or custom URL
    #[clap(long, default_value = "devnet")]
    network: String,

    /// Dry run - only show accounts that would be reinitialized
    #[clap(long, default_value = "false")]
    dry_run: bool,
}

fn create_reinit_cpi_context_instruction(cpi_context_account: Pubkey) -> Instruction {
    Instruction {
        program_id: LIGHT_SYSTEM_PROGRAM_ID.parse().unwrap(),
        accounts: vec![AccountMeta::new(cpi_context_account, false)],
        data: REINIT_CPI_CONTEXT_DISCRIMINATOR.to_vec(),
    }
}

pub async fn reinit_cpi_accounts(options: Options) -> anyhow::Result<()> {
    let rpc_url = if options.network == "local" {
        String::from("http://127.0.0.1:8899")
    } else if options.network == "devnet" {
        String::from("https://api.devnet.solana.com")
    } else if options.network == "mainnet" {
        String::from("https://api.mainnet-beta.solana.com")
    } else {
        options.network.clone()
    };

    println!("Connecting to network: {}", rpc_url);

    let mut rpc = LightClient::new(LightClientConfig {
        url: rpc_url,
        photon_url: None,
        commitment_config: None,
        fetch_active_tree: false,
        api_key: None,
    })
    .await?;

    let payer = if let Some(payer_path) = options.payer.as_ref() {
        read_keypair_file(payer_path)
            .unwrap_or_else(|_| panic!("Failed to read keypair from {:?}", payer_path))
    } else {
        let keypair_path: PathBuf = home_dir()
            .expect("Could not find home directory")
            .join(".config/solana/id.json");
        read_keypair_file(keypair_path.clone())
            .unwrap_or_else(|_| panic!("Keypair not found in default path {:?}", keypair_path))
    };

    println!("Using payer: {}", payer.pubkey());

    let balance = rpc.get_balance(&payer.pubkey()).await?;
    println!(
        "Payer balance: {} lamports ({} SOL)",
        balance,
        balance as f64 / 1_000_000_000.0
    );

    let program_id: Pubkey = LIGHT_SYSTEM_PROGRAM_ID.parse()?;
    println!(
        "\nFetching all accounts for light system program: {}",
        program_id
    );

    // Get all program accounts
    let accounts = rpc.get_program_accounts(&program_id).await?;
    println!("Total accounts found: {}", accounts.len());

    // Filter for CPI context accounts with legacy discriminator
    let mut cpi_context_accounts = Vec::new();
    for (pubkey, account) in accounts {
        if account.data.len() >= 8 {
            let discriminator = &account.data[0..8];
            if discriminator == CPI_CONTEXT_ACCOUNT_1_DISCRIMINATOR {
                cpi_context_accounts.push(pubkey);
            }
        }
    }

    println!(
        "\nFound {} CPI context accounts with legacy discriminator",
        cpi_context_accounts.len()
    );

    if cpi_context_accounts.is_empty() {
        println!("No accounts to reinitialize!");
        return Ok(());
    }

    // Display accounts
    for (i, account) in cpi_context_accounts.iter().enumerate() {
        println!("  {}. {}", i + 1, account);
    }

    if options.dry_run {
        println!("\nDry run mode - no transactions will be sent");
        return Ok(());
    }

    println!(
        "\nReinitializing {} accounts...",
        cpi_context_accounts.len()
    );

    // Process each account
    for (i, account_pubkey) in cpi_context_accounts.iter().enumerate() {
        println!(
            "\n[{}/{}] Reinitializing {}",
            i + 1,
            cpi_context_accounts.len(),
            account_pubkey
        );

        let instruction = create_reinit_cpi_context_instruction(*account_pubkey);
        let latest_blockhash = rpc.get_latest_blockhash().await?;

        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
        transaction.sign(&[&payer], latest_blockhash.0);

        match rpc.process_transaction(transaction).await {
            Ok(sig) => {
                println!("  ✓ Successfully reinitialized. Signature: {}", sig);
            }
            Err(e) => {
                println!("  ✗ Failed to reinitialize: {}", e);
                println!("  Continuing with next account...");
            }
        }
    }

    println!("\n✓ Reinitialization complete!");
    Ok(())
}
