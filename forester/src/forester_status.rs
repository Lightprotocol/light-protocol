use std::sync::Arc;

use anchor_lang::{AccountDeserialize, Discriminator};
use forester_utils::forester_epoch::{get_epoch_phases, TreeAccounts};
use itertools::Itertools;
use light_client::rpc::{LightClient, LightClientConfig, Rpc};
use light_compressed_account::TreeType;
use light_registry::{protocol_config::state::ProtocolConfigPda, EpochPda, ForesterEpochPda};
use solana_program::{clock::Slot, pubkey::Pubkey};
use solana_sdk::{account::ReadableAccount, commitment_config::CommitmentConfig};
use tracing::{debug, warn};

use crate::{
    cli::StatusArgs,
    metrics::{push_metrics, register_metrics, update_registered_foresters},
    rollover::get_tree_fullness,
    run_queue_info,
    tree_data_sync::fetch_trees,
    ForesterConfig,
};

pub async fn fetch_forester_status(args: &StatusArgs) -> crate::Result<()> {
    let commitment_config = CommitmentConfig::confirmed();

    let client = solana_client::rpc_client::RpcClient::new_with_commitment(
        args.rpc_url.clone(),
        commitment_config,
    );
    let registry_accounts = client
        .get_program_accounts(&light_registry::ID)
        .expect("Failed to fetch accounts for registry program.");

    let mut forester_epoch_pdas = vec![];
    let mut epoch_pdas = vec![];
    let mut protocol_config_pdas = vec![];
    for (_, account) in registry_accounts {
        match account.data()[0..8].try_into()? {
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
    let slot = client.get_slot().expect("Failed to fetch slot.");

    println!("Current Solana Slot: {}", slot);

    let current_active_epoch = protocol_config_pdas[0]
        .config
        .get_current_active_epoch(slot)?;
    let current_registration_epoch = protocol_config_pdas[0]
        .config
        .get_latest_register_epoch(slot)?;
    println!("Current active epoch: {:?}", current_active_epoch);

    println!(
        "Current registration epoch: {:?}",
        current_registration_epoch
    );

    println!("Forester registrations by epoch:");

    let grouped = forester_epoch_pdas
        .clone()
        .into_iter()
        .chunk_by(|pda| pda.epoch);

    for (epoch, group) in &grouped {
        if epoch == current_active_epoch {
            println!("Active Epoch:");
        } else if epoch == current_registration_epoch {
            println!("Registration Epoch:");
        }
        let foresters: Vec<_> = group.collect();
        for (idx, forester) in foresters.iter().enumerate() {
            if (epoch == current_active_epoch) || (epoch == current_registration_epoch) {
                println!("  {}: {}", idx, forester.authority);
            }
            update_registered_foresters(epoch, &forester.authority.to_string());
        }
    }

    println!(
        "Forester registered for active epoch: {:?}",
        forester_epoch_pdas
            .iter()
            .any(|pda| pda.epoch == current_active_epoch)
    );
    println!(
        "current active epoch progress {:?} / {}",
        protocol_config_pdas[0]
            .config
            .get_current_active_epoch_progress(slot),
        protocol_config_pdas[0].config.active_phase_length
    );
    println!(
        "current active epoch progress {:.2?}%",
        protocol_config_pdas[0]
            .config
            .get_current_active_epoch_progress(slot) as f64
            / protocol_config_pdas[0].config.active_phase_length as f64
            * 100f64
    );
    println!("Hours until next epoch : {:?} hours", {
        // slotduration is 460ms and 1000ms is 1 second and 3600 seconds is 1 hour
        protocol_config_pdas[0]
            .config
            .active_phase_length
            .saturating_sub(
                protocol_config_pdas[0]
                    .config
                    .get_current_active_epoch_progress(slot),
            )
            * 460
            / 1000
            / 3600
    });
    let slots_until_next_registration = protocol_config_pdas[0]
        .config
        .registration_phase_length
        .saturating_sub(
            protocol_config_pdas[0]
                .config
                .get_current_active_epoch_progress(slot),
        );
    println!(
        "Slots until next registration : {:?}",
        slots_until_next_registration
    );
    println!(
        "Hours until next registration : {:?} hours",
        // slotduration is 460ms and 1000ms is 1 second and 3600 seconds is 1 hour
        slots_until_next_registration * 460 / 1000 / 3600
    );
    if args.full {
        for epoch in &epoch_pdas {
            println!("Epoch: {:?}", epoch.epoch);
            let registered_foresters_in_epoch = forester_epoch_pdas
                .iter()
                .filter(|pda| pda.epoch == epoch.epoch);
            for forester in registered_foresters_in_epoch {
                println!("Forester authority: {:?}", forester.authority);
            }
        }
    }
    if args.protocol_config {
        println!("protocol config: {:?}", protocol_config_pdas[0]);
    }

    let config = Arc::new(ForesterConfig::new_for_status(args)?);

    if config.general_config.enable_metrics {
        register_metrics();
    }

    debug!("Fetching trees...");
    debug!("RPC URL: {}", config.external_services.rpc_url);
    let mut rpc = LightClient::new(LightClientConfig {
        url: config.external_services.rpc_url.to_string(),
        photon_url: config.external_services.indexer_url.clone(),
        api_key: config.external_services.photon_api_key.clone(),
        commitment_config: None,
        fetch_active_tree: false,
    })
    .await?;
    let trees = fetch_trees(&rpc)
        .await?
        .iter()
        .sorted_by_key(|t| t.merkle_tree.to_string())
        .cloned()
        .collect::<Vec<_>>();

    if trees.is_empty() {
        warn!("No trees found. Exiting.");
    }
    run_queue_info(config.clone(), &trees, TreeType::StateV1).await?;
    run_queue_info(config.clone(), &trees, TreeType::AddressV1).await?;

    run_queue_info(config.clone(), &trees, TreeType::StateV2).await?;
    run_queue_info(config.clone(), &trees, TreeType::AddressV2).await?;

    for tree in &trees {
        // Skip rolled-over trees
        if tree.is_rolledover {
            continue;
        }

        let tree_type = format!("{}", tree.tree_type);
        let tree_info = get_tree_fullness(&mut rpc, tree.merkle_tree, tree.tree_type).await?;
        let fullness_percentage = tree_info.fullness * 100.0;
        println!(
            "{} {}: Fullness: {:.4}% | Next Index: {} | Threshold: {}",
            tree_type,
            &tree.merkle_tree,
            format!("{:.2}%", fullness_percentage),
            tree_info.next_index,
            tree_info.threshold
        );

        if args.full {
            println!("Checking Forester Assignment for {}...", tree.merkle_tree);

            let active_epoch_foresters: Vec<ForesterEpochPda> = forester_epoch_pdas
                .iter()
                .filter(|item| item.epoch == current_active_epoch)
                .cloned()
                .collect();

            let current_epoch_pda_entry = epoch_pdas
                .iter()
                .find(|pda| pda.epoch == current_active_epoch);

            let protocol_config = protocol_config_pdas[0].clone();

            print_tree_schedule_by_forester(
                slot,
                current_active_epoch,
                active_epoch_foresters,
                tree.merkle_tree,
                tree.queue,
                current_epoch_pda_entry,
                &protocol_config,
            );
        }
    }

    println!("\n=== CURRENT ACTIVE FORESTER ASSIGNMENTS ===");
    let active_epoch_foresters: Vec<ForesterEpochPda> = forester_epoch_pdas
        .iter()
        .filter(|item| item.epoch == current_active_epoch)
        .cloned()
        .collect();

    let current_epoch_pda_entry = epoch_pdas
        .iter()
        .find(|pda| pda.epoch == current_active_epoch);

    let protocol_config = protocol_config_pdas[0].clone();

    // Filter out rolled-over trees
    let active_trees: Vec<TreeAccounts> =
        trees.iter().filter(|t| !t.is_rolledover).cloned().collect();

    if !active_epoch_foresters.is_empty() && current_epoch_pda_entry.is_some() {
        print_current_forester_assignments(
            slot,
            current_active_epoch,
            active_epoch_foresters,
            &active_trees,
            current_epoch_pda_entry,
            &protocol_config,
        );
    } else {
        println!("No active foresters found for the current epoch.");
    }

    push_metrics(&config.external_services.pushgateway_url).await?;

    Ok(())
}

fn print_current_forester_assignments(
    slot: Slot,
    current_active_epoch: u64,
    active_epoch_foresters: Vec<ForesterEpochPda>,
    trees: &Vec<TreeAccounts>,
    current_epoch_pda_entry: Option<&EpochPda>,
    protocol_config: &ProtocolConfigPda,
) {
    if let Some(_current_epoch_pda) = current_epoch_pda_entry {
        if active_epoch_foresters.is_empty() {
            println!(
                "ERROR: No foresters registered for active epoch {}",
                current_active_epoch
            );
            return;
        }

        let total_epoch_weight = match active_epoch_foresters[0].total_epoch_weight {
            Some(w) if w > 0 => w,
            _ => {
                println!(
                    "ERROR: Registration not finalized (total_epoch_weight is None or 0) for epoch {}.",
                    current_active_epoch
                );
                return;
            }
        };

        let epoch_phases = get_epoch_phases(&protocol_config.config, current_active_epoch);

        if slot < epoch_phases.active.start || slot >= epoch_phases.active.end {
            println!(
                "Info: Not currently within the active phase of epoch {}.",
                current_active_epoch
            );
            return;
        }

        if protocol_config.config.slot_length == 0 {
            println!("ERROR: ProtocolConfig slot_length is zero. Cannot calculate light slots.");
            return;
        }

        let current_light_slot_index =
            (slot - epoch_phases.active.start) / protocol_config.config.slot_length;
        let start_solana_slot_of_current_light_slot = epoch_phases.active.start
            + current_light_slot_index * protocol_config.config.slot_length;
        let end_solana_slot_of_current_light_slot =
            start_solana_slot_of_current_light_slot + protocol_config.config.slot_length;

        let slots_remaining_in_light_slot =
            end_solana_slot_of_current_light_slot.saturating_sub(slot);
        let time_remaining_secs = slots_remaining_in_light_slot as f64 * 0.460;

        println!(
            "Current Light Slot Index: {} (Solana slots {}-{}, Approx. {:.2}s remaining)",
            current_light_slot_index,
            start_solana_slot_of_current_light_slot,
            end_solana_slot_of_current_light_slot - 1,
            time_remaining_secs
        );

        println!("Queue processors for the current light slot:");
        println!("Tree Type\t\tTree Address\tForester");

        for tree in trees {
            let eligible_forester_slot_index = match ForesterEpochPda::get_eligible_forester_index(
                current_light_slot_index,
                &tree.queue,
                total_epoch_weight,
                current_active_epoch,
            ) {
                Ok(idx) => idx,
                Err(e) => {
                    println!(
                        "{:12}\t\t{}\tERROR: {:?}",
                        tree.tree_type, tree.merkle_tree, e
                    );
                    continue;
                }
            };

            let assigned_forester = active_epoch_foresters
                .iter()
                .find(|pda| pda.is_eligible(eligible_forester_slot_index));

            if let Some(forester_pda) = assigned_forester {
                println!(
                    "{:12}\t\t{}\t{}",
                    tree.tree_type, tree.merkle_tree, forester_pda.authority
                );
            } else {
                println!(
                    "{:12}\t\t{}\tUNASSIGNED (Eligible Index: {})",
                    tree.tree_type, tree.merkle_tree, eligible_forester_slot_index
                );
            }
        }
    } else {
        println!(
            "ERROR: Could not find EpochPda for active epoch {}. Cannot determine forester assignments.",
            current_active_epoch
        );
    }
}

fn print_tree_schedule_by_forester(
    slot: Slot,
    current_active_epoch: u64,
    active_epoch_foresters: Vec<ForesterEpochPda>,
    tree: Pubkey,
    queue: Pubkey,
    current_epoch_pda_entry: Option<&EpochPda>,
    protocol_config: &ProtocolConfigPda,
) {
    if let Some(_current_epoch_pda) = current_epoch_pda_entry {
        if active_epoch_foresters.is_empty() {
            println!(
                "ERROR: No foresters registered for tree {} in active epoch {}",
                tree, current_active_epoch
            );
        } else {
            let total_epoch_weight = match active_epoch_foresters[0].total_epoch_weight {
                Some(w) if w > 0 => w,
                _ => {
                    println!(
                        "ERROR: Registration not finalized (total_epoch_weight is None or 0) for epoch {}. Cannot check assignments.",
                        current_active_epoch
                    );
                    0
                }
            };

            if total_epoch_weight > 0 {
                let epoch_phases = get_epoch_phases(&protocol_config.config, current_active_epoch);

                if slot >= epoch_phases.active.start && slot < epoch_phases.active.end {
                    if protocol_config.config.slot_length > 0 {
                        let current_light_slot_index =
                            (slot - epoch_phases.active.start) / protocol_config.config.slot_length;
                        let start_solana_slot_of_current_light_slot = epoch_phases.active.start
                            + current_light_slot_index * protocol_config.config.slot_length;
                        let end_solana_slot_of_current_light_slot =
                            start_solana_slot_of_current_light_slot
                                + protocol_config.config.slot_length;

                        let slots_remaining_in_light_slot =
                            end_solana_slot_of_current_light_slot.saturating_sub(slot);
                        let time_remaining_secs = slots_remaining_in_light_slot as f64 * 0.460;

                        println!(
                            "Current Light Slot Index: {} (Approx. {:.2}s remaining)",
                            current_light_slot_index, time_remaining_secs
                        );
                    } else {
                        println!("WARN: Cannot calculate light slot info because ProtocolConfig slot_length is zero.");
                    }
                } else {
                    println!(
                        "Info: Not currently within the active phase of epoch {}.",
                        current_active_epoch
                    );
                }

                let num_light_slots = if protocol_config.config.slot_length > 0 {
                    epoch_phases.active.length() / protocol_config.config.slot_length
                } else {
                    println!(
                        "ERROR: ProtocolConfig slot_length is zero. Cannot calculate light slots."
                    );
                    0
                };

                let mut all_slots_checked = true;
                let mut first_missing_slot = -1i64;

                println!(
                    "Checking assignment for {} light slots (Epoch {}, Tree {}, Queue {})...",
                    num_light_slots, current_active_epoch, tree, queue
                );

                for i in 0..num_light_slots {
                    let current_light_slot = i;

                    let eligible_forester_slot_index =
                        match ForesterEpochPda::get_eligible_forester_index(
                            current_light_slot,
                            &queue,
                            total_epoch_weight,
                            current_active_epoch,
                        ) {
                            Ok(idx) => idx,
                            Err(e) => {
                                println!(
                                    "ERROR calculating eligible index for light slot {}: {:?}",
                                    i, e
                                );
                                all_slots_checked = false;
                                if first_missing_slot == -1 {
                                    first_missing_slot = i as i64;
                                }
                                continue;
                            }
                        };

                    let is_any_forester_eligible = active_epoch_foresters
                        .iter()
                        .any(|pda| pda.is_eligible(eligible_forester_slot_index));

                    if !is_any_forester_eligible {
                        all_slots_checked = false;
                        if first_missing_slot == -1 {
                            first_missing_slot = i as i64;
                        }
                        warn!(
                             "Check WARNING: Tree {} is missing forester assignment for light slot index {} (eligible index: {}) in epoch {}.",
                             tree, i, eligible_forester_slot_index, current_active_epoch
                         );
                    }
                }

                if all_slots_checked {
                    println!(
                        "Check PASSED: Tree {} has a forester assigned for all {} light slots in epoch {}.",
                        tree, num_light_slots, current_active_epoch
                    );

                    let current_light_slot_index = if slot >= epoch_phases.active.start
                        && slot < epoch_phases.active.end
                    {
                        match active_epoch_foresters[0].get_current_light_slot(slot) {
                            Ok(ls) => ls,
                            Err(e) => {
                                println!("WARN: Could not calculate current light slot from PDA (using approximation): {:?}", e);
                                (slot - epoch_phases.active.start)
                                    / protocol_config.config.slot_length // Fallback calculation
                            }
                        }
                    } else {
                        println!(
                            "WARN: Currently not in the active phase for epoch {}. Showing assignments from light slot index 0.",
                            current_active_epoch
                        );
                        0
                    };

                    println!(
                        "Forester assignments for tree {} (queue {}) starting light slot index {}:",
                        tree, queue, current_light_slot_index
                    );

                    for i in 0..=10 {
                        let light_slot_to_check = current_light_slot_index + i;
                        if light_slot_to_check < num_light_slots {
                            let eligible_forester_slot_index =
                                match ForesterEpochPda::get_eligible_forester_index(
                                    light_slot_to_check,
                                    &queue,
                                    total_epoch_weight,
                                    current_active_epoch,
                                ) {
                                    Ok(idx) => idx,
                                    Err(e) => {
                                        println!(
                                            "  Light Slot Index {}: ERROR calculating index: {:?}",
                                            light_slot_to_check, e
                                        );
                                        continue;
                                    }
                                };

                            let assigned_forester = active_epoch_foresters
                                .iter()
                                .find(|pda| pda.is_eligible(eligible_forester_slot_index));

                            if let Some(forester_pda) = assigned_forester {
                                if light_slot_to_check == current_light_slot_index {
                                    println!(
                                        "  Light Slot Index {} (CURRENT): Authority: {} (Eligible Index: {})",
                                        light_slot_to_check,
                                        forester_pda.authority,
                                        eligible_forester_slot_index
                                    );
                                } else {
                                    println!(
                                        "  Light Slot Index {}: Authority: {} (Eligible Index: {})",
                                        light_slot_to_check,
                                        forester_pda.authority,
                                        eligible_forester_slot_index
                                    );
                                }
                            } else {
                                println!(
                                    "  Light Slot Index {}: UNASSIGNED (Eligible Index: {}) - Error in logic?",
                                    light_slot_to_check, eligible_forester_slot_index
                                );
                            }
                        } else {
                            println!(
                                "  Light Slot Index {}: (Exceeds epoch length)",
                                light_slot_to_check
                            );
                            break;
                        }
                    }
                } else {
                    println!(
                        "Check FAILED: Tree {} is missing forester assignment starting at least at light slot index {} in epoch {}.",
                        tree, first_missing_slot, current_active_epoch
                    );
                }
            }
        }
    } else if current_epoch_pda_entry.is_none() {
        println!(
            "ERROR: Could not find EpochPda for active epoch {}. Cannot check forester assignments.",
            current_active_epoch
        );
    }
}
