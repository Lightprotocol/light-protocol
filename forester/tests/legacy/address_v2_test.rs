use std::{collections::HashMap, sync::Arc, time::Duration};

use anchor_lang::Discriminator;
use borsh::BorshSerialize;
use create_address_test_program::create_invoke_cpi_instruction;
use forester::{config::GeneralConfig, epoch_manager::WorkReport, run_pipeline, ForesterConfig};
use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount,
};
use light_client::{
    indexer::AddressWithTree,
    local_test_validator::LightValidatorConfig,
    rpc::{merkle_tree::MerkleTreeExt, LightClient, LightClientConfig, Rpc},
};
use light_compressed_account::{
    address::derive_address,
    compressed_account::PackedCompressedAccountWithMerkleContext,
    instruction_data::{
        data::{NewAddressParams, NewAddressParamsAssigned, OutputCompressedAccountWithContext},
        with_readonly::{InAccount, InstructionDataInvokeCpiWithReadOnly},
    },
};
use light_compressed_token::process_transfer::transfer_sdk::to_account_metas;
use light_program_test::{accounts::test_accounts::TestAccounts, Indexer};
use light_test_utils::{
    create_address_test_program_sdk::{
        create_pda_instruction, CreateCompressedPdaInstructionInputs,
    },
    pack::{pack_new_address_params_assigned, pack_output_compressed_accounts},
};
use rand::{prelude::StdRng, Rng, SeedableRng};
use serial_test::serial;
use solana_program::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use solana_sdk::{signature::Keypair, signer::Signer};
use tokio::sync::{mpsc, oneshot};

use crate::test_utils::{forester_config, init};

mod test_utils;

const DEFAULT_TIMEOUT_SECONDS: u64 = 120;
const COMPUTE_BUDGET_LIMIT: u32 = 1_000_000;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[serial]
#[ignore = "legacy, left for for photon e2e test snapshot"]
async fn test_create_v2_address() {
    let seed = 0;
    println!("\n\ne2e test seed {}\n\n", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    let tree_params = InitAddressTreeAccountsInstructionData::test_default();

    init(Some(LightValidatorConfig {
        enable_indexer: true,
        enable_prover: true,
        wait_time: 90,
        sbf_programs: vec![(
            "FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy".to_string(),
            "../target/deploy/create_address_test_program.so".to_string(),
        )],
        limit_ledger_size: Some(500000),
    }))
    .await;

    let env = TestAccounts::get_local_test_validator_accounts();
    let mut config = forester_config();
    config.payer_keypair = env.protocol.forester.insecure_clone();
    config.derivation_pubkey = env.protocol.forester.pubkey();
    config.general_config = GeneralConfig::test_address_v2();

    let mut rpc = LightClient::new(LightClientConfig::local()).await.unwrap();
    rpc.payer = env.protocol.forester.insecure_clone();

    ensure_sufficient_balance(
        &mut rpc,
        &env.protocol.forester.pubkey(),
        LAMPORTS_PER_SOL * 100,
    )
    .await;

    let (_, _, pre_root) = get_initial_merkle_tree_state(&mut rpc, &env.v2_address_trees[0]).await;

    let batch_payer = Keypair::from_bytes(&[
        88, 117, 248, 40, 40, 5, 251, 124, 235, 221, 10, 212, 169, 203, 91, 203, 255, 67, 210, 150,
        87, 182, 238, 155, 87, 24, 176, 252, 157, 119, 68, 81, 148, 156, 30, 0, 60, 63, 34, 247,
        192, 120, 4, 170, 32, 149, 221, 144, 74, 244, 181, 142, 37, 197, 196, 136, 159, 196, 101,
        21, 194, 56, 163, 1,
    ])
    .unwrap();
    ensure_sufficient_balance(&mut rpc, &batch_payer.pubkey(), LAMPORTS_PER_SOL * 100).await;

    let batch_size = get_batch_size(&mut rpc, &env.v2_address_trees[0]).await;
    let num_addresses = 2;

    let num_batches = batch_size / num_addresses;
    let remaining_addresses = batch_size % num_addresses;

    println!("num_addresses: {:?}", num_addresses);
    println!("batch_size: {:?}", batch_size);
    println!("num_batches: {:?}", num_batches);
    println!("remaining_addresses: {:?}", remaining_addresses);

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

    let (service_handle, shutdown_sender, mut work_report_receiver) =
        setup_forester_pipeline(&config).await;

    for i in 0..num_batches * 10 {
        println!("====== Creating v2 address {} ======", i);
        let result = create_v2_addresses(
            &mut rpc,
            &env.v2_address_trees[0],
            &env.protocol.registered_program_pda,
            &batch_payer,
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
            &batch_payer,
            &env,
            &mut rng,
            1,
        )
        .await;
        println!("====== result: {:?} ======", result);
    }

    wait_for_work_report(&mut work_report_receiver, &tree_params).await;

    verify_root_changed(&mut rpc, &env.v2_address_trees[0], &pre_root).await;

    shutdown_sender
        .send(())
        .expect("Failed to send shutdown signal");
    service_handle.await.unwrap().unwrap();
}

async fn ensure_sufficient_balance(rpc: &mut LightClient, pubkey: &Pubkey, target_balance: u64) {
    if rpc.get_balance(pubkey).await.unwrap() < target_balance {
        rpc.airdrop_lamports(pubkey, target_balance).await.unwrap();
    }
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

    let service_handle = tokio::spawn(run_pipeline::<LightClient>(
        Arc::from(config.clone()),
        None,
        None,
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

    if num_addresses == 1 {
        let data: [u8; 31] = [1; 31];
        let new_address_params = NewAddressParams {
            seed: address_seeds[0],
            address_merkle_tree_pubkey: (*batch_address_merkle_tree).into(),
            address_queue_pubkey: (*batch_address_merkle_tree).into(),
            address_merkle_tree_root_index: proof_result.value.get_address_root_indices()[0],
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
            [
                light_system_program::instruction::InvokeCpiWithReadOnly::DISCRIMINATOR.to_vec(),
                ix_data.try_to_vec()?,
            ]
            .concat(),
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

async fn get_batch_size<R: Rpc>(rpc: &mut R, merkle_tree_pubkey: &Pubkey) -> u64 {
    let mut merkle_tree_account = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
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
