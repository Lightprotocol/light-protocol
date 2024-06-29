use log::{info, LevelFilter};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;

use forester::external_services_config::ExternalServicesConfig;
use forester::indexer::PhotonIndexer;
use forester::utils::{spawn_validator, LightValidatorConfig};
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::indexer::TestIndexer;
use light_test_utils::indexer::{Indexer, NewAddressProofWithContext};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::get_test_env_accounts;

async fn init() {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(LevelFilter::Info.to_string()),
    )
    .is_test(true)
    .try_init();

    let validator_config = LightValidatorConfig {
        enable_forester: true,
        enable_prover: true,
        enable_indexer: true,
        wait_time: 25,
        ..LightValidatorConfig::default()
    };
    spawn_validator(validator_config).await;
}

// truncate to <254 bit
pub fn generate_pubkey_254() -> Pubkey {
    let mock_address: Pubkey = Pubkey::new_unique();
    let mut mock_address_less_than_254_bit: [u8; 32] = mock_address.to_bytes();
    mock_address_less_than_254_bit[0] = 0;
    Pubkey::from(mock_address_less_than_254_bit)
}

pub async fn assert_new_address_proofs_for_photon_and_test_indexer(
    indexer: &mut TestIndexer<500, SolanaRpcConnection>,
    trees: &[Pubkey],
    addresses: &[Pubkey],
    photon_indexer: &PhotonIndexer,
) {
    for (tree, address) in trees.iter().zip(addresses.iter()) {
        let address_proof_test_indexer = indexer
            .get_multiple_new_address_proofs(tree.to_bytes(), address.to_bytes())
            .await;

        let address_proof_photon = photon_indexer
            .get_multiple_new_address_proofs(tree.to_bytes(), address.to_bytes())
            .await;

        if address_proof_photon.is_err() {
            panic!("Photon error: {:?}", address_proof_photon);
        }

        if address_proof_test_indexer.is_err() {
            panic!("Test indexer error: {:?}", address_proof_test_indexer);
        }

        let photon_result: NewAddressProofWithContext = address_proof_photon.unwrap();
        let test_indexer_result: NewAddressProofWithContext = address_proof_test_indexer.unwrap();
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

#[ignore = "TokenData breaking changes break photon 0.26.0 and because of leafIndex to nextIndex renaming"]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_photon_interop_address() {
    init().await;
    let env_accounts = get_test_env_accounts();

    let services_config = ExternalServicesConfig {
        rpc_url: "http://localhost:8899".to_string(),
        ws_rpc_url: "ws://localhost:8900".to_string(),
        indexer_url: "http://localhost:8784".to_string(),
        prover_url: "http://localhost:3001".to_string(),
        derivation: "En9a97stB3Ek2n6Ey3NJwCUJnmTzLMMEA5C69upGDuQP".to_string(),
    };
    let mut rpc = SolanaRpcConnection::new(services_config.rpc_url, None);

    // Airdrop because currently TestEnv.new() transfers funds from get_payer.
    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), LAMPORTS_PER_SOL * 100_000)
        .await
        .unwrap();

    rpc.airdrop_lamports(&env_accounts.forester.pubkey(), LAMPORTS_PER_SOL * 1000)
        .await
        .unwrap();

    let mut env = E2ETestEnv::<500, SolanaRpcConnection>::new(
        rpc,
        &env_accounts,
        KeypairActionConfig {
            max_output_accounts: Some(1),
            ..KeypairActionConfig::all_default()
        },
        GeneralActionConfig {
            nullify_compressed_accounts: Some(1.0),
            empty_address_queue: Some(1.0),
            add_keypair: None,
            create_state_mt: None,
            create_address_mt: None,
            rollover: None,
        },
        0,
        Some(1),
    )
    .await;

    let photon_indexer = PhotonIndexer::new(services_config.indexer_url.to_string());

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
