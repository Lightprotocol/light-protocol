use log::info;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Signer;

use forester::indexer::PhotonIndexer;
use forester::utils::LightValidatorConfig;
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::indexer::TestIndexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::get_test_env_accounts;
mod test_utils;
use test_utils::*;

#[ignore = "TokenData breaking changes break photon 0.26.0"]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_photon_interop_nullify_account() {
    let validator_config = LightValidatorConfig {
        enable_indexer: true,
        enable_prover: true,
        enable_forester: true,
        wait_time: 25,
        ..LightValidatorConfig::default()
    };
    init(Some(validator_config)).await;

    let env_accounts = get_test_env_accounts();

    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);

    // Airdrop because currently TestEnv.new() transfers funds from get_payer.
    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), LAMPORTS_PER_SOL * 1000)
        .await
        .unwrap();

    rpc.airdrop_lamports(&env_accounts.forester.pubkey(), LAMPORTS_PER_SOL * 1000)
        .await
        .unwrap();

    let forester_config = forester_config();

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

    let rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    let photon_indexer = PhotonIndexer::new(
        forester_config.external_services.indexer_url,
        forester_config.external_services.photon_api_key,
        rpc,
    );
    let user_index = 0;
    let balance = env
        .rpc
        .get_balance(&env.users[user_index].keypair.pubkey())
        .await
        .unwrap();

    let iterations = 10;

    for i in 0..iterations {
        info!("Round {} of {}", i, iterations);

        // Create starting output account
        info!("Compressing sol");
        env.compress_sol(user_index, balance).await;

        {
            let alice = &mut env.users[0];
            assert_accounts_by_owner(&mut env.indexer, alice, &photon_indexer).await;
            assert_account_proofs_for_photon_and_test_indexer(
                &mut env.indexer,
                &alice.keypair.pubkey(),
                &photon_indexer,
            )
            .await;
        }

        // Insert output into nullifier queue
        info!("Transferring sol");
        env.transfer_sol(user_index).await;

        {
            let alice = &mut env.users[0];
            assert_account_proofs_for_photon_and_test_indexer(
                &mut env.indexer,
                &alice.keypair.pubkey(),
                &photon_indexer,
            )
            .await;
        }
    }

    // Nullifies all hashes in nullifier queue
    info!("Nullifying queue");
    env.activate_general_actions().await;

    {
        let alice = &mut env.users[0];
        assert_accounts_by_owner(&mut env.indexer, alice, &photon_indexer).await;
        // TODO(photon): Test-indexer and photon should return equivalent
        // merkle proofs for the same account.
        assert_account_proofs_for_photon_and_test_indexer(
            &mut env.indexer,
            &alice.keypair.pubkey(),
            &photon_indexer,
        )
        .await;
    }

    // Ensures test-indexer is creating valid proofs.
    info!("Transferring sol");
    env.transfer_sol(user_index).await;
    {
        let alice = &mut env.users[0];
        assert_accounts_by_owner(&mut env.indexer, alice, &photon_indexer).await;
    };
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
