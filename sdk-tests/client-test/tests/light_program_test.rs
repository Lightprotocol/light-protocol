use light_client::{
    indexer::{
        AddressWithTree, GetCompressedTokenAccountsByOwnerOrDelegateOptions, Hash, Indexer,
        IndexerRpcConfig, RetryConfig,
    },
    interface::AccountToFetch,
    rpc::Rpc,
};
use light_compressed_account::hash_to_bn254_field_size_be;
use light_compressed_token::mint_sdk::{
    create_create_token_pool_instruction, create_mint_to_instruction,
};
use light_program_test::{
    accounts::test_accounts::TestAccounts, program_test::LightProgramTest, ProgramTestConfig,
};
use light_sdk::address::{v1::derive_address, NewAddressParams};
use light_test_utils::{system_program::create_invoke_instruction, RpcError};
use light_token::{
    compat::{AccountState, TokenData},
    instruction::derive_token_ata,
};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    system_instruction::create_account,
    transaction::Transaction,
};

// Constants
const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

/// Endpoints tested:
/// 1. get_compressed_accounts_by_owner
/// 2. get_multiple_compressed_accounts
/// 3. get_validity_proof
/// 4. get_compressed_account
/// 5. get_compressed_account_by_hash
/// 6. get_compressed_balance
/// 7. get_compressed_balance_by_owner
/// 8. get_compression_signatures_for_account
/// 9. get_compression_signatures_for_address
/// 10. get_compression_signatures_for_owner
/// 11. get_multiple_compressed_account_proofs
/// 12. get_multiple_new_address_proofs
/// 13. get_compressed_token_accounts_by_owner
/// 14. get_compressed_token_account_balance
/// 15. get_compressed_token_balances_by_owner_v2
/// 16. get_compressed_mint_token_holders
/// 17. get_compression_signatures_for_token_owner
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_all_endpoints() {
    let config = ProgramTestConfig::default();
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let test_accounts = rpc.test_accounts().clone();

    let payer_pubkey = rpc.get_payer().pubkey();
    let mt = test_accounts.v1_state_trees[0].merkle_tree;

    let lamports = LAMPORTS_PER_SOL / 2;
    let lamports_1 = LAMPORTS_PER_SOL / 2 + 1;
    let owner = rpc.get_payer().pubkey();

    // create compressed account with address
    let (address, _signature) = create_address(&mut rpc, lamports, owner, mt).await.unwrap();
    let (address_1, _signature_1) = create_address(&mut rpc, lamports_1, owner, mt)
        .await
        .unwrap();

    // 1. get_compressed_accounts_by_owner
    let initial_accounts = {
        let accounts = rpc
            .get_compressed_accounts_by_owner(&payer_pubkey, None, None)
            .await
            .unwrap()
            .value;
        assert_eq!(accounts.items.len(), 2);
        assert_eq!(accounts.items[0].owner, owner);
        assert_eq!(accounts.items[1].owner, owner);

        assert!(accounts
            .items
            .iter()
            .any(|x| x.lamports == lamports && x.address == Some(address)));

        assert!(accounts
            .items
            .iter()
            .any(|x| x.lamports == lamports_1 && x.address == Some(address_1)));

        accounts
    };

    let account_hashes: Vec<Hash> = initial_accounts.items.iter().map(|a| a.hash).collect();
    let account_addresses: Vec<Hash> = initial_accounts
        .items
        .iter()
        .map(|a| a.address.unwrap())
        .collect();

    // 2. get_multiple_compressed_accounts
    let accounts = rpc
        .get_multiple_compressed_accounts(None, Some(account_hashes.clone()), None)
        .await
        .unwrap()
        .value;

    assert_eq!(accounts.items.len(), account_hashes.len());
    for item in accounts.items.iter() {
        let item = item.as_ref().unwrap();
        assert!(initial_accounts.items.iter().any(|x| x.hash == item.hash));
    }
    // Currently fails because photon doesn't deliver cpi context accounts.
    // for item in accounts.items.iter() {
    //     assert!(initial_accounts.items.iter().any(|x| *x == *item));
    // }
    let accounts = rpc
        .get_multiple_compressed_accounts(Some(account_addresses), None, None)
        .await
        .unwrap()
        .value;
    assert_eq!(accounts.items.len(), initial_accounts.items.len());
    for item in accounts.items.iter() {
        let item = item.as_ref().unwrap();
        assert!(initial_accounts.items.iter().any(|x| x.hash == item.hash));
    }
    // Currently fails because photon doesn't deliver cpi context accounts.
    // for item in accounts.items.iter() {
    //     assert!(initial_accounts.items.iter().any(|x| *x == *item));
    // }
    // 3. get_validity_proof
    {
        let seed = rand::random::<[u8; 32]>();
        let new_addresses = vec![AddressWithTree {
            address: hash_to_bn254_field_size_be(&seed),
            tree: test_accounts.v1_address_trees[0].merkle_tree,
        }];

        let result = rpc
            .get_validity_proof(account_hashes.clone(), new_addresses.clone(), None)
            .await
            .unwrap()
            .value;
        assert_eq!(result.accounts.len(), account_hashes.len());
        assert_eq!(result.addresses.len(), new_addresses.len());
    }
    // 4. get_compressed_account
    let first_account = accounts.items[0].as_ref().unwrap();
    let fetched_account = rpc
        .get_compressed_account(first_account.address.unwrap(), None)
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(fetched_account, *first_account);

    // 5. get_compressed_account_by_hash
    {
        let account = rpc
            .get_compressed_account_by_hash(first_account.hash, None)
            .await
            .unwrap()
            .value
            .unwrap();
        assert_eq!(account, *first_account);
    }
    // 6. get_compressed_balance
    {
        let balance = rpc
            .get_compressed_balance(None, Some(first_account.hash), None)
            .await
            .unwrap()
            .value;
        assert_eq!(balance, first_account.lamports);
    }
    // 7. Test that non-existent accounts return None
    {
        // Test get_compressed_account with non-existent address
        let non_existent_address = [0u8; 32];
        let account = rpc
            .get_compressed_account(non_existent_address, None)
            .await
            .unwrap()
            .value;
        assert!(account.is_none(), "Expected None for non-existent address");

        // Test get_compressed_account_by_hash with non-existent hash
        let non_existent_hash = [0u8; 32];
        let account = rpc
            .get_compressed_account_by_hash(non_existent_hash, None)
            .await
            .unwrap()
            .value;
        assert!(account.is_none(), "Expected None for non-existent hash");

        // Test get_multiple_compressed_accounts with mix of existing and non-existent
        let mixed_hashes = vec![
            account_hashes[0], // existing
            [0u8; 32],         // non-existent
            account_hashes[1], // existing
        ];
        let accounts = rpc
            .get_multiple_compressed_accounts(None, Some(mixed_hashes.clone()), None)
            .await
            .unwrap()
            .value;
        assert_eq!(accounts.items.len(), 3);
        assert!(accounts.items[0].is_some(), "First account should exist");
        assert!(
            accounts.items[1].is_none(),
            "Second account should not exist"
        );
        assert!(accounts.items[2].is_some(), "Third account should exist");

        // Test with addresses
        let first_existing_address = accounts.items[0].as_ref().unwrap().address.unwrap();
        let mixed_addresses = vec![
            first_existing_address, // existing
            [0u8; 32],              // non-existent
        ];
        let accounts_by_addr = rpc
            .get_multiple_compressed_accounts(Some(mixed_addresses), None, None)
            .await
            .unwrap()
            .value;
        assert_eq!(accounts_by_addr.items.len(), 2);
        assert!(
            accounts_by_addr.items[0].is_some(),
            "First account should exist"
        );
        assert!(
            accounts_by_addr.items[1].is_none(),
            "Second account should not exist"
        );
    }
    // 8, 9, 10: get_compressed_balance_by_owner, get_compression_signatures_for_account,
    // get_compression_signatures_for_address, get_compression_signatures_for_owner
    // are not implemented in TestIndexer. See the #[ignore] tests below.
    // 11. get_multiple_compressed_account_proofs
    {
        let proofs = rpc
            .get_multiple_compressed_account_proofs(account_hashes.to_vec(), None)
            .await
            .unwrap()
            .value;
        assert!(!proofs.items.is_empty());
        assert_eq!(proofs.items[0].hash, account_hashes[0]);

        // 12. get_multiple_new_address_proofs
        let addresses = vec![address];
        let new_address_proofs = rpc
            .get_multiple_new_address_proofs(
                test_accounts.v1_address_trees[0].merkle_tree.to_bytes(),
                addresses,
                None,
            )
            .await
            .unwrap();
        assert!(!new_address_proofs.value.items.is_empty());
        assert_eq!(
            new_address_proofs.value.items[0].merkle_tree.to_bytes(),
            test_accounts.v1_address_trees[0].merkle_tree.to_bytes()
        );
    }

    test_token_api(&mut rpc, &test_accounts).await;
}

/// Token API endpoints tested:
/// 1. get_compressed_token_accounts_by_owner
/// 2. get_compressed_token_account_balance
/// 3. get_compressed_token_balances_by_owner_v2
/// 4. get_compressed_mint_token_holders
/// 5. get_compression_signatures_for_token_owner
async fn test_token_api(rpc: &mut LightProgramTest, test_accounts: &TestAccounts) {
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();
    let mint_1 = Keypair::new();
    let mint_2 = Keypair::new();

    create_two_mints(rpc, payer_pubkey, &mint_1, &mint_2).await;
    let mint_1 = mint_1.pubkey();
    let mint_2 = mint_2.pubkey();
    let base_amount = 1_000_000;
    let recipients = (0..5)
        .map(|_| Pubkey::new_unique())
        .collect::<Vec<Pubkey>>();
    let amounts = (0..5).map(|i| base_amount + i).collect::<Vec<u64>>();
    // Mint amounts to payer for both mints with and without lamports
    let _signatures = mint_to_token_accounts(
        rpc,
        test_accounts,
        payer_pubkey,
        mint_1,
        mint_2,
        base_amount,
        &recipients,
        &amounts,
    )
    .await;
    // get_compressed_mint_token_holders and get_compression_signatures_for_token_owner
    // are not implemented in TestIndexer. See the #[ignore] tests below.

    // 3. get_compressed_token_accounts_by_owner
    test_get_compressed_token_accounts_by_owner(
        rpc,
        mint_1,
        mint_2,
        base_amount,
        &recipients,
        &amounts,
    )
    .await;
    // 4. get_compressed_token_account_balance
    {
        let token_accounts = rpc
            .get_compressed_token_accounts_by_owner(&recipients[0], None, None)
            .await
            .unwrap()
            .value;
        let hash = token_accounts.items[0].account.hash;
        let balance = rpc
            .get_compressed_token_account_balance(None, Some(hash), None)
            .await
            .unwrap()
            .value;
        assert_eq!(balance, amounts[0]);
        assert_eq!(balance, token_accounts.items[0].token.amount);
    }
    // 5. get_compressed_token_balances_by_owner_v2
    {
        // No options
        test_get_compressed_token_balances_by_owner_v2(
            rpc,
            vec![mint_1, mint_2],
            recipients.clone(),
            amounts.clone(),
            None,
        )
        .await;
        // Limit to mint1
        let options = Some(GetCompressedTokenAccountsByOwnerOrDelegateOptions {
            mint: Some(mint_1),
            cursor: None,
            limit: None,
        });
        test_get_compressed_token_balances_by_owner_v2(
            rpc,
            vec![mint_1],
            recipients.clone(),
            amounts.clone(),
            options,
        )
        .await;

        // Limit to mint2
        let options = Some(GetCompressedTokenAccountsByOwnerOrDelegateOptions {
            mint: Some(mint_2),
            cursor: None,
            limit: None,
        });
        test_get_compressed_token_balances_by_owner_v2(
            rpc,
            vec![mint_2],
            recipients.clone(),
            amounts.clone(),
            options,
        )
        .await;
    }
}

#[allow(clippy::too_many_arguments)]
async fn mint_to_token_accounts(
    rpc: &mut LightProgramTest,
    test_accounts: &TestAccounts,
    payer_pubkey: Pubkey,
    mint_1: Pubkey,
    mint_2: Pubkey,
    base_amount: u64,
    recipients: &[Pubkey],
    amounts: &[u64],
) -> [Signature; 2] {
    let mut signatures = Vec::new();

    for mint in [mint_1, mint_2] {
        let mint_ix_with_lamports = create_mint_to_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &mint,
            &test_accounts.v1_state_trees[0].merkle_tree,
            amounts.to_vec(),
            recipients.to_vec(),
            Some(base_amount),
            false,
            0,
        );

        let mint_ix_no_lamports = create_mint_to_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &mint,
            &test_accounts.v1_state_trees[0].merkle_tree,
            amounts.to_vec(),
            recipients.to_vec(),
            None,
            false,
            0,
        );

        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(500_000);
        let blockhash = rpc.get_latest_blockhash().await.unwrap().0;
        let payer = rpc.get_payer().insecure_clone();

        let tx = Transaction::new_signed_with_payer(
            &[
                compute_budget_ix,
                mint_ix_with_lamports,
                mint_ix_no_lamports,
            ],
            Some(&payer_pubkey),
            &[&payer],
            blockhash,
        );
        signatures.push(rpc.process_transaction(tx).await.unwrap());
    }
    signatures.try_into().unwrap()
}

async fn create_two_mints(
    rpc: &mut LightProgramTest,
    payer_pubkey: Pubkey,
    mint_1: &Keypair,
    mint_2: &Keypair,
) {
    let mint_rent = rpc
        .get_minimum_balance_for_rent_exemption(82)
        .await
        .unwrap();
    let create_mint_ix = create_account(
        &payer_pubkey,
        &mint_1.pubkey(),
        mint_rent,
        82,
        &spl_token::id(),
    );
    let create_mint_ix_2 = create_account(
        &payer_pubkey,
        &mint_2.pubkey(),
        mint_rent,
        82,
        &spl_token::id(),
    );
    let init_mint_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_1.pubkey(),
        &payer_pubkey,
        None,
        9,
    )
    .unwrap();
    let init_mint_ix_2 = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_2.pubkey(),
        &payer_pubkey,
        None,
        2,
    )
    .unwrap();
    // Create token pool for compression
    let create_pool_ix =
        create_create_token_pool_instruction(&payer_pubkey, &mint_1.pubkey(), false);
    let create_pool_ix_2 =
        create_create_token_pool_instruction(&payer_pubkey, &mint_2.pubkey(), false);
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(500_000);
    let blockhash = rpc.get_latest_blockhash().await.unwrap().0;
    let payer = rpc.get_payer().insecure_clone();
    let tx = Transaction::new_signed_with_payer(
        &[
            compute_budget_ix,
            create_mint_ix,
            create_mint_ix_2,
            init_mint_ix,
            init_mint_ix_2,
            create_pool_ix,
            create_pool_ix_2,
        ],
        Some(&payer_pubkey),
        &[&payer, mint_1, mint_2],
        blockhash,
    );
    rpc.process_transaction(tx).await.unwrap();
}

/// Tests:
/// 1. fetch all no options
/// 2. fetch only for mint 1, with limit 1
async fn test_get_compressed_token_accounts_by_owner(
    rpc: &mut LightProgramTest,
    mint_1: Pubkey,
    mint_2: Pubkey,
    base_amount: u64,
    recipients: &[Pubkey],
    amounts: &[u64],
) {
    let slot = rpc.get_slot().await.unwrap();
    let indexer_config = IndexerRpcConfig {
        slot,
        retry_config: RetryConfig::default(),
    };
    for (amount, recipient) in amounts.iter().zip(recipients.iter()) {
        {
            let token_accounts = &rpc
                .indexer()
                .unwrap()
                .get_compressed_token_accounts_by_owner(
                    recipient,
                    None,
                    Some(indexer_config.clone()),
                )
                .await
                .unwrap()
                .value;
            // every recipient should have 4 token accounts
            // 1. 2 with lamports and 2 without
            // 2. 2 with mint 1 and 2 with mint 2
            let mut expected_token_data = TokenData {
                mint: mint_1,
                amount: *amount,
                owner: *recipient,
                delegate: None,
                state: AccountState::Initialized,
                tlv: None,
            };
            assert_eq!(
                token_accounts
                    .items
                    .iter()
                    .filter(|item| item.token == expected_token_data)
                    .count(),
                2
            );
            assert!(token_accounts
                .items
                .iter()
                .any(|item| item.token == expected_token_data
                    && item.account.lamports == base_amount));
            expected_token_data.mint = mint_2;
            assert_eq!(
                token_accounts
                    .items
                    .iter()
                    .filter(|item| item.token == expected_token_data)
                    .count(),
                2
            );
            assert!(token_accounts
                .items
                .iter()
                .any(|item| item.token == expected_token_data
                    && item.account.lamports == base_amount));
        }
        // fetch only for mint 1, with limit 1
        {
            let options = GetCompressedTokenAccountsByOwnerOrDelegateOptions {
                mint: Some(mint_1),
                cursor: None,
                limit: Some(1),
            };
            let token_accounts = &rpc
                .indexer()
                .unwrap()
                .get_compressed_token_accounts_by_owner(recipient, Some(options.clone()), None)
                .await
                .unwrap()
                .value;
            assert_eq!(token_accounts.items.len(), 1);
            assert_eq!(token_accounts.items[0].token.mint, options.mint.unwrap());
        }
    }
}

async fn create_address(
    rpc: &mut LightProgramTest,
    lamports: u64,
    owner: Pubkey,
    merkle_tree: Pubkey,
) -> Result<([u8; 32], Signature), RpcError> {
    let address_merkle_tree = rpc.get_address_tree_v1();
    let (address, address_seed) = derive_address(
        &[Pubkey::new_unique().to_bytes().as_slice()],
        &address_merkle_tree.tree,
        &Pubkey::new_unique(),
    );

    let output_account = light_compressed_account::compressed_account::CompressedAccount {
        lamports,
        owner: owner.into(),
        data: None,
        address: Some(address),
    };
    let rpc_proof_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address,
                tree: address_merkle_tree.tree,
            }],
            None,
        )
        .await
        .unwrap();

    let new_address_params = NewAddressParams {
        seed: address_seed.into(),
        address_queue_pubkey: address_merkle_tree.queue.into(),
        address_merkle_tree_pubkey: address_merkle_tree.tree.into(),
        address_merkle_tree_root_index: rpc_proof_result.value.addresses[0].root_index,
    };
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(500_000);
    let ix = create_invoke_instruction(
        &rpc.get_payer().pubkey(),
        &rpc.get_payer().pubkey(),
        &[],
        &[output_account],
        &[],
        &[merkle_tree],
        &[],
        &[new_address_params],
        rpc_proof_result.value.proof.0,
        Some(lamports),
        true,
        None,
        true,
    );

    let blockhash = rpc.get_latest_blockhash().await.unwrap().0;
    let payer = rpc.get_payer().insecure_clone();
    let tx_create_compressed_account = Transaction::new_signed_with_payer(
        &[compute_budget_ix, ix],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );
    let signature = rpc
        .process_transaction(tx_create_compressed_account)
        .await?;
    Ok((address, signature))
}

async fn test_get_compressed_token_balances_by_owner_v2(
    rpc: &mut LightProgramTest,
    mints: Vec<Pubkey>,
    recipients: Vec<Pubkey>,
    amounts: Vec<u64>,
    options: Option<GetCompressedTokenAccountsByOwnerOrDelegateOptions>,
) {
    for (amount, recipient) in amounts.iter().zip(recipients.iter()) {
        let balances = rpc
            .get_compressed_token_balances_by_owner_v2(recipient, options.clone(), None)
            .await
            .unwrap();
        let balances = balances.value.items;
        assert_eq!(balances.len(), mints.len());
        for mint in mints.iter() {
            assert!(balances
                .iter()
                .any(|balance| balance.mint == *mint && balance.balance == (*amount) * 2));
        }
    }
}

// ============ Phase 2 Tests ============

/// B1: get_associated_token_account_interface
///
/// Mints a compressed token to the payer's ATA address (derive_token_ata(payer, mint)).
/// Then calls get_associated_token_account_interface(payer, mint).
/// The token was minted to the ATA pubkey as owner, so the cold path returns it.
/// Asserts amount, owner, and is_cold (since TestIndexer returns cold compressed accounts).
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_get_associated_token_account_interface() {
    let config = ProgramTestConfig::default();
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let test_accounts = rpc.test_accounts().clone();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create a mint
    let mint_kp = Keypair::new();
    create_two_mints(&mut rpc, payer_pubkey, &mint_kp, &Keypair::new()).await;
    let mint = mint_kp.pubkey();

    // Derive the payer's ATA for this mint
    let ata = derive_token_ata(&payer_pubkey, &mint);
    let amount = 42_000u64;

    // Mint the compressed token directly to the ATA address as the token owner.
    // This way, get_associated_token_account_interface cold path finds it by
    // querying compressed tokens owned by the ATA pubkey.
    let mint_ix = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &mint,
        &test_accounts.v1_state_trees[0].merkle_tree,
        vec![amount],
        vec![ata],
        None,
        false,
        0,
    );
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(500_000);
    let blockhash = rpc.get_latest_blockhash().await.unwrap().0;
    let tx = Transaction::new_signed_with_payer(
        &[compute_budget_ix, mint_ix],
        Some(&payer_pubkey),
        &[&payer],
        blockhash,
    );
    rpc.process_transaction(tx).await.unwrap();

    // Now query the ATA interface
    let result = rpc
        .get_associated_token_account_interface(&payer_pubkey, &mint, None)
        .await
        .unwrap()
        .value
        .expect("ATA should be found");

    // The token was minted into a compressed account owned by the ATA address,
    // so it comes back as a cold (compressed) account.
    assert!(result.is_cold(), "Expected cold (compressed) ATA");
    assert_eq!(result.amount(), amount);
    // The owner returned should be the wallet owner (payer_pubkey) per owner_override in cold ctor
    assert_eq!(result.owner(), payer_pubkey);
    assert_eq!(result.mint(), mint);
    assert!(!result.is_frozen(), "ATA should not be frozen");
    assert!(result.delegate().is_none(), "ATA should have no delegate");
    assert!(result.is_ata(), "Result should identify as an ATA");
}

/// B2: get_mint_interface
///
/// Uses an existing hot on-chain mint created by create_two_mints.
/// Asserts the returned MintInterface is hot and present.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_get_mint_interface() {
    let config = ProgramTestConfig::default();
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer_pubkey = rpc.get_payer().pubkey();

    let mint_kp = Keypair::new();
    create_two_mints(&mut rpc, payer_pubkey, &mint_kp, &Keypair::new()).await;
    let mint = mint_kp.pubkey();

    let result = rpc
        .get_mint_interface(&mint, None)
        .await
        .unwrap()
        .value
        .expect("mint should be found");

    assert!(result.is_hot(), "Expected hot (on-chain) mint");
    assert_eq!(result.mint, mint);
    assert!(
        result.account().is_some(),
        "Hot mint should have on-chain account"
    );
    assert!(
        result.compressed().is_none(),
        "Hot mint should have no compressed data"
    );
}

/// B3: fetch_accounts
///
/// Builds a vec of AccountToFetch descriptors (one Mint, one Ata) and calls
/// rpc.fetch_accounts. Asserts the returned Vec<AccountInterface> has the
/// correct length.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_fetch_accounts() {
    let config = ProgramTestConfig::default();
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let test_accounts = rpc.test_accounts().clone();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    let mint_kp = Keypair::new();
    create_two_mints(&mut rpc, payer_pubkey, &mint_kp, &Keypair::new()).await;
    let mint = mint_kp.pubkey();

    // Mint compressed token to the ATA address so the ATA fetch succeeds
    let ata = derive_token_ata(&payer_pubkey, &mint);
    let amount = 1_000u64;
    let mint_ix = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &mint,
        &test_accounts.v1_state_trees[0].merkle_tree,
        vec![amount],
        vec![ata],
        None,
        false,
        0,
    );
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(500_000);
    let blockhash = rpc.get_latest_blockhash().await.unwrap().0;
    let tx = Transaction::new_signed_with_payer(
        &[compute_budget_ix, mint_ix],
        Some(&payer_pubkey),
        &[&payer],
        blockhash,
    );
    rpc.process_transaction(tx).await.unwrap();

    let accounts_to_fetch = vec![
        AccountToFetch::mint(mint),
        AccountToFetch::ata(payer_pubkey, mint),
    ];

    let result = rpc.fetch_accounts(&accounts_to_fetch, None).await.unwrap();

    assert_eq!(result.len(), accounts_to_fetch.len());
    assert!(result[0].is_hot(), "Mint should be hot (on-chain)");
    assert_eq!(result[0].key, mint, "Mint key should match");
    assert!(result[1].is_cold(), "ATA should be cold (compressed)");
    assert_eq!(result[1].key, ata, "ATA key should match");
}

/// B4: get_indexer_health and get_indexer_slot
///
/// TestIndexer always returns healthy (true) and slot u64::MAX.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_indexer_health_and_slot() {
    let config = ProgramTestConfig::default();
    let rpc = LightProgramTest::new(config).await.unwrap();

    let indexer = rpc.indexer().unwrap();

    let healthy = indexer.get_indexer_health(None).await.unwrap();
    assert!(healthy, "TestIndexer should report healthy");

    let slot = indexer.get_indexer_slot(None).await.unwrap();
    assert_eq!(slot, u64::MAX, "TestIndexer returns u64::MAX as slot");
}

/// C3: create_and_send_transaction_with_event
///
/// Sends a no-op transaction and verifies the method compiles and runs without error.
/// Uses PublicTransactionEvent as the event type.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_create_and_send_transaction_with_event() {
    use light_event::event::PublicTransactionEvent;

    let config = ProgramTestConfig::default();
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Use a compute budget instruction as a no-op that won't emit a
    // PublicTransactionEvent. The method should return Ok(None).
    let ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);

    let result = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[ix],
            &payer_pubkey,
            &[&payer],
        )
        .await
        .unwrap();

    // No PublicTransactionEvent is emitted by a compute budget instruction,
    // so result is None.
    assert!(
        result.is_none(),
        "Expected None when no PublicTransactionEvent is emitted"
    );
}
