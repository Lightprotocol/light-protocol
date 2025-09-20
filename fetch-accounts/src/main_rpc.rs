use std::{fs::File, io::Write, str::FromStr};

use base64::{engine::general_purpose, Engine as _};
use serde_json::json;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    // Set via RPC_URL env or use preset.
    // env takes precedence.
    let rpc_url = get_rpc_url();
    println!("Using RPC: {}", rpc_url);

    let client = RpcClient::new(rpc_url);

    let is_lut = std::env::var("IS_LUT").unwrap_or_default() == "true";
    for address_str in &args[1..] {
        if is_lut {
            fetch_and_process_lut(&client, address_str)?;
        } else {
            fetch_and_save_account(&client, address_str)?;
        }
    }

    println!("Processed {} accounts", args.len() - 1);
    Ok(())
}

fn get_rpc_url() -> String {
    if let Ok(custom_url) = std::env::var("RPC_URL") {
        return custom_url;
    }

    match std::env::var("NETWORK").as_deref() {
        Ok("mainnet") => "https://api.mainnet-beta.solana.com".to_string(),
        Ok("devnet") => "https://api.devnet.solana.com".to_string(),
        Ok("testnet") => "https://api.testnet.solana.com".to_string(),
        Ok("localnet") | Ok("local") => "http://localhost:8899".to_string(),
        _ => "http://localhost:8899".to_string(),
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
    println!("  Add ADD_PUBKEY=<pubkey> to add a single pubkey to the lookup table:");
    println!("    IS_LUT=true ADD_PUBKEY=ComputeBudget111111111111111111111111111111 \\");
    println!("      NETWORK=mainnet cargo run --bin fetch_rpc <lut_pubkey>");
    println!("  Add ADD_PUBKEYS=<pubkey1,pubkey2,...> to add multiple pubkeys:");
    println!("    IS_LUT=true ADD_PUBKEYS=ComputeBudget111111111111111111111111111111,TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA \\");
    println!("      NETWORK=mainnet cargo run --bin fetch_rpc <lut_pubkey>");
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
    println!();
    println!("  # Add single pubkey to lookup table");
    println!("  IS_LUT=true ADD_PUBKEY=ComputeBudget111111111111111111111111111111 \\");
    println!("    NETWORK=mainnet cargo run --bin fetch_rpc <lut_pubkey>");
    println!();
    println!("  # Add multiple pubkeys to lookup table");
    println!("  IS_LUT=true ADD_PUBKEYS=EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK,6L7SzhYB3anwEQ9cphpJ1U7Scwj57bx2xueReg7R9cKU \\");
    println!("    NETWORK=mainnet cargo run --bin fetch_rpc <lut_pubkey>");
}

fn fetch_and_process_lut(
    client: &RpcClient,
    address_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pubkey = Pubkey::from_str(address_str)?;
    println!("Fetching lookup table: {}", pubkey);

    match client.get_account(&pubkey) {
        Ok(account) => {
            let modified_data = decode_and_modify_lut(&account.data)?;

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
        }
        Err(e) => {
            println!("Error fetching LUT {}: {}", pubkey, e);
            return Err(e.into());
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

    // CHECK: disc = 1
    let discriminator = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if discriminator != 1 {
        return Err(format!(
            "Not a lookup table account (discriminator: {})",
            discriminator
        )
        .into());
    }

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

    // MUT: last_extended_slot to 0 (at offset 12, 8 bytes)
    let zero_bytes = 0u64.to_le_bytes();
    modified_data[12..20].copy_from_slice(&zero_bytes);

    println!(
        "🔧 Modified last_extended_slot: {} -> 0",
        current_last_extended_slot
    );

    // Calculate number of addresses
    let addresses_start = 56; // LOOKUP_TABLE_META_SIZE
    let mut num_addresses = 0;
    if data.len() > addresses_start {
        let addresses_data_len = data.len() - addresses_start;
        num_addresses = addresses_data_len / 32;
        println!("  Number of addresses: {}", num_addresses);
    }

    // Check if we should add pubkeys
    if let Ok(add_pubkeys_str) = std::env::var("ADD_PUBKEYS") {
        let pubkey_strings: Vec<&str> = add_pubkeys_str.split(',').map(|s| s.trim()).collect();
        let mut added_count = 0;

        for pubkey_str in pubkey_strings {
            if pubkey_str.is_empty() {
                continue;
            }

            let add_pubkey = Pubkey::from_str(pubkey_str)?;

            // Check if pubkey already exists
            let mut already_exists = false;
            let current_num_addresses = num_addresses + added_count;

            for i in 0..current_num_addresses {
                let addr_start = addresses_start + (i * 32);
                let addr_end = addr_start + 32;
                if addr_end <= modified_data.len() {
                    let existing_pubkey = Pubkey::try_from(&modified_data[addr_start..addr_end])?;
                    if existing_pubkey == add_pubkey {
                        already_exists = true;
                        println!("  Pubkey {} already exists at index {}", add_pubkey, i);
                        break;
                    }
                }
            }

            if !already_exists {
                // Add the new pubkey to the end
                modified_data.extend_from_slice(&add_pubkey.to_bytes());
                println!(
                    "🔧 Added pubkey: {} at index {}",
                    add_pubkey, current_num_addresses
                );
                added_count += 1;
            }
        }
    }
    // Fallback to single ADD_PUBKEY for backward compatibility
    else if let Ok(add_pubkey_str) = std::env::var("ADD_PUBKEY") {
        let add_pubkey = Pubkey::from_str(&add_pubkey_str)?;

        // Check if pubkey already exists
        let mut already_exists = false;
        for i in 0..num_addresses {
            let addr_start = addresses_start + (i * 32);
            let addr_end = addr_start + 32;
            if addr_end <= data.len() {
                let existing_pubkey = Pubkey::try_from(&data[addr_start..addr_end])?;
                if existing_pubkey == add_pubkey {
                    already_exists = true;
                    println!("  Pubkey {} already exists at index {}", add_pubkey, i);
                    break;
                }
            }
        }

        if !already_exists {
            // Add the new pubkey to the end
            modified_data.extend_from_slice(&add_pubkey.to_bytes());
            println!("🔧 Added pubkey: {} at index {}", add_pubkey, num_addresses);
        }
    }

    Ok(modified_data)
}

fn fetch_and_save_account(
    client: &RpcClient,
    address_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pubkey = Pubkey::from_str(address_str)?;

    match client.get_account(&pubkey) {
        Ok(account) => {
            let filename = format!("account_{}.json", pubkey);
            write_account_json(&account, &pubkey, &filename)?;

            println!(
                "Saved {} ({} bytes, {} lamports)",
                filename,
                account.data.len(),
                account.lamports
            );
        }
        Err(e) => {
            println!("Error fetching {}: {}", pubkey, e);
            return Err(e.into());
        }
    }

    Ok(())
}

fn write_account_json(
    account: &solana_sdk::account::Account,
    pubkey: &Pubkey,
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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
