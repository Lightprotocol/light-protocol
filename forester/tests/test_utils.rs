use account_compression::initialize_address_merkle_tree::Pubkey;
use forester::indexer::PhotonIndexer;
use forester::utils::{spawn_validator, LightValidatorConfig};
use forester::{external_services_config::ExternalServicesConfig, ForesterConfig};
use light_test_utils::e2e_test_env::{GeneralActionConfig, KeypairActionConfig, User};
use light_test_utils::indexer::{Indexer, NewAddressProofWithContext, TestIndexer};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{get_test_env_accounts, REGISTRY_ID_TEST_KEYPAIR};
use log::{info, LevelFilter};
use solana_sdk::signature::{Keypair, Signer};

#[allow(dead_code)]
pub async fn init(config: Option<LightValidatorConfig>) {
    setup_logger();
    spawn_test_validator(config).await;
}

#[allow(dead_code)]
pub fn setup_logger() {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(LevelFilter::Info.to_string()),
    )
    .is_test(true)
    .try_init();
}

#[allow(dead_code)]
pub async fn spawn_test_validator(config: Option<LightValidatorConfig>) {
    if let Some(config) = config {
        spawn_validator(config).await;
    } else {
        let config = LightValidatorConfig {
            enable_indexer: true,
            ..LightValidatorConfig::default()
        };
        spawn_validator(config).await;
    }
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
        fee_assert: true,
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
    }
}

#[allow(dead_code)]
pub fn forester_config() -> ForesterConfig {
    let env_accounts = get_test_env_accounts();
    let registry_keypair = Keypair::from_bytes(&REGISTRY_ID_TEST_KEYPAIR).unwrap();
    ForesterConfig {
        external_services: ExternalServicesConfig {
            rpc_url: "http://localhost:8899".to_string(),
            ws_rpc_url: "ws://localhost:8900".to_string(),
            indexer_url: "http://localhost:8784".to_string(),
            prover_url: "http://localhost:3001".to_string(),
            photon_api_key: None,
            derivation: "En9a97stB3Ek2n6Ey3NJwCUJnmTzLMMEA5C69upGDuQP".to_string(),
        },
        registry_pubkey: registry_keypair.pubkey(),
        payer_keypair: env_accounts.forester.insecure_clone(),
        concurrency_limit: 1,
        batch_size: 1,
        max_retries: 5,
        cu_limit: 1_000_000,
        rpc_pool_size: 20,
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
pub async fn assert_new_address_proofs_for_photon_and_test_indexer<R: RpcConnection>(
    indexer: &mut TestIndexer<SolanaRpcConnection>,
    trees: &[Pubkey],
    addresses: &[Pubkey],
    photon_indexer: &PhotonIndexer<R>,
) {
    for (tree, address) in trees.iter().zip(addresses.iter()) {
        let address_proof_test_indexer = indexer
            .get_multiple_new_address_proofs(tree.to_bytes(), vec![address.to_bytes()])
            .await;

        let address_proof_photon = photon_indexer
            .get_multiple_new_address_proofs(tree.to_bytes(), vec![address.to_bytes()])
            .await;

        if address_proof_photon.is_err() {
            panic!("Photon error: {:?}", address_proof_photon);
        }

        if address_proof_test_indexer.is_err() {
            panic!("Test indexer error: {:?}", address_proof_test_indexer);
        }

        let photon_result: NewAddressProofWithContext =
            address_proof_photon.unwrap().first().unwrap().clone();
        let test_indexer_result: NewAddressProofWithContext =
            address_proof_test_indexer.unwrap().first().unwrap().clone();
        info!(
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
pub async fn assert_accounts_by_owner<R: RpcConnection>(
    indexer: &mut TestIndexer<R>,
    user: &User,
    photon_indexer: &PhotonIndexer<R>,
) {
    let mut photon_accs = photon_indexer
        .get_rpc_compressed_accounts_by_owner(&user.keypair.pubkey())
        .await
        .unwrap();
    photon_accs.sort();

    let mut test_accs = indexer
        .get_rpc_compressed_accounts_by_owner(&user.keypair.pubkey())
        .await
        .unwrap();
    test_accs.sort();

    info!(
        "asserting accounts for user: {} Test accs: {:?} Photon accs: {:?}",
        user.keypair.pubkey().to_string(),
        test_accs.len(),
        photon_accs.len()
    );
    assert_eq!(test_accs.len(), photon_accs.len());

    info!("test_accs: {:?}", test_accs);
    info!("photon_accs: {:?}", photon_accs);

    for (test_acc, indexer_acc) in test_accs.iter().zip(photon_accs.iter()) {
        assert_eq!(test_acc, indexer_acc);
    }
}

#[allow(dead_code)]
pub async fn assert_account_proofs_for_photon_and_test_indexer<R: RpcConnection>(
    indexer: &mut TestIndexer<R>,
    user_pubkey: &Pubkey,
    photon_indexer: &PhotonIndexer<R>,
) {
    let accs: Result<Vec<String>, light_test_utils::indexer::IndexerError> = indexer
        .get_rpc_compressed_accounts_by_owner(user_pubkey)
        .await;
    for account_hash in accs.unwrap() {
        let photon_result = photon_indexer
            .get_multiple_compressed_account_proofs(vec![account_hash.clone()])
            .await;
        let test_indexer_result = indexer
            .get_multiple_compressed_account_proofs(vec![account_hash.clone()])
            .await;

        if photon_result.is_err() {
            panic!("Photon error: {:?}", photon_result);
        }

        if test_indexer_result.is_err() {
            panic!("Test indexer error: {:?}", test_indexer_result);
        }

        let photon_result = photon_result.unwrap();
        let test_indexer_result = test_indexer_result.unwrap();
        info!(
            "assert proofs for account: {} photon result: {:?} test indexer result: {:?}",
            account_hash, photon_result, test_indexer_result
        );

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
