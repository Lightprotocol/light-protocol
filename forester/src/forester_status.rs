use anchor_lang::{AccountDeserialize, Discriminator};
use forester_utils::forester_epoch::TreeType;
use light_client::rpc::{RpcConnection, SolanaRpcConnection};
use light_registry::{protocol_config::state::ProtocolConfigPda, EpochPda, ForesterEpochPda};
use solana_sdk::{account::ReadableAccount, commitment_config::CommitmentConfig};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::{
    cli::StatusArgs,
    metrics::{push_metrics, register_metrics},
    run_queue_info,
    tree_data_sync::fetch_trees,
    ForesterConfig,
};

pub async fn fetch_forester_status(args: &StatusArgs) {
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
    let slot = client.get_slot().expect("Failed to fetch slot.");
    let current_active_epoch = protocol_config_pdas[0]
        .config
        .get_current_active_epoch(slot)
        .unwrap();
    let current_registration_epoch = protocol_config_pdas[0]
        .config
        .get_latest_register_epoch(slot)
        .unwrap();
    println!("Current active epoch: {:?}", current_active_epoch);

    println!(
        "Current registration epoch: {:?}",
        current_registration_epoch
    );

    println!(
        "Forester registered for latest epoch: {:?}",
        forester_epoch_pdas
            .iter()
            .any(|pda| pda.epoch == current_registration_epoch)
    );
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
    let config = Arc::new(ForesterConfig::new_for_status(args).unwrap());

    if config.general_config.enable_metrics {
        register_metrics();
    }

    debug!("Fetching trees...");
    debug!("RPC URL: {}", config.external_services.rpc_url);
    let rpc = SolanaRpcConnection::new(config.external_services.rpc_url.clone(), None);
    let trees = fetch_trees(&rpc).await.unwrap();
    if trees.is_empty() {
        warn!("No trees found. Exiting.");
    }
    run_queue_info(config.clone(), trees.clone(), TreeType::State).await;
    run_queue_info(config.clone(), trees.clone(), TreeType::Address).await;

    push_metrics(&config.external_services.pushgateway_url)
        .await
        .unwrap();
}
