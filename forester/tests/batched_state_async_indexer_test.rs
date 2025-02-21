use forester_utils::registry::{register_test_forester, update_test_forester};
use light_batched_merkle_tree::{
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount,
};
use light_client::{
    indexer::{photon_indexer::PhotonIndexer, Indexer},
    rpc::{solana_rpc::SolanaRpcUrl, RpcConnection, SolanaRpcConnection},
    rpc_pool::SolanaRpcPool,
};
use light_program_test::{test_env::EnvAccounts};
use light_prover_client::gnark::helpers::{LightValidatorConfig};
use serial_test::serial;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_sdk::{
    commitment_config::CommitmentConfig, signature::Keypair, signer::Signer,
};
use tracing::log::info;
use forester_utils::airdrop_lamports;
use light_client::transaction_params::{FeeConfig, TransactionParams};
use light_compressed_account::compressed_account::{CompressedAccount};
use light_registry::protocol_config::state::ProtocolConfigPda;
use light_registry::utils::get_protocol_config_pda_address;
use light_test_utils::system_program::{create_invoke_instruction};
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
    println!("here");
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

    println!(
        "get_compressed_accounts_by_owner({}) initial",
        &forester_keypair.pubkey()
    );
    let compressed_balance_photon = photon_indexer
        .get_compressed_accounts_by_owner_v2(&forester_keypair.pubkey())
        .await
        .unwrap();
    println!("compressed_balance_photon before transfer: {:?}", compressed_balance_photon);

    for i in 0..merkle_tree.get_metadata().queue_batches.batch_size {
        println!("\ntx {}", i);

        // compress
        {
            let lamports = 100_000;

            let compress_account = CompressedAccount {
                lamports,
                owner: forester_keypair.pubkey(),
                address: None,
                data: None,
            };

            println!("env.batched_output_queue: {:?}", env.batched_output_queue);
            println!("env.batched_state_merkle_tree: {:?}", env.batched_state_merkle_tree);

            let instruction = create_invoke_instruction(
                &forester_keypair.pubkey(),
                &forester_keypair.pubkey(),
                &[],
                &[compress_account],
                &[],
                &[env.batched_output_queue],
                &[],
                &[],
                None,
                Some(lamports),
                true,
                None,
                true,
            );

            let event = rpc
                .create_and_send_transaction_with_public_event(
                    &[instruction],
                    &forester_keypair.pubkey(),
                    &[&forester_keypair],
                    Some(TransactionParams {
                        num_input_compressed_accounts: 0,
                        num_output_compressed_accounts: 1,
                        num_new_addresses: 0,
                        compress: lamports as i64,
                        fee_config: FeeConfig::test_batched(),
                    }),
                )
                .await
                .unwrap();

            println!("compress event: {:?}", event);
        }
    }

    let num_output_zkp_batches =
        tree_params.input_queue_batch_size / tree_params.output_queue_zkp_batch_size;
    println!("num_output_zkp_batches: {}", num_output_zkp_batches);
}
