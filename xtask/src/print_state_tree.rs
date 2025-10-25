use std::{mem, str::FromStr};

use clap::Parser;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_client::rpc::{LightClient, LightClientConfig, Rpc};
use light_compressed_account::{pubkey::Pubkey, TreeType};
use light_concurrent_merkle_tree::zero_copy::ConcurrentMerkleTreeZeroCopy;
use light_hasher::Poseidon;
use light_merkle_tree_metadata::merkle_tree::MerkleTreeMetadata;
use solana_sdk::{account::Account, bs58, pubkey::Pubkey as SolanaPubkey};

#[derive(Debug, Parser)]
pub struct Options {
    /// The pubkey of the state Merkle tree to print
    #[clap(long)]
    pubkey: String,
    /// Network: mainnet, devnet, local, or custom URL
    #[clap(long, default_value = "mainnet")]
    network: String,
}

pub async fn print_state_tree(options: Options) -> anyhow::Result<()> {
    let rpc_url = match options.network.as_str() {
        "local" => String::from("http://127.0.0.1:8899"),
        "devnet" => String::from("https://api.devnet.solana.com"),
        "mainnet" => String::from("https://api.mainnet-beta.solana.com"),
        _ => options.network.clone(),
    };

    let rpc = LightClient::new(LightClientConfig {
        url: rpc_url.clone(),
        photon_url: None,
        commitment_config: None,
        fetch_active_tree: false,
        api_key: None,
    })
    .await?;

    let pubkey = SolanaPubkey::from_str(&options.pubkey)?;
    let pubkey_bytes: [u8; 32] = pubkey.to_bytes();
    let light_pubkey = Pubkey::new_from_array(pubkey_bytes);

    println!("Fetching account: {}", pubkey);
    println!("RPC URL: {}", rpc_url);
    println!();

    let account = rpc
        .get_account(pubkey)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Account not found"))?;
    let mut account_data = account.data.clone();

    // Try v2 (batched) first
    if let Ok(tree) = BatchedMerkleTreeAccount::state_from_bytes(&mut account_data, &light_pubkey) {
        print_v2_tree(&tree, &pubkey, &account)?;
    } else {
        // Try v1 (concurrent)
        print_v1_tree(&account_data, &pubkey, &account)?;
    }

    Ok(())
}

fn print_v2_tree(
    tree: &BatchedMerkleTreeAccount,
    pubkey: &SolanaPubkey,
    account: &Account,
) -> anyhow::Result<()> {
    println!("=== Batched State Merkle Tree (V2) Metadata ===");
    println!();
    println!("Pubkey: {}", pubkey);
    println!("Tree Type: {:?}", TreeType::from(tree.tree_type));
    println!("Height: {}", tree.height);
    println!("Capacity: {}", tree.capacity);
    println!();

    println!("=== Tree State ===");
    println!("Next Index: {}", tree.next_index);
    println!("Sequence Number: {}", tree.sequence_number);
    println!("Nullifier Next Index: {}", tree.nullifier_next_index);
    println!();

    println!("=== Root History ===");
    println!("Root History Capacity: {}", tree.root_history_capacity);
    println!("Current Root Index: {}", tree.get_root_index());
    if let Some(current_root) = tree.get_root() {
        println!("Current Root: {}", bs58::encode(current_root).into_string());
    }
    println!();

    println!("=== Access Metadata ===");
    println!(
        "Owner: {}",
        SolanaPubkey::from(tree.metadata.access_metadata.owner.to_bytes())
    );
    let program_owner = SolanaPubkey::from(tree.metadata.access_metadata.program_owner.to_bytes());
    if program_owner != SolanaPubkey::default() {
        println!("Program Owner: {}", program_owner);
    } else {
        println!("Program Owner: None");
    }
    let forester = SolanaPubkey::from(tree.metadata.access_metadata.forester.to_bytes());
    if forester != SolanaPubkey::default() {
        println!("Forester: {}", forester);
    } else {
        println!("Forester: None");
    }
    println!();

    println!("=== Rollover Metadata ===");
    println!("Index: {}", tree.metadata.rollover_metadata.index);
    println!(
        "Rollover Fee: {}",
        tree.metadata.rollover_metadata.rollover_fee
    );
    let threshold = tree.metadata.rollover_metadata.rollover_threshold;
    if threshold > 0 {
        println!("Rollover Threshold: {}", threshold);
    } else {
        println!("Rollover Threshold: None");
    }
    println!(
        "Network Fee: {}",
        tree.metadata.rollover_metadata.network_fee
    );
    let next_merkle_tree = SolanaPubkey::from(tree.metadata.next_merkle_tree.to_bytes());
    if next_merkle_tree != SolanaPubkey::default() {
        println!("Next Merkle Tree: {}", next_merkle_tree);
    } else {
        println!("Next Merkle Tree: None");
    }
    println!();

    println!("=== Queue Configuration ===");
    let associated_queue = SolanaPubkey::from(tree.metadata.associated_queue.to_bytes());
    if associated_queue != SolanaPubkey::default() {
        println!("Associated Queue: {}", associated_queue);
    } else {
        println!("Associated Queue: None");
    }
    println!("Num Batches: {}", tree.queue_batches.num_batches);
    println!("Batch Size: {}", tree.queue_batches.batch_size);
    println!("ZKP Batch Size: {}", tree.queue_batches.zkp_batch_size);
    println!(
        "Bloom Filter Capacity: {}",
        tree.queue_batches.bloom_filter_capacity
    );
    println!(
        "Currently Processing Batch Index: {}",
        tree.queue_batches.currently_processing_batch_index
    );
    println!(
        "Pending Batch Index: {}",
        tree.queue_batches.pending_batch_index
    );
    println!("Next Index: {}", tree.queue_batches.next_index);
    println!();

    println!("=== Batch States ===");
    for (i, batch) in tree.queue_batches.batches.iter().enumerate() {
        println!("Batch {}:", i);
        println!("  State: {:?}", batch.get_state());
        println!(
            "  Num Inserted Elements: {}",
            batch.get_num_inserted_elements()
        );
        println!("  Num Inserted ZKPs: {}", batch.get_num_inserted_zkps());
        println!("  Bloom Filter Zeroed: {}", batch.bloom_filter_is_zeroed());
        println!("  Start Index: {}", batch.start_index);
        println!("  Start Slot: {}", batch.start_slot);
        println!("  Sequence Number: {}", batch.sequence_number);
        println!("  Root Index: {}", batch.root_index);
        println!();
    }

    println!("=== Account Info ===");
    println!("Account Size: {} bytes", account.data.len());
    println!("Lamports: {}", account.lamports);
    println!("Owner: {}", account.owner);
    println!();

    Ok(())
}

fn print_v1_tree(
    account_data: &[u8],
    pubkey: &SolanaPubkey,
    account: &Account,
) -> anyhow::Result<()> {
    // Skip discriminator (8 bytes)
    let metadata_offset = 8;
    let metadata_size = mem::size_of::<MerkleTreeMetadata>();
    let metadata_bytes = &account_data[metadata_offset..metadata_offset + metadata_size];

    // Safety: MerkleTreeMetadata is Pod and we're reading exactly the right size
    let metadata: &MerkleTreeMetadata =
        unsafe { &*(metadata_bytes.as_ptr() as *const MerkleTreeMetadata) };

    // Parse the concurrent merkle tree
    let tree_data = &account_data[8 + mem::size_of::<MerkleTreeMetadata>()..];
    let tree = ConcurrentMerkleTreeZeroCopy::<Poseidon, 26>::from_bytes_zero_copy(tree_data)?;

    println!("=== State Merkle Tree (V1) Metadata ===");
    println!();
    println!("Pubkey: {}", pubkey);
    println!("Height: {}", tree.height);
    println!("Canopy Depth: {}", tree.canopy_depth);
    println!("Capacity: {}", 2_usize.pow(tree.height as u32));
    println!();

    println!("=== Tree State ===");
    println!("Next Index: {}", tree.next_index());
    println!("Sequence Number: {}", tree.sequence_number());
    println!();

    println!("=== Root History ===");
    println!("Roots Capacity: {}", tree.roots.capacity());
    println!("Current Root Index: {}", tree.root_index());
    println!("Current Root: {}", bs58::encode(tree.root()).into_string());
    println!();

    println!("=== Changelog ===");
    println!("Changelog Capacity: {}", tree.changelog.capacity());
    println!();

    println!("=== Access Metadata ===");
    println!(
        "Owner: {}",
        SolanaPubkey::from(metadata.access_metadata.owner.to_bytes())
    );
    let program_owner = SolanaPubkey::from(metadata.access_metadata.program_owner.to_bytes());
    if program_owner != SolanaPubkey::default() {
        println!("Program Owner: {}", program_owner);
    } else {
        println!("Program Owner: None");
    }
    let forester = SolanaPubkey::from(metadata.access_metadata.forester.to_bytes());
    if forester != SolanaPubkey::default() {
        println!("Forester: {}", forester);
    } else {
        println!("Forester: None");
    }
    println!();

    println!("=== Rollover Metadata ===");
    println!("Index: {}", metadata.rollover_metadata.index);
    println!("Rollover Fee: {}", metadata.rollover_metadata.rollover_fee);
    let threshold = metadata.rollover_metadata.rollover_threshold;
    if threshold > 0 {
        println!("Rollover Threshold: {}", threshold);
    } else {
        println!("Rollover Threshold: None");
    }
    println!("Network Fee: {}", metadata.rollover_metadata.network_fee);
    let next_merkle_tree = SolanaPubkey::from(metadata.next_merkle_tree.to_bytes());
    if next_merkle_tree != SolanaPubkey::default() {
        println!("Next Merkle Tree: {}", next_merkle_tree);
    } else {
        println!("Next Merkle Tree: None");
    }
    println!();

    println!("=== Queue Configuration ===");
    let associated_queue = SolanaPubkey::from(metadata.associated_queue.to_bytes());
    if associated_queue != SolanaPubkey::default() {
        println!("Associated Queue: {}", associated_queue);
    } else {
        println!("Associated Queue: None");
    }
    println!();

    println!("=== Account Info ===");
    println!("Account Size: {} bytes", account.data.len());
    println!("Lamports: {}", account.lamports);
    println!("Owner: {}", account.owner);
    println!();

    Ok(())
}
