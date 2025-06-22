use std::{collections::HashMap, sync::Arc, time::Duration};

use account_compression::{AddressMerkleTreeAccount, QueueAccount};
use anchor_lang::Discriminator;
use borsh::BorshSerialize;
use create_address_test_program::create_invoke_cpi_instruction;
use forester::{
    config::GeneralConfig, epoch_manager::WorkReport, run_pipeline, utils::get_protocol_config,
    ForesterConfig,
};
use forester_utils::utils::wait_for_indexer;
use light_batched_merkle_tree::{
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount,
};
use light_client::{
    indexer::{
        photon_indexer::PhotonIndexer, AddressWithTree,
        GetCompressedTokenAccountsByOwnerOrDelegateOptions, Indexer,
    },
    local_test_validator::{LightValidatorConfig, ProverConfig},
    rpc::{merkle_tree::MerkleTreeExt, LightClient, LightClientConfig, Rpc},
};
use light_compressed_account::{
    address::derive_address,
    address::derive_address_legacy,
    compressed_account::CompressedAccount,
    instruction_data::{
        compressed_proof::CompressedProof,
        data::{NewAddressParams, NewAddressParamsAssigned},
        with_readonly::InstructionDataInvokeCpiWithReadOnly,
    },
    TreeType,
};
use light_compressed_token::process_transfer::{
    transfer_sdk::{create_transfer_instruction, to_account_metas},
    TokenTransferOutputData,
};
use light_hasher::Poseidon;
use light_program_test::accounts::test_accounts::TestAccounts;
use light_prover_client::prover::spawn_prover;
use light_sdk::token::TokenDataWithMerkleContext;
use light_test_utils::{
    conversions::sdk_to_program_token_data, get_indexed_merkle_tree,
    pack::pack_new_address_params_assigned, spl::create_mint_helper_with_keypair,
    system_program::create_invoke_instruction,
};
use rand::{prelude::SliceRandom, rngs::StdRng, Rng, SeedableRng};
use serial_test::serial;
use solana_program::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use solana_sdk::{
    signature::{Keypair, Signature},
    signer::Signer,
};
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    time::{sleep, timeout},
};

use crate::test_utils::{forester_config, get_active_phase_start_slot, init, wait_for_slot};

mod test_utils;

const MINT_TO_NUM: u64 = 5;
const BATCHES_NUM: u64 = 10;
const DEFAULT_TIMEOUT_SECONDS: u64 = 60 * 15;
const PHOTON_INDEXER_URL: &str = "http://127.0.0.1:8784";
const COMPUTE_BUDGET_LIMIT: u32 = 1_000_000;

#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
#[serial]
async fn test_e2e_v2() {
    let state_tree_params = InitStateTreeAccountsInstructionData::test_default();

    init(Some(LightValidatorConfig {
        enable_indexer: true,
        wait_time: 30,
        prover_config: None,
        sbf_programs: vec![(
            "FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy".to_string(),
            "../target/deploy/create_address_test_program.so".to_string(),
        )],
        limit_ledger_size: None,
    }))
    .await;
    spawn_prover(ProverConfig::default()).await;

    let env = TestAccounts::get_local_test_validator_accounts();
    let mut config = forester_config();
    config.transaction_config.batch_ixs_per_tx = 3;
    config.payer_keypair = env.protocol.forester.insecure_clone();
    config.derivation_pubkey = env.protocol.forester.pubkey();
    config.general_config = GeneralConfig::default();

    let mut rpc = setup_rpc_connection(&env.protocol.forester).await;
    ensure_sufficient_balance(
        &mut rpc,
        &env.protocol.forester.pubkey(),
        LAMPORTS_PER_SOL * 100,
    )
    .await;
    ensure_sufficient_balance(
        &mut rpc,
        &env.protocol.governance_authority.pubkey(),
        LAMPORTS_PER_SOL * 100,
    )
    .await;

    let mut photon_indexer = create_photon_indexer();
    let protocol_config = get_protocol_config(&mut rpc).await;

    let (service_handle, shutdown_sender, mut work_report_receiver) =
        setup_forester_pipeline(&config).await;

    let active_phase_slot = get_active_phase_start_slot(&mut rpc, &protocol_config).await;
    wait_for_slot(&mut rpc, active_phase_slot).await;

    let (_initial_state_next_index, _initial_state_sequence_number, pre_state_root) =
        get_initial_merkle_tree_state(
            &mut rpc,
            &env.v2_state_trees[0].merkle_tree,
            TreeType::StateV2,
        )
        .await;
    let (_initial_address_next_index, _initial_address_sequence_number, pre_address_root) =
        get_initial_merkle_tree_state(&mut rpc, &env.v2_address_trees[0], TreeType::AddressV2)
            .await;

    let batch_payer = Keypair::new();
    let legacy_payer = Keypair::new();

    println!("batch payer pubkey: {:?}", batch_payer.pubkey());
    println!("legacy payer pubkey: {:?}", legacy_payer.pubkey());

    ensure_sufficient_balance(&mut rpc, &legacy_payer.pubkey(), LAMPORTS_PER_SOL * 100).await;
    ensure_sufficient_balance(&mut rpc, &batch_payer.pubkey(), LAMPORTS_PER_SOL * 100).await;

    let mint_keypair = Keypair::new();
    let mint_pubkey = create_mint_helper_with_keypair(&mut rpc, &batch_payer, &mint_keypair).await;

    let sig = mint_to(
        &mut rpc,
        &env.v2_state_trees[0].output_queue,
        &batch_payer,
        &mint_pubkey,
    )
    .await;
    println!("mint_to: {:?}", sig);

    let mut sender_batched_accs_counter = 0;
    let mut sender_legacy_accs_counter = 0;
    let mut sender_batched_token_counter: u64 = MINT_TO_NUM * 2;
    let mut address_v1_counter = 0;
    let mut address_v2_counter = 0;

    wait_for_indexer(&rpc, &photon_indexer).await.unwrap();

    let rng_seed = rand::thread_rng().gen::<u64>();
    println!("seed {}", rng_seed);
    let rng = &mut StdRng::seed_from_u64(rng_seed);

    execute_test_transactions(
        &mut rpc,
        &mut photon_indexer,
        rng,
        &env,
        &batch_payer,
        &legacy_payer,
        &mint_pubkey,
        &mut sender_batched_accs_counter,
        &mut sender_legacy_accs_counter,
        &mut sender_batched_token_counter,
        &mut address_v1_counter,
        &mut address_v2_counter,
    )
    .await;

    wait_for_work_report(&mut work_report_receiver, &state_tree_params).await;

    verify_root_changed(
        &mut rpc,
        &env.v2_state_trees[0].merkle_tree,
        &pre_state_root,
        TreeType::StateV2,
    )
    .await;

    verify_root_changed(
        &mut rpc,
        &env.v2_address_trees[0],
        &pre_address_root,
        TreeType::AddressV2,
    )
    .await;

    shutdown_sender
        .send(())
        .expect("Failed to send shutdown signal");
    service_handle.await.unwrap().unwrap();
}

async fn setup_rpc_connection(forester: &Keypair) -> LightClient {
    let mut rpc = LightClient::new(LightClientConfig::local()).await.unwrap();
    rpc.payer = forester.insecure_clone();
    rpc
}

async fn ensure_sufficient_balance(rpc: &mut LightClient, pubkey: &Pubkey, target_balance: u64) {
    if rpc.get_balance(pubkey).await.unwrap() < target_balance {
        rpc.airdrop_lamports(pubkey, target_balance).await.unwrap();
    }
}

fn create_photon_indexer() -> PhotonIndexer {
    PhotonIndexer::new(PHOTON_INDEXER_URL.to_string(), None)
}

async fn get_initial_merkle_tree_state(
    rpc: &mut LightClient,
    merkle_tree_pubkey: &Pubkey,
    kind: TreeType,
) -> (u64, u64, [u8; 32]) {
    let mut merkle_tree_account = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree = match kind {
        TreeType::StateV2 => BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &merkle_tree_pubkey.into(),
        )
        .unwrap(),
        TreeType::AddressV2 => BatchedMerkleTreeAccount::address_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &merkle_tree_pubkey.into(),
        )
        .unwrap(),
        _ => {
            panic!("Unsupported tree type");
        }
    };

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
    kind: TreeType,
) {
    let mut merkle_tree_account = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree = match kind {
        TreeType::StateV1 | TreeType::StateV2 => BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &merkle_tree_pubkey.into(),
        )
        .unwrap(),
        TreeType::AddressV1 | TreeType::AddressV2 => BatchedMerkleTreeAccount::address_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &merkle_tree_pubkey.into(),
        )
        .unwrap(),
    };
    println!(
        "Final merkle tree metadata: {:?}",
        merkle_tree.get_metadata()
    );
    assert_ne!(
        *pre_root,
        merkle_tree.get_root().unwrap(),
        "Root should have changed"
    );
}

async fn get_state_zkp_batch_size<R: Rpc>(rpc: &mut R, merkle_tree_pubkey: &Pubkey) -> u64 {
    let mut merkle_tree_account = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
        merkle_tree_account.data.as_mut_slice(),
        &merkle_tree_pubkey.into(),
    )
    .unwrap();

    merkle_tree.get_metadata().queue_batches.zkp_batch_size
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

    let forester_photon_indexer = create_photon_indexer();
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
    tree_params: &InitStateTreeAccountsInstructionData,
) {
    let batch_size = tree_params.output_queue_zkp_batch_size as usize;
    let minimum_processed_items: usize = tree_params.output_queue_batch_size as usize;
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
        match timeout(
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

#[allow(clippy::too_many_arguments)]
async fn execute_test_transactions<R: Rpc + Indexer + MerkleTreeExt, I: Indexer>(
    rpc: &mut R,
    indexer: &mut I,
    rng: &mut StdRng,
    env: &TestAccounts,
    batch_payer: &Keypair,
    legacy_payer: &Keypair,
    mint_pubkey: &Pubkey,
    sender_batched_accs_counter: &mut u64,
    sender_legacy_accs_counter: &mut u64,
    sender_batched_token_counter: &mut u64,
    address_v1_counter: &mut u64,
    address_v2_counter: &mut u64,
) {
    let batch_size = get_state_zkp_batch_size(rpc, &env.v2_state_trees[0].merkle_tree).await;
    println!("batch size: {}", batch_size);
    for i in 0..batch_size * BATCHES_NUM {
        let batch_compress_sig = compress(
            rpc,
            &env.v2_state_trees[0].output_queue,
            batch_payer,
            if i == 0 { 1_000_000 } else { 10_000 },
            sender_batched_accs_counter,
        )
        .await;
        println!("{} v2 compress: {:?}", i, batch_compress_sig);

        let compress_sig = compress(
            rpc,
            &env.v1_state_trees[0].merkle_tree,
            legacy_payer,
            10_000,
            sender_legacy_accs_counter,
        )
        .await;
        println!("{} v1 compress: {:?}", i, compress_sig);

        sleep(Duration::from_millis(1000)).await;

        let batch_transfer_sig = transfer::<true, R, I>(
            rpc,
            indexer,
            &env.v2_state_trees[0].output_queue,
            batch_payer,
            sender_batched_accs_counter,
            env,
        )
        .await;
        println!("{} v2 transfer: {:?}", i, batch_transfer_sig);

        let legacy_transfer_sig = transfer::<false, R, I>(
            rpc,
            indexer,
            &env.v1_state_trees[0].merkle_tree,
            legacy_payer,
            sender_legacy_accs_counter,
            env,
        )
        .await;
        println!("{} v1 transfer: {:?}", i, legacy_transfer_sig);

        let batch_transfer_token_sig = compressed_token_transfer(
            rpc,
            indexer,
            &env.v2_state_trees[0].output_queue,
            batch_payer,
            mint_pubkey,
            sender_batched_token_counter,
        )
        .await;
        println!("{} v2 token transfer: {:?}", i, batch_transfer_token_sig);

        // let sig_v1_addr = create_v1_address(
        //     rpc,
        //     indexer,
        //     rng,
        //     &env.v1_address_trees[0].merkle_tree,
        //     &env.v1_address_trees[0].queue,
        //     legacy_payer,
        //     address_v1_counter,
        // )
        // .await;
        // println!("{} v1 address create: {:?}", i, sig_v1_addr);

        let sig_v2_addr = create_v2_addresses(
            rpc,
            &env.v2_address_trees[0],
            &env.protocol.registered_program_pda,
            batch_payer,
            env,
            rng,
            2,
        )
        .await;

        if sig_v2_addr.is_ok() {
            *address_v2_counter += 2;
        }
        println!("{} v2 address create: {:?}", i, sig_v2_addr);

        sleep(Duration::from_millis(1000)).await;
    }
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
        vec![100_000; MINT_TO_NUM as usize],
        vec![payer.pubkey(); MINT_TO_NUM as usize],
        None,
        false,
        0,
    );
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

async fn compressed_token_transfer<R: Rpc, I: Indexer>(
    rpc: &mut R,
    indexer: &I,
    merkle_tree_pubkey: &Pubkey,
    payer: &Keypair,
    mint: &Pubkey,
    counter: &mut u64,
) -> Signature {
    wait_for_indexer(rpc, indexer).await.unwrap();
    let mut input_compressed_accounts: Vec<TokenDataWithMerkleContext> = indexer
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
    if input_compressed_accounts.is_empty() {
        return Signature::default();
    }

    let rng = &mut rand::thread_rng();
    input_compressed_accounts.shuffle(rng);
    input_compressed_accounts.truncate(1);

    let tokens = input_compressed_accounts[0].token_data.amount;
    let compressed_account_hashes = vec![input_compressed_accounts[0]
        .compressed_account
        .hash()
        .unwrap()];

    let proof_for_compressed_accounts = indexer
        .get_validity_proof(compressed_account_hashes, vec![], None)
        .await
        .unwrap();

    let root_indices = proof_for_compressed_accounts.value.get_root_indices();
    let merkle_contexts = vec![
        input_compressed_accounts[0]
            .compressed_account
            .merkle_context,
    ];

    let compressed_accounts = vec![TokenTransferOutputData {
        amount: tokens,
        owner: payer.pubkey(),
        lamports: None,
        merkle_tree: *merkle_tree_pubkey,
    }];

    let proof = proof_for_compressed_accounts
        .value
        .proof
        .0
        .map(|p| CompressedProof {
            a: p.a,
            b: p.b,
            c: p.c,
        });
    let input_token_data = vec![sdk_to_program_token_data(
        input_compressed_accounts[0].token_data.clone(),
    )];
    let input_compressed_accounts_data = vec![input_compressed_accounts[0]
        .compressed_account
        .compressed_account
        .clone()];

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

    let instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            COMPUTE_BUDGET_LIMIT,
        ),
        instruction,
    ];
    let sig = rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .unwrap();
    *counter += compressed_accounts.len() as u64;
    *counter -= input_compressed_accounts.len() as u64;
    sig
}

async fn transfer<const V2: bool, R: Rpc + Indexer, I: Indexer>(
    rpc: &mut R,
    indexer: &I,
    merkle_tree_pubkey: &Pubkey,
    payer: &Keypair,
    counter: &mut u64,
    test_accounts: &TestAccounts,
) -> Signature {
    wait_for_indexer(rpc, indexer).await.unwrap();
    let mut input_compressed_accounts = indexer
        .get_compressed_accounts_by_owner(&payer.pubkey(), None, None)
        .await
        .map(|response| response.value.items)
        .unwrap_or(vec![]);

    input_compressed_accounts = if V2 {
        input_compressed_accounts
            .into_iter()
            .filter(|x| {
                test_accounts
                    .v2_state_trees
                    .iter()
                    .any(|y| y.merkle_tree == x.tree_info.tree)
            })
            .collect()
    } else {
        input_compressed_accounts
            .into_iter()
            .filter(|x| {
                test_accounts
                    .v1_state_trees
                    .iter()
                    .any(|y| y.merkle_tree == x.tree_info.tree)
            })
            .collect()
    };

    if input_compressed_accounts.is_empty() {
        return Signature::default();
    }

    let rng = &mut rand::thread_rng();
    input_compressed_accounts.shuffle(rng);
    input_compressed_accounts.truncate(1);

    let lamports = input_compressed_accounts[0].lamports;
    let compressed_account_hashes = vec![input_compressed_accounts[0].hash];

    let proof_for_compressed_accounts = indexer
        .get_validity_proof(compressed_account_hashes, vec![], None)
        .await
        .unwrap();
    let root_indices = proof_for_compressed_accounts.value.get_root_indices();
    let merkle_contexts = vec![
        light_compressed_account::compressed_account::MerkleContext {
            merkle_tree_pubkey: input_compressed_accounts[0].tree_info.tree.into(),
            queue_pubkey: input_compressed_accounts[0].tree_info.queue.into(),
            leaf_index: input_compressed_accounts[0].leaf_index,
            prove_by_index: false,
            tree_type: if V2 {
                TreeType::StateV2
            } else {
                TreeType::StateV1
            },
        },
    ];

    let compressed_accounts = vec![CompressedAccount {
        lamports,
        owner: payer.pubkey().into(),
        address: None,
        data: None,
    }];
    let proof = proof_for_compressed_accounts
        .value
        .proof
        .0
        .map(|p| CompressedProof {
            a: p.a,
            b: p.b,
            c: p.c,
        });
    let input_compressed_accounts_data = vec![CompressedAccount {
        lamports: input_compressed_accounts[0].lamports,
        owner: input_compressed_accounts[0].owner.into(),
        address: input_compressed_accounts[0].address,
        data: input_compressed_accounts[0].data.clone(),
    }];

    let instruction = create_invoke_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &input_compressed_accounts_data,
        &compressed_accounts,
        &merkle_contexts,
        &[*merkle_tree_pubkey],
        &root_indices,
        &[],
        proof,
        None,
        false,
        None,
        true,
    );

    let instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            COMPUTE_BUDGET_LIMIT,
        ),
        instruction,
    ];
    let sig = rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .unwrap();
    *counter += compressed_accounts.len() as u64;
    *counter -= input_compressed_accounts_data.len() as u64;
    sig
}

async fn compress<R: Rpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    payer: &Keypair,
    lamports: u64,
    counter: &mut u64,
) -> Signature {
    let payer_balance = rpc.get_balance(&payer.pubkey()).await.unwrap();
    println!("payer balance: {}", payer_balance);

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
    let instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            COMPUTE_BUDGET_LIMIT,
        ),
        instruction,
    ];
    match rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
    {
        Ok(sig) => {
            *counter += 1;
            sig
        }
        Err(e) => {
            panic!("compress error: {:?}", e);
        }
    }
}

async fn create_v1_address<R: Rpc, I: Indexer>(
    rpc: &mut R,
    indexer: &mut I,
    rng: &mut StdRng,
    merkle_tree_pubkey: &Pubkey,
    queue: &Pubkey,
    payer: &Keypair,
    counter: &mut u64,
) -> Signature {
    let seed = rng.gen::<[u8; 32]>();
    let address = derive_address_legacy(
        &light_compressed_account::Pubkey::from(*merkle_tree_pubkey),
        &seed,
    )
    .unwrap();
    let address_proof_inputs = vec![AddressWithTree {
        address,
        tree: *merkle_tree_pubkey,
    }];

    wait_for_indexer(rpc, indexer).await.unwrap();
    let proof_for_addresses = indexer
        .get_validity_proof(vec![], address_proof_inputs, None)
        .await
        .unwrap();

    let new_address_params = vec![NewAddressParams {
        seed,
        address_queue_pubkey: (*queue).into(),
        address_merkle_tree_pubkey: (*merkle_tree_pubkey).into(),
        address_merkle_tree_root_index: proof_for_addresses.value.get_address_root_indices()[0],
    }];

    let proof = proof_for_addresses.value.proof.0.map(|p| CompressedProof {
        a: p.a,
        b: p.b,
        c: p.c,
    });

    println!("creating v1 address");
    let root = proof_for_addresses.value.addresses[0].root;
    let index = proof_for_addresses.value.addresses[0].root_index;

    println!("indexer root: {:?}, index: {}", root, index);

    {
        let account = rpc
            .get_anchor_account::<AddressMerkleTreeAccount>(merkle_tree_pubkey)
            .await
            .unwrap();
        // let queue_account = rpc
        //     .get_anchor_account::<QueueAccount>(&account.unwrap().metadata.associated_queue.into())
        //     .await
        //     .unwrap();
        let merkle_tree =
            get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
                rpc,
                *merkle_tree_pubkey,
            )
            .await;

        for (idx, root) in merkle_tree.roots.iter().enumerate() {
            println!("root[{}]: {:?}", idx, root);
        }
        println!("root index: {}", merkle_tree.root_index());
    }

    let instruction = create_invoke_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &[],
        &[],
        &[],
        &[],
        &proof_for_addresses.value.get_root_indices(),
        &new_address_params,
        proof,
        None,
        false,
        None,
        false,
    );

    let instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            COMPUTE_BUDGET_LIMIT,
        ),
        instruction,
    ];
    let sig = rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .unwrap();
    *counter += 1;
    sig
}

async fn create_v2_addresses<R: Rpc + MerkleTreeExt + Indexer>(
    rpc: &mut R,
    batch_address_merkle_tree: &Pubkey,
    _registered_program_pda: &Pubkey,
    payer: &Keypair,
    _env: &TestAccounts,
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

    let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
    let packed_new_address_params =
        pack_new_address_params_assigned(&new_address_params, &mut remaining_accounts);

    let ix_data = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 255,
        with_cpi_context: false,
        invoking_program_id: create_address_test_program::ID.into(),
        proof: proof_result.value.proof.0,
        new_address_params: packed_new_address_params,
        is_compress: false,
        compress_or_decompress_lamports: 0,
        output_compressed_accounts: Default::default(),
        input_compressed_accounts: Default::default(),
        with_transaction_hash: true,
        read_only_accounts: Vec::new(),
        read_only_addresses: Vec::new(),
        cpi_context: Default::default(),
    };

    let remaining_accounts_metas = to_account_metas(remaining_accounts);

    let instruction = create_invoke_cpi_instruction(
        payer.pubkey(),
        [
            light_system_program::instruction::InvokeCpiWithReadOnly::DISCRIMINATOR.to_vec(),
            ix_data.try_to_vec()?,
        ]
        .concat(),
        remaining_accounts_metas,
        None,
    );

    let instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            COMPUTE_BUDGET_LIMIT,
        ),
        instruction,
    ];

    rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .map(|_| ())
}
