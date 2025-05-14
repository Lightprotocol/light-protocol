use light_client::{
    indexer::{AddressWithTree, Base58Conversions, Hash, Indexer},
    rpc::{rpc_connection::RpcConnectionConfig, SolanaRpcConnection},
};
use light_compressed_account::{
    compressed_account::CompressedAccount, hash_to_bn254_field_size_be,
};
use light_compressed_token::mint_sdk::{
    create_create_token_pool_instruction, create_mint_to_instruction,
};
use light_program_test::accounts::test_accounts::TestAccounts;
use light_prover_client::gnark::helpers::{
    spawn_validator, LightValidatorConfig, ProofType, ProverConfig,
};
use light_test_utils::{system_program::create_invoke_instruction, RpcConnection};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_system_interface::instruction::create_account;
use solana_transaction::Transaction;

// Constants
const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

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
        prover_config: Some(ProverConfig::default()),
        wait_time: 60,
        sbf_programs: vec![],
        limit_ledger_size: None,
    };

    spawn_validator(config).await;

    let test_accounts = TestAccounts::get_local_test_validator_accounts();
    let mut rpc: SolanaRpcConnection = SolanaRpcConnection::new(RpcConnectionConfig::local());

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
        &[test_accounts.v1_state_trees[0].merkle_tree],
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
    let create_mint_ix = create_account(
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
    let create_pool_ix = create_create_token_pool_instruction(&payer_pubkey, &mint.pubkey(), false);

    let tx = Transaction::new_signed_with_payer(
        &[create_mint_ix, init_mint_ix, create_pool_ix],
        Some(&payer_pubkey),
        &[rpc.get_payer(), &mint],
        rpc.client.get_latest_blockhash().unwrap(),
    );
    rpc.client.send_and_confirm_transaction(&tx).unwrap();

    let amount = 1_000_000;

    let mint_ix = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &mint.pubkey(),
        &test_accounts.v1_state_trees[0].merkle_tree,
        vec![amount],
        vec![payer_pubkey],
        None,
        false,
        0,
    );

    let tx = Transaction::new_signed_with_payer(
        &[mint_ix],
        Some(&payer_pubkey),
        &[&rpc.get_payer()],
        rpc.client.get_latest_blockhash().unwrap(),
    );
    rpc.client.send_and_confirm_transaction(&tx).unwrap();

    let pubkey = payer_pubkey;
    let accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_accounts_by_owner_v2(&pubkey)
        .await
        .unwrap();
    assert!(!accounts.is_empty());
    let first_account = accounts[0].clone();
    let seed = rand::random::<[u8; 32]>();
    let new_addresses = vec![AddressWithTree {
        address: hash_to_bn254_field_size_be(&seed),
        tree: test_accounts.v1_address_trees[0].merkle_tree,
    }];

    let account_hashes: Vec<Hash> = accounts.iter().map(|a| a.hash().unwrap()).collect();
    let accounts = rpc
        .indexer()
        .unwrap()
        .get_multiple_compressed_accounts(None, Some(account_hashes.clone()))
        .await
        .unwrap();

    assert!(!accounts.is_empty());
    assert_eq!(
        Hash::from_base58(&accounts[0].hash).unwrap(),
        first_account.hash().unwrap()
    );

    let result = rpc
        .indexer()
        .unwrap()
        .get_validity_proof(account_hashes.clone(), new_addresses.clone())
        .await
        .unwrap();
    assert_eq!(result.root_indices.len(), account_hashes.len());
    assert_eq!(result.address_root_indices.len(), new_addresses.len());

    let account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(None, Some(first_account.hash().unwrap()))
        .await
        .unwrap();
    assert_eq!(account.lamports, lamports);
    assert_eq!(account.owner, rpc.get_payer().pubkey().to_string());

    let balance = rpc
        .indexer()
        .unwrap()
        .get_compressed_account_balance(None, Some(first_account.hash().unwrap()))
        .await
        .unwrap();
    assert_eq!(balance, lamports);

    let signatures = rpc
        .indexer()
        .unwrap()
        .get_compression_signatures_for_account(first_account.hash().unwrap())
        .await
        .unwrap();
    assert_eq!(
        signatures[0],
        tx_create_compressed_account.signatures[0].to_string()
    );

    let token_accounts = &rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&pubkey, None)
        .await
        .unwrap();

    assert_eq!(token_accounts[0].token_data.mint, mint.pubkey());
    assert_eq!(token_accounts[0].token_data.owner, payer_pubkey);

    let hash = token_accounts[0].compressed_account.hash().unwrap();

    let balance = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_account_balance(None, Some(hash))
        .await
        .unwrap();
    assert_eq!(balance, amount);

    assert_eq!(token_accounts[0].token_data.mint, mint.pubkey());
    assert_eq!(token_accounts[0].token_data.owner, payer_pubkey);

    let hash = token_accounts[0].compressed_account.hash().unwrap();

    let balances = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_balances_by_owner(&pubkey, None)
        .await
        .unwrap();

    assert_eq!(balances.token_balances[0].balance, amount);

    let balance = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_account_balance(None, Some(hash))
        .await
        .unwrap();
    assert_eq!(balance, amount);

    let hashes_str = account_hashes.iter().map(|h| h.to_base58()).collect();
    let proofs = rpc
        .indexer()
        .unwrap()
        .get_multiple_compressed_account_proofs(hashes_str)
        .await
        .unwrap();
    assert!(!proofs.is_empty());
    assert_eq!(proofs[0].hash, account_hashes[0].to_base58());

    let addresses = vec![hash_to_bn254_field_size_be(&seed)];
    let new_address_proofs = rpc
        .indexer()
        .unwrap()
        .get_multiple_new_address_proofs(
            test_accounts.v1_address_trees[0].merkle_tree.to_bytes(),
            addresses,
        )
        .await
        .unwrap();
    assert!(!new_address_proofs.is_empty());
    assert_eq!(
        new_address_proofs[0].merkle_tree.to_bytes(),
        test_accounts.v1_address_trees[0].merkle_tree.to_bytes()
    );
}
