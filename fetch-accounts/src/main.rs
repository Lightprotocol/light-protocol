use std::{fs::File, io::Write};

use base64::encode;
use light_client::indexer::Indexer;
use light_program_test::{LightProgramTest, ProgramTestConfig, Rpc};
use serde_json::json;
use solana_sdk::pubkey::Pubkey;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting to fetch accounts...");

    // Initialize test environment
    // You can adjust the config based on your needs
    let config = ProgramTestConfig::new_v2(false, None);
    let rpc = LightProgramTest::new(config).await?;

    // Get tree infos
    let tree_infos = rpc.get_state_tree_infos();
    println!("Found {} tree infos", tree_infos.len());

    // Get a random state tree info
    let random_info = rpc.get_random_state_tree_info();
    match random_info {
        Ok(info) => println!("Random info: {:?}", info),
        Err(e) => println!("Error getting random info: {:?}", e),
    }

    // Check if we have at least 2 tree infos
    if tree_infos.len() < 2 {
        println!("Warning: Less than 2 tree infos available");
        return Ok(());
    }

    // Get the cpi_context addresses
    let address_0 = tree_infos[0]
        .cpi_context
        .ok_or("No cpi_context for tree_info[0]")?;
    let address_1 = tree_infos[1]
        .cpi_context
        .ok_or("No cpi_context for tree_info[1]")?;

    println!("Address 0: {}", address_0);
    println!("Address 1: {}", address_1);

    // Fetch accounts
    let account_0 = rpc
        .get_account(address_0)
        .await?
        .ok_or("Account 0 not found")?;
    let account_1 = rpc
        .get_account(address_1)
        .await?
        .ok_or("Account 1 not found")?;

    println!("Fetched account_0: {} bytes", account_0.data.len());
    println!("Fetched account_1: {} bytes", account_1.data.len());

    // Write accounts to JSON files
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

    println!("Successfully wrote account JSON files");
    println!(
        "Account 0 details: lamports={}, owner={}, executable={}, data_len={}",
        account_0.lamports,
        account_0.owner,
        account_0.executable,
        account_0.data.len()
    );
    println!(
        "Account 1 details: lamports={}, owner={}, executable={}, data_len={}",
        account_1.lamports,
        account_1.owner,
        account_1.executable,
        account_1.data.len()
    );

    Ok(())
}

fn write_account_json(
    account: &solana_sdk::account::Account,
    pubkey: &Pubkey,
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let data_base64 = encode(&account.data);
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
