use forester::indexer::PhotonIndexer;
use forester::utils::LightValidatorConfig;
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::indexer::TestIndexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::get_test_env_accounts;
use log::info;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Signer;

mod test_utils;
use test_utils::*;

#[ignore = "TokenData breaking changes break photon 0.26.0 and because of leafIndex to nextIndex renaming"]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_photon_interop_address() {
    let validator_config = LightValidatorConfig {
        enable_forester: true,
        enable_prover: true,
        enable_indexer: true,
        wait_time: 25,
        ..LightValidatorConfig::default()
    };
    init(Some(validator_config)).await;
    let env_accounts = get_test_env_accounts();

    let forester_config = forester_config();
    let mut rpc = SolanaRpcConnection::new(forester_config.external_services.rpc_url.clone(), None);

    // Airdrop because currently TestEnv.new() transfers funds from get_payer.
    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), LAMPORTS_PER_SOL * 100_000)
        .await
        .unwrap();

    rpc.airdrop_lamports(&env_accounts.forester.pubkey(), LAMPORTS_PER_SOL * 1000)
        .await
        .unwrap();

    let indexer: TestIndexer<SolanaRpcConnection> = TestIndexer::init_from_env(
        &forester_config.payer_keypair,
        &env_accounts,
        keypair_action_config().inclusion(),
        keypair_action_config().non_inclusion(),
    )
    .await;

    let mut env = E2ETestEnv::<SolanaRpcConnection, TestIndexer<SolanaRpcConnection>>::new(
        rpc,
        indexer,
        &env_accounts,
        keypair_action_config(),
        general_action_config(),
        0,
        Some(1),
    )
    .await;

    let indexer_rpc = SolanaRpcConnection::new(forester_config.external_services.rpc_url, None);
    let photon_indexer = PhotonIndexer::new(
        forester_config.external_services.indexer_url.to_string(),
        forester_config.external_services.photon_api_key.clone(),
        indexer_rpc,
    );

    // Insert value into address queue
    info!("Creating address 1");
    let mut trees = env.get_address_merkle_tree_pubkeys(1).0;

    let iterations = 10;
    for i in 0..iterations {
        info!("Round {} of {}", i, iterations);
        let address_1 = generate_pubkey_254();
        {
            assert_new_address_proofs_for_photon_and_test_indexer(
                &mut env.indexer,
                &trees,
                [address_1].as_ref(),
                &photon_indexer,
            )
            .await;
        }
        let _created_addresses = env.create_address(Some(vec![address_1])).await;
        trees = env.get_address_merkle_tree_pubkeys(1).0;
    }
    // Empties address queue and updates address tree
    info!("Emptying address queue");
    env.activate_general_actions().await;

    // Creates new address with new tree root. Expects Photon to index the
    // updated address tree.
    info!("Creating address 2");
    let address_2 = generate_pubkey_254();
    // Test-indexer and photon should return equivalent
    // address-proofs for the same address.
    {
        assert_new_address_proofs_for_photon_and_test_indexer(
            &mut env.indexer,
            &trees,
            [address_2].as_ref(),
            &photon_indexer,
        )
        .await;
    }

    // Ensure test-indexer returns the correct proof.
    let _ = env.create_address(Some(vec![address_2])).await;
}

fn keypair_action_config() -> KeypairActionConfig {
    KeypairActionConfig {
        max_output_accounts: Some(1),
        ..KeypairActionConfig::all_default()
    }
}

fn general_action_config() -> GeneralActionConfig {
    GeneralActionConfig {
        nullify_compressed_accounts: Some(1.0),
        empty_address_queue: Some(1.0),
        add_keypair: None,
        create_state_mt: None,
        create_address_mt: None,
        rollover: None,
    }
}
