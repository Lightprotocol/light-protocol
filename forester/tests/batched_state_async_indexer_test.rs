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
use tokio::time::sleep;
use tracing::log::info;

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

    let (initial_next_index, initial_sequence_number, _pre_root) = {
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

    let compressed_balance_photon = photon_indexer
        .get_compressed_accounts_by_owner_v2(&forester_keypair.pubkey())
        .await
        .unwrap_or(vec![]);

    println!(
        "compressed_balance_photon before transfer: {:?}",
        compressed_balance_photon
    );

    compress(&mut rpc, &env.batched_output_queue, &forester_keypair, 1_000_000).await;
    for i in 0..merkle_tree.get_metadata().queue_batches.batch_size {
        println!("\ntx {}", i);

        compress(&mut rpc, &env.batched_output_queue, &forester_keypair, 10_000).await;
        transfer(&mut rpc, &photon_indexer, &env.batched_output_queue, &forester_keypair).await;
    }

    let num_output_zkp_batches =
        tree_params.input_queue_batch_size / tree_params.output_queue_zkp_batch_size;
    println!("num_output_zkp_batches: {}", num_output_zkp_batches);
}

async fn transfer(
    rpc: &mut SolanaRpcConnection,
    indexer: &PhotonIndexer<SolanaRpcConnection>,
    merkle_tree_pubkey: &Pubkey,
    forester_keypair: &Keypair,
) {
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
            &[&forester_keypair],
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

    println!("transfer tx: {:?}", sig);
}

async fn compress(rpc: &mut SolanaRpcConnection, merkle_tree_pubkey: &Pubkey, payer: &Keypair, lamports: u64) {
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
            &[&payer],
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

    println!("compress tx: {:?}", sig);
}
