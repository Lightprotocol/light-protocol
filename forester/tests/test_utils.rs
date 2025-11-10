use std::time::Duration;

use forester::{
    config::{ExternalServicesConfig, GeneralConfig, RpcPoolConfig},
    metrics::register_metrics,
    telemetry::setup_telemetry,
    ForesterConfig,
};
use forester_utils::forester_epoch::get_epoch_phases;
use light_client::{
    indexer::{photon_indexer::PhotonIndexer, Indexer, NewAddressProofWithContext},
    local_test_validator::{spawn_validator, LightValidatorConfig},
    rpc::{LightClient, Rpc},
};
use light_program_test::{accounts::test_accounts::TestAccounts, indexer::TestIndexerExtensions};
use light_registry::{
    protocol_config::state::{ProtocolConfig, ProtocolConfigPda},
    utils::get_protocol_config_pda_address,
};
use light_test_utils::e2e_test_env::{GeneralActionConfig, KeypairActionConfig, User};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use tokio::time::sleep;
use tracing::debug;

#[allow(dead_code)]
pub async fn init(config: Option<LightValidatorConfig>) {
    setup_telemetry();
    register_metrics();
    spawn_test_validator(config).await;
}

#[allow(dead_code)]
pub async fn spawn_test_validator(config: Option<LightValidatorConfig>) {
    let config = config.unwrap_or_default();
    spawn_validator(config).await;
}

#[allow(dead_code)]
pub fn keypair_action_config() -> KeypairActionConfig {
    KeypairActionConfig {
        compress_sol: Some(1.0),
        decompress_sol: Some(1.0),
        transfer_sol: Some(1.0),
        create_address: Some(1.0),
        compress_spl: Some(1.0),
        decompress_spl: Some(1.0),
        mint_spl: Some(1.0),
        transfer_spl: Some(1.0),
        max_output_accounts: Some(3),
        fee_assert: false,
        approve_spl: None,
        revoke_spl: None,
        freeze_spl: None,
        thaw_spl: None,
        burn_spl: None,
    }
}

#[allow(dead_code)]
pub fn general_action_config() -> GeneralActionConfig {
    GeneralActionConfig {
        add_keypair: Some(1.0),
        create_state_mt: Some(1.0),
        create_address_mt: Some(1.0),
        nullify_compressed_accounts: Some(1.0),
        empty_address_queue: Some(1.0),
        rollover: None,
        add_forester: None,
        disable_epochs: true,
    }
}

#[allow(dead_code)]
pub fn forester_config() -> ForesterConfig {
    let mut test_accounts = TestAccounts::get_program_test_test_accounts();
    test_accounts.protocol.forester = Keypair::new();

    ForesterConfig {
        external_services: ExternalServicesConfig {
            rpc_url: "http://localhost:8899".to_string(),
            ws_rpc_url: Some("ws://localhost:8900".to_string()),
            indexer_url: Some("http://localhost:8784".to_string()),
            prover_url: Some("http://localhost:3001".to_string()),
            prover_append_url: None,
            prover_update_url: None,
            prover_address_append_url: None,
            prover_api_key: None,
            photon_api_key: None,
            photon_grpc_url: None,
            pushgateway_url: None,
            pagerduty_routing_key: None,
            rpc_rate_limit: None,
            photon_rate_limit: None,
            send_tx_rate_limit: None,
        },
        retry_config: Default::default(),
        queue_config: Default::default(),
        indexer_config: Default::default(),
        transaction_config: Default::default(),
        general_config: GeneralConfig {
            slot_update_interval_seconds: 10,
            tree_discovery_interval_seconds: 5,
            enable_metrics: false,
            skip_v1_state_trees: false,
            skip_v2_state_trees: false,
            skip_v1_address_trees: false,
            skip_v2_address_trees: false,
            tree_id: None,
            sleep_after_processing_ms: 50,
            sleep_when_idle_ms: 100,
        },
        rpc_pool_config: RpcPoolConfig {
            max_size: 50,
            connection_timeout_secs: 15,
            idle_timeout_secs: 300,
            max_retries: 10,
            initial_retry_delay_ms: 1000,
            max_retry_delay_ms: 16000,
        },
        registry_pubkey: light_registry::ID,
        payer_keypair: test_accounts.protocol.forester.insecure_clone(),
        derivation_pubkey: test_accounts.protocol.forester.pubkey(),
        address_tree_data: vec![],
        state_tree_data: vec![],
    }
}

// truncate to <254 bit
#[allow(dead_code)]
pub fn generate_pubkey_254() -> Pubkey {
    let mock_address: Pubkey = Pubkey::new_unique();
    let mut mock_address_less_than_254_bit: [u8; 32] = mock_address.to_bytes();
    mock_address_less_than_254_bit[0] = 0;
    Pubkey::from(mock_address_less_than_254_bit)
}

#[allow(dead_code)]
pub async fn assert_new_address_proofs_for_photon_and_test_indexer<
    I: Indexer + TestIndexerExtensions,
>(
    indexer: &mut I,
    trees: &[Pubkey],
    addresses: &[Pubkey],
    photon_indexer: &PhotonIndexer,
) {
    for (tree, address) in trees.iter().zip(addresses.iter()) {
        let address_proof_test_indexer = indexer
            .get_multiple_new_address_proofs(tree.to_bytes(), vec![address.to_bytes()], None)
            .await;

        let address_proof_photon = photon_indexer
            .get_multiple_new_address_proofs(tree.to_bytes(), vec![address.to_bytes()], None)
            .await;

        if address_proof_photon.is_err() {
            panic!("Photon error: {:?}", address_proof_photon);
        }

        if address_proof_test_indexer.is_err() {
            panic!("Test indexer error: {:?}", address_proof_test_indexer);
        }

        let photon_result: NewAddressProofWithContext = address_proof_photon
            .unwrap()
            .value
            .items
            .first()
            .unwrap()
            .clone();
        let test_indexer_result: NewAddressProofWithContext = address_proof_test_indexer
            .unwrap()
            .value
            .items
            .first()
            .unwrap()
            .clone();
        debug!(
            "assert proofs for address: {} photon result: {:?} test indexer result: {:?}",
            address, photon_result, test_indexer_result
        );

        assert_eq!(photon_result.merkle_tree, test_indexer_result.merkle_tree);
        assert_eq!(
            photon_result.low_address_index,
            test_indexer_result.low_address_index
        );
        assert_eq!(
            photon_result.low_address_value,
            test_indexer_result.low_address_value
        );
        assert_eq!(
            photon_result.low_address_next_index,
            test_indexer_result.low_address_next_index
        );
        assert_eq!(
            photon_result.low_address_next_value,
            test_indexer_result.low_address_next_value
        );
        assert_eq!(
            photon_result.low_address_proof.len(),
            test_indexer_result.low_address_proof.len()
        );

        assert_eq!(photon_result.root, test_indexer_result.root);
        assert_eq!(photon_result.root_seq, test_indexer_result.root_seq);

        for (photon_proof_hash, test_indexer_proof_hash) in photon_result
            .low_address_proof
            .iter()
            .zip(test_indexer_result.low_address_proof.iter())
        {
            assert_eq!(photon_proof_hash, test_indexer_proof_hash);
        }
    }
}

#[allow(dead_code)]
pub async fn assert_accounts_by_owner<I: Indexer + TestIndexerExtensions>(
    indexer: &mut I,
    user: &User,
    photon_indexer: &PhotonIndexer,
) {
    let mut photon_accs = photon_indexer
        .get_compressed_accounts_by_owner(&user.keypair.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;
    photon_accs.sort_by_key(|a| a.hash);

    let mut test_accs = indexer
        .get_compressed_accounts_by_owner(&user.keypair.pubkey(), None, None)
        .await
        .unwrap();
    test_accs.value.items.sort_by_key(|a| a.hash);

    debug!(
        "asserting accounts for user: {} Test accs: {:?} Photon accs: {:?}",
        user.keypair.pubkey().to_string(),
        test_accs.value.items.len(),
        photon_accs.len()
    );
    assert_eq!(test_accs.value.items.len(), photon_accs.len());

    debug!("test_accs: {:?}", test_accs);
    debug!("photon_accs: {:?}", photon_accs);

    for (test_acc, indexer_acc) in test_accs.value.items.iter().zip(photon_accs.iter()) {
        assert_eq!(test_acc, indexer_acc);
    }
}

#[allow(dead_code)]
pub async fn assert_account_proofs_for_photon_and_test_indexer<
    I: Indexer + TestIndexerExtensions,
>(
    indexer: &mut I,
    user_pubkey: &Pubkey,
    photon_indexer: &PhotonIndexer,
) {
    let accs = indexer
        .get_compressed_accounts_by_owner(user_pubkey, None, None)
        .await;
    for account in accs.unwrap().value.items {
        let photon_result = photon_indexer
            .get_multiple_compressed_account_proofs(vec![account.hash], None)
            .await;
        let test_indexer_result = indexer
            .get_multiple_compressed_account_proofs(vec![account.hash], None)
            .await;

        if photon_result.is_err() {
            panic!("Photon error: {:?}", photon_result);
        }

        if test_indexer_result.is_err() {
            panic!("Test indexer error: {:?}", test_indexer_result);
        }

        let photon_result = photon_result.unwrap().value.items;
        let test_indexer_result = test_indexer_result.unwrap().value.items;

        assert_eq!(photon_result.len(), test_indexer_result.len());
        for (photon_proof, test_indexer_proof) in
            photon_result.iter().zip(test_indexer_result.iter())
        {
            assert_eq!(photon_proof.hash, test_indexer_proof.hash);
            assert_eq!(photon_proof.leaf_index, test_indexer_proof.leaf_index);
            assert_eq!(photon_proof.merkle_tree, test_indexer_proof.merkle_tree);
            assert_eq!(photon_proof.root_seq, test_indexer_proof.root_seq);
            assert_eq!(photon_proof.proof.len(), test_indexer_proof.proof.len());
            for (photon_proof_hash, test_indexer_proof_hash) in photon_proof
                .proof
                .iter()
                .zip(test_indexer_proof.proof.iter())
            {
                assert_eq!(photon_proof_hash, test_indexer_proof_hash);
            }
        }
    }
}

#[allow(dead_code)]
pub async fn get_registration_phase_start_slot<R: Rpc>(
    rpc: &mut R,
    protocol_config: &ProtocolConfig,
) -> u64 {
    let current_slot = rpc.get_slot().await.unwrap();
    let current_epoch = protocol_config.get_current_epoch(current_slot);
    let phases = get_epoch_phases(protocol_config, current_epoch);
    phases.registration.start
}

#[allow(dead_code)]
pub async fn get_active_phase_start_slot<R: Rpc>(
    rpc: &mut R,
    protocol_config: &ProtocolConfig,
) -> u64 {
    let current_slot = rpc.get_slot().await.unwrap();
    let current_epoch = protocol_config.get_current_epoch(current_slot);
    let phases = get_epoch_phases(protocol_config, current_epoch);
    phases.active.start
}

#[allow(dead_code)]
pub async fn wait_for_slot(rpc: &mut LightClient, target_slot: u64) {
    while rpc.get_slot().await.unwrap() < target_slot {
        println!(
            "waiting for active phase slot: {}, current slot: {}",
            target_slot,
            rpc.get_slot().await.unwrap()
        );
        sleep(Duration::from_millis(400)).await;
    }
}

#[allow(dead_code)]
async fn get_protocol_config(rpc: &mut LightClient) -> ProtocolConfig {
    let protocol_config_pda_address = get_protocol_config_pda_address().0;
    rpc.get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda_address)
        .await
        .unwrap()
        .unwrap()
        .config
}
