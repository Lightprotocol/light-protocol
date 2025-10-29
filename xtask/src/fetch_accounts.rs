use std::{fs::File, io::Write, str::FromStr};

use anyhow::Context;
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use light_program_test::{LightProgramTest, ProgramTestConfig, Rpc};
use serde_json::json;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{account::Account, pubkey::Pubkey};

#[derive(Debug, Parser)]
pub struct Options {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Parser)]
enum Command {
    /// Fetch test accounts from LightProgramTest state trees
    Test,
    /// Fetch accounts from RPC
    Rpc(RpcOptions),
}

#[derive(Debug, Parser)]
struct RpcOptions {
    /// Account pubkeys to fetch and store as JSON file
    #[clap(required = true)]
    pubkeys: Vec<String>,

    /// Network to use (mainnet, devnet, testnet, local)
    #[clap(long)]
    network: Option<String>,

    /// Custom RPC URL (overrides network param)
    #[clap(long)]
    rpc_url: Option<String>,

    /// Parse as lookup table (sets last_extended_slot to 0 so users can test with LUTs on localnet)
    #[clap(long)]
    lut: bool,

    /// Pubkeys to add to lookup table (comma-separated)
    #[clap(long)]
    add_pubkeys: Option<String>,
}

pub async fn fetch_accounts(opts: Options) -> anyhow::Result<()> {
    match opts.command {
        Command::Test => fetch_test().await,
        Command::Rpc(opts) => fetch_rpc(opts).await,
    }
}

async fn fetch_test() -> anyhow::Result<()> {
    let config = ProgramTestConfig::new_v2(false, None);
    let rpc = LightProgramTest::new(config).await?;
    let tree_infos = rpc.get_state_tree_infos();

    if tree_infos.len() < 2 {
        anyhow::bail!("Less than 2 tree infos available");
    }

    let address_0 = tree_infos[0]
        .cpi_context
        .context("No cpi_context for tree_info[0]")?;
    let address_1 = tree_infos[1]
        .cpi_context
        .context("No cpi_context for tree_info[1]")?;

    let account_0 = rpc
        .get_account(address_0)
        .await?
        .context("Account 0 not found")?;
    let account_1 = rpc
        .get_account(address_1)
        .await?
        .context("Account 1 not found")?;

    write_account_json(
        &account_0,
        &address_0,
        &format!("test_batched_cpi_context_{}.json", address_0),
    )?;
    write_account_json(
        &account_1,
        &address_1,
        &format!("test_batched_cpi_context_{}.json", address_1),
    )?;

    println!(
        "Wrote test accounts:\n  - test_batched_cpi_context_{}.json\n  - test_batched_cpi_context_{}.json",
        address_0, address_1
    );

    Ok(())
}

async fn fetch_rpc(opts: RpcOptions) -> anyhow::Result<()> {
    let rpc_url = opts.rpc_url.unwrap_or_else(|| {
        opts.network
            .as_deref()
            .map(network_to_url)
            .unwrap_or_else(|| "http://localhost:8899".to_string())
    });

    println!("Using RPC: {}", rpc_url);
    let client = RpcClient::new(rpc_url);

    for pubkey_str in &opts.pubkeys {
        let pubkey = Pubkey::from_str(pubkey_str)?;

        if opts.lut {
            fetch_and_process_lut(&client, &pubkey, &opts.add_pubkeys)?;
        } else {
            fetch_and_save_account(&client, &pubkey)?;
        }
    }

    println!("Processed {} accounts", opts.pubkeys.len());
    Ok(())
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

fn fetch_and_save_account(client: &RpcClient, pubkey: &Pubkey) -> anyhow::Result<()> {
    let account = client
        .get_account(pubkey)
        .context(format!("Failed to fetch account {}", pubkey))?;

    let filename = format!("account_{}.json", pubkey);
    write_account_json(&account, pubkey, &filename)?;

    println!(
        "Saved {} ({} bytes, {} lamports)",
        filename,
        account.data.len(),
        account.lamports
    );
    Ok(())
}

fn fetch_and_process_lut(
    client: &RpcClient,
    pubkey: &Pubkey,
    add_pubkeys: &Option<String>,
) -> anyhow::Result<()> {
    println!("Fetching lookup table: {}", pubkey);

    let account = client
        .get_account(pubkey)
        .context(format!("Failed to fetch LUT {}", pubkey))?;

    let modified_data = decode_and_modify_lut(&account.data, add_pubkeys)?;
    let filename = format!("modified_lut_{}.json", pubkey);

    let data_base64 = general_purpose::STANDARD.encode(&modified_data);
    let json_obj = json!({
        "pubkey": pubkey.to_string(),
        "account": {
            "lamports": account.lamports,
            "data": [data_base64, "base64"],
            "owner": account.owner.to_string(),
            "executable": account.executable,
            "rentEpoch": account.rent_epoch,
            "space": modified_data.len(),
        }
    });

    let mut file = File::create(&filename)?;
    file.write_all(json_obj.to_string().as_bytes())?;

    println!("Saved LUT {} with last_extended_slot set to 0", filename);
    Ok(())
}

fn decode_and_modify_lut(data: &[u8], add_pubkeys: &Option<String>) -> anyhow::Result<Vec<u8>> {
    if data.len() < 56 {
        anyhow::bail!("LUT data too small");
    }

    let discriminator = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if discriminator != 1 {
        anyhow::bail!(
            "Not a lookup table account (discriminator: {})",
            discriminator
        );
    }

    let mut modified_data = data.to_vec();

    let current_last_extended_slot = u64::from_le_bytes([
        data[12], data[13], data[14], data[15], data[16], data[17], data[18], data[19],
    ]);
    modified_data[12..20].copy_from_slice(&0u64.to_le_bytes());

    let addresses_start = 56;
    let mut num_addresses = (data.len().saturating_sub(addresses_start)) / 32;

    println!("  Number of addresses: {}", num_addresses);
    println!(
        "  Modified last_extended_slot: {} -> 0",
        current_last_extended_slot
    );

    if let Some(pubkeys_str) = add_pubkeys {
        for pubkey_str in pubkeys_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            let add_pubkey = Pubkey::from_str(pubkey_str)?;

            let mut exists = false;
            for i in 0..num_addresses {
                let start = addresses_start + (i * 32);
                let end = start + 32;
                if end <= modified_data.len() {
                    let existing = Pubkey::try_from(&modified_data[start..end])?;
                    if existing == add_pubkey {
                        println!("  Pubkey {} already exists at index {}", add_pubkey, i);
                        exists = true;
                        break;
                    }
                }
            }

            if !exists {
                modified_data.extend_from_slice(&add_pubkey.to_bytes());
                println!("  Added pubkey: {} at index {}", add_pubkey, num_addresses);
                num_addresses += 1;
            }
        }
    }

    Ok(modified_data)
}

fn write_account_json(account: &Account, pubkey: &Pubkey, filename: &str) -> anyhow::Result<()> {
    let data_base64 = general_purpose::STANDARD.encode(&account.data);
    let json_obj = json!({
        "pubkey": pubkey.to_string(),
        "account": {
            "lamports": account.lamports,
            "data": [data_base64, "base64"],
            "owner": account.owner.to_string(),
            "executable": account.executable,
            "rentEpoch": account.rent_epoch,
            "space": account.data.len(),
        }
    });

    let mut file = File::create(filename)?;
    file.write_all(json_obj.to_string().as_bytes())?;
    Ok(())
}
