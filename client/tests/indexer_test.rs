use light_client::Indexer;
use light_client::{indexer::PhotonIndexer, rpc::SolanaRpcConnection, RpcConnection};
use light_prover_client::gnark::helpers::{
    spawn_validator, LightValidatorConfig, ProofType, ProverConfig,
};
use light_system_program::sdk::compressed_account::CompressedAccount;
use light_system_program::sdk::invoke::create_invoke_instruction;
use light_test_utils::{test_env::EnvAccounts, SolanaRpcUrl};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, signature::Keypair, signer::Signer};

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
async fn test_compare_indexers() {
    // Start actual test validator
    let config = LightValidatorConfig {
        enable_indexer: true,
        wait_time: 10,
        prover_config: Some(ProverConfig {
            run_mode: None,
            circuits: vec![ProofType::Inclusion],
        }),
    };
    spawn_validator(config).await;

    // Setup test environment
    let payer = Keypair::new();
    let env_accounts = EnvAccounts::get_local_test_validator_accounts();

    // Setup RPC connection
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    rpc.payer = payer.insecure_clone();

    // Fund the payer account
    rpc.airdrop_lamports(&payer.pubkey(), LAMPORTS_PER_SOL * 100)
        .await
        .unwrap();

    // Create and execute a test transaction
    let amount = 0;

    // Compress SOL using test transaction
    compress_sol_test(
        &mut rpc,
        &payer,
        false,
        amount,
        &env_accounts.merkle_tree_pubkey,
    )
    .await;

    // Initialize photon indexer
    let photon_indexer = PhotonIndexer::new(
        "http://127.0.0.1:8784".to_string(), // Local indexer URL
        None,
        rpc,
    );

    // Fetch accounts from photon indexer
    let photon_accounts = photon_indexer
        .get_rpc_compressed_accounts_by_owner(&payer.pubkey())
        .await
        .unwrap();
    // Ensure photon indexer returned accounts
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    assert!(
        !photon_accounts.is_empty(),
        "Photon indexer returned no accounts"
    );
}

async fn compress_sol_test(
    rpc: &mut SolanaRpcConnection,
    payer: &Keypair,
    is_compress: bool,
    amount: u64,
    merkle_tree_pubkey: &Pubkey,
) {
    let output_compressed_account: CompressedAccount = CompressedAccount {
        lamports: if is_compress { amount } else { 0 },
        owner: payer.pubkey(),
        data: None,
        address: None,
    };

    let instruction = create_invoke_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &[],
        &[output_compressed_account],
        &[],
        &[merkle_tree_pubkey.clone()],
        &[],
        &[],
        None,
        None,
        false,
        None,
        true,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer],
        rpc.client.get_latest_blockhash().unwrap(),
    );

    rpc.client
        .send_and_confirm_transaction(&transaction)
        .unwrap();
}
