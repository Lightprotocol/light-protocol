use std::sync::Arc;

use account_compression::utils::constants::{ADDRESS_QUEUE_VALUES, STATE_NULLIFIER_QUEUE_VALUES};
use anchor_lang::{AccountDeserialize, Discriminator};
use itertools::Itertools;
use light_client::rpc::{RpcConnection, SolanaRpcConnection};
use light_merkle_tree_metadata::merkle_tree::TreeType;
use light_registry::{protocol_config::state::ProtocolConfigPda, EpochPda, ForesterEpochPda};
use prettytable::{format, Cell, Row, Table};
use solana_sdk::{account::ReadableAccount, commitment_config::CommitmentConfig};
use tracing::{debug, warn};

use crate::{
    cli::StatusArgs,
    metrics::{push_metrics, register_metrics, update_registered_foresters},
    queue_helpers::fetch_queue_item_data,
    rollover::get_tree_fullness,
    tree_data_sync::fetch_trees,
    ForesterConfig,
};

pub async fn fetch_forester_status(args: &StatusArgs) {
    let commitment_config = CommitmentConfig::confirmed();

    let client = solana_client::rpc_client::RpcClient::new_with_commitment(
        args.rpc_url.clone(),
        commitment_config,
    );

    // Fetch and parse registry accounts
    let registry_accounts = client
        .get_program_accounts(&light_registry::ID)
        .expect("Failed to fetch accounts for registry program.");

    let mut forester_epoch_pdas = vec![];
    let mut epoch_pdas = vec![];
    let mut protocol_config_pdas = vec![];

    for (_, account) in registry_accounts {
        match account.data()[0..8].try_into().unwrap() {
            ForesterEpochPda::DISCRIMINATOR => {
                let forester_epoch_pda =
                    ForesterEpochPda::try_deserialize_unchecked(&mut account.data())
                        .expect("Failed to deserialize ForesterEpochPda");
                forester_epoch_pdas.push(forester_epoch_pda);
            }
            EpochPda::DISCRIMINATOR => {
                let epoch_pda = EpochPda::try_deserialize_unchecked(&mut account.data())
                    .expect("Failed to deserialize EpochPda");
                epoch_pdas.push(epoch_pda);
            }
            ProtocolConfigPda::DISCRIMINATOR => {
                let protocol_config_pda =
                    ProtocolConfigPda::try_deserialize_unchecked(&mut account.data())
                        .expect("Failed to deserialize ProtocolConfigPda");
                protocol_config_pdas.push(protocol_config_pda);
            }
            _ => (),
        }
    }

    forester_epoch_pdas.sort_by(|a, b| a.epoch.cmp(&b.epoch));
    epoch_pdas.sort_by(|a, b| a.epoch.cmp(&b.epoch));

    // Get current slot and epochs
    let slot = client.get_slot().expect("Failed to fetch slot.");
    let current_active_epoch = protocol_config_pdas[0]
        .config
        .get_current_active_epoch(slot)
        .unwrap();
    let current_registration_epoch = protocol_config_pdas[0]
        .config
        .get_latest_register_epoch(slot)
        .unwrap();

    // Print epoch information
    println!("\n=== Epoch Status ===");
    println!("Current Active Epoch: {}", current_active_epoch);
    println!("Registration Epoch: {}", current_registration_epoch);

    // Progress and time information
    let current_progress = protocol_config_pdas[0]
        .config
        .get_current_active_epoch_progress(slot);
    let total_length = protocol_config_pdas[0].config.active_phase_length;
    let progress_percentage = current_progress as f64 / total_length as f64 * 100.0;

    println!(
        "\nActive Epoch Progress: ({}/{}) {}",
        current_progress,
        total_length,
        format_progress_bar(progress_percentage, 50)
    );

    let hours_until_epoch = total_length.saturating_sub(current_progress) * 460 / 1000 / 3600;

    let slots_until_registration = protocol_config_pdas[0]
        .config
        .registration_phase_length
        .saturating_sub(current_progress);

    let hours_until_registration = slots_until_registration * 460 / 1000 / 3600;

    println!(
        "Time Until Next Epoch: {}",
        format_time_duration(hours_until_epoch)
    );
    println!(
        "Time Until Registration: {}",
        format_time_duration(hours_until_registration)
    );
    println!("Slots Until Registration: {}", slots_until_registration);

    println!("\n=== Active Foresters ===");
    let grouped = forester_epoch_pdas
        .clone()
        .into_iter()
        .chunk_by(|pda| pda.epoch);

    for (epoch, group) in &grouped {
        if epoch == current_active_epoch {
            println!("\nActive Epoch Foresters:");
        } else if epoch == current_registration_epoch {
            println!("\nRegistration Epoch Foresters:");
        }

        let foresters: Vec<_> = group.collect();
        for (idx, forester) in foresters.iter().enumerate() {
            if (epoch == current_active_epoch) || (epoch == current_registration_epoch) {
                println!("  {}. {}", idx + 1, forester.authority);
            }
            update_registered_foresters(epoch, &forester.authority.to_string());
        }
    }

    // Print full epoch information if requested
    if args.full {
        println!("\n=== Full Epoch Information ===");
        for epoch in &epoch_pdas {
            println!("\nEpoch {}", epoch.epoch);
            let registered_foresters_in_epoch = forester_epoch_pdas
                .iter()
                .filter(|pda| pda.epoch == epoch.epoch);
            for forester in registered_foresters_in_epoch {
                println!("  Forester: {}", forester.authority);
            }
        }
    }

    // Print protocol config if requested
    if args.protocol_config {
        println!("\n=== Protocol Configuration ===");
        println!("{:#?}", protocol_config_pdas[0]);
    }

    // Initialize config and metrics
    let config = Arc::new(ForesterConfig::new_for_status(args).unwrap());
    if config.general_config.enable_metrics {
        register_metrics();
    }

    // Fetch tree information
    debug!("Fetching trees...");
    let mut rpc = SolanaRpcConnection::new(config.external_services.rpc_url.clone(), None);
    let trees = fetch_trees(&rpc).await.unwrap();

    if trees.is_empty() {
        warn!("No trees found. Exiting.");
        return;
    }

    // Create and display combined tree and queue status table
    println!("\n=== Tree Status ===");
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_BOX_CHARS);
    table.add_row(Row::new(vec![
        Cell::new("Type"),
        Cell::new("Tree Address"),
        Cell::new("Queue Address"),
        Cell::new("Fullness"),
        Cell::new("Next Index"),
        Cell::new("Threshold"),
        Cell::new("Queue Size"),
    ]));

    // Sort trees by type and address for stable output
    let mut sorted_trees = trees.clone();
    sorted_trees.sort_by(|a, b| {
        let type_cmp = a.tree_type.cmp(&b.tree_type);
        let queue_address_cmp = a.queue.cmp(&b.queue);

        if type_cmp == std::cmp::Ordering::Equal {
            queue_address_cmp
        } else {
            type_cmp
        }
    });

    for tree in &sorted_trees {
        let tree_fullness = get_tree_fullness(&mut rpc, tree.merkle_tree, tree.tree_type)
            .await
            .unwrap();

        let queue_length = fetch_queue_item_data(
            &mut rpc,
            &tree.queue,
            0,
            match tree.tree_type {
                TreeType::State => STATE_NULLIFIER_QUEUE_VALUES,
                _ => ADDRESS_QUEUE_VALUES,
            },
            match tree.tree_type {
                TreeType::State => STATE_NULLIFIER_QUEUE_VALUES,
                _ => ADDRESS_QUEUE_VALUES,
            },
        )
        .await
        .unwrap()
        .len();

        table.add_row(Row::new(vec![
            Cell::new(match tree.tree_type {
                TreeType::State => "State",
                TreeType::Address => "Address",
                TreeType::BatchedState => "BatchedState",
                TreeType::BatchedAddress => "BatchedAddress",
            }),
            Cell::new(&tree.merkle_tree.to_string()),
            Cell::new(&tree.queue.to_string()),
            Cell::new(&format!("{:.2}%", tree_fullness.fullness * 100.0)),
            Cell::new(&tree_fullness.next_index.to_string()),
            Cell::new(&tree_fullness.threshold.to_string()),
            Cell::new(&queue_length.to_string()),
        ]));
    }
    table.printstd();

    // Push metrics if enabled
    if let Err(e) = push_metrics(&config.external_services.pushgateway_url).await {
        warn!("Failed to push metrics: {}", e);
    }
}

pub fn format_progress_bar(progress: f64, width: usize) -> String {
    let filled_width = ((progress / 100.0) * width as f64) as usize;
    let empty_width = width - filled_width;

    format!(
        "[{}{}] {:.2}%",
        "=".repeat(filled_width),
        " ".repeat(empty_width),
        progress
    )
}

pub fn format_time_duration(hours: u64) -> String {
    if hours >= 24 {
        let days = hours / 24;
        let remaining_hours = hours % 24;
        format!("{} days {} hours", days, remaining_hours)
    } else {
        format!("{} hours", hours)
    }
}
