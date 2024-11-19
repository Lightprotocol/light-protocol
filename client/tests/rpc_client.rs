use light_client::photon_rpc::Base58Conversions;
use light_client::photon_rpc::Hash;
use light_client::{
    photon_rpc::{AddressWithTree, PhotonClient},
    rpc::SolanaRpcConnection,
};
use light_compressed_token::mint_sdk::{
    create_create_token_pool_instruction, create_mint_to_instruction,
};
use light_prover_client::gnark::helpers::{
    spawn_validator, LightValidatorConfig, ProofType, ProverConfig,
};
use light_system_program::sdk::{
    compressed_account::CompressedAccount, invoke::create_invoke_instruction,
};
use light_test_utils::test_env::EnvAccounts;
use light_test_utils::RpcConnection;
use light_utils::hash_to_bn254_field_size_be;
use num_traits::ToPrimitive;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, signature::Keypair, signer::Signer, system_instruction,
    transaction::Transaction,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_all_endpoints() {
    // Endpoints tested:
    // 1. get_rpc_compressed_accounts_by_owner
    // 2. get_multiple_compressed_accounts
    // 3. get_validity_proof
    // 4. get_compressed_account
    // 5. get_compressed_account_balance
    // 6. get_compression_signatures_for_account
    // 7. get_compressed_token_accounts_by_owner
    // 8. get_compressed_token_account_balance
    // 9. get_compressed_token_balances_by_owner
    // 10. get_multiple_compressed_account_proofs
    // 11. get_multiple_new_address_proofs

    let config = LightValidatorConfig {
        enable_indexer: true,
        prover_config: Some(ProverConfig {
            run_mode: None,
            circuits: vec![ProofType::Combined],
        }),
        wait_time: 20,
    };

    spawn_validator(config).await;

    let env_accounts = EnvAccounts::get_local_test_validator_accounts();
    let mut rpc: SolanaRpcConnection =
        SolanaRpcConnection::new("http://127.0.0.1:8899".to_string(), None);
    let client = PhotonClient::new("http://127.0.0.1:8784".to_string());

    let payer_pubkey = rpc.get_payer().pubkey();
    rpc.airdrop_lamports(&payer_pubkey, LAMPORTS_PER_SOL)
        .await
        .unwrap();

    // create compressed account
    let lamports = LAMPORTS_PER_SOL / 2;
    let output_account = CompressedAccount {
        lamports,
        owner: rpc.get_payer().pubkey(),
        data: None,
        address: None,
    };

    let ix = create_invoke_instruction(
        &rpc.get_payer().pubkey(),
        &rpc.get_payer().pubkey(),
        &[],
        &[output_account],
        &[],
        &[env_accounts.merkle_tree_pubkey],
        &[],
        &[],
        None,
        Some(lamports),
        true,
        None,
        true,
    );

    let tx_create_compressed_account = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer_pubkey),
        &[&rpc.get_payer()],
        rpc.client.get_latest_blockhash().unwrap(),
    );
    rpc.client
        .send_and_confirm_transaction(&tx_create_compressed_account)
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let mint = Keypair::new();

    // Setup mint and create compressed token account
    let mint_rent = rpc
        .client
        .get_minimum_balance_for_rent_exemption(82)
        .unwrap();
    let create_mint_ix = system_instruction::create_account(
        &payer_pubkey,
        &mint.pubkey(),
        mint_rent,
        82,
        &spl_token::id(),
    );

    let init_mint_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint.pubkey(),
        &payer_pubkey,
        None,
        9,
    )
    .unwrap();

    // Create token pool for compression
    let create_pool_ix = create_create_token_pool_instruction(&payer_pubkey, &mint.pubkey());

    let tx = Transaction::new_signed_with_payer(
        &[create_mint_ix, init_mint_ix, create_pool_ix],
        Some(&payer_pubkey),
        &[&rpc.get_payer(), &mint],
        rpc.client.get_latest_blockhash().unwrap(),
    );
    rpc.client.send_and_confirm_transaction(&tx).unwrap();

    let amount = 1_000_000;

    let mint_ix = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &mint.pubkey(),
        &env_accounts.merkle_tree_pubkey,
        vec![amount],
        vec![payer_pubkey],
        None,
    );

    let tx = Transaction::new_signed_with_payer(
        &[mint_ix],
        Some(&payer_pubkey),
        &[&rpc.get_payer()],
        rpc.client.get_latest_blockhash().unwrap(),
    );
    rpc.client.send_and_confirm_transaction(&tx).unwrap();

    let pubkey = payer_pubkey;
    let hashes = client
        .get_rpc_compressed_accounts_by_owner(&pubkey)
        .await
        .unwrap();
    assert!(!hashes.is_empty());

    let first_hash = hashes[0];

    let seed = rand::random::<[u8; 32]>();
    let new_addresses = vec![AddressWithTree {
        address: hash_to_bn254_field_size_be(&seed).unwrap().0,
        tree: env_accounts.address_merkle_tree_pubkey,
    }];

    let accounts = client
        .get_multiple_compressed_accounts(None, Some(hashes.clone()))
        .await
        .unwrap();

    assert!(!accounts.value.is_empty());
    assert_eq!(accounts.value[0].hash, first_hash);

    let result = client
        .get_validity_proof(hashes.clone(), new_addresses)
        .await
        .unwrap();
    assert_eq!(
        Hash::from_base58(result.value.leaves[0].as_ref()).unwrap(),
        hashes[0]
    );

    let account = client
        .get_compressed_account(None, Some(first_hash))
        .await
        .unwrap();
    assert_eq!(account.value.lamports, lamports);
    assert_eq!(account.value.owner, rpc.get_payer().pubkey().to_string());

    let balance = client
        .get_compressed_account_balance(None, Some(first_hash))
        .await
        .unwrap();
    assert_eq!(balance.value.lamports, lamports);

    let signatures = client
        .get_compression_signatures_for_account(first_hash)
        .await
        .unwrap();
    assert_eq!(
        signatures.value.items[0].signature,
        tx_create_compressed_account.signatures[0].to_string()
    );

    let token_account = &client
        .get_compressed_token_accounts_by_owner(&pubkey, None)
        .await
        .unwrap()
        .value
        .items[0];
    assert_eq!(token_account.token_data.mint, mint.pubkey().to_string());
    assert_eq!(token_account.token_data.owner, payer_pubkey.to_string());

    let balance = client
        .get_compressed_token_account_balance(
            None,
            Some(
                Hash::from_base58(token_account.account.hash.as_ref())
                    .unwrap()
                    .to_bytes(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(balance.value.amount, amount.to_string());

    let balances = client
        .get_compressed_token_balances_by_owner(&pubkey, None)
        .await
        .unwrap();
    assert_eq!(
        balances.value.token_balances[0].balance,
        amount.to_i32().unwrap()
    );

    let proofs = client
        .get_multiple_compressed_account_proofs(hashes.clone())
        .await
        .unwrap();
    assert!(!proofs.is_empty());
    assert_eq!(proofs[0].hash, hashes[0].to_base58());

    let addresses = vec![hash_to_bn254_field_size_be(&seed).unwrap().0];
    let new_address_proofs = client
        .get_multiple_new_address_proofs(env_accounts.merkle_tree_pubkey, addresses)
        .await
        .unwrap();
    assert!(!new_address_proofs.is_empty());
    assert_eq!(
        new_address_proofs[0].merkle_tree.to_bytes(),
        env_accounts.merkle_tree_pubkey.to_bytes()
    );
}
