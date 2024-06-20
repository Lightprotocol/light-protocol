use env_logger::Env;
use forester::constants::{INDEXER_URL, SERVER_URL};
use forester::indexer::PhotonIndexer;
use forester::utils::{spawn_validator, LightValidatorConfig, get_state_queue_length};
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig, User};
use light_test_utils::indexer::Indexer;
use light_test_utils::indexer::TestIndexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{get_test_env_accounts, REGISTRY_ID_TEST_KEYPAIR};
use log::info;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use account_compression::StateMerkleTreeAccount;
use forester::nullifier::{Config, get_nullifier_queue, nullify};
use light_hasher::Poseidon;
use light_test_utils::get_concurrent_merkle_tree;

pub async fn assert_accounts_by_owner(
    indexer: &mut TestIndexer<500, SolanaRpcConnection>,
    user: &User,
    photon_indexer: &PhotonIndexer,
)
{
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

pub async fn assert_account_proofs_for_photon_and_test_indexer(
    indexer: &mut TestIndexer<500, SolanaRpcConnection>,
    user_pubkey: &Pubkey,
    photon_indexer: &PhotonIndexer,
)
{
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

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_photon_interop_nullify_account() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let mut validator_config = LightValidatorConfig::default();
    validator_config.enable_indexer = true;
    validator_config.enable_prover = true;
    validator_config.enable_forester = true;
    validator_config.wait_time = 25;
    spawn_validator(validator_config).await;

    let env_accounts = get_test_env_accounts();

    let mut rpc = SolanaRpcConnection::new(None);

    // Airdrop because currently TestEnv.new() transfers funds from get_payer.
    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), LAMPORTS_PER_SOL * 1000)
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
            ..GeneralActionConfig::default()
        },
        0,
        Some(1),
        "../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;

    let photon_indexer = PhotonIndexer::new(INDEXER_URL.to_string());
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
        // merkleproofs for the same account.
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