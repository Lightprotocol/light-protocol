use std::{sync::Arc, time::Duration};

use forester::{
    config::{ForesterConfig, GeneralConfig},
    run_pipeline,
};
use forester_utils::registry::update_test_forester;
use light_batched_merkle_tree::{
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount,
};
use light_client::{
    indexer::Indexer,
    local_test_validator::LightValidatorConfig,
    rpc::{client::RpcUrl, LightClient, LightClientConfig, Rpc},
};
use light_compressed_account::{
    compressed_account::{CompressedAccount, MerkleContext},
    instruction_data::compressed_proof::CompressedProof,
    TreeType,
};
use light_program_test::{accounts::test_accounts::TestAccounts, indexer::TestIndexer};
use light_test_utils::{
    e2e_test_env::{init_program_test_env, E2ETestEnv},
    register_test_forester,
    system_program::create_invoke_instruction,
};
use serial_test::serial;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::Signer,
};
use tokio::{
    sync::{mpsc, oneshot},
    time::timeout,
};
use tracing::{error, info, warn};

mod test_utils;
use test_utils::{forester_config, init};

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[serial]
async fn test_batch_sequence() {
    let tree_params = InitStateTreeAccountsInstructionData::test_default();

    init(Some(LightValidatorConfig {
        enable_indexer: false,
        enable_prover: true,
        wait_time: 10,
        sbf_programs: vec![],
        limit_ledger_size: None,
        grpc_port: None,
    }))
    .await;

    let forester_keypair = Keypair::new();
    let mut env = TestAccounts::get_local_test_validator_accounts();
    env.protocol.forester = forester_keypair.insecure_clone();

    let mut config = forester_config();
    config.payer_keypair = forester_keypair.insecure_clone();
    config.general_config = GeneralConfig::test_state_v2();

    let commitment_config = CommitmentConfig::confirmed();
    let mut rpc = LightClient::new(LightClientConfig {
        url: RpcUrl::Localnet.to_string(),
        photon_url: Some("http://localhost:8784".to_string()),
        api_key: None,
        commitment_config: Some(commitment_config),
        fetch_active_tree: false,
    })
    .await
    .unwrap();
    rpc.payer = forester_keypair.insecure_clone();

    rpc.airdrop_lamports(&forester_keypair.pubkey(), LAMPORTS_PER_SOL * 100_000)
        .await
        .unwrap();
    rpc.airdrop_lamports(
        &env.protocol.governance_authority.pubkey(),
        LAMPORTS_PER_SOL * 100_000,
    )
    .await
    .unwrap();

    register_test_forester(
        &mut rpc,
        &env.protocol.governance_authority,
        &forester_keypair.pubkey(),
        light_registry::ForesterConfig::default(),
    )
    .await
    .unwrap();

    let new_forester_keypair = Keypair::new();
    rpc.airdrop_lamports(&new_forester_keypair.pubkey(), LAMPORTS_PER_SOL * 100_000)
        .await
        .unwrap();

    update_test_forester(
        &mut rpc,
        &forester_keypair,
        &forester_keypair.pubkey(),
        Some(&new_forester_keypair),
        light_registry::ForesterConfig::default(),
    )
    .await
    .unwrap();

    config.derivation_pubkey = forester_keypair.pubkey();
    config.payer_keypair = new_forester_keypair.insecure_clone();

    let mut e2e_env: E2ETestEnv<LightClient, TestIndexer> =
        init_program_test_env(rpc, &env, tree_params.output_queue_batch_size as usize).await;

    let (_, batched_state_merkle_tree_pubkey, nullifier_queue_pubkey) = e2e_env
        .indexer
        .state_merkle_trees
        .iter()
        .find(|tree| tree.tree_type == TreeType::StateV2)
        .map(|tree| (0, tree.accounts.merkle_tree, tree.accounts.nullifier_queue))
        .unwrap();

    let test_user = Keypair::new();
    e2e_env
        .rpc
        .airdrop_lamports(&test_user.pubkey(), LAMPORTS_PER_SOL * 100)
        .await
        .unwrap();

    info!("Target tree: {}", batched_state_merkle_tree_pubkey);
    info!("Output queue: {}", nullifier_queue_pubkey);

    for _ in 0..10 {
        let lamports = 2_000_000;
        let compress_account = CompressedAccount {
            lamports,
            owner: test_user.pubkey().into(),
            address: None,
            data: None,
        };

        let instruction = create_invoke_instruction(
            &test_user.pubkey(),
            &test_user.pubkey(),
            &[],
            &[compress_account],
            &[],
            &[nullifier_queue_pubkey],
            &[],
            &[],
            None,
            Some(lamports),
            true,
            None,
            true,
        );

        e2e_env
            .rpc
            .create_and_send_transaction(
                &[
                    solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
                        1_000_000,
                    ),
                    instruction,
                ],
                &test_user.pubkey(),
                &[&test_user],
            )
            .await
            .unwrap();
    }

    for batch in 0..4 {
        for i in 0..5 {
            let indexer = e2e_env.rpc.indexer().unwrap();
            let mut accounts = indexer
                .get_compressed_accounts_by_owner(&test_user.pubkey(), None, None)
                .await
                .unwrap()
                .value
                .items;

            accounts
                .retain(|a| a.tree_info.tree == batched_state_merkle_tree_pubkey && a.lamports > 0);

            let mut seen_indices = std::collections::HashSet::new();
            accounts.retain(|a| seen_indices.insert(a.leaf_index));

            accounts.sort_by_key(|a| a.leaf_index);

            if accounts.is_empty() {
                warn!(
                    "No more accounts to nullify in batch {}, iteration {}",
                    batch + 1,
                    i
                );
                break;
            }

            let account_idx = i % accounts.len();
            let account = &accounts[account_idx];
            info!(
                " Nullifying account with {:?} hash at index {}",
                account.hash, account.leaf_index
            );

            let indexer = e2e_env.rpc.indexer().unwrap();
            let proof = indexer
                .get_validity_proof(vec![account.hash], vec![], None)
                .await
                .unwrap();

            let instruction = create_invoke_instruction(
                &test_user.pubkey(),
                &test_user.pubkey(),
                &[CompressedAccount {
                    owner: account.owner.into(),
                    lamports: account.lamports,
                    data: account.data.clone(),
                    address: account.address,
                }],
                &[CompressedAccount {
                    owner: test_user.pubkey().into(),
                    lamports: account.lamports,
                    data: None,
                    address: None,
                }],
                &[MerkleContext {
                    merkle_tree_pubkey: account.tree_info.tree.into(),
                    queue_pubkey: account.tree_info.queue.into(),
                    leaf_index: account.leaf_index,
                    prove_by_index: false,
                    tree_type: TreeType::StateV2,
                }],
                &[nullifier_queue_pubkey],
                &proof.value.get_root_indices(),
                &[],
                proof.value.proof.0.map(|p| CompressedProof {
                    a: p.a,
                    b: p.b,
                    c: p.c,
                }),
                None,
                false,
                None,
                false,
            );

            e2e_env.rpc
                .create_and_send_transaction(
                    &[
                        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
                        instruction,
                    ],
                    &test_user.pubkey(),
                    &[&test_user],
                )
                .await
                .unwrap();
        }
    }

    run_forester(&config, Duration::from_secs(240)).await;

    let (_, _, _) = get_onchain_root(&e2e_env.rpc, batched_state_merkle_tree_pubkey).await;

    for _ in 0..10 {
        let lamports = 2_000_000;
        let compress_account = CompressedAccount {
            lamports,
            owner: test_user.pubkey().into(),
            address: None,
            data: None,
        };

        let instruction = create_invoke_instruction(
            &test_user.pubkey(),
            &test_user.pubkey(),
            &[],
            &[compress_account],
            &[],
            &[nullifier_queue_pubkey],
            &[],
            &[],
            None,
            Some(lamports),
            true,
            None,
            true,
        );

        e2e_env
            .rpc
            .create_and_send_transaction(
                &[
                    solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
                        1_000_000,
                    ),
                    instruction,
                ],
                &test_user.pubkey(),
                &[&test_user],
            )
            .await
            .unwrap();
    }

    run_forester(&config, Duration::from_secs(240)).await;

    assert!(
        verify_roots(
            &e2e_env.rpc,
            e2e_env.rpc.indexer().expect("PhotonIndexer not configured"),
            batched_state_merkle_tree_pubkey,
            &test_user.pubkey(),
        )
        .await,
    );
}

async fn run_forester(config: &ForesterConfig, duration: Duration) {
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let (work_report_sender, _) = mpsc::channel(100);

    let service_handle = tokio::spawn(run_pipeline::<LightClient>(
        Arc::from(config.clone()),
        None,
        None,
        shutdown_receiver,
        work_report_sender,
    ));

    tokio::time::sleep(duration).await;

    let _ = shutdown_sender.send(());
    let _ = timeout(Duration::from_secs(5), service_handle).await;
}

async fn get_onchain_root(rpc: &LightClient, tree_pubkey: Pubkey) -> (String, u64, u64) {
    let mut account = rpc.get_account(tree_pubkey).await.unwrap().unwrap();
    let merkle_tree =
        BatchedMerkleTreeAccount::state_from_bytes(&mut account.data, &tree_pubkey.into()).unwrap();

    let root = bs58::encode(merkle_tree.get_root().unwrap()).into_string();
    let seq = merkle_tree.get_metadata().sequence_number;
    let index = merkle_tree.get_metadata().next_index;

    info!("On-chain root: {} (seq: {}, index: {})", root, seq, index);

    (root, seq, index)
}

async fn verify_roots<I: Indexer>(
    rpc: &LightClient,
    indexer: &I,
    tree_pubkey: Pubkey,
    test_user: &Pubkey,
) -> bool {
    let (onchain_root, sequence, next_index) = get_onchain_root(rpc, tree_pubkey).await;

    info!("   On-chain root: {}", onchain_root);
    info!("   Sequence: {}", sequence);
    info!("   Next index: {}", next_index);

    match indexer
        .get_compressed_accounts_by_owner(test_user, None, None)
        .await
    {
        Ok(accounts_response) if !accounts_response.value.items.is_empty() => {
            let tree_accounts: Vec<_> = accounts_response
                .value
                .items
                .iter()
                .filter(|a| a.tree_info.tree == tree_pubkey)
                .collect();

            if !tree_accounts.is_empty() {
                let account_hash = tree_accounts[0].hash;
                match indexer
                    .get_validity_proof(vec![account_hash], vec![], None)
                    .await
                {
                    Ok(proof_response) => {
                        for account in &proof_response.value.accounts {
                            if account.tree_info.tree == tree_pubkey {
                                let indexer_root = bs58::encode(&account.root).into_string();
                                info!("Indexer root:  {}", indexer_root);

                                if onchain_root != indexer_root {
                                    error!("   ROOT MISMATCH at sequence {}!", sequence);
                                    error!("   On-chain: {}", onchain_root);
                                    error!("   Indexer:  {}", indexer_root);
                                    return false;
                                }
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Could not get validity proof: {:?}", e);
                    }
                }
            } else {
                warn!("No accounts found in tree {} for verification", tree_pubkey);
            }
        }
        _ => {
            warn!("Could not get user accounts for root verification");
        }
    }

    true
}
