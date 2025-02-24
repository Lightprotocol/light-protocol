use std::sync::Arc;
use std::time::Duration;
use rand::prelude::SliceRandom;
use rand::Rng;
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
use light_prover_client::gnark::helpers::{LightValidatorConfig, ProverConfig, ProverMode};
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
use forester::run_pipeline;
use forester_utils::forester_epoch::get_epoch_phases;
use light_batched_merkle_tree::queue::BatchedQueueAccount;
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_registry::protocol_config::state::ProtocolConfig;
use crate::test_utils::{forester_config, init};

mod test_utils;

const DO_TXS: bool = true;
const OUTPUT_ACCOUNT_NUM: usize = 5;
const RESTART_VALIDATOR: bool = false;

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[serial]
async fn test_state_indexer_fetch_root() {
    let env = EnvAccounts::get_local_test_validator_accounts();
    let batched_state_merkle_tree = env.batched_state_merkle_tree;

    let mut account = {
        let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
        rpc.get_account(batched_state_merkle_tree).await.unwrap().unwrap()
    };

    let batched_merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
        &mut account.data,
        &batched_state_merkle_tree.into(),
    ).unwrap();

    println!("root: {:?}", batched_merkle_tree.get_root().unwrap());
    for (index, root) in batched_merkle_tree.root_history.iter().enumerate() {
        println!("root[{}]: {:?}", index, root);
    }

    println!("root sequence number: {}", batched_merkle_tree.get_metadata().sequence_number);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[serial]
async fn test_state_indexer_async_batched() {
    let tree_params = InitStateTreeAccountsInstructionData::default();

    if RESTART_VALIDATOR {
        init(Some(LightValidatorConfig {
            enable_indexer: false,
            wait_time: 1,
            prover_config: None, /*Some(ProverConfig {
            run_mode: Some(ProverMode::Forester),
            circuits: vec![],
        })*/
            sbf_programs: vec![],
        }))
            .await;
    }
    // println!("waiting for indexer to start");
    // sleep(Duration::from_secs(5)).await;

    let mut env = EnvAccounts::get_local_test_validator_accounts();
    // env.forester = forester_keypair.insecure_clone();

    let mut config = forester_config();
    config.payer_keypair =  env.forester.insecure_clone();

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
    rpc.payer =  env.forester.insecure_clone();

    if rpc.get_balance(&env.forester.pubkey()).await.unwrap() < LAMPORTS_PER_SOL {
        rpc.airdrop_lamports(&env.forester.pubkey(), LAMPORTS_PER_SOL * 100)
            .await
            .unwrap();
    }

    if rpc.get_balance(&env.governance_authority.pubkey()).await.unwrap() < LAMPORTS_PER_SOL {
        rpc.airdrop_lamports(&env.governance_authority.pubkey(), LAMPORTS_PER_SOL * 100)
            .await
            .unwrap();
    }

    // register_test_forester(
    //     &mut rpc,
    //     &env.governance_authority,
    //     &env.forester.pubkey(),
    //     light_registry::ForesterConfig::default(),
    // )
    // .await
    // .unwrap();
    config.derivation_pubkey = env.forester.pubkey();

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

    println!(
        "Initial state:
        next_index: {}
        sequence_number: {}
        batch_size: {}",
        initial_next_index,
        initial_sequence_number,
        merkle_tree.get_metadata().queue_batches.zkp_batch_size
    );

    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let (work_report_sender, mut work_report_receiver) = mpsc::channel(100);

    let forester_photon_indexer = {
        let rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
        PhotonIndexer::new("http://127.0.0.1:8784".to_string(), None, rpc)
    };

    let service_handle = tokio::spawn(run_pipeline(
        Arc::from(config.clone()),
        None,
        None,
        Arc::new(Mutex::new(forester_photon_indexer)),
        shutdown_receiver,
        work_report_sender,
    ));

    // let active_phase_slot = get_active_phase_start_slot(&mut rpc, &protocol_config).await;
    // while rpc.get_slot().await.unwrap() < active_phase_slot {
    //     println!("waiting for active phase slot: {}, current slot: {}", active_phase_slot, rpc.get_slot().await.unwrap());
    //     sleep(Duration::from_millis(400)).await;
    // }


    let batch_payer = Keypair::from_bytes(
        &[88, 117, 248, 40, 40, 5, 251, 124, 235, 221, 10, 212, 169, 203, 91, 203, 255, 67, 210, 150, 87, 182, 238, 155, 87, 24, 176, 252, 157, 119, 68, 81, 148, 156, 30, 0, 60, 63, 34, 247, 192, 120, 4, 170, 32, 149, 221, 144, 74, 244, 181, 142, 37, 197, 196, 136, 159, 196, 101, 21, 194, 56, 163, 1]
    )
        .unwrap();

    let legacy_payer = Keypair::from_bytes(
        &[58, 94, 30, 2, 133, 249, 254, 202, 188, 51, 184, 201, 173, 158, 211, 81, 202, 46, 41, 227, 38, 227, 101, 115, 246, 157, 174, 33, 64, 96, 207, 87, 161, 151, 87, 233, 147, 93, 116, 35, 227, 168, 135, 146, 45, 183, 134, 2, 97, 130, 200, 207, 211, 117, 232, 198, 233, 80, 205, 75, 41, 148, 68, 97]
    )
        .unwrap();
    if rpc.get_balance(&legacy_payer.pubkey()).await.unwrap() < LAMPORTS_PER_SOL {
        rpc.airdrop_lamports(&legacy_payer.pubkey(), LAMPORTS_PER_SOL * 100)
            .await
            .unwrap();
    }

    if rpc.get_balance(&batch_payer.pubkey()).await.unwrap() < LAMPORTS_PER_SOL {
        rpc.airdrop_lamports(&batch_payer.pubkey(), LAMPORTS_PER_SOL * 100)
            .await
            .unwrap();
    }

    if DO_TXS {
        for i in 0..merkle_tree.get_metadata().queue_batches.batch_size * 10 {
            let batch_compress_sig = compress(&mut rpc, &env.batched_output_queue, &batch_payer, if i == 0 { 1_000_000 } else { 10_000 }).await;
            println!("{} batch compress: {:?}", i, batch_compress_sig);

            let compress_sig = compress(&mut rpc, &env.merkle_tree_pubkey, &legacy_payer, if i == 0 { 1_000_000 } else { 10_000 }).await;
            println!("{} legacy compress: {:?}", i, compress_sig);

            {
                let mut output_queue_account = rpc
                    .get_account(env.batched_output_queue)
                    .await
                    .unwrap()
                    .unwrap();

                let output_queue = BatchedQueueAccount::output_from_bytes(
                    output_queue_account.data.as_mut_slice(),
                )
                    .unwrap();

                println!("output queue metadata: {:?}", output_queue.get_metadata());
            }
            let batch_transfer_sig = transfer(&mut rpc, &photon_indexer, &env.batched_output_queue, &batch_payer).await;
            println!("{} batch transfer: {:?}", i, batch_transfer_sig);

            // let legacy_transfer_sig = transfer(&mut rpc, &photon_indexer, &env.merkle_tree_pubkey, &legacy_payer).await;
            // println!("{} legacy transfer: {:?}", i, legacy_transfer_sig);
        }
    }

    let num_output_zkp_batches =
        tree_params.input_queue_batch_size / tree_params.output_queue_zkp_batch_size;
    println!("num_output_zkp_batches: {}", num_output_zkp_batches);


    let timeout_duration = Duration::from_secs(60 * 10);
    match timeout(timeout_duration, work_report_receiver.recv()).await {
        Ok(Some(report)) => {
            println!("Received work report: {:?}", report);
            println!(
                "Work report debug:
                reported_items: {}
                batch_size: {}
                complete_batches: {}",
                report.processed_items,
                tree_params.input_queue_zkp_batch_size,
                report.processed_items / tree_params.input_queue_zkp_batch_size as usize,
            );
            assert!(report.processed_items > 0, "No items were processed");

            let batch_size = tree_params.input_queue_zkp_batch_size;
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

        println!("merkle tree metadata: {:?}", merkle_tree.get_metadata());

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

    let rng = &mut rand::thread_rng();
    let num_inputs = rng.gen_range(1..4);
    input_compressed_accounts.shuffle(rng);
    input_compressed_accounts.truncate(num_inputs);

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
        .await;

    if proof_for_compressed_accounts.is_err() {
        println!("proof_for_compressed_accounts error: {:?}", proof_for_compressed_accounts);
        return Signature::default();
    }

    let proof_for_compressed_accounts = proof_for_compressed_accounts.unwrap();

    let root_indices = proof_for_compressed_accounts
        .root_indices
        .iter()
        .zip(input_compressed_accounts.iter_mut())
        .map(|(root_index, acc)|
            match root_index.in_tree {
                true => {
                    acc.merkle_context.prove_by_index = false;
                    Some(root_index.root_index)
                }
                false => {
                    acc.merkle_context.prove_by_index = true;
                    None
                },
            }
        )
        .collect::<Vec<Option<u16>>>();

    let merkle_contexts = input_compressed_accounts
        .iter()
        .map(|x| x.merkle_context)
        .collect::<Vec<MerkleContext>>();

    let lamp = lamports / OUTPUT_ACCOUNT_NUM as u64;
    let lamport_remained = lamports % OUTPUT_ACCOUNT_NUM as u64;

    let mut compressed_accounts = vec![CompressedAccount {
        lamports: lamp,
        owner: forester_keypair.pubkey(),
        address: None,
        data: None,
    }; OUTPUT_ACCOUNT_NUM];

    compressed_accounts[0].lamports += lamport_remained;

    println!("transfer input_compressed_accounts: {:?}", input_compressed_accounts);
    println!("transfer compressed_accounts: {:?}", compressed_accounts);

    let proof = if root_indices.iter().all(|x| x.is_none()) {
        None
    } else {
        Some(CompressedProof {
            a: proof_for_compressed_accounts.compressed_proof.a.try_into().unwrap(),
            b: proof_for_compressed_accounts.compressed_proof.b.try_into().unwrap(),
            c: proof_for_compressed_accounts.compressed_proof.c.try_into().unwrap(),
        })
    };

    let input_compressed_accounts = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.clone())
        .collect::<Vec<CompressedAccount>>();

    let instruction = create_invoke_instruction(
        &forester_keypair.pubkey(),
        &forester_keypair.pubkey(),
        &input_compressed_accounts,
        compressed_accounts.as_slice(),
        &merkle_contexts,
        &[*merkle_tree_pubkey; OUTPUT_ACCOUNT_NUM],
        &root_indices,
        &[],
        proof,
        None,
        false,
        None,
        true,
    );

    println!("transfer compressed_accounts: {:?}", input_compressed_accounts);
    println!("transfer root_indices: {:?}", root_indices);

    let (_, sig, _) = rpc
        .create_and_send_transaction_with_public_event(
            &[instruction],
            &forester_keypair.pubkey(),
            &[forester_keypair],
            None,
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

    println!("compress instruction: {:?}", instruction);

    let (_, sig, _) = rpc
        .create_and_send_transaction_with_public_event(
            &[instruction],
            &payer.pubkey(),
            &[payer],
            None,
        )
        .await
        .unwrap()
        .unwrap();

    sig
}


pub async fn get_active_phase_start_slot<R: RpcConnection>(
    rpc: &mut R,
    protocol_config: &ProtocolConfig,
) -> u64 {
    let current_slot = rpc.get_slot().await.unwrap();
    let current_epoch = protocol_config.get_current_epoch(current_slot);
    let phases = get_epoch_phases(protocol_config, current_epoch);
    phases.active.start
}