use std::{sync::Arc, time::Duration};

use bs58;
use forester::run_pipeline;
use forester_utils::{forester_epoch::get_epoch_phases, instructions::wait_for_indexer};
use light_batched_merkle_tree::{
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_client::{
    indexer::{photon_indexer::PhotonIndexer, AddressWithTree, Indexer},
    rpc::{solana_rpc::SolanaRpcUrl, RpcConnection, SolanaRpcConnection},
    rpc_pool::SolanaRpcPool,
};
use light_compressed_account::{
    address::derive_address_legacy,
    compressed_account::{CompressedAccount, MerkleContext},
    instruction_data::{compressed_proof::CompressedProof, data::NewAddressParams},
};
use light_compressed_token::process_transfer::{
    transfer_sdk::create_transfer_instruction, TokenTransferOutputData,
};
use light_hasher::Poseidon;
use light_program_test::test_env::EnvAccounts;
use light_prover_client::gnark::helpers::LightValidatorConfig;
use light_registry::{
    protocol_config::state::{ProtocolConfig, ProtocolConfigPda},
    utils::get_protocol_config_pda_address,
};
use light_test_utils::{
    conversions::sdk_to_program_token_data, spl::create_mint_helper_with_keypair,
    system_program::create_invoke_instruction,
};
use rand::{prelude::SliceRandom, rngs::StdRng, Rng, SeedableRng};
use serial_test::serial;
use solana_program::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signature},
    signer::Signer,
};
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    time::{sleep, timeout},
};

use crate::test_utils::{forester_config, init};

mod test_utils;

const DO_TXS: bool = true;
const OUTPUT_ACCOUNT_NUM: usize = 5;
const RESTART_VALIDATOR: bool = true;

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[serial]
async fn test_state_indexer_fetch_root() {
    let env = EnvAccounts::get_local_test_validator_accounts();
    let batched_state_merkle_tree = env.batched_state_merkle_tree;

    let mut account = {
        let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
        rpc.get_account(batched_state_merkle_tree)
            .await
            .unwrap()
            .unwrap()
    };

    let batched_merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
        &mut account.data,
        &batched_state_merkle_tree.into(),
    )
    .unwrap();

    println!("root: {:?}", batched_merkle_tree.get_root().unwrap());
    for (index, root) in batched_merkle_tree.root_history.iter().enumerate() {
        println!("root[{}]: {:?}", index, root);
    }

    println!(
        "root sequence number: {}",
        batched_merkle_tree.get_metadata().sequence_number
    );
}

#[test]
fn bs58_inputs_test() {
    use bs58;

    let hex_strings = vec![
        "20EE6D2049072E817EAEF3A14AC32E2904D57CB26738887E74583147F542DF98",
        "15BE50C563647D786CDDF5B69C0B109C25C5BD0B91CBBDE0AC76AD3A9F9406B5",
    ];

    for hex in hex_strings {
        let bytes = hex::decode(hex).expect("Invalid hex string");
        let base58 = bs58::encode(bytes).into_string();
        println!("Base58: {}", base58);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[serial]
async fn test_state_indexer_async_batched() {
    let tree_params = InitStateTreeAccountsInstructionData::default();

    if RESTART_VALIDATOR {
        init(Some(LightValidatorConfig {
            enable_indexer: false,
            wait_time: 1,
            prover_config: None,
            // prover_config: Some(ProverConfig {
            // run_mode: Some(ProverMode::Forester),
            // circuits: vec![],
            // }),
            sbf_programs: vec![],
            limit_ledger_size: Some(500000),
        }))
        .await;

        println!("waiting for indexer to start");
        sleep(Duration::from_secs(5)).await;
    }

    let env = EnvAccounts::get_local_test_validator_accounts();
    // env.forester = forester_keypair.insecure_clone();

    let mut config = forester_config();
    config.payer_keypair = env.forester.insecure_clone();

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
    rpc.payer = env.forester.insecure_clone();

    if rpc.get_balance(&env.forester.pubkey()).await.unwrap() < LAMPORTS_PER_SOL {
        rpc.airdrop_lamports(&env.forester.pubkey(), LAMPORTS_PER_SOL * 100)
            .await
            .unwrap();
    }

    if rpc
        .get_balance(&env.governance_authority.pubkey())
        .await
        .unwrap()
        < LAMPORTS_PER_SOL
    {
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

    let mut photon_indexer = {
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

    let batch_payer = Keypair::from_bytes(&[
        88, 117, 248, 40, 40, 5, 251, 124, 235, 221, 10, 212, 169, 203, 91, 203, 255, 67, 210, 150,
        87, 182, 238, 155, 87, 24, 176, 252, 157, 119, 68, 81, 148, 156, 30, 0, 60, 63, 34, 247,
        192, 120, 4, 170, 32, 149, 221, 144, 74, 244, 181, 142, 37, 197, 196, 136, 159, 196, 101,
        21, 194, 56, 163, 1,
    ])
    .unwrap();

    println!("batch payer pubkey: {:?}", batch_payer.pubkey());

    let legacy_payer = Keypair::from_bytes(&[
        58, 94, 30, 2, 133, 249, 254, 202, 188, 51, 184, 201, 173, 158, 211, 81, 202, 46, 41, 227,
        38, 227, 101, 115, 246, 157, 174, 33, 64, 96, 207, 87, 161, 151, 87, 233, 147, 93, 116, 35,
        227, 168, 135, 146, 45, 183, 134, 2, 97, 130, 200, 207, 211, 117, 232, 198, 233, 80, 205,
        75, 41, 148, 68, 97,
    ])
    .unwrap();

    println!("legacy payer pubkey: {:?}", legacy_payer.pubkey());

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

    let mut sender_batched_accs_counter = 0;
    let mut sender_legacy_accs_counter = 0;
    let mut sender_batched_token_counter = 0;

    let mint_keypair = Keypair::from_bytes(&[
        87, 206, 67, 171, 178, 112, 231, 204, 169, 148, 206, 45, 217, 171, 233, 199, 226, 229, 142,
        204, 52, 3, 40, 197, 103, 125, 199, 80, 17, 18, 42, 42, 72, 237, 17, 77, 168, 248, 87, 226,
        202, 233, 163, 7, 148, 155, 201, 160, 255, 17, 124, 254, 98, 74, 111, 251, 24, 230, 93,
        130, 105, 104, 119, 110,
    ])
    .unwrap();
    let mint_pubkey = create_mint_helper_with_keypair(&mut rpc, &batch_payer, &mint_keypair).await;

    let sig = mint_to(
        &mut rpc,
        &env.batched_output_queue,
        &batch_payer,
        &mint_pubkey,
    )
    .await;
    println!("mint_to: {:?}", sig);

    sender_batched_token_counter = 10;

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

        let mut output_queue_account = rpc
            .get_account(env.batched_output_queue)
            .await
            .unwrap()
            .unwrap();

        let output_queue =
            BatchedQueueAccount::output_from_bytes(output_queue_account.data.as_mut_slice())
                .unwrap();

        println!("queue metadata: {:?}", output_queue.get_metadata());
    }
    wait_for_indexer(&mut rpc, &photon_indexer).await.unwrap();

    let input_compressed_accounts = photon_indexer
        .get_compressed_token_accounts_by_owner_v2(&batch_payer.pubkey(), Some(mint_pubkey))
        .await
        .unwrap();

    let compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.hash().unwrap())
        .collect::<Vec<[u8; 32]>>();

    println!(
        "get_validity_proof_v2 for {:?}",
        compressed_account_hashes
            .iter()
            .map(|x| bs58::encode(x).into_string())
            .collect::<Vec<_>>()
    );
    let proof_for_compressed_accounts = photon_indexer
        .get_validity_proof_v2(compressed_account_hashes, vec![])
        .await
        .unwrap();
    println!(
        "proof_for_compressed_accounts: {:?}",
        proof_for_compressed_accounts
    );
    let rng = &mut rand::thread_rng();
    let seed = rng.gen::<u64>();
    // Printing seed for debugging. If the test fails we can start with the same seed to derive the same addresses.
    println!("seed {}", seed);
    let rng = &mut StdRng::seed_from_u64(seed);
    let mut address_counter = 0;

    if DO_TXS {
        for i in 0..merkle_tree.get_metadata().queue_batches.batch_size * 10 {
            let batch_compress_sig = compress(
                &mut rpc,
                &env.batched_output_queue,
                &batch_payer,
                if i == 0 { 1_000_000 } else { 10_000 },
                &mut sender_batched_accs_counter,
            )
            .await;
            println!("{} batch compress: {:?}", i, batch_compress_sig);

            let compress_sig = compress(
                &mut rpc,
                &env.merkle_tree_pubkey,
                &legacy_payer,
                if i == 0 { 1_000_000 } else { 10_000 },
                &mut sender_legacy_accs_counter,
            )
            .await;
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

                let mut input_queue_account = rpc
                    .get_account(env.batched_state_merkle_tree)
                    .await
                    .unwrap()
                    .unwrap();
                let account = BatchedMerkleTreeAccount::state_from_bytes(
                    input_queue_account.data.as_mut_slice(),
                    &env.batched_state_merkle_tree.into(),
                )
                .unwrap();

                println!("input queue next_index: {}, output queue next_index: {} sender_batched_accs_counter: {} sender_batched_token_counter: {}",
                         account.queue_batches.next_index, output_queue.batch_metadata.next_index, sender_batched_accs_counter, sender_batched_token_counter);

                assert_eq!(
                    output_queue.batch_metadata.next_index - account.queue_batches.next_index,
                    sender_batched_accs_counter + sender_batched_token_counter
                );
            }

            let batch_transfer_sig = transfer(
                &mut rpc,
                &photon_indexer,
                &env.batched_output_queue,
                &batch_payer,
                &mut sender_batched_accs_counter,
            )
            .await;
            println!("{} batch transfer: {:?}", i, batch_transfer_sig);

            let legacy_transfer_sig = transfer(
                &mut rpc,
                &photon_indexer,
                &env.merkle_tree_pubkey,
                &legacy_payer,
                &mut sender_legacy_accs_counter,
            )
            .await;
            println!("{} legacy transfer: {:?}", i, legacy_transfer_sig);

            let batch_transfer_token_sig = compressed_token_transfer(
                &mut rpc,
                &photon_indexer,
                &env.batched_output_queue,
                &batch_payer,
                &mint_pubkey,
                &mut sender_batched_token_counter,
            )
            .await;
            println!("{} batch token transfer: {:?}", i, batch_transfer_token_sig);
        }

        {
            let sig = create_v1_address(
                &mut rpc,
                &mut photon_indexer,
                rng,
                &env.address_merkle_tree_pubkey,
                &env.address_merkle_tree_queue_pubkey,
                &legacy_payer,
                &mut address_counter,
            )
            .await;
            println!(
                "total num addresses created {}, create address: {:?}",
                address_counter, sig,
            );
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

async fn mint_to(
    rpc: &mut SolanaRpcConnection,
    merkle_tree_pubkey: &Pubkey,
    payer: &Keypair,
    mint_pubkey: &Pubkey,
) -> Signature {
    let mint_to_ix = light_compressed_token::process_mint::mint_sdk::create_mint_to_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        mint_pubkey,
        merkle_tree_pubkey,
        vec![100_000; 10],
        vec![payer.pubkey(); 10],
        None,
        false,
        0,
    );

    let instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
        mint_to_ix,
    ];

    rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[&payer])
        .await
        .unwrap()
}

async fn compressed_token_transfer<R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &I,
    merkle_tree_pubkey: &Pubkey,
    payer: &Keypair,
    mint: &Pubkey,
    counter: &mut u64,
) -> Signature {
    wait_for_indexer(rpc, indexer).await.unwrap();
    let mut input_compressed_accounts = indexer
        .get_compressed_token_accounts_by_owner_v2(&payer.pubkey(), Some(*mint))
        .await
        .unwrap();

    println!(
        "get_compressed_accounts_by_owner_v2({:?}): input_compressed_accounts: {:?}",
        payer.pubkey(),
        input_compressed_accounts
    );
    assert_eq!(
        std::cmp::min(input_compressed_accounts.len(), 1000),
        std::cmp::min((*counter as usize), 1000)
    );
    let rng = &mut rand::thread_rng();
    let num_inputs = rng.gen_range(1..4);
    input_compressed_accounts.shuffle(rng);
    input_compressed_accounts.truncate(num_inputs);

    let tokens = input_compressed_accounts
        .iter()
        .map(|x| x.token_data.amount)
        .sum::<u64>();

    let compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| {
            println!("compressed_account hash: {:?}", x.compressed_account.hash());
            println!("merkle_context: {:?}", x.compressed_account.merkle_context);
            x.compressed_account.hash().unwrap()
        })
        .collect::<Vec<[u8; 32]>>();

    wait_for_indexer(rpc, indexer).await.unwrap();
    let proof_for_compressed_accounts = indexer
        .get_validity_proof_v2(compressed_account_hashes, vec![])
        .await
        .unwrap();

    let root_indices = proof_for_compressed_accounts
        .root_indices
        .iter()
        .zip(input_compressed_accounts.iter_mut())
        .map(|(root_index, acc)| match root_index.prove_by_index {
            true => None,
            false => Some(root_index.root_index),
        })
        .collect::<Vec<Option<u16>>>();

    let merkle_contexts = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.merkle_context)
        .collect::<Vec<MerkleContext>>();

    let tokens_divided = tokens / OUTPUT_ACCOUNT_NUM as u64;
    let tokens_remained = tokens % OUTPUT_ACCOUNT_NUM as u64;

    let mut compressed_accounts = vec![
        TokenTransferOutputData {
            amount: tokens_divided,
            owner: payer.pubkey(),
            lamports: None,
            merkle_tree: *merkle_tree_pubkey,
        };
        OUTPUT_ACCOUNT_NUM
    ];

    compressed_accounts[0].amount += tokens_remained;

    println!(
        "transfer input_compressed_accounts: {:?}",
        input_compressed_accounts
    );
    println!("transfer compressed_accounts: {:?}", compressed_accounts);

    let proof = if root_indices.iter().all(|x| x.is_none()) {
        None
    } else {
        proof_for_compressed_accounts
            .compressed_proof
            .map(|proof| CompressedProof {
                a: proof.a.try_into().unwrap(),
                b: proof.b.try_into().unwrap(),
                c: proof.c.try_into().unwrap(),
            })
    };

    let input_token_data = input_compressed_accounts
        .iter()
        .map(|x| sdk_to_program_token_data(x.token_data.clone()))
        .collect::<Vec<_>>();

    let input_compressed_accounts = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.compressed_account.clone())
        .collect::<Vec<_>>();

    let instruction = create_transfer_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &merkle_contexts,
        compressed_accounts.as_slice(),
        &root_indices,
        &proof,
        input_token_data.as_slice(),
        input_compressed_accounts.as_slice(),
        *mint,
        None,
        false,
        None,
        None,
        None,
        true,
        None,
        None,
        false,
        &[],
    )
    .unwrap();

    println!(
        "transfer compressed_accounts: {:?}",
        input_compressed_accounts
    );
    println!("transfer root_indices: {:?}", root_indices);

    let mut instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
    ];
    instructions.push(instruction);

    let sig = rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .unwrap();

    *counter += OUTPUT_ACCOUNT_NUM as u64;
    *counter -= input_compressed_accounts.len() as u64;

    sig
}

async fn transfer<R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &I,
    merkle_tree_pubkey: &Pubkey,
    payer: &Keypair,
    counter: &mut u64,
) -> Signature {
    wait_for_indexer(rpc, indexer).await.unwrap();
    let mut input_compressed_accounts = indexer
        .get_compressed_accounts_by_owner_v2(&payer.pubkey())
        .await
        .unwrap_or(vec![]);

    println!(
        "get_compressed_accounts_by_owner_v2({:?}): input_compressed_accounts: {:?}",
        payer.pubkey(),
        input_compressed_accounts
    );

    assert_eq!(
        std::cmp::min(input_compressed_accounts.len(), 1000),
        std::cmp::min((*counter as usize), 1000)
    );

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

    wait_for_indexer(rpc, indexer).await.unwrap();
    let proof_for_compressed_accounts = indexer
        .get_validity_proof_v2(compressed_account_hashes, vec![])
        .await
        .unwrap();

    let root_indices = proof_for_compressed_accounts
        .root_indices
        .iter()
        .zip(input_compressed_accounts.iter_mut())
        .map(|(root_index, acc)| match root_index.prove_by_index {
            true => {
                acc.merkle_context.prove_by_index = true;
                None
            }
            false => {
                acc.merkle_context.prove_by_index = false;
                Some(root_index.root_index)
            }
        })
        .collect::<Vec<Option<u16>>>();

    let merkle_contexts = input_compressed_accounts
        .iter()
        .map(|x| x.merkle_context)
        .collect::<Vec<MerkleContext>>();

    let lamp = lamports / OUTPUT_ACCOUNT_NUM as u64;
    let lamport_remained = lamports % OUTPUT_ACCOUNT_NUM as u64;

    let mut compressed_accounts = vec![
        CompressedAccount {
            lamports: lamp,
            owner: payer.pubkey(),
            address: None,
            data: None,
        };
        OUTPUT_ACCOUNT_NUM
    ];

    compressed_accounts[0].lamports += lamport_remained;

    println!(
        "transfer input_compressed_accounts: {:?}",
        input_compressed_accounts
    );
    println!("transfer compressed_accounts: {:?}", compressed_accounts);

    let proof = if root_indices.iter().all(|x| x.is_none()) {
        None
    } else {
        proof_for_compressed_accounts
            .compressed_proof
            .map(|proof| CompressedProof {
                a: proof.a.try_into().unwrap(),
                b: proof.b.try_into().unwrap(),
                c: proof.c.try_into().unwrap(),
            })
    };

    let input_compressed_accounts = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.clone())
        .collect::<Vec<CompressedAccount>>();

    let instruction = create_invoke_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
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

    println!(
        "transfer compressed_accounts: {:?}",
        input_compressed_accounts
    );
    println!("transfer root_indices: {:?}", root_indices);

    let mut instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
    ];
    instructions.push(instruction);

    let sig = rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .unwrap();

    *counter += OUTPUT_ACCOUNT_NUM as u64;
    *counter -= input_compressed_accounts.len() as u64;

    sig
}

async fn compress(
    rpc: &mut SolanaRpcConnection,
    merkle_tree_pubkey: &Pubkey,
    payer: &Keypair,
    lamports: u64,
    counter: &mut u64,
) -> Signature {
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

    let mut instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
    ];
    instructions.push(instruction);

    let sig = rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .unwrap();

    *counter += 1;

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

/// Creates an address without account
async fn create_v1_address<R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &mut I,
    rng: &mut StdRng,
    merkle_tree_pubkey: &Pubkey,
    queue: &Pubkey,
    payer: &Keypair,
    counter: &mut u64,
) -> Signature {
    let num_addresses = rng.gen_range(1..=2);
    let mut address_proof_inputs = Vec::new();
    let mut seeds = Vec::new();
    for _ in 0..num_addresses {
        let seed = rng.gen::<[u8; 32]>();
        seeds.push(seed);
        let address = derive_address_legacy(merkle_tree_pubkey, &seed).unwrap();
        address_proof_inputs.push(AddressWithTree {
            address,
            tree: *merkle_tree_pubkey,
        });
    }

    wait_for_indexer(rpc, indexer).await.unwrap();
    let proof_for_compressed_accounts = indexer
        .get_validity_proof_v2(vec![], address_proof_inputs)
        .await
        .unwrap();
    let mut new_address_params = Vec::new();
    for (seed, root_index) in seeds
        .iter()
        .zip(proof_for_compressed_accounts.root_indices.iter())
    {
        assert!(
            !root_index.prove_by_index,
            "Addresses have no proof by index."
        );
        new_address_params.push(NewAddressParams {
            seed: *seed,
            address_queue_pubkey: *queue,
            address_merkle_tree_pubkey: *merkle_tree_pubkey,
            address_merkle_tree_root_index: root_index.root_index,
        })
    }

    let proof = proof_for_compressed_accounts
        .compressed_proof
        .map(|proof| CompressedProof {
            a: proof.a.try_into().unwrap(),
            b: proof.b.try_into().unwrap(),
            c: proof.c.try_into().unwrap(),
        });

    let instruction = create_invoke_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &[],
        &[],
        &[],
        &[],
        proof_for_compressed_accounts
            .root_indices
            .iter()
            .map(|x| Some(x.root_index))
            .collect::<Vec<_>>()
            .as_slice(),
        &new_address_params,
        proof,
        None,
        false,
        None,
        false,
    );

    println!("create address instruction: {:?}", instruction);

    let mut instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
    ];
    instructions.push(instruction);

    let sig = rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .unwrap();

    *counter += 1;

    sig
}
