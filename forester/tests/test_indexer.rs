use account_compression::StateMerkleTreeAccount;
use env_logger::Env;
use forester::constants::{INDEXER_URL, SERVER_URL};
use forester::indexer::PhotonIndexer;
use forester::nullifier::{get_nullifier_queue, nullify, Config};
use forester::utils::{spawn_test_validator_with_indexer, spawn_test_validator};
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig, User};
use light_test_utils::indexer::{Indexer, TestIndexer};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{get_test_env_accounts, REGISTRY_ID_TEST_KEYPAIR};
use light_test_utils::AccountZeroCopy;
use log::info;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_indexer() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    spawn_test_validator().await;
    let env_accounts = get_test_env_accounts();
    let registry_keypair = Keypair::from_bytes(&REGISTRY_ID_TEST_KEYPAIR).unwrap();
    let config = Config {
        server_url: SERVER_URL.to_string(),
        nullifier_queue_pubkey: env_accounts.nullifier_queue_pubkey,
        state_merkle_tree_pubkey: env_accounts.merkle_tree_pubkey,
        address_merkle_tree_pubkey: env_accounts.address_merkle_tree_pubkey,
        address_merkle_tree_queue_pubkey: env_accounts.address_merkle_tree_queue_pubkey,
        registry_pubkey: registry_keypair.pubkey(),
        payer_keypair: env_accounts.governance_authority.insecure_clone(),
        concurrency_limit: 1,
        batch_size: 1,
        max_retries: 5,
    };

    let rpc = SolanaRpcConnection::new(None).await;
    let mut env = E2ETestEnv::<500, SolanaRpcConnection>::new(
        rpc,
        &env_accounts,
        KeypairActionConfig::test_forester_default(),
        GeneralActionConfig::test_forester_default(),
        5,
        None,
        "../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    env.execute_rounds().await;
    let mut indexer = env.indexer;

    assert_ne!(get_state_queue_length(&mut env.rpc, &config).await, 0);
    info!(
        "Nullifying queue of {} accounts...",
        get_state_queue_length(&mut env.rpc, &config).await
    );
    let _ = nullify(&mut indexer, &mut env.rpc, &config).await;
    assert_eq!(get_state_queue_length(&mut env.rpc, &config).await, 0);

    let rpc = SolanaRpcConnection::new(None).await;
    let mut env = E2ETestEnv::<500, SolanaRpcConnection>::new(
        rpc,
        &env_accounts,
        KeypairActionConfig::test_forester_default(),
        GeneralActionConfig::test_forester_default(),
        5,
        None,
        "../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    env.execute_rounds().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn photon_interop_state_nullification_test() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    spawn_test_validator().await;
    let env_accounts = get_test_env_accounts();
    let registry_keypair = Keypair::from_bytes(&REGISTRY_ID_TEST_KEYPAIR).unwrap();
    let config = Config {
        server_url: SERVER_URL.to_string(),
        nullifier_queue_pubkey: env_accounts.nullifier_queue_pubkey,
        state_merkle_tree_pubkey: env_accounts.merkle_tree_pubkey,
        address_merkle_tree_pubkey: env_accounts.address_merkle_tree_pubkey,
        address_merkle_tree_queue_pubkey: env_accounts.address_merkle_tree_queue_pubkey,
        registry_pubkey: registry_keypair.pubkey(),
        payer_keypair: env_accounts.governance_authority.insecure_clone(),
        concurrency_limit: 1,
        batch_size: 1,
        max_retries: 5,
    };
    let rpc = SolanaRpcConnection::new(None).await;
    let mut env = E2ETestEnv::<500, SolanaRpcConnection>::new(
        rpc,
        &env_accounts,
        KeypairActionConfig::test_forester_default(),
        GeneralActionConfig::test_forester_default(),
        10,
        None,
        "../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;

    let alice =
        E2ETestEnv::<500, SolanaRpcConnection>::create_user(&mut env.rng, &mut env.rpc).await;
    let bob = E2ETestEnv::<500, SolanaRpcConnection>::create_user(&mut env.rng, &mut env.rpc).await;

    tokio::time::sleep(Duration::from_secs(16)).await;

    let alice_balance = env.rpc.get_balance(&alice.keypair.pubkey()).await.unwrap();
    info!("Alice account_balance: {}", alice_balance);

    let bob_balance = env.rpc.get_balance(&alice.keypair.pubkey()).await.unwrap();
    info!("Bob account_balance: {}", bob_balance);

    env.compress_sol_deterministic(&alice.keypair, 10_000_000)
        .await;

    let photon_indexer = PhotonIndexer::new(INDEXER_URL.to_string());
    assert_accounts_by_owner(&mut env.indexer, &alice, &photon_indexer).await;
    assert_account_proofs_for_photon_and_test_indexer(
        &env,
        &alice.keypair.pubkey(),
        &photon_indexer,
    )
    .await;

    let transfer_result = env
        .transfer_sol_deterministic(&alice.keypair, &bob.keypair.pubkey())
        .await;
    assert!(transfer_result.is_ok());
    info!("Transfer sig: {:?}", transfer_result.unwrap());

    assert_accounts_by_owner(&mut env.indexer, &alice, &photon_indexer).await;
    assert_accounts_by_owner(&mut env.indexer, &bob, &photon_indexer).await;

    assert_ne!(get_state_queue_length(&mut env.rpc, &config).await, 0);
    info!(
        "Nullifying queue of {} accounts...",
        get_state_queue_length(&mut env.rpc, &config).await
    );

    let _ = nullify(&mut env.indexer, &mut env.rpc, &config).await;
    info!("Getting nullifier queue...");

    let accounts = get_nullifier_queue(&config.nullifier_queue_pubkey, &mut env.rpc)
        .await
        .unwrap();
    info!("Nullifier queue length: {}", accounts.len());

    assert_eq!(get_state_queue_length(&mut env.rpc, &config).await, 0);

    assert_account_proofs_for_photon_and_test_indexer(
        &env,
        &alice.keypair.pubkey(),
        &photon_indexer,
    )
    .await;

    // This should fail because state_tree on photon's side has not been updated after nullification:
    assert_account_proofs_for_photon_and_test_indexer(&env, &bob.keypair.pubkey(), &photon_indexer)
        .await;

    // Check that the state is the same
    let tree = &env.indexer.state_merkle_trees[0];
    let root = tree.merkle_tree.root();

    let merkle_tree_account =
        AccountZeroCopy::<StateMerkleTreeAccount>::new(&mut env.rpc, tree.accounts.merkle_tree)
            .await;
    let fetched_merkle_tree_account = merkle_tree_account.deserialized();
    let fetched_merkle_tree = fetched_merkle_tree_account.copy_merkle_tree().unwrap();

    let fetched_root = fetched_merkle_tree.root();

    info!("Root: {:?}", root);
    info!("Fetched root: {:?}", fetched_root);
    assert_eq!(root, fetched_root);

    info!(
        "Alice balance: {}",
        env.get_balance(&alice.keypair.pubkey()).await
    );
    info!(
        "Bob balance: {}",
        env.get_balance(&bob.keypair.pubkey()).await
    );

    let transfer_result = env
        .transfer_sol_deterministic(&bob.keypair, &alice.keypair.pubkey())
        .await;
    assert!(transfer_result.is_ok());
    info!("Transfer sig: {:?}", transfer_result.unwrap());

    // assert_accounts_by_owner(&mut env, &alice, &photon_indexer).await;
    // assert_accounts_by_owner(&mut env, &bob, &photon_indexer).await;

    assert_ne!(get_state_queue_length(&mut env.rpc, &config).await, 0);
    info!(
        "Nullifying queue of {} accounts...",
        get_state_queue_length(&mut env.rpc, &config).await
    );

    let _ = nullify(&mut env.indexer, &mut env.rpc, &config).await;
    // assert_account_proofs_for_photon_and_test_indexer(&env, &alice.keypair.pubkey(), &photon_indexer).await;

    info!("Getting nullifier queue...");

    let accounts = get_nullifier_queue(&config.nullifier_queue_pubkey, &mut env.rpc)
        .await
        .unwrap();
    info!("Nullifier queue length: {}", accounts.len());

    assert_eq!(get_state_queue_length(&mut env.rpc, &config).await, 0);
}
async fn assert_accounts_by_owner(
    indexer: &mut TestIndexer<500, SolanaRpcConnection>,
    user: &User,
    photon_indexer: &PhotonIndexer,
) {
    let photon_accs = photon_indexer
        .get_rpc_compressed_accounts_by_owner(&user.keypair.pubkey())
        .await
        .unwrap();
    let test_accs = indexer
        .get_rpc_compressed_accounts_by_owner(&user.keypair.pubkey())
        .await
        .unwrap();
    info!("Test accs: {:?}", test_accs);
    info!("Photon accs: {:?}", photon_accs);
    assert_eq!(test_accs.len(), photon_accs.len());
    for (test_acc, indexer_acc) in test_accs.iter().zip(photon_accs.iter()) {
        assert_eq!(test_acc, indexer_acc);
    }
}


async fn assert_account_proofs_for_photon_and_test_indexer(
    env: &E2ETestEnv<500, SolanaRpcConnection>,
    user_pubkey: &Pubkey,
    photon_indexer: &PhotonIndexer,
) {
    let accs = env.get_compressed_sol_accounts(user_pubkey);
    for acc in accs {
        let account_hash = bs58::encode(acc.hash().unwrap()).into_string();
        info!("getting proof for account: {}", account_hash);
        let photon_result = photon_indexer
            .get_multiple_compressed_account_proofs(vec![account_hash.clone()])
            .await;
        let test_indexer_result = env
            .indexer
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

async fn get_state_queue_length<R: RpcConnection>(rpc: &mut R, config: &Config) -> usize {
    let queue = get_nullifier_queue(&config.nullifier_queue_pubkey, rpc)
        .await
        .unwrap();
    queue.len()
}


#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_photon_interop() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    spawn_test_validator_with_indexer().await;
    let env_accounts = get_test_env_accounts();
    let rpc = SolanaRpcConnection::new(None).await;
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
            ..GeneralActionConfig::default()
        },
        0,
        Some(1),
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;

    let photon_indexer = PhotonIndexer::new(INDEXER_URL.to_string());
    let user_index = 0;

    // If user has no spl balance it receives an airdrop
    env.transfer_sol(user_index).await;
    // Nullifies alls tx in queue with probability 1, also empties the queue with probability 1
    env.activate_general_actions().await;
    // TODO: wait for photon to index
    env.transfer_sol(user_index).await;
    // TODO: wait for photon to index

    {
        // print all users
        for user in &env.users {
            info!("User: {}", user.keypair.pubkey().to_string());
        }
        let alice = &mut env.users[0];
        assert_accounts_by_owner(&mut env.indexer, alice, &photon_indexer).await;
    }
    // {
    //     let bob = &mut env.users[1];
    //     assert_accounts_by_owner(&mut env.indexer, bob, &photon_indexer).await;

    // }
}
