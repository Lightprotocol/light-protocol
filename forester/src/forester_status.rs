use std::{collections::HashMap, sync::Arc};

use account_compression::{AddressMerkleTreeAccount, QueueAccount, StateMerkleTreeAccount};
use anchor_lang::{AccountDeserialize, Discriminator};
use anyhow::Context;
use borsh::BorshDeserialize;
use forester_utils::{
    account_zero_copy::{
        parse_concurrent_merkle_tree_from_bytes, parse_hash_set_from_bytes,
        parse_indexed_merkle_tree_from_bytes,
    },
    forester_epoch::{get_epoch_phases, TreeAccounts},
};
use itertools::Itertools;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_client::rpc::{LightClient, LightClientConfig, Rpc};
use light_compressed_account::TreeType;
use light_hasher::Poseidon;
use light_registry::{protocol_config::state::ProtocolConfigPda, EpochPda, ForesterEpochPda};
use serde::{Deserialize, Serialize};
use solana_program::{clock::Slot, pubkey::Pubkey};
use solana_sdk::{
    account::{Account, ReadableAccount},
    commitment_config::CommitmentConfig,
};
use tracing::{debug, warn};

use crate::{
    cli::StatusArgs,
    metrics::{push_metrics, register_metrics, update_registered_foresters},
    queue_helpers::{parse_address_v2_queue_info, parse_state_v2_queue_info, V2QueueInfo},
    rollover::get_tree_fullness,
    run_queue_info,
    tree_data_sync::{fetch_protocol_group_authority, fetch_trees},
    ForesterConfig,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForesterInfo {
    pub authority: String,
    pub balance_sol: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForesterStatus {
    pub slot: u64,
    pub current_active_epoch: u64,
    pub current_registration_epoch: u64,
    pub active_epoch_progress: u64,
    pub active_phase_length: u64,
    pub active_epoch_progress_percentage: f64,
    pub hours_until_next_epoch: u64,
    pub slots_until_next_registration: u64,
    pub hours_until_next_registration: u64,
    pub active_epoch_foresters: Vec<ForesterInfo>,
    pub registration_epoch_foresters: Vec<ForesterInfo>,
    pub trees: Vec<TreeStatus>,
    /// Current light slot index (None if not in active phase)
    pub current_light_slot: Option<u64>,
    /// Solana slots per light slot (forester rotation interval)
    pub light_slot_length: u64,
    /// Slots remaining until next light slot (forester rotation)
    pub slots_until_next_light_slot: Option<u64>,
    /// Total number of light slots in the active phase
    pub total_light_slots: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeStatus {
    pub tree_type: String,
    pub merkle_tree: String,
    pub queue: String,
    pub fullness_percentage: f64,
    pub next_index: u64,
    pub threshold: u64,
    pub is_rolledover: bool,
    pub queue_length: Option<u64>,
    pub v2_queue_info: Option<V2QueueInfo>,
    /// Currently assigned forester for this tree (in current light slot)
    pub assigned_forester: Option<String>,
    /// Schedule: forester index (into active_epoch_foresters) for each light slot
    /// None means no forester assigned for that slot
    pub schedule: Vec<Option<usize>>,
    /// Owner (group authority) of the tree
    pub owner: String,
}

pub fn get_forester_status_blocking(rpc_url: &str) -> crate::Result<ForesterStatus> {
    tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?
        .block_on(get_forester_status_async(rpc_url))
}

async fn get_forester_status_async(rpc_url: &str) -> crate::Result<ForesterStatus> {
    let rpc = LightClient::new(LightClientConfig {
        url: rpc_url.to_string(),
        photon_url: None,
        api_key: None,
        commitment_config: None,
        fetch_active_tree: false,
    })
    .await
    .context("Failed to create LightClient")?;

    // Phase 1: Fetch registry accounts and slot in parallel
    let (registry_result, slot_result) =
        tokio::join!(fetch_registry_accounts_filtered(&rpc), rpc.get_slot(),);

    let (forester_epoch_pdas, _epoch_pdas, protocol_config_pdas) = registry_result?;
    let slot = slot_result.context("Failed to get slot")?;

    let protocol_config_pda = protocol_config_pdas
        .first()
        .cloned()
        .context("No ProtocolConfigPda found in registry program accounts")?;

    let current_active_epoch = protocol_config_pda.config.get_current_active_epoch(slot)?;
    let current_registration_epoch = protocol_config_pda.config.get_latest_register_epoch(slot)?;

    let active_epoch_progress = protocol_config_pda
        .config
        .get_current_active_epoch_progress(slot);
    let active_phase_length = protocol_config_pda.config.active_phase_length;
    let active_epoch_progress_percentage =
        active_epoch_progress as f64 / active_phase_length as f64 * 100f64;

    let hours_until_next_epoch =
        active_phase_length.saturating_sub(active_epoch_progress) * 460 / 1000 / 3600;

    let slots_until_next_registration = protocol_config_pda
        .config
        .registration_phase_length
        .saturating_sub(active_epoch_progress);
    let hours_until_next_registration = slots_until_next_registration * 460 / 1000 / 3600;

    // Collect forester authorities for both epochs
    let active_forester_authorities: Vec<Pubkey> = forester_epoch_pdas
        .iter()
        .filter(|pda| pda.epoch == current_active_epoch)
        .map(|pda| pda.authority)
        .collect();

    let registration_forester_authorities: Vec<Pubkey> = forester_epoch_pdas
        .iter()
        .filter(|pda| pda.epoch == current_registration_epoch)
        .map(|pda| pda.authority)
        .collect();

    // Fetch all forester balances in one batch call using Rpc trait
    let all_forester_pubkeys: Vec<Pubkey> = active_forester_authorities
        .iter()
        .chain(registration_forester_authorities.iter())
        .cloned()
        .collect();

    let forester_balances = fetch_forester_balances(&rpc, &all_forester_pubkeys).await;

    // Build ForesterInfo with balances
    let active_epoch_foresters: Vec<ForesterInfo> = active_forester_authorities
        .iter()
        .map(|authority| {
            let balance = forester_balances.get(authority).copied().flatten();
            ForesterInfo {
                authority: authority.to_string(),
                balance_sol: balance,
            }
        })
        .collect();

    let registration_epoch_foresters: Vec<ForesterInfo> = registration_forester_authorities
        .iter()
        .map(|authority| {
            let balance = forester_balances.get(authority).copied().flatten();
            ForesterInfo {
                authority: authority.to_string(),
                balance_sol: balance,
            }
        })
        .collect();

    // Phase 2: Fetch trees using existing optimized method
    let mut trees = match fetch_trees(&rpc).await {
        Ok(trees) => trees,
        Err(e) => {
            warn!("Failed to fetch trees: {:?}", e);
            return Ok(ForesterStatus {
                slot,
                current_active_epoch,
                current_registration_epoch,
                active_epoch_progress,
                active_phase_length,
                active_epoch_progress_percentage,
                hours_until_next_epoch,
                slots_until_next_registration,
                hours_until_next_registration,
                active_epoch_foresters,
                registration_epoch_foresters,
                trees: vec![],
                current_light_slot: None,
                light_slot_length: protocol_config_pda.config.slot_length,
                slots_until_next_light_slot: None,
                total_light_slots: 0,
            });
        }
    };

    // Filter trees by protocol group authority
    if let Ok(group_authority) = fetch_protocol_group_authority(&rpc).await {
        let before_count = trees.len();
        trees.retain(|tree| tree.owner == group_authority);
        debug!(
            "Filtered trees by group authority {}: {} -> {} trees",
            group_authority,
            before_count,
            trees.len()
        );
    } else {
        warn!("Failed to fetch protocol group authority, showing all trees");
    }

    // Phase 3: Batch fetch all tree and queue accounts using Rpc trait
    let mut tree_statuses = fetch_tree_statuses_batched(&rpc, &trees).await;

    // Phase 4: Compute light slot info and forester assignments
    let light_slot_length = protocol_config_pda.config.slot_length;
    let mut current_light_slot: Option<u64> = None;
    let mut slots_until_next_light_slot: Option<u64> = None;
    let mut total_light_slots: u64 = 0;

    let active_epoch_forester_pdas: Vec<&ForesterEpochPda> = forester_epoch_pdas
        .iter()
        .filter(|pda| pda.epoch == current_active_epoch)
        .collect();

    // Build authority -> index map for schedule
    let authority_to_index: HashMap<String, usize> = active_epoch_foresters
        .iter()
        .enumerate()
        .map(|(i, f)| (f.authority.clone(), i))
        .collect();

    if !active_epoch_forester_pdas.is_empty() {
        if let Some(total_epoch_weight) = active_epoch_forester_pdas
            .first()
            .and_then(|pda| pda.total_epoch_weight)
            .filter(|&w| w > 0)
        {
            let epoch_phases = get_epoch_phases(&protocol_config_pda.config, current_active_epoch);

            if light_slot_length > 0 {
                total_light_slots = epoch_phases.active.length() / light_slot_length;

                // Compute current light slot if in active phase
                if slot >= epoch_phases.active.start && slot < epoch_phases.active.end {
                    let current_light_slot_index =
                        (slot - epoch_phases.active.start) / light_slot_length;
                    current_light_slot = Some(current_light_slot_index);

                    // Calculate slots until next light slot
                    let next_light_slot_start = epoch_phases.active.start
                        + (current_light_slot_index + 1) * light_slot_length;
                    slots_until_next_light_slot = Some(next_light_slot_start.saturating_sub(slot));
                }

                // Build full schedule for each tree
                for status in &mut tree_statuses {
                    let queue_pubkey: Pubkey = status.queue.parse().unwrap_or_default();
                    let mut schedule: Vec<Option<usize>> =
                        Vec::with_capacity(total_light_slots as usize);

                    for light_slot_idx in 0..total_light_slots {
                        let forester_idx = ForesterEpochPda::get_eligible_forester_index(
                            light_slot_idx,
                            &queue_pubkey,
                            total_epoch_weight,
                            current_active_epoch,
                        )
                        .ok()
                        .and_then(|eligible_idx| {
                            active_epoch_forester_pdas
                                .iter()
                                .find(|pda| pda.is_eligible(eligible_idx))
                                .and_then(|pda| authority_to_index.get(&pda.authority.to_string()))
                                .copied()
                        });
                        schedule.push(forester_idx);
                    }

                    // Set current assigned forester
                    if let Some(current_idx) = current_light_slot {
                        if let Some(Some(forester_idx)) = schedule.get(current_idx as usize) {
                            status.assigned_forester =
                                Some(active_epoch_foresters[*forester_idx].authority.clone());
                        }
                    }

                    status.schedule = schedule;
                }
            }
        }
    }

    Ok(ForesterStatus {
        slot,
        current_active_epoch,
        current_registration_epoch,
        active_epoch_progress,
        active_phase_length,
        active_epoch_progress_percentage,
        hours_until_next_epoch,
        slots_until_next_registration,
        hours_until_next_registration,
        active_epoch_foresters,
        registration_epoch_foresters,
        trees: tree_statuses,
        current_light_slot,
        light_slot_length,
        slots_until_next_light_slot,
        total_light_slots,
    })
}

async fn fetch_registry_accounts_filtered<R: Rpc>(
    rpc: &R,
) -> crate::Result<(Vec<ForesterEpochPda>, Vec<EpochPda>, Vec<ProtocolConfigPda>)> {
    let program_id = light_registry::ID;

    let (forester_result, epoch_result, config_result) = tokio::join!(
        rpc.get_program_accounts_with_discriminator(&program_id, ForesterEpochPda::DISCRIMINATOR),
        rpc.get_program_accounts_with_discriminator(&program_id, EpochPda::DISCRIMINATOR),
        rpc.get_program_accounts_with_discriminator(&program_id, ProtocolConfigPda::DISCRIMINATOR),
    );

    let mut forester_epoch_pdas = Vec::new();
    let mut epoch_pdas = Vec::new();
    let mut protocol_config_pdas = Vec::new();

    if let Ok(accounts) = forester_result {
        for (_, account) in accounts {
            let mut data: &[u8] = &account.data;
            if let Ok(pda) = ForesterEpochPda::try_deserialize_unchecked(&mut data) {
                forester_epoch_pdas.push(pda);
            }
        }
    }

    if let Ok(accounts) = epoch_result {
        for (_, account) in accounts {
            let mut data: &[u8] = &account.data;
            if let Ok(pda) = EpochPda::try_deserialize_unchecked(&mut data) {
                epoch_pdas.push(pda);
            }
        }
    }

    if let Ok(accounts) = config_result {
        for (_, account) in accounts {
            let mut data: &[u8] = &account.data;
            if let Ok(pda) = ProtocolConfigPda::try_deserialize_unchecked(&mut data) {
                protocol_config_pdas.push(pda);
            }
        }
    }

    forester_epoch_pdas.sort_by(|a, b| a.epoch.cmp(&b.epoch));
    epoch_pdas.sort_by(|a, b| a.epoch.cmp(&b.epoch));

    Ok((forester_epoch_pdas, epoch_pdas, protocol_config_pdas))
}

async fn fetch_tree_statuses_batched<R: Rpc>(rpc: &R, trees: &[TreeAccounts]) -> Vec<TreeStatus> {
    if trees.is_empty() {
        return vec![];
    }

    let mut pubkeys: Vec<Pubkey> = Vec::with_capacity(trees.len() * 2);
    let mut pubkey_map: Vec<(usize, &str)> = Vec::with_capacity(trees.len() * 2);

    for (i, tree) in trees.iter().enumerate() {
        pubkeys.push(tree.merkle_tree);
        pubkey_map.push((i, "merkle_tree"));

        if tree.tree_type != TreeType::AddressV2 {
            pubkeys.push(tree.queue);
            pubkey_map.push((i, "queue"));
        }
    }

    let accounts = match rpc.get_multiple_accounts(&pubkeys).await {
        Ok(accounts) => accounts,
        Err(e) => {
            tracing::warn!("Failed to batch fetch accounts: {:?}", e);
            return vec![];
        }
    };

    let mut tree_accounts: Vec<(Option<Account>, Option<Account>)> =
        vec![(None, None); trees.len()];

    for (idx, (tree_idx, account_type)) in pubkey_map.iter().enumerate() {
        if let Some(Some(account)) = accounts.get(idx) {
            match *account_type {
                "merkle_tree" => tree_accounts[*tree_idx].0 = Some(account.clone()),
                "queue" => tree_accounts[*tree_idx].1 = Some(account.clone()),
                _ => {}
            }
        }
    }

    let mut tree_statuses = Vec::with_capacity(trees.len());

    for (i, tree) in trees.iter().enumerate() {
        let (merkle_account, queue_account) = &tree_accounts[i];

        match parse_tree_status(tree, merkle_account.clone(), queue_account.clone()) {
            Ok(status) => tree_statuses.push(status),
            Err(e) => {
                tracing::warn!(
                    "Failed to parse tree status for {}: {:?}",
                    tree.merkle_tree,
                    e
                );
            }
        }
    }

    tree_statuses
}

async fn fetch_forester_balances<R: Rpc>(
    rpc: &R,
    pubkeys: &[Pubkey],
) -> HashMap<Pubkey, Option<f64>> {
    let mut balances = HashMap::new();

    if pubkeys.is_empty() {
        return balances;
    }

    match rpc.get_multiple_accounts(pubkeys).await {
        Ok(accounts) => {
            for (i, account_opt) in accounts.iter().enumerate() {
                if let Some(pubkey) = pubkeys.get(i) {
                    let balance = account_opt
                        .as_ref()
                        .map(|acc| acc.lamports as f64 / 1_000_000_000.0);
                    balances.insert(*pubkey, balance);
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to fetch forester balances: {:?}", e);
            for pubkey in pubkeys {
                balances.insert(*pubkey, None);
            }
        }
    }

    balances
}

fn parse_tree_status(
    tree: &TreeAccounts,
    merkle_account: Option<Account>,
    queue_account: Option<Account>,
) -> crate::Result<TreeStatus> {
    let mut merkle_account =
        merkle_account.ok_or_else(|| anyhow::anyhow!("Merkle tree account not found"))?;

    let (fullness_percentage, next_index, threshold, queue_length, v2_queue_info) = match tree
        .tree_type
    {
        TreeType::StateV1 => {
            let tree_account = StateMerkleTreeAccount::deserialize(&mut &merkle_account.data[8..])
                .map_err(|e| anyhow::anyhow!("Failed to deserialize StateV1 metadata: {}", e))?;

            let height = 26u64;
            let capacity = 1u64 << height;
            let threshold_val = capacity
                .saturating_mul(tree_account.metadata.rollover_metadata.rollover_threshold)
                / 100;

            let merkle_tree =
                parse_concurrent_merkle_tree_from_bytes::<StateMerkleTreeAccount, Poseidon, 26>(
                    &merkle_account.data,
                )
                .map_err(|e| anyhow::anyhow!("Failed to parse StateV1 tree: {:?}", e))?;

            let next_index = merkle_tree.next_index() as u64;
            let fullness = next_index as f64 / capacity as f64 * 100.0;

            let queue_len = queue_account.and_then(|acc| {
                unsafe { parse_hash_set_from_bytes::<QueueAccount>(&acc.data) }
                    .ok()
                    .map(|hs| {
                        hs.iter()
                            .filter(|(_, cell)| cell.sequence_number.is_none())
                            .count() as u64
                    })
            });

            (fullness, next_index, threshold_val, queue_len, None)
        }
        TreeType::AddressV1 => {
            let height = 26u64;
            let capacity = 1u64 << height;

            let threshold_val = queue_account
                .as_ref()
                .and_then(|acc| QueueAccount::deserialize(&mut &acc.data[8..]).ok())
                .map(|q| {
                    capacity.saturating_mul(q.metadata.rollover_metadata.rollover_threshold) / 100
                })
                .unwrap_or(0);

            let merkle_tree = parse_indexed_merkle_tree_from_bytes::<
                AddressMerkleTreeAccount,
                Poseidon,
                usize,
                26,
                16,
            >(&merkle_account.data)
            .map_err(|e| anyhow::anyhow!("Failed to parse AddressV1 tree: {:?}", e))?;

            let next_index = merkle_tree.next_index().saturating_sub(3) as u64;
            let fullness = next_index as f64 / capacity as f64 * 100.0;

            let queue_len = queue_account.and_then(|acc| {
                unsafe { parse_hash_set_from_bytes::<QueueAccount>(&acc.data) }
                    .ok()
                    .map(|hs| {
                        hs.iter()
                            .filter(|(_, cell)| cell.sequence_number.is_none())
                            .count() as u64
                    })
            });

            (fullness, next_index, threshold_val, queue_len, None)
        }
        TreeType::StateV2 => {
            let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
                &mut merkle_account.data,
                &tree.merkle_tree.into(),
            )
            .map_err(|e| anyhow::anyhow!("Failed to parse StateV2 tree: {:?}", e))?;

            let height = merkle_tree.height as u64;
            let capacity = 1u64 << height;
            let threshold_val =
                (1u64 << height) * merkle_tree.metadata.rollover_metadata.rollover_threshold / 100;
            let next_index = merkle_tree.next_index;
            let fullness = next_index as f64 / capacity as f64 * 100.0;

            let v2_info = queue_account
                .and_then(|mut acc| parse_state_v2_queue_info(&merkle_tree, &mut acc.data).ok());
            let queue_len = v2_info
                .as_ref()
                .map(|i| (i.input_pending_batches + i.output_pending_batches) * i.zkp_batch_size);

            (fullness, next_index, threshold_val, queue_len, v2_info)
        }
        TreeType::AddressV2 => {
            let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
                &mut merkle_account.data,
                &tree.merkle_tree.into(),
            )
            .map_err(|e| anyhow::anyhow!("Failed to parse AddressV2 tree: {:?}", e))?;

            let height = merkle_tree.height as u64;
            let capacity = 1u64 << height;
            let threshold_val =
                capacity * merkle_tree.metadata.rollover_metadata.rollover_threshold / 100;
            let fullness = merkle_tree.next_index as f64 / capacity as f64 * 100.0;

            let v2_info = parse_address_v2_queue_info(&merkle_tree);
            let queue_len = Some(v2_info.input_pending_batches * v2_info.zkp_batch_size);

            (
                fullness,
                merkle_tree.next_index,
                threshold_val,
                queue_len,
                Some(v2_info),
            )
        }
        TreeType::Unknown => (0.0, 0, 0, None, None),
    };

    Ok(TreeStatus {
        tree_type: tree.tree_type.to_string(),
        merkle_tree: tree.merkle_tree.to_string(),
        queue: tree.queue.to_string(),
        fullness_percentage,
        next_index,
        threshold,
        is_rolledover: tree.is_rolledover,
        queue_length,
        v2_queue_info,
        assigned_forester: None,
        schedule: Vec::new(),
        owner: tree.owner.to_string(),
    })
}

pub async fn fetch_forester_status(args: &StatusArgs) -> crate::Result<()> {
    let commitment_config = CommitmentConfig::confirmed();

    let client = solana_client::rpc_client::RpcClient::new_with_commitment(
        args.rpc_url.clone(),
        commitment_config,
    );
    let registry_accounts = client
        .get_program_accounts(&light_registry::ID)
        .context("Failed to fetch accounts for registry program")?;

    let mut forester_epoch_pdas = vec![];
    let mut epoch_pdas = vec![];
    let mut protocol_config_pdas = vec![];
    for (_, account) in registry_accounts {
        let discriminator_bytes = match account.data().get(0..8) {
            Some(d) => d,
            None => continue,
        };

        if discriminator_bytes == ForesterEpochPda::DISCRIMINATOR {
            let mut data: &[u8] = account.data();
            let forester_epoch_pda = ForesterEpochPda::try_deserialize_unchecked(&mut data)
                .context("Failed to deserialize ForesterEpochPda")?;
            forester_epoch_pdas.push(forester_epoch_pda);
        } else if discriminator_bytes == EpochPda::DISCRIMINATOR {
            let mut data: &[u8] = account.data();
            let epoch_pda = EpochPda::try_deserialize_unchecked(&mut data)
                .context("Failed to deserialize EpochPda")?;
            epoch_pdas.push(epoch_pda);
        } else if discriminator_bytes == ProtocolConfigPda::DISCRIMINATOR {
            let mut data: &[u8] = account.data();
            let protocol_config_pda = ProtocolConfigPda::try_deserialize_unchecked(&mut data)
                .context("Failed to deserialize ProtocolConfigPda")?;
            protocol_config_pdas.push(protocol_config_pda);
        }
    }
    forester_epoch_pdas.sort_by(|a, b| a.epoch.cmp(&b.epoch));
    epoch_pdas.sort_by(|a, b| a.epoch.cmp(&b.epoch));
    let slot = client.get_slot().context("Failed to fetch slot")?;

    let protocol_config_pda = protocol_config_pdas
        .first()
        .cloned()
        .context("No ProtocolConfigPda found in registry program accounts")?;

    println!("Current Solana Slot: {}", slot);

    let current_active_epoch = protocol_config_pda.config.get_current_active_epoch(slot)?;
    let current_registration_epoch = protocol_config_pda.config.get_latest_register_epoch(slot)?;
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
        protocol_config_pda
            .config
            .get_current_active_epoch_progress(slot),
        protocol_config_pda.config.active_phase_length
    );
    println!(
        "current active epoch progress {:.2?}%",
        protocol_config_pda
            .config
            .get_current_active_epoch_progress(slot) as f64
            / protocol_config_pda.config.active_phase_length as f64
            * 100f64
    );
    println!("Hours until next epoch : {:?} hours", {
        // slotduration is 460ms and 1000ms is 1 second and 3600 seconds is 1 hour
        protocol_config_pda
            .config
            .active_phase_length
            .saturating_sub(
                protocol_config_pda
                    .config
                    .get_current_active_epoch_progress(slot),
            )
            * 460
            / 1000
            / 3600
    });
    let slots_until_next_registration = protocol_config_pda
        .config
        .registration_phase_length
        .saturating_sub(
            protocol_config_pda
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
        println!("protocol config: {:?}", protocol_config_pda);
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

            print_tree_schedule_by_forester(
                slot,
                current_active_epoch,
                active_epoch_foresters,
                tree.merkle_tree,
                tree.queue,
                current_epoch_pda_entry,
                &protocol_config_pda,
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
            &protocol_config_pda,
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
                "error: no foresters registered for active epoch {}",
                current_active_epoch
            );
            return;
        }

        let total_epoch_weight = match active_epoch_foresters
            .first()
            .and_then(|pda| pda.total_epoch_weight)
        {
            Some(w) if w > 0 => w,
            _ => {
                println!(
                    "error: registration not finalized (total_epoch_weight is none or 0) for epoch {}",
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
            println!("error: protocol config slot_length is zero; cannot calculate light slots");
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
                        "{:12}\t\t{}\terror: {:?}",
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
            "error: could not find EpochPda for active epoch {}; cannot determine forester assignments",
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
                "error: no foresters registered for tree {} in active epoch {}",
                tree, current_active_epoch
            );
        } else {
            let total_epoch_weight = match active_epoch_foresters
                .first()
                .and_then(|pda| pda.total_epoch_weight)
            {
                Some(w) if w > 0 => w,
                _ => {
                    println!(
                        "error: registration not finalized (total_epoch_weight is none or 0) for epoch {}; cannot check assignments",
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
                        "error: protocol config slot_length is zero; cannot calculate light slots"
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
                                    "error calculating eligible index for light slot {}: {:?}",
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
                        match active_epoch_foresters
                            .first()
                            .context("No foresters registered for active epoch")
                            .and_then(|pda| {
                                pda.get_current_light_slot(slot)
                                    .context("get_current_light_slot failed")
                            }) {
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
                        "check failed: tree {} is missing forester assignment starting at least at light slot index {} in epoch {}",
                        tree, first_missing_slot, current_active_epoch
                    );
                }
            }
        }
    } else if current_epoch_pda_entry.is_none() {
        println!(
            "error: could not find EpochPda for active epoch {}; cannot check forester assignments",
            current_active_epoch
        );
    }
}
