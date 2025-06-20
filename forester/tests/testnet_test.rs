use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::test_utils::{forester_config, init};
use borsh::BorshSerialize;
use create_address_test_program::create_invoke_cpi_instruction;
use forester::{config::GeneralConfig, epoch_manager::WorkReport, run_pipeline, ForesterConfig};
use forester_utils::{
    instructions::state_batch_append::{get_merkle_tree_metadata, get_output_queue_metadata},
    utils::wait_for_indexer,
};
use light_batched_merkle_tree::{
    batch::BatchState, initialize_address_tree::InitAddressTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_client::{
    indexer::{
        photon_indexer::PhotonIndexer, AddressWithTree,
        GetCompressedTokenAccountsByOwnerOrDelegateOptions,
    },
    rpc::{client::RpcUrl, merkle_tree::MerkleTreeExt, LightClient, LightClientConfig, Rpc},
};
use light_compressed_account::{
    address::derive_address,
    compressed_account::{CompressedAccount, PackedCompressedAccountWithMerkleContext},
    instruction_data::{
        compressed_proof::CompressedProof,
        data::{NewAddressParams, NewAddressParamsAssigned, OutputCompressedAccountWithContext},
        with_readonly::{InAccount, InstructionDataInvokeCpiWithReadOnly},
    },
    QueueType, TreeType,
};
use light_compressed_token::process_transfer::{
    transfer_sdk::{create_transfer_instruction, to_account_metas},
    TokenTransferOutputData,
};
use light_program_test::{accounts::test_accounts::TestAccounts, Indexer};
use light_prover_client::proof::deserialize_hex_string_to_be_bytes;
use light_sdk::{
    constants::LIGHT_SYSTEM_PROGRAM_ID, instruction::MerkleContext,
    token::TokenDataWithMerkleContext,
};
use light_test_utils::{
    conversions::sdk_to_program_token_data,
    create_address_test_program_sdk::{
        create_pda_instruction, CreateCompressedPdaInstructionInputs,
    },
    pack::{pack_new_address_params_assigned, pack_output_compressed_accounts},
    spl::create_mint_helper_with_keypair,
    system_program::create_invoke_instruction,
};
use rand::{prelude::StdRng, seq::SliceRandom, Rng, SeedableRng};
use serial_test::serial;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signature},
    signer::Signer,
};
use std::env;
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::debug;

mod test_utils;

fn get_photon_indexer_url() -> String {
    let indexer_url =
        env::var("PHOTON_INDEXER_URL").unwrap_or_else(|_| "http://127.0.0.1:8784".to_string());
    println!("Photon Indexer URL: {}", indexer_url);
    indexer_url
}

fn create_photon_indexer() -> PhotonIndexer {
    PhotonIndexer::new(get_photon_indexer_url(), None)
}

fn get_testnet_governance_authority_keypair() -> [u8; 64] {
    let keypair_str = env::var("TESTNET_GOVERNANCE_AUTHORITY_KEYPAIR")
        .expect("TESTNET_GOVERNANCE_AUTHORITY_KEYPAIR env var not set");

    let keypair_vec: Vec<u8> = serde_json::from_str(&keypair_str)
        .expect("Failed to parse TESTNET_GOVERNANCE_AUTHORITY_KEYPAIR");

    keypair_vec
        .try_into()
        .expect("TESTNET_GOVERNANCE_AUTHORITY_KEYPAIR must be exactly 64 bytes")
}

fn get_testnet_forester_test_keypair() -> [u8; 64] {
    let keypair_str = env::var("TESTNET_FORESTER_TEST_KEYPAIR")
        .expect("TESTNET_FORESTER_TEST_KEYPAIR env var not set");

    let keypair_vec: Vec<u8> =
        serde_json::from_str(&keypair_str).expect("Failed to parse TESTNET_FORESTER_TEST_KEYPAIR");

    keypair_vec
        .try_into()
        .expect("TESTNET_FORESTER_TEST_KEYPAIR must be exactly 64 bytes")
}

const DEFAULT_TIMEOUT_SECONDS: u64 = 120;
const COMPUTE_BUDGET_LIMIT: u32 = 1_000_000;
const OUTPUT_ACCOUNT_NUM: usize = 2;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[serial]
async fn test_testnet() {
    let program_id = LIGHT_SYSTEM_PROGRAM_ID;
    let pda = Pubkey::find_program_address(&[program_id.as_slice()], &account_compression::ID).0;
    println!("pda: {}", pda);
    let seed = 2;
    println!("\n\ne2e test seed {}\n\n", seed);

    let mut rng = StdRng::seed_from_u64(seed);

    for _ in 0..1100 {
        let _ = rng.gen::<u64>();
    }

    let tree_params = InitAddressTreeAccountsInstructionData::testnet_default();
    let mut env = TestAccounts::get_testnet_accounts();
    env.protocol.forester = Keypair::from_bytes(&get_testnet_forester_test_keypair()).unwrap();
    env.protocol.governance_authority =
        Keypair::from_bytes(&get_testnet_governance_authority_keypair()).unwrap();

    println!("env: {:?}", env);

    let mut config = forester_config();
    config.transaction_config.batch_ixs_per_tx = 1;
    config.payer_keypair = env.protocol.forester.insecure_clone();
    config.derivation_pubkey = env.protocol.forester.pubkey();
    config.general_config = GeneralConfig::test_address_v2();

    let mut rpc = LightClient::new(LightClientConfig {
        url: RpcUrl::Devnet.to_string(),
        photon_url: Some(get_photon_indexer_url()),
        commitment_config: Some(CommitmentConfig::processed()),
        fetch_active_tree: false,
    })
    .await
    .unwrap();
    rpc.payer = env.protocol.forester.insecure_clone();
    let mut photon_indexer = create_photon_indexer();

    {
        println!("==== state tree debug ====");
        let output_queue_pubkey = &env.v2_state_trees[0].output_queue;
        let merkle_tree_pubkey = &env.v2_state_trees[0].merkle_tree;

        println!("state merkle tree: {:?}", merkle_tree_pubkey);
        println!("state merkle tree: {:?}", merkle_tree_pubkey.to_bytes());

        let mut account = rpc
            .get_account(*output_queue_pubkey)
            .await
            .unwrap()
            .unwrap();
        let queue = BatchedQueueAccount::output_from_bytes(account.data.as_mut_slice()).unwrap();

        let batch_metadata = &queue.get_metadata().batch_metadata;
        let zkp_batch_size = batch_metadata.zkp_batch_size as usize;

        let mut queue_length: usize = 0;
        for (idx, batch) in batch_metadata.batches.iter().enumerate() {
            let total_zkp_batches = batch.get_num_zkp_batches() as usize;
            let inserted_zkp_batches = batch.get_num_inserted_zkps() as usize;
            let remaining_zkp_batches = total_zkp_batches.saturating_sub(inserted_zkp_batches);

            println!("total_zkp_batches[{}]: {}", idx, total_zkp_batches);
            println!("inserted_zkp_batches[{}]: {}", idx, inserted_zkp_batches);
            println!("remaining_zkp_batches[{}]: {}", idx, remaining_zkp_batches);
            queue_length += remaining_zkp_batches * zkp_batch_size;
        }

        println!("queue_length = {}", queue_length);

        let (zkp_batch_size, leaves_hash_chains) =
            get_output_queue_metadata(&mut rpc, *output_queue_pubkey)
                .await
                .unwrap();

        println!("zkp_batch_size: {}", zkp_batch_size);
        println!("leaves_hash_chains: {:?}", leaves_hash_chains);

        let (merkle_tree_next_index, current_root, root_history) =
            get_merkle_tree_metadata(&mut rpc, *merkle_tree_pubkey)
                .await
                .unwrap();

        println!("merkle_tree_next_index: {}", merkle_tree_next_index);
        println!("current_root: {:?}", current_root);
        println!("root_history: {:?}", root_history);

        let indexer_root = "0x08457E3019E9FD7496FF3C7BDDF684A91869F2EA44ACBD30A3AF886A5C42803F";
        let indexer_root_bytes = deserialize_hex_string_to_be_bytes(indexer_root);
        println!("indexer_root: {:?}", indexer_root_bytes);

        let total_elements = zkp_batch_size as usize * leaves_hash_chains.len();
        let offset = merkle_tree_next_index;

        let queue_elements = photon_indexer
            .get_queue_elements(
                merkle_tree_pubkey.to_bytes(),
                QueueType::OutputStateV2,
                total_elements as u16,
                Some(offset),
                None,
            )
            .await
            .map_err(|e| {
                panic!("Failed to get queue elements from indexer: {:?}", e);
            })
            .unwrap()
            .value
            .items;

        println!("queue_elements: {:?}", queue_elements);
    }

    {
        println!("==== address tree debug ====");
        // let acc_pubkey = &env.v2_state_trees[0].output_queue;
        // let mut account = rpc.get_account(*acc_pubkey).await.unwrap().unwrap();
        // println!("acc_pubkey {:?}", acc_pubkey);
        // println!("account {:?}", account.owner);
        // println!("account[0..8] = {:?}", account.data[0..8].to_vec());
        // let queue = BatchedQueueAccount::output_from_bytes(account.data.as_mut_slice()).unwrap();
        // println!("state queue {:?}", queue.get_metadata());

        let merkle_tree_pubkey = &env.v2_address_trees[0];
        println!("address merkle tree: {:?}", merkle_tree_pubkey);
        println!("address merkle tree: {:?}", merkle_tree_pubkey.to_bytes());
        // 0xA52648440B8BB4F0F061FDF1933099A041C03A451F7E847F83FE695C9B879B5C
        let mut merkle_tree_account = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();

        let (leaves_hash_chains, start_index, current_root, batch_size) = {
            let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
                merkle_tree_account.data.as_mut_slice(),
                &(*merkle_tree_pubkey).into(),
            )
            .unwrap();

            let full_batch_index = merkle_tree.queue_batches.pending_batch_index;
            let batch = &merkle_tree.queue_batches.batches[full_batch_index as usize];

            let mut hash_chains = Vec::new();
            let zkp_batch_index = batch.get_num_inserted_zkps();
            let current_zkp_batch_index = batch.get_current_zkp_batch_index();

            println!(
                "Full batch index: {}, inserted ZKPs: {}, current ZKP index: {}, ready for insertion: {}",
                full_batch_index, zkp_batch_index, current_zkp_batch_index, current_zkp_batch_index - zkp_batch_index
            );

            for i in zkp_batch_index..current_zkp_batch_index {
                hash_chains
                    .push(merkle_tree.hash_chain_stores[full_batch_index as usize][i as usize]);
            }

            let start_index = merkle_tree.next_index;

            merkle_tree
                .root_history
                .as_slice()
                .iter()
                .enumerate()
                .for_each(|(i, root)| {
                    println!("Root at index {}: {:?}", i, root);
                });

            let current_root = *merkle_tree.root_history.last().unwrap();
            let zkp_batch_size = batch.zkp_batch_size as u16;

            (hash_chains, start_index, current_root, zkp_batch_size)
        };

        // let indexer_root = "0x28144AF4BCE54E81FC4E524A73F06B9EE79A015498A5454CAD7B618AD4115426";
        // let indexer_root_bytes = deserialize_hex_string_to_be_bytes(indexer_root);
        // println!("indexer_root: {:?}", indexer_root_bytes);

        println!("leaves_hash_chains.len(): {:?}", leaves_hash_chains.len());
        println!("zkp_batch_size: {:?}", batch_size);
        println!("on-chain root: {:?}", current_root);

        /*
        batch 0 state: Fill
        batch 0 zkp_batch_size: 250
        batch 0 total_zkp_batches: 60
        batch 0 get_current_zkp_batch_index: 10
        batch 0 inserted_zkp_batches: 10
        batch 0 remaining_zkp_batches: 50
        batch 0 is ready to insert? false
        AddressV2 queue C7g8NqRsEDhi3v9AyVpCfL16YYdHPhrR74douckfrhqu length: 12500
        */

        let total_elements = std::cmp::min(batch_size as usize * leaves_hash_chains.len(), 500);
        println!("Requesting {} total elements from indexer", total_elements);

        let indexer_update_info = photon_indexer
            .get_address_queue_with_proofs(merkle_tree_pubkey, total_elements as u16, None, None)
            .await
            .unwrap();
        println!("indexer_update_info {:?}", indexer_update_info);

        if indexer_update_info.value.non_inclusion_proofs.is_empty() {
            println!("No non-inclusion proofs found");
        } else {
            let indexer_root = indexer_update_info
                .value
                .non_inclusion_proofs
                .first()
                .unwrap()
                .root;

            if indexer_root != current_root {
                println!("Indexer root does not match on-chain root");
                println!("Indexer root: {:?}", indexer_root);
                println!("On-chain root: {:?}", current_root);
            }
        }
    }

    let batch_payer = &env.protocol.forester.insecure_clone();

    {
        let (_, _, pre_root) =
            get_initial_merkle_tree_state(&mut rpc, &env.v2_address_trees[0]).await;

        let batch_size = get_address_batch_size(&mut rpc, &env.v2_address_trees[0]).await;
        let num_addresses = 2;

        let num_batches = batch_size / num_addresses;
        let remaining_addresses = batch_size % num_addresses;

        println!("num_addresses: {:?}", num_addresses);
        println!("batch_size: {:?}", batch_size);
        println!("num_batches: {:?}", num_batches);
        println!("remaining_addresses: {:?}", remaining_addresses);

        for i in 0..num_batches {
            println!("====== Creating v2 address {} ======", i);
            let result = create_v2_addresses(
                &mut rpc,
                &env.v2_address_trees[0],
                &env.protocol.registered_program_pda,
                batch_payer,
                &env,
                &mut rng,
                num_addresses as usize,
            )
            .await;
            println!("====== result: {:?} ======", result);
            result.expect("Create address in v2 tree not successful.");
        }
        for i in 0..remaining_addresses {
            println!("====== Creating v2 address {} ======", i);
            let result = create_v2_addresses(
                &mut rpc,
                &env.v2_address_trees[0],
                &env.protocol.registered_program_pda,
                batch_payer,
                &env,
                &mut rng,
                1,
            )
            .await;
            println!("====== result: {:?} ======", result);
        }

        let mut address_tree_account = rpc
            .get_account(env.v2_address_trees[0])
            .await
            .unwrap()
            .unwrap();

        let address_tree = BatchedMerkleTreeAccount::address_from_bytes(
            address_tree_account.data.as_mut_slice(),
            &env.v2_address_trees[0].into(),
        )
        .unwrap();

        println!("Address tree metadata: {:?}", address_tree.get_metadata());
    }

    {
        // let mint_keypair: [u8; 64] = [
        //     252, 188, 100, 55, 45, 34, 146, 113, 156, 209, 84, 80, 67, 178, 150, 224, 27, 158, 159,
        //     140, 54, 122, 217, 223, 134, 145, 104, 172, 55, 171, 181, 115, 144, 165, 49, 170, 28, 148,
        //     60, 153, 101, 66, 81, 199, 63, 165, 38, 240, 206, 220, 169, 234, 29, 230, 22, 74, 49, 189,
        //     28, 226, 242, 128, 191, 112,
        // ];

        let mint_pubkey = [
            144, 165, 49, 170, 28, 148, 60, 153, 101, 66, 81, 199, 63, 165, 38, 240, 206, 220, 169,
            234, 29, 230, 22, 74, 49, 189, 28, 226, 242, 128, 191, 112,
        ];

        // let mint_keypair: [u8; 64] = [
        //     34, 68, 161, 27, 78, 253, 99, 153, 78, 49, 80, 3, 91, 36, 109, 239, 124, 205, 252, 8, 215,
        //     224, 39, 252, 166, 9, 245, 56, 195, 218, 140, 14, 173, 222, 249, 91, 197, 119, 150, 178,
        //     25, 88, 80, 224, 210, 133, 225, 204, 170, 35, 60, 253, 39, 235, 125, 43, 59, 137, 54, 5,
        //     38, 118, 47, 170,
        // ];
        // let mint_keypair = Keypair::from_bytes(&mint_keypair).unwrap();
        let mint_keypair = Keypair::new();
        println!("mint keypair: {:?}", mint_keypair.to_bytes());
        let mint_pubkey =
            create_mint_helper_with_keypair(&mut rpc, batch_payer, &mint_keypair).await;
        // let mint_pubkey: [u8; 32] = [
        //     173, 222, 249, 91, 197, 119, 150, 178, 25, 88, 80, 224, 210, 133, 225, 204, 170, 35, 60,
        //     253, 39, 235, 125, 43, 59, 137, 54, 5, 38, 118, 47, 170,
        // ];
        let mint_pubkey = Pubkey::from(mint_pubkey);
        // println!("mint_pubkey: {:?}", mint_pubkey.to_pubkey_bytes());
        // println!("mint_pubkey: {:?}", mint_pubkey.to_string());

        let sig = mint_to(
            &mut rpc,
            &env.v2_state_trees[0].output_queue,
            batch_payer,
            &mint_pubkey,
        )
        .await;
        println!("mint_to: {:?}", sig);

        wait_for_indexer(&mut rpc, &photon_indexer).await.unwrap();
        let batch_size = get_state_batch_size(&mut rpc, &env.v2_state_trees[0].merkle_tree).await;
        println!("state batch size: {}", batch_size);
        for i in 0..batch_size {
            {
                let batch_compress_sig = compress(
                    &mut rpc,
                    &env.v2_state_trees[0].output_queue,
                    batch_payer,
                    if i == 0 { 1_000_000 } else { 10_000 },
                )
                .await;
                println!("{} batch compress: {:?}", i, batch_compress_sig);
            }

            {
                let batch_transfer_sig = transfer::<true, LightClient>(
                    &mut rpc,
                    &env.v2_state_trees[0].output_queue,
                    batch_payer,
                    &env,
                )
                .await;
                println!("{} batch transfer: {:?}", i, batch_transfer_sig);
            }

            {
                let batch_transfer_token_sig = compressed_token_transfer::<LightClient>(
                    &mut rpc,
                    &env.v2_state_trees[0].output_queue,
                    batch_payer,
                    &mint_pubkey,
                )
                .await;
                println!("{} batch token transfer: {:?}", i, batch_transfer_token_sig);
            }
        }
    }

    // let (service_handle, shutdown_sender, mut work_report_receiver) =
    //     setup_forester_pipeline(&config).await;

    // wait_for_work_report(&mut work_report_receiver, &tree_params).await;

    // verify_root_changed(&mut rpc, &env.v2_address_trees[0], &pre_root).await;

    // shutdown_sender
    //     .send(())
    //     .expect("Failed to send shutdown signal");
    // service_handle.await.unwrap().unwrap();
}

async fn setup_forester_pipeline(
    config: &ForesterConfig,
) -> (
    tokio::task::JoinHandle<anyhow::Result<()>>,
    oneshot::Sender<()>,
    mpsc::Receiver<WorkReport>,
) {
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let (work_report_sender, work_report_receiver) = mpsc::channel(100);

    let forester_photon_indexer = PhotonIndexer::new(get_photon_indexer_url(), None);

    let service_handle = tokio::spawn(run_pipeline::<LightClient, PhotonIndexer>(
        Arc::from(config.clone()),
        None,
        None,
        Arc::new(Mutex::new(forester_photon_indexer)),
        shutdown_receiver,
        work_report_sender,
    ));

    (service_handle, shutdown_sender, work_report_receiver)
}

async fn wait_for_work_report(
    work_report_receiver: &mut mpsc::Receiver<WorkReport>,
    tree_params: &InitAddressTreeAccountsInstructionData,
) {
    let batch_size = tree_params.input_queue_batch_size as usize;
    let minimum_processed_items: usize = tree_params.input_queue_batch_size as usize;
    let mut total_processed_items: usize = 0;
    let timeout_duration = Duration::from_secs(DEFAULT_TIMEOUT_SECONDS);

    println!("Waiting for work reports...");
    println!("Batch size: {}", batch_size);
    println!(
        "Minimum required processed items: {}",
        minimum_processed_items
    );

    let start_time = tokio::time::Instant::now();
    while total_processed_items < minimum_processed_items {
        match tokio::time::timeout(
            timeout_duration.saturating_sub(start_time.elapsed()),
            work_report_receiver.recv(),
        )
        .await
        {
            Ok(Some(report)) => {
                println!("Received work report: {:?}", report);
                total_processed_items += report.processed_items;
            }
            Ok(None) => {
                println!("Work report channel closed unexpectedly");
                break;
            }
            Err(_) => {
                println!("Timed out after waiting for {:?}", timeout_duration);
                break;
            }
        }
    }

    println!("Total processed items: {}", total_processed_items);
    assert!(
        total_processed_items >= minimum_processed_items,
        "Processed fewer items ({}) than required ({})",
        total_processed_items,
        minimum_processed_items
    );
}

async fn create_v2_addresses<R: Rpc + MerkleTreeExt + Indexer>(
    rpc: &mut R,
    batch_address_merkle_tree: &Pubkey,
    registered_program_pda: &Pubkey,
    payer: &Keypair,
    env: &TestAccounts,
    rng: &mut StdRng,
    num_addresses: usize,
) -> Result<(), light_client::rpc::RpcError> {
    let mut address_seeds = Vec::with_capacity(num_addresses);
    let mut addresses = Vec::with_capacity(num_addresses);

    for _ in 0..num_addresses {
        let seed = rng.gen();
        let address = derive_address(
            &seed,
            &batch_address_merkle_tree.to_bytes(),
            &create_address_test_program::ID.to_bytes(),
        );

        address_seeds.push(seed);
        addresses.push(address);
    }

    for (i, (address, seed)) in addresses.iter().zip(address_seeds.iter()).enumerate() {
        println!("Creating v2 address #{} with:", i + 1);
        println!("- address: {:?}", address);
        println!(
            "- address_merkle_tree_pubkey: {:?}",
            batch_address_merkle_tree
        );
        println!("- program_id: {:?}", create_address_test_program::ID);
        println!("- seed: {:?}", seed);
    }

    let address_with_trees = addresses
        .into_iter()
        .map(|address| AddressWithTree {
            address,
            tree: *batch_address_merkle_tree,
        })
        .collect::<Vec<_>>();

    let proof_result = rpc
        .get_validity_proof(Vec::new(), address_with_trees, None)
        .await
        .unwrap();

    println!(
        "- new root: {:?}",
        proof_result.value.addresses.first().unwrap().root
    );
    println!(
        "- new root index: {:?}",
        proof_result.value.addresses.first().unwrap().root_index
    );
    println!("=====================================");

    if num_addresses == 1 {
        let data: [u8; 31] = [1; 31];
        let new_address_params = NewAddressParams {
            seed: address_seeds[0],
            address_merkle_tree_pubkey: (*batch_address_merkle_tree).into(),
            address_queue_pubkey: (*batch_address_merkle_tree).into(),
            address_merkle_tree_root_index: 10, //proof_result.value.get_address_root_indices()[0],
        };

        let create_ix_inputs = CreateCompressedPdaInstructionInputs {
            data,
            signer: &payer.pubkey(),
            output_compressed_account_merkle_tree_pubkey: &env.v1_state_trees[0].merkle_tree,
            proof: &proof_result.value.proof.0.unwrap(),
            new_address_params,
            registered_program_pda,
        };

        let instruction = create_pda_instruction(create_ix_inputs);
        let instructions = vec![
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
                COMPUTE_BUDGET_LIMIT,
            ),
            instruction,
        ];

        let result = rpc
            .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
            .await;

        println!("Transaction result: {:?}", result);
        result.map(|_| ())
    } else {
        let new_address_params = address_seeds
            .iter()
            .enumerate()
            .map(|(i, seed)| NewAddressParamsAssigned {
                seed: *seed,
                address_queue_pubkey: (*batch_address_merkle_tree).into(),
                address_merkle_tree_pubkey: (*batch_address_merkle_tree).into(),
                address_merkle_tree_root_index: proof_result.value.get_address_root_indices()[i],
                assigned_account_index: None,
            })
            .collect::<Vec<_>>();

        let output_accounts: Vec<OutputCompressedAccountWithContext> = Vec::new();
        let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
        let packed_new_address_params =
            pack_new_address_params_assigned(&new_address_params, &mut remaining_accounts);
        let packed_inputs: Vec<PackedCompressedAccountWithMerkleContext> = Vec::new();
        let output_compressed_accounts = pack_output_compressed_accounts(
            output_accounts
                .iter()
                .map(|x| x.compressed_account.clone())
                .collect::<Vec<_>>()
                .as_slice(),
            output_accounts
                .iter()
                .map(|x| x.merkle_tree.into())
                .collect::<Vec<_>>()
                .as_slice(),
            &mut remaining_accounts,
        );

        let ix_data = InstructionDataInvokeCpiWithReadOnly {
            mode: 0,
            bump: 255,
            with_cpi_context: false,
            invoking_program_id: create_address_test_program::ID.into(),
            proof: proof_result.value.proof.0,
            new_address_params: packed_new_address_params,
            is_compress: false,
            compress_or_decompress_lamports: 0,
            output_compressed_accounts: output_compressed_accounts.clone(),
            input_compressed_accounts: packed_inputs
                .iter()
                .map(|x| InAccount {
                    address: x.compressed_account.address,
                    merkle_context: x.merkle_context,
                    lamports: x.compressed_account.lamports,
                    discriminator: x
                        .compressed_account
                        .data
                        .as_ref()
                        .map_or([0u8; 8], |d| d.discriminator),
                    data_hash: x
                        .compressed_account
                        .data
                        .as_ref()
                        .map_or([0u8; 32], |d| d.data_hash),
                    root_index: x.root_index,
                })
                .collect::<Vec<_>>(),
            with_transaction_hash: true,
            read_only_accounts: Vec::new(),
            read_only_addresses: Vec::new(),
            cpi_context: Default::default(),
        };

        let remaining_accounts = to_account_metas(remaining_accounts);

        let instruction = create_invoke_cpi_instruction(
            payer.pubkey(),
            ix_data.try_to_vec()?,
            remaining_accounts,
            None,
        );

        let instructions = vec![
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
                COMPUTE_BUDGET_LIMIT,
            ),
            instruction,
        ];

        let result = rpc
            .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
            .await;

        println!("Transaction result for multiple addresses: {:?}", result);
        result.map(|_| ())
    }
}

async fn get_address_batch_size<R: Rpc>(rpc: &mut R, merkle_tree_pubkey: &Pubkey) -> u64 {
    let mut merkle_tree_account = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
        merkle_tree_account.data.as_mut_slice(),
        &merkle_tree_pubkey.into(),
    )
    .unwrap();

    merkle_tree.get_metadata().queue_batches.batch_size
}

async fn get_state_batch_size<R: Rpc>(rpc: &mut R, merkle_tree_pubkey: &Pubkey) -> u64 {
    let mut merkle_tree_account = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
        merkle_tree_account.data.as_mut_slice(),
        &merkle_tree_pubkey.into(),
    )
    .unwrap();

    merkle_tree.get_metadata().queue_batches.batch_size
}

async fn get_initial_merkle_tree_state(
    rpc: &mut LightClient,
    merkle_tree_pubkey: &Pubkey,
) -> (u64, u64, [u8; 32]) {
    let mut merkle_tree_account = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
        merkle_tree_account.data.as_mut_slice(),
        &merkle_tree_pubkey.into(),
    )
    .unwrap();

    let initial_next_index = merkle_tree.get_metadata().next_index;
    let initial_sequence_number = merkle_tree.get_metadata().sequence_number;
    (
        initial_next_index,
        initial_sequence_number,
        merkle_tree.get_root().unwrap(),
    )
}

async fn verify_root_changed(
    rpc: &mut LightClient,
    merkle_tree_pubkey: &Pubkey,
    pre_root: &[u8; 32],
) {
    let mut merkle_tree_account = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
        merkle_tree_account.data.as_mut_slice(),
        &merkle_tree_pubkey.into(),
    )
    .unwrap();
    println!("merkle tree metadata: {:?}", merkle_tree.get_metadata());
    assert_ne!(
        *pre_root,
        merkle_tree.get_root().unwrap(),
        "Root should have changed"
    );
}

async fn mint_to<R: Rpc>(
    rpc: &mut R,
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

    println!("mint_to_ix: {:?}", mint_to_ix);

    let instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            COMPUTE_BUDGET_LIMIT,
        ),
        mint_to_ix,
    ];

    rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .unwrap()
}

async fn compressed_token_transfer<R: Rpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    payer: &Keypair,
    mint: &Pubkey,
) -> Signature {
    println!("compressed_token_transfer begin");
    let mut input_compressed_accounts: Vec<TokenDataWithMerkleContext> = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(
            &payer.pubkey(),
            Some(GetCompressedTokenAccountsByOwnerOrDelegateOptions {
                mint: Some(*mint),
                cursor: None,
                limit: None,
            }),
            None,
        )
        .await
        .unwrap()
        .into();

    // println!(
    //     "compressed_token_transfer input_compressed_accounts: {:?}",
    //     input_compressed_accounts
    // );
    // assert_eq!(
    // std::cmp::min(input_compressed_accounts.len(), 1000),
    // std::cmp::min(*counter as usize, 1000)
    // );
    let rng = &mut rand::thread_rng();
    let num_inputs = rng.gen_range(1..2);
    input_compressed_accounts.shuffle(rng);
    input_compressed_accounts.truncate(num_inputs);
    let tokens = input_compressed_accounts
        .iter()
        .map(|x| x.token_data.amount)
        .sum::<u64>();
    let compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| {
            // println!(
            //     "compressed_token_transfer compressed_account hash: {:?}",
            //     x.compressed_account.hash()
            // );
            // println!(
            //     "compressed_token_transfer merkle_context: {:?}",
            //     x.compressed_account.merkle_context
            // );
            x.compressed_account.hash().unwrap()
        })
        .collect::<Vec<[u8; 32]>>();
    let proof_for_compressed_accounts = rpc
        .indexer()
        .unwrap()
        .get_validity_proof(compressed_account_hashes, vec![], None)
        .await
        .unwrap();
    let root_indices = proof_for_compressed_accounts.value.get_root_indices();
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
    // println!(
    //     "compressed_token_transfer input_compressed_accounts: {:?}",
    //     input_compressed_accounts
    // );
    // println!(
    //     "compressed_token_transfer compressed_accounts: {:?}",
    //     compressed_accounts
    // );
    let proof = if root_indices.iter().all(|x| x.is_none()) {
        None
    } else {
        proof_for_compressed_accounts
            .value
            .proof
            .0
            .map(|proof| CompressedProof {
                a: proof.a,
                b: proof.b,
                c: proof.c,
            })
    };
    let input_token_data = input_compressed_accounts
        .iter()
        .map(|x| sdk_to_program_token_data(x.token_data.clone()))
        .collect::<Vec<_>>();
    let input_compressed_accounts_data = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.compressed_account.clone())
        .collect::<Vec<_>>();
    let instruction = create_transfer_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &merkle_contexts,
        &compressed_accounts,
        &root_indices,
        &proof,
        &input_token_data,
        &input_compressed_accounts_data,
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
        false,
    )
    .unwrap();
    // println!(
    //     "compressed_token_transfer compressed_accounts: {:?}",
    //     input_compressed_accounts_data
    // );
    // println!("compressed_token_transfer root_indices: {:?}", root_indices);
    let mut instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            COMPUTE_BUDGET_LIMIT,
        ),
    ];
    instructions.push(instruction);
    let sig = rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .unwrap();
    // *counter += OUTPUT_ACCOUNT_NUM as u64;
    // *counter -= input_compressed_accounts_data.len() as u64;
    sig
}

async fn transfer<const V2: bool, R: Rpc + Indexer>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    payer: &Keypair,
    // counter: &mut u64,
    test_accounts: &TestAccounts,
) -> Signature {
    let input_compressed_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_accounts_by_owner(&payer.pubkey(), None, None)
        .await
        .map(|response| response.value.items)
        .unwrap_or(vec![]);
    let mut input_compressed_accounts = if V2 {
        input_compressed_accounts
            .into_iter()
            .filter(|x| {
                test_accounts
                    .v2_state_trees
                    .iter()
                    .any(|y| y.merkle_tree == x.tree_info.tree)
            })
            .collect::<Vec<_>>()
    } else {
        input_compressed_accounts
            .into_iter()
            .filter(|x| {
                test_accounts
                    .v1_state_trees
                    .iter()
                    .any(|y| y.merkle_tree == x.tree_info.tree)
            })
            .collect::<Vec<_>>()
    };
    // assert_eq!(
    //     std::cmp::min(input_compressed_accounts.len(), 1000),
    //     std::cmp::min(*counter as usize, 1000)
    // );
    let rng = &mut rand::thread_rng();
    let num_inputs = rng.gen_range(1..2);
    input_compressed_accounts.shuffle(rng);
    input_compressed_accounts.truncate(num_inputs);
    let lamports = input_compressed_accounts
        .iter()
        .map(|x| x.lamports)
        .sum::<u64>();
    let compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| x.hash)
        .collect::<Vec<[u8; 32]>>();
    println!("compressed_account_hashes: {:?}", compressed_account_hashes);
    let proof_for_compressed_accounts = rpc
        .get_validity_proof(compressed_account_hashes, vec![], None)
        .await
        .unwrap();
    let root_indices = proof_for_compressed_accounts.value.get_root_indices();
    let merkle_contexts = input_compressed_accounts
        .iter()
        .map(
            |x| light_compressed_account::compressed_account::MerkleContext {
                merkle_tree_pubkey: x.tree_info.tree.into(),
                queue_pubkey: x.tree_info.queue.into(),
                leaf_index: x.leaf_index,
                prove_by_index: false,
                tree_type: TreeType::StateV2,
            },
        )
        .collect::<Vec<light_compressed_account::compressed_account::MerkleContext>>();
    let lamp = lamports / OUTPUT_ACCOUNT_NUM as u64;
    let lamport_remained = lamports % OUTPUT_ACCOUNT_NUM as u64;
    let mut compressed_accounts = vec![
        CompressedAccount {
            lamports: lamp,
            owner: payer.pubkey().into(),
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
            .value
            .proof
            .0
            .map(|proof| CompressedProof {
                a: proof.a,
                b: proof.b,
                c: proof.c,
            })
    };
    let input_compressed_accounts_data = input_compressed_accounts
        .iter()
        .map(|x| CompressedAccount {
            lamports: x.lamports,
            owner: x.owner.into(),
            address: x.address,
            data: x.data.clone(),
        })
        .collect::<Vec<CompressedAccount>>();
    let instruction = create_invoke_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &input_compressed_accounts_data,
        &compressed_accounts,
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
        input_compressed_accounts_data
    );
    println!("transfer root_indices: {:?}", root_indices);
    let mut instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            COMPUTE_BUDGET_LIMIT,
        ),
    ];
    instructions.push(instruction);
    let sig = rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .unwrap();
    // *counter += OUTPUT_ACCOUNT_NUM as u64;
    // *counter -= input_compressed_accounts_data.len() as u64;
    sig
}

async fn compress<R: Rpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    payer: &Keypair,
    lamports: u64,
) -> Signature {
    let compress_account = CompressedAccount {
        lamports,
        owner: payer.pubkey().into(),
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
    let mut instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            COMPUTE_BUDGET_LIMIT,
        ),
    ];
    instructions.push(instruction);
    match rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
    {
        Ok(sig) => {
            // *counter += 1;
            sig
        }
        Err(e) => {
            println!("compress error: {:?}", e);
            panic!("compress error: {:?}", e);
        }
    }
}
