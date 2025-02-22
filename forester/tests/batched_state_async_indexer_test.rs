use std::sync::Arc;
use std::time::Duration;

use forester_utils::{
    airdrop_lamports,
    registry::{register_test_forester, update_test_forester},
};
use light_batched_merkle_tree::{
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount,
};
use light_client::{
    indexer::{photon_indexer::PhotonIndexer, Indexer},
    rpc::{solana_rpc::SolanaRpcUrl, RpcConnection, SolanaRpcConnection},
    rpc_pool::SolanaRpcPool,
    transaction_params::{FeeConfig, TransactionParams},
};
use light_compressed_account::compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext, MerkleContext};
use light_hasher::Poseidon;
use light_program_test::test_env::EnvAccounts;
use light_prover_client::gnark::helpers::LightValidatorConfig;
use light_registry::{
    protocol_config::state::ProtocolConfigPda, utils::get_protocol_config_pda_address,
};
use light_test_utils::system_program::create_invoke_instruction;
use serial_test::serial;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::pubkey::Pubkey;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signer::Signer};
use solana_sdk::signature::Signature;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{sleep, timeout};
use tracing::log::info;
use forester::run_pipeline;
use light_batched_merkle_tree::batch::BatchState;
use light_batched_merkle_tree::queue::BatchedQueueAccount;
use crate::test_utils::{forester_config, init};

mod test_utils;

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[serial]
async fn test_state_indexer_async_batched() {
    let tree_params = InitStateTreeAccountsInstructionData::test_default();

    init(Some(LightValidatorConfig {
        enable_indexer: true,
        wait_time: 1,
        prover_config: None,
        sbf_programs: vec![],
    }))
    .await;

    let forester_keypair = Keypair::new();
    let mut env = EnvAccounts::get_local_test_validator_accounts();
    env.forester = forester_keypair.insecure_clone();

    let mut config = forester_config();
    config.payer_keypair = forester_keypair.insecure_clone();

    let pool = SolanaRpcPool::<SolanaRpcConnection>::new(
        config.external_services.rpc_url.to_string(),
        CommitmentConfig::processed(),
        config.general_config.rpc_pool_size as u32,
        None,
        None,
    )
    .await
    .unwrap();

    let commitment_config = CommitmentConfig::confirmed();
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, Some(commitment_config));
    rpc.payer = forester_keypair.insecure_clone();

    rpc.airdrop_lamports(&forester_keypair.pubkey(), LAMPORTS_PER_SOL * 100_000)
        .await
        .unwrap();

    rpc.airdrop_lamports(
        &env.governance_authority.pubkey(),
        LAMPORTS_PER_SOL * 100_000,
    )
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    airdrop_lamports(&mut rpc, &payer.pubkey(), 1_000_000_000_000)
        .await
        .unwrap();

    register_test_forester(
        &mut rpc,
        &env.governance_authority,
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

    let forester_keypair = new_forester_keypair;

    let forester_balance = rpc.get_balance(&forester_keypair.pubkey()).await.unwrap();
    assert!(forester_balance > LAMPORTS_PER_SOL);

    let photon_indexer = {
        let rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
        PhotonIndexer::new("http://127.0.0.1:8784".to_string(), None, rpc)
    };

    let protocol_config_pda_address = get_protocol_config_pda_address().0;
    let _protocol_config = rpc
        .get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda_address)
        .await
        .unwrap()
        .unwrap()
        .config;

    let mut merkle_tree_account = rpc
        .get_account(env.batched_state_merkle_tree)
        .await
        .unwrap()
        .unwrap();
    let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
        &mut merkle_tree_account.data,
        &env.batched_state_merkle_tree.into(),
    )
    .unwrap();

    let (initial_next_index, initial_sequence_number, pre_root) = {
        let mut rpc = pool.get_connection().await.unwrap();
        let mut merkle_tree_account = rpc
            .get_account(env.batched_state_merkle_tree)
            .await
            .unwrap()
            .unwrap();

        let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &env.batched_state_merkle_tree.into(),
        )
        .unwrap();

        let initial_next_index = merkle_tree.get_metadata().next_index;
        let initial_sequence_number = merkle_tree.get_metadata().sequence_number;

        (
            initial_next_index,
            initial_sequence_number,
            merkle_tree.get_root().unwrap(),
        )
    };


    info!(
        "Initial state:
        next_index: {}
        sequence_number: {}
        batch_size: {}",
        initial_next_index,
        initial_sequence_number,
        merkle_tree.get_metadata().queue_batches.batch_size
    );

    for i in 0..merkle_tree.get_metadata().queue_batches.batch_size  {
        let compress_sig = compress(&mut rpc, &env.batched_output_queue, &forester_keypair, if i == 0 { 1_000_000 } else { 10_000 } ).await;
        let transfer_sig = transfer(&mut rpc, &photon_indexer, &env.batched_output_queue, &forester_keypair).await;

        println!("{} compress: {:?}", i, compress_sig);
        println!("{} transfer: {:?}", i, transfer_sig);
    }

    let num_output_zkp_batches =
        tree_params.input_queue_batch_size / tree_params.output_queue_zkp_batch_size;
    println!("num_output_zkp_batches: {}", num_output_zkp_batches);

    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let (work_report_sender, mut work_report_receiver) = mpsc::channel(100);

    let service_handle = tokio::spawn(run_pipeline(
        Arc::from(config.clone()),
        None,
        None,
        Arc::new(Mutex::new(photon_indexer)),
        shutdown_receiver,
        work_report_sender,
    ));

    let timeout_duration = Duration::from_secs(60 * 10);
    match timeout(timeout_duration, work_report_receiver.recv()).await {
        Ok(Some(report)) => {
            info!("Received work report: {:?}", report);
            info!(
                "Work report debug:
                reported_items: {}
                batch_size: {}
                complete_batches: {}",
                report.processed_items,
                tree_params.input_queue_batch_size,
                report.processed_items / tree_params.input_queue_batch_size as usize,
            );
            assert!(report.processed_items > 0, "No items were processed");

            let batch_size = tree_params.input_queue_batch_size;
            assert_eq!(
                report.processed_items % batch_size as usize,
                0,
                "Processed items {} should be a multiple of batch size {}",
                report.processed_items,
                batch_size
            );
        }
        Ok(None) => panic!("Work report channel closed unexpectedly"),
        Err(_) => panic!("Test timed out after {:?}", timeout_duration),
    }


    let mut rpc = pool.get_connection().await.unwrap();
    let mut merkle_tree_account = rpc
        .get_account(env.batched_state_merkle_tree)
        .await
        .unwrap()
        .unwrap();

    let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
        merkle_tree_account.data.as_mut_slice(),
        &env.batched_state_merkle_tree.into(),
    )
        .unwrap();

    assert!(
        merkle_tree.get_metadata().queue_batches.pending_batch_index > 0,
        "No batches were processed"
    );

    {
        let mut rpc = pool.get_connection().await.unwrap();

        let mut merkle_tree_account = rpc
            .get_account(env.batched_state_merkle_tree)
            .await
            .unwrap()
            .unwrap();

        let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &env.batched_state_merkle_tree.into(),
        )
            .unwrap();

        let final_metadata = merkle_tree.get_metadata();

        let mut output_queue_account = rpc
            .get_account(env.nullifier_queue_pubkey)
            .await
            .unwrap()
            .unwrap();

        let output_queue =
            BatchedQueueAccount::output_from_bytes(output_queue_account.data.as_mut_slice())
                .unwrap();

        let batch_size = merkle_tree.get_metadata().queue_batches.batch_size;
        let zkp_batch_size = merkle_tree.get_metadata().queue_batches.zkp_batch_size;
        let num_zkp_batches = batch_size / zkp_batch_size;

        let mut completed_items = 0;
        for batch_idx in 0..output_queue.batch_metadata.batches.len() {
            let batch = output_queue.batch_metadata.batches.get(batch_idx).unwrap();
            if batch.get_state() == BatchState::Inserted {
                completed_items += batch_size;
            }
        }
        info!(
            "initial_next_index: {}
            final_next_index: {}
            batch_size: {}
            zkp_batch_size: {}
            num_zkp_batches per full batch: {}
            completed_items from batch states: {}
            input_queue_metadata: {:?}
            output_queue_metadata: {:?}",
            initial_next_index,
            final_metadata.next_index,
            batch_size,
            zkp_batch_size,
            num_zkp_batches,
            completed_items,
            final_metadata.queue_batches,
            output_queue.get_metadata().batch_metadata
        );

        assert_eq!(
            final_metadata.next_index,
            initial_next_index + completed_items,
            "Merkle tree next_index did not advance by expected amount",
        );

        assert_eq!(
            merkle_tree.get_metadata().queue_batches.pending_batch_index,
            1
        );

        assert!(
            final_metadata.sequence_number > initial_sequence_number,
            "Sequence number should have increased"
        );

        // compress_sol_deterministic creates 1 output
        // transfer_sol_deterministic invalidates 1 input and creates 1 output
        // 1 + 1 + 1 = 3
        const UPDATES_PER_BATCH: u64 = 3;

        let expected_sequence_number =
            initial_sequence_number + (num_zkp_batches * UPDATES_PER_BATCH);

        assert_eq!(final_metadata.sequence_number, expected_sequence_number);

        assert_eq!(
            merkle_tree.root_history.last_index(),
            expected_sequence_number as usize
        );

        assert_ne!(
            pre_root,
            merkle_tree.get_root().unwrap(),
            "Root should have changed"
        );
    }

    shutdown_sender
        .send(())
        .expect("Failed to send shutdown signal");
    service_handle.await.unwrap().unwrap();

}

async fn transfer(
    rpc: &mut SolanaRpcConnection,
    indexer: &PhotonIndexer<SolanaRpcConnection>,
    merkle_tree_pubkey: &Pubkey,
    forester_keypair: &Keypair,
) -> Signature {
    let mut input_compressed_accounts: Vec<CompressedAccountWithMerkleContext> = vec![];

    while input_compressed_accounts.is_empty() {
        input_compressed_accounts = indexer
            .get_compressed_accounts_by_owner_v2(&forester_keypair.pubkey())
            .await
            .unwrap_or(vec![]);
        sleep(Duration::from_millis(10)).await;
    }
    let input_compressed_account_length = input_compressed_accounts.len();

    let lamports = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.lamports)
        .sum::<u64>();

    let compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| {
            x.compressed_account
                .hash::<Poseidon>(
                    &x.merkle_context.merkle_tree_pubkey,
                    &x.merkle_context.leaf_index,
                )
                .unwrap()
        })
        .collect::<Vec<[u8; 32]>>();

    let proof_for_compressed_accounts = indexer
        .get_validity_proof_v2(compressed_account_hashes, vec![])
        .await
        .unwrap();

    let root_indices = proof_for_compressed_accounts
        .root_indices
        .iter()
        .map(|x|
            match x.in_tree {
                true => Some(x.root_index),
                false => None,
            }
        )
        .collect::<Vec<Option<u16>>>();

    let merkle_contexts = input_compressed_accounts
        .iter()
        .map(|x| x.merkle_context)
        .collect::<Vec<MerkleContext>>();

    let compress_account = CompressedAccount {
        lamports,
        owner: forester_keypair.pubkey(),
        address: None,
        data: None,
    };

    let input_compressed_accounts = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.clone())
        .collect::<Vec<CompressedAccount>>();

    let instruction = create_invoke_instruction(
        &forester_keypair.pubkey(),
        &forester_keypair.pubkey(),
        &input_compressed_accounts,
        &[compress_account],
        &merkle_contexts,
        &[*merkle_tree_pubkey],
        &root_indices,
        &[],
        None,
        None,
        false,
        None,
        true,
    );

    let (_, sig, _) = rpc
        .create_and_send_transaction_with_public_event(
            &[instruction],
            &forester_keypair.pubkey(),
            &[forester_keypair],
            Some(TransactionParams {
                num_input_compressed_accounts: input_compressed_account_length as u8,
                num_output_compressed_accounts: 1,
                num_new_addresses: 0,
                compress: 0,
                fee_config: FeeConfig::test_batched(),
            }),
        )
        .await
        .unwrap()
        .unwrap();

    sig
}

async fn compress(rpc: &mut SolanaRpcConnection, merkle_tree_pubkey: &Pubkey, payer: &Keypair, lamports: u64) -> Signature {
    let compress_account = CompressedAccount {
        lamports,
        owner: payer.pubkey(),
        address: None,
        data: None,
    };

    let instruction = create_invoke_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &[],
        &[compress_account],
        &[],
        &[*merkle_tree_pubkey],
        &[],
        &[],
        None,
        Some(lamports),
        true,
        None,
        true,
    );

    let (_, sig, _) = rpc
        .create_and_send_transaction_with_public_event(
            &[instruction],
            &payer.pubkey(),
            &[payer],
            Some(TransactionParams {
                num_input_compressed_accounts: 0,
                num_output_compressed_accounts: 1,
                num_new_addresses: 0,
                compress: lamports as i64,
                fee_config: FeeConfig::test_batched(),
            }),
        )
        .await
        .unwrap()
        .unwrap();

    sig
}
