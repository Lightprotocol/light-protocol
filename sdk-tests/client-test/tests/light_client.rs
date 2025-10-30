use light_client::{
    indexer::{
        AccountProofInputs, AddressProofInputs, AddressWithTree,
        GetCompressedTokenAccountsByOwnerOrDelegateOptions, Hash, Indexer, IndexerRpcConfig,
        MerkleProof, PaginatedOptions, RetryConfig, RootIndex, TreeInfo, ValidityProofWithContext,
    },
    local_test_validator::{spawn_validator, LightValidatorConfig},
    rpc::{LightClient, LightClientConfig},
};
use light_compressed_account::{hash_to_bn254_field_size_be, TreeType};
use light_compressed_token::mint_sdk::{
    create_create_token_pool_instruction, create_mint_to_instruction,
};
use light_hasher::Poseidon;
use light_merkle_tree_reference::{indexed::IndexedMerkleTree, MerkleTree};
use light_program_test::accounts::test_accounts::TestAccounts;
use light_sdk::{
    address::{v1::derive_address, NewAddressParams},
    token::{AccountState, TokenData},
};
use light_test_utils::{system_program::create_invoke_instruction, Rpc, RpcError};
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_sdk::system_instruction::create_account;
use solana_signature::Signature;
use solana_signer::Signer;
use solana_transaction::Transaction;

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
    let config = LightValidatorConfig {
        enable_indexer: true,
        enable_prover: true,
        wait_time: 10,
        sbf_programs: vec![],
        limit_ledger_size: None,
        grpc_port: None,
    };

    spawn_validator(config).await;

    let test_accounts = TestAccounts::get_local_test_validator_accounts();
    let mut rpc: LightClient = LightClient::new(LightClientConfig::local()).await.unwrap();

    let payer_pubkey = rpc.get_payer().pubkey();
    rpc.airdrop_lamports(&payer_pubkey, 10 * LAMPORTS_PER_SOL)
        .await
        .unwrap();
    let mt = test_accounts.v1_state_trees[0].merkle_tree;
    let _address_mt = test_accounts.v1_address_trees[0].merkle_tree;

    let lamports = LAMPORTS_PER_SOL / 2;
    let lamports_1 = LAMPORTS_PER_SOL / 2 + 1;
    let owner = rpc.get_payer().pubkey();

    // create compressed account with address
    let (address, signature) = create_address(&mut rpc, lamports, owner, mt).await.unwrap();
    let (address_1, signature_1) = create_address(&mut rpc, lamports_1, owner, mt)
        .await
        .unwrap();

    // 1. get_compressed_accounts_by_owner
    let initial_accounts = {
        let accounts = rpc
            .get_compressed_accounts_by_owner(
                &payer_pubkey,
                None,
                Some(IndexerRpcConfig {
                    slot: rpc.client.get_slot().unwrap(),
                    retry_config: RetryConfig::default(),
                }),
            )
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
    let mut reference_tree = MerkleTree::<Poseidon>::new(26, 10);
    for hash in &account_hashes {
        reference_tree.append(hash).unwrap();
    }
    let account_addresses: Vec<Hash> = initial_accounts
        .items
        .iter()
        .map(|a| a.address.unwrap())
        .collect();

    // Create reference address tree and add the addresses
    let _reference_address_tree = IndexedMerkleTree::<Poseidon, usize>::new(26, 10).unwrap();

    // Don't add the test address to the reference tree since we want non-inclusion proof

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

        println!("account_proof {:?}", result);

        // Build expected ValidityProofWithContext using reference tree
        let expected_result = ValidityProofWithContext {
            proof: result.proof, // Keep the actual proof as-is
            accounts: account_hashes
                .iter()
                .enumerate()
                .map(|(i, &hash)| AccountProofInputs {
                    hash,
                    root: reference_tree.root(),
                    root_index: RootIndex::new_some(2),
                    leaf_index: i as u64,
                    tree_info: TreeInfo {
                        cpi_context: None,
                        next_tree_info: None,
                        queue: test_accounts.v1_state_trees[0].nullifier_queue,
                        tree: mt,
                        tree_type: TreeType::StateV1,
                    },
                })
                .collect(),
            addresses: new_addresses
                .iter()
                .enumerate()
                .map(|(i, addr_with_tree)| {
                    // TODO: enable once photon bug is fixed
                    // let address_bigint = BigUint::from_bytes_be(&addr_with_tree.address);
                    // let non_inclusion_proof = reference_address_tree.get_non_inclusion_proof(&address_bigint).unwrap();
                    AddressProofInputs {
                        address: addr_with_tree.address,
                        root: result.addresses[i].root,
                        root_index: 3,
                        tree_info: TreeInfo {
                            cpi_context: None,
                            next_tree_info: None,
                            queue: test_accounts.v1_address_trees[0].queue,
                            tree: addr_with_tree.tree,
                            tree_type: TreeType::AddressV1,
                        },
                    }
                })
                .collect(),
        };

        assert_eq!(result, expected_result);
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
        assert_eq!(balance, lamports);
    }
    // 7. get_compressed_balance_by_owner
    {
        let balance = rpc
            .get_compressed_balance_by_owner(&payer_pubkey, None)
            .await
            .unwrap()
            .value;
        assert_eq!(balance, lamports + lamports_1);
    }
    // 8. get_compression_signatures_for_account
    {
        let signatures = rpc
            .get_compression_signatures_for_account(first_account.hash, None)
            .await
            .unwrap()
            .value;
        assert_eq!(signatures.items[0].signature, signature.to_string());
    }
    // 9. get_compression_signatures_for_address
    {
        let signatures = rpc
            .get_compression_signatures_for_address(&first_account.address.unwrap(), None, None)
            .await
            .unwrap()
            .value;
        assert_eq!(signatures.items[0].signature, signature.to_string());
    }
    // 10. get_compression_signatures_for_owner
    {
        let signatures = rpc
            .get_compression_signatures_for_owner(&owner, None, None)
            .await
            .unwrap()
            .value;
        assert_eq!(signatures.items.len(), 2);
        assert!(signatures
            .items
            .iter()
            .any(|s| s.signature == signature.to_string()));
        assert!(signatures
            .items
            .iter()
            .any(|s| s.signature == signature_1.to_string()));
        let options = PaginatedOptions {
            limit: Some(1),
            cursor: None,
        };
        let signatures = rpc
            .get_compression_signatures_for_owner(&owner, Some(options), None)
            .await
            .unwrap()
            .value;
        assert_eq!(signatures.items.len(), 1);
        assert!(signatures.items.iter().any(
            |s| s.signature == signature_1.to_string() || s.signature == signature.to_string()
        ));
    }

    // 11. Test that non-existent accounts return None
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
    // 12. get_multiple_compressed_account_proofs
    {
        let proofs = rpc
            .get_multiple_compressed_account_proofs(account_hashes.to_vec(), None)
            .await
            .unwrap()
            .value;
        assert!(!proofs.items.is_empty());
        assert_eq!(proofs.items[0].hash, account_hashes[0]);

        // Build expected Vec<MerkleProof> using reference tree
        let expected_proofs: Vec<MerkleProof> = account_hashes
            .iter()
            .enumerate()
            .map(|(i, &hash)| {
                let expected_proof = reference_tree.get_proof_of_leaf(i, false).unwrap();
                MerkleProof {
                    hash,
                    leaf_index: i as u64,
                    merkle_tree: mt,
                    proof: expected_proof,
                    root_seq: 2,
                    root: reference_tree.root(),
                }
            })
            .collect();

        assert_eq!(proofs.items, expected_proofs);

        // 12. get_multiple_new_address_proofs
        let addresses = vec![address];
        let new_address_proofs = rpc
            .get_multiple_new_address_proofs(
                test_accounts.v1_address_trees[0].merkle_tree.to_bytes(),
                addresses.clone(),
                None,
            )
            .await
            .unwrap();
        assert!(!new_address_proofs.value.items.is_empty());
        // TODO: update once photon is ready
        // Build expected Vec<NewAddressProofWithContext> using reference address tree
        // let expected_address_proofs: Vec<NewAddressProofWithContext> = addresses
        //     .iter()
        //     .map(|&addr| {
        //         let address_bigint = BigUint::from_bytes_be(&addr);
        //         let non_inclusion_proof = reference_address_tree
        //             .get_non_inclusion_proof(&address_bigint)
        //             .unwrap();

        //         NewAddressProofWithContext {
        //             merkle_tree: address_mt,
        //             root: non_inclusion_proof.root,
        //             root_seq: 3,
        //             low_address_index: non_inclusion_proof.leaf_index as u64,
        //             low_address_value: non_inclusion_proof.leaf_lower_range_value,
        //             low_address_next_index: non_inclusion_proof.next_index as u64,
        //             low_address_next_value: non_inclusion_proof.leaf_higher_range_value,
        //             low_address_proof: non_inclusion_proof.merkle_proof,
        //             new_low_element: None,
        //             new_element: None,
        //             new_element_next_value: None,
        //         }
        //     })
        //     .collect();
        assert_eq!(new_address_proofs.value.items.len(), 1);
    }

    test_token_api(&rpc, &test_accounts).await;
}

/// Token API endpoints tested:
/// 1. get_compressed_token_accounts_by_owner
/// 2. get_compressed_token_account_balance
/// 3. get_compressed_token_balances_by_owner_v2
/// 4. get_compressed_mint_token_holders
/// 5. get_compression_signatures_for_token_owner
async fn test_token_api(rpc: &LightClient, test_accounts: &TestAccounts) {
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();
    let mint_1 = Keypair::new();
    let mint_2 = Keypair::new();

    create_two_mints(rpc, payer_pubkey, &mint_1, &mint_2);
    let mint_1 = mint_1.pubkey();
    let mint_2 = mint_2.pubkey();
    let base_amount = 1_000_000;
    let recipients = (0..5)
        .map(|_| Pubkey::new_unique())
        .collect::<Vec<Pubkey>>();
    let amounts = (0..5).map(|i| base_amount + i).collect::<Vec<u64>>();
    // Mint amounts to payer for both mints with and without lamports
    let signatures = mint_to_token_accounts(
        rpc,
        test_accounts,
        payer_pubkey,
        mint_1,
        mint_2,
        base_amount,
        &recipients,
        &amounts,
    );
    let slot = rpc.get_slot().await.unwrap();
    let config = IndexerRpcConfig {
        slot,
        retry_config: RetryConfig::default(),
    };
    // 1. get_compressed_mint_token_holders
    for mint in [mint_1, mint_2] {
        let res = rpc
            .get_compressed_mint_token_holders(&mint, None, Some(config.clone()))
            .await
            .unwrap()
            .value
            .items;
        assert_eq!(res.len(), 5);

        let mut owners = res.iter().map(|x| x.owner).collect::<Vec<_>>();
        owners.sort();
        owners.dedup();
        assert_eq!(owners.len(), 5);
        for (amount, recipient) in amounts.iter().zip(recipients.iter()) {
            // * 2 because we mint two times the same amount per token mint (with and without lamports)
            assert!(res
                .iter()
                .any(|item| item.balance == (*amount * 2) && item.owner == *recipient));
        }
        let option = PaginatedOptions {
            limit: Some(1),
            cursor: None,
        };
        let res = rpc
            .get_compressed_mint_token_holders(&mint, Some(option), None)
            .await
            .unwrap()
            .value
            .items;
        assert_eq!(res.len(), 1);
    }

    // 2. get_compression_signatures_for_token_owner
    for recipient in &recipients {
        let res = rpc
            .get_compression_signatures_for_token_owner(recipient, None, None)
            .await
            .unwrap()
            .value
            .items;
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].signature, signatures[1].to_string());
        assert_eq!(res[1].signature, signatures[0].to_string());
        let option = PaginatedOptions {
            limit: Some(1),
            cursor: None,
        };
        let res = rpc
            .get_compression_signatures_for_token_owner(recipient, Some(option), None)
            .await
            .unwrap()
            .value
            .items;
        assert_eq!(res.len(), 1);
    }

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
fn mint_to_token_accounts(
    rpc: &LightClient,
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

        let tx = Transaction::new_signed_with_payer(
            &[
                compute_budget_ix,
                mint_ix_with_lamports,
                mint_ix_no_lamports,
            ],
            Some(&payer_pubkey),
            &[&rpc.get_payer()],
            rpc.client.get_latest_blockhash().unwrap(),
        );
        signatures.push(rpc.client.send_and_confirm_transaction(&tx).unwrap());
    }
    signatures.try_into().unwrap()
}

fn create_two_mints(rpc: &LightClient, payer_pubkey: Pubkey, mint_1: &Keypair, mint_2: &Keypair) {
    let mint_rent = rpc
        .client
        .get_minimum_balance_for_rent_exemption(82)
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
        &[rpc.get_payer(), mint_1, mint_2],
        rpc.client.get_latest_blockhash().unwrap(),
    );
    rpc.client.send_and_confirm_transaction(&tx).unwrap();
}

/// Tests:
/// 1. fetch all no options
/// 2. fetch only for mint 1, with limit 1
async fn test_get_compressed_token_accounts_by_owner(
    rpc: &LightClient,
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
    rpc: &mut LightClient,
    lamports: u64,
    owner: Pubkey,
    merkle_tree: Pubkey,
) -> Result<([u8; 32], Signature), RpcError> {
    #[cfg(feature = "v2")]
    let address_merkle_tree = rpc.get_address_tree_v2();
    #[cfg(not(feature = "v2"))]
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

    let tx_create_compressed_account = Transaction::new_signed_with_payer(
        &[compute_budget_ix, ix],
        Some(&rpc.get_payer().pubkey()),
        &[&rpc.get_payer()],
        rpc.client.get_latest_blockhash().unwrap(),
    );
    let signature = rpc
        .client
        .send_and_confirm_transaction(&tx_create_compressed_account)?;
    Ok((address, signature))
}

async fn test_get_compressed_token_balances_by_owner_v2(
    rpc: &LightClient,
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
