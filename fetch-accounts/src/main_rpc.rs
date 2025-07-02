use std::{fs::File, io::Write, str::FromStr};

use base64::encode;
use serde_json::json;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    // Get RPC URL - can be set via environment variable or use predefined networks
    let rpc_url = get_rpc_url();
    println!("Using RPC: {}", rpc_url);

    let client = RpcClient::new(rpc_url);

    // Check if we should process as lookup tables
    let is_lut = std::env::var("IS_LUT").unwrap_or_default() == "true";

    // Fetch all addresses provided as command line arguments
    for address_str in &args[1..] {
        if is_lut {
            fetch_and_process_lut(&client, address_str)?;
        } else {
            fetch_and_save_account(&client, address_str)?;
        }
    }

    println!("✅ Finished processing {} accounts", args.len() - 1);
    Ok(())
}

fn get_rpc_url() -> String {
    // Check for custom RPC_URL environment variable first
    if let Ok(custom_url) = std::env::var("RPC_URL") {
        return custom_url;
    }

    // Check for NETWORK environment variable for predefined networks
    match std::env::var("NETWORK").as_deref() {
        Ok("mainnet") => "https://api.mainnet-beta.solana.com".to_string(),
        Ok("devnet") => "https://api.devnet.solana.com".to_string(),
        Ok("testnet") => "https://api.testnet.solana.com".to_string(),
        Ok("localnet") | Ok("local") => "http://localhost:8899".to_string(),
        _ => "http://localhost:8899".to_string(), // default to localnet
    }
}

fn print_usage() {
    println!("Account Fetcher - Fetch Solana accounts and save as JSON");
    println!();
    println!("USAGE:");
    println!("  cargo run --bin fetch_rpc <pubkey1> <pubkey2> ...");
    println!();
    println!("NETWORKS:");
    println!("  Set NETWORK environment variable:");
    println!("    NETWORK=mainnet   - Solana Mainnet");
    println!("    NETWORK=devnet    - Solana Devnet");
    println!("    NETWORK=testnet   - Solana Testnet");
    println!("    NETWORK=local     - Local validator (default)");
    println!();
    println!("  Or set custom RPC_URL:");
    println!("    RPC_URL=https://your-custom-rpc.com");
    println!();
    println!("LOOKUP TABLE MODE:");
    println!("  Set IS_LUT=true to decode/modify lookup tables:");
    println!("    IS_LUT=true NETWORK=mainnet cargo run --bin fetch_rpc <lut_pubkey>");
    println!();
    println!("EXAMPLES:");
    println!("  # Fetch from mainnet");
    println!("  NETWORK=mainnet cargo run --bin fetch_rpc 11111111111111111111111111111111");
    println!();
    println!("  # Fetch multiple accounts from devnet");
    println!("  NETWORK=devnet cargo run --bin fetch_rpc \\");
    println!("    11111111111111111111111111111111 \\");
    println!("    TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
    println!();
    println!("  # Process lookup table");
    println!("  IS_LUT=true NETWORK=mainnet cargo run --bin fetch_rpc <lut_pubkey>");
}

fn fetch_and_process_lut(
    client: &RpcClient,
    address_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pubkey = Pubkey::from_str(address_str)?;
    println!("📥 Fetching lookup table: {}", pubkey);

    match client.get_account(&pubkey) {
        Ok(account) => {
            println!("✅ Fetched LUT: {} bytes", account.data.len());

            // Decode the lookup table
            let modified_data = decode_and_modify_lut(&account.data)?;

            let filename = format!("modified_lut_{}.json", pubkey);

            // Create JSON with modified data
            let data_base64 = encode(&modified_data);
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

            println!(
                "✅ Modified LUT saved: {} (last_extended_slot set to 0)",
                filename
            );
        }
        Err(e) => {
            println!("❌ Error fetching LUT {}: {}", pubkey, e);
        }
    }

    Ok(())
}

fn decode_and_modify_lut(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if data.len() < 56 {
        return Err("LUT data too small".into());
    }

    let mut modified_data = data.to_vec();

    // Based on Solana's AddressLookupTable structure:
    // - discriminator: u32 (4 bytes) - should be 1 for LookupTable
    // - deactivation_slot: u64 (8 bytes) - at offset 4
    // - last_extended_slot: u64 (8 bytes) - at offset 12 *** THIS IS WHAT WE MODIFY ***
    // - last_extended_slot_start_index: u8 (1 byte) - at offset 20
    // - authority: Option<Pubkey> (33 bytes max) - at offset 21
    // - _padding: u16 (2 bytes)
    // - addresses follow...

    // Verify this is a lookup table (discriminator should be 1)
    let discriminator = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if discriminator != 1 {
        return Err(format!(
            "Not a lookup table account (discriminator: {})",
            discriminator
        )
        .into());
    }

    println!("📊 LUT Analysis:");

    // Read current values for logging
    let deactivation_slot = u64::from_le_bytes([
        data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
    ]);
    let current_last_extended_slot = u64::from_le_bytes([
        data[12], data[13], data[14], data[15], data[16], data[17], data[18], data[19],
    ]);
    let last_extended_slot_start_index = data[20];

    println!("  Discriminator: {}", discriminator);
    println!("  Deactivation slot: {}", deactivation_slot);
    println!(
        "  Current last_extended_slot: {}",
        current_last_extended_slot
    );
    println!(
        "  Last extended slot start index: {}",
        last_extended_slot_start_index
    );

    // Check authority (1 byte for Some/None + potentially 32 bytes for pubkey)
    let has_authority = data[21] == 1;
    if has_authority && data.len() >= 54 {
        let authority_bytes = &data[22..54];
        let authority = Pubkey::try_from(authority_bytes)?;
        println!("  Authority: {}", authority);
    } else {
        println!("  Authority: None");
    }

    // Modify last_extended_slot to 0 (at offset 12, 8 bytes)
    let zero_bytes = 0u64.to_le_bytes();
    modified_data[12..20].copy_from_slice(&zero_bytes);

    println!(
        "🔧 Modified last_extended_slot: {} -> 0",
        current_last_extended_slot
    );

    // Calculate number of addresses
    let addresses_start = 56; // LOOKUP_TABLE_META_SIZE
    if data.len() > addresses_start {
        let addresses_data_len = data.len() - addresses_start;
        let num_addresses = addresses_data_len / 32; // Each address is 32 bytes
        println!("  Number of addresses: {}", num_addresses);
    }

    Ok(modified_data)
}

fn fetch_and_save_account(
    client: &RpcClient,
    address_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pubkey = Pubkey::from_str(address_str)?;
    println!("📥 Fetching account: {}", pubkey);

    match client.get_account(&pubkey) {
        Ok(account) => {
            let filename = format!("account_{}.json", pubkey);
            write_account_json(&account, &pubkey, &filename)?;

            println!(
                "✅ Saved: {} ({} bytes, {} lamports)",
                filename,
                account.data.len(),
                account.lamports
            );
        }
        Err(e) => {
            println!("❌ Error fetching {}: {}", pubkey, e);
        }
    }

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
