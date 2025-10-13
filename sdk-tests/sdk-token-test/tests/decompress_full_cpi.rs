//#![cfg(feature = "test-sbf")]

use anchor_lang::{AnchorDeserialize, InstructionData};
/// Test input range for multi-input tests
const TEST_INPUT_RANGE: [usize; 4] = [1, 2, 3, 4];

use light_compressed_token_sdk::instructions::{
    decompress_full::DecompressFullAccounts, find_spl_mint_address, MintToRecipient,
};
use light_ctoken_types::instructions::mint_action::{CompressedMintWithContext, Recipient};
use light_program_test::{Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_sdk::instruction::PackedAccounts;
use light_test_utils::airdrop_lamports;
use light_token_client::{actions::mint_action_comprehensive, instructions::mint_action::NewMint};
use sdk_token_test::mint_compressed_tokens_cpi_write::MintCompressedTokensCpiWriteParams;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};

/// Test context containing all the common test data
struct TestContext {
    payer: Keypair,
    owner: Keypair,
    mint_seed: Keypair,
    mint_pubkey: Pubkey,
    destination_accounts: Vec<Pubkey>,
    compressed_amount_per_account: u64,
    total_compressed_amount: u64,
}

/// Setup function for decompress_full tests
/// Creates compressed tokens (source) and empty decompressed accounts (destination)
async fn setup_decompress_full_test(num_inputs: usize) -> (LightProgramTest, TestContext) {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_token_test", sdk_token_test::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    let mint_seed = Keypair::new();
    let mint_pubkey = find_spl_mint_address(&mint_seed.pubkey()).0;
    let mint_authority = payer.pubkey();
    let decimals = 9u8;

    let owner = Keypair::new();
    airdrop_lamports(&mut rpc, &owner.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    use light_compressed_token_sdk::instructions::{
        create_compressible_associated_token_account, derive_ctoken_ata,
        CreateCompressibleAssociatedTokenAccountInputs,
    };

    let mut destination_accounts = Vec::with_capacity(num_inputs);

    for i in 0..num_inputs {
        let destination_owner = if i == 0 {
            owner.pubkey()
        } else {
            let additional_owner = Keypair::new();
            airdrop_lamports(&mut rpc, &additional_owner.pubkey(), 10_000_000_000)
                .await
                .unwrap();
            additional_owner.pubkey()
        };

        let (destination_account, _) = derive_ctoken_ata(&destination_owner, &mint_pubkey);

        let create_token_account_ix = create_compressible_associated_token_account(
            CreateCompressibleAssociatedTokenAccountInputs {
                payer: payer.pubkey(),
                mint: mint_pubkey,
                owner: destination_owner,
                rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
                pre_pay_num_epochs: 0,
                lamports_per_write: None,
                compressible_config: rpc
                    .test_accounts
                    .funding_pool_config
                    .compressible_config_pda,
                token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            },
        )
        .unwrap();

        rpc.create_and_send_transaction(&[create_token_account_ix], &payer.pubkey(), &[&payer])
            .await
            .unwrap();

        destination_accounts.push(destination_account);
    }

    let total_compressed_amount = 1000;
    let compressed_amount_per_account = total_compressed_amount / num_inputs as u64;

    let compressed_recipients: Vec<Recipient> = (0..num_inputs)
        .map(|_| Recipient {
            recipient: owner.pubkey().into(),
            amount: compressed_amount_per_account,
        })
        .collect();

    println!(
        "Minting {} tokens to {} compressed accounts ({} per account) for owner",
        total_compressed_amount, num_inputs, compressed_amount_per_account
    );

    mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &payer,
        &payer,
        compressed_recipients,
        Vec::new(),
        None,
        None,
        Some(NewMint {
            decimals,
            mint_authority,
            supply: 0,
            freeze_authority: None,
            metadata: None,
            version: 3,
        }),
    )
    .await
    .unwrap();

    (
        rpc,
        TestContext {
            payer,
            owner,
            mint_seed,
            mint_pubkey,
            destination_accounts,
            compressed_amount_per_account,
            total_compressed_amount,
        },
    )
}

/// Test the decompress_full_cpi instruction
/// This test verifies that DecompressFull mode works correctly through CPI
/// Moving tokens from compressed state to decompressed ctoken account
#[tokio::test]
async fn test_decompress_full_cpi() {
    for num_inputs in TEST_INPUT_RANGE {
        println!("Testing decompress_full_cpi with {} inputs", num_inputs);
        let (mut rpc, ctx) = setup_decompress_full_test(num_inputs).await;
        let payer_pubkey = ctx.payer.pubkey();

        let compressed_accounts = rpc
            .get_compressed_token_accounts_by_owner(&ctx.owner.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items;

        assert_eq!(
            compressed_accounts.len(),
            num_inputs,
            "Should have {} compressed accounts",
            num_inputs
        );

        for compressed_account in &compressed_accounts {
            assert_eq!(
                compressed_account.token.amount,
                ctx.compressed_amount_per_account
            );
            assert_eq!(compressed_account.token.mint, ctx.mint_pubkey);
        }

        for destination_account in &ctx.destination_accounts {
            let dest_account = rpc
                .get_account(*destination_account)
                .await
                .unwrap()
                .unwrap();
            use light_ctoken_types::state::CToken;
            use light_zero_copy::traits::ZeroCopyAt;
            let (dest_token, _) = CToken::zero_copy_at(&dest_account.data).unwrap();
            assert_eq!(
                *dest_token.amount, 0,
                "Destination should be empty initially"
            );
        }

        let mut remaining_accounts = PackedAccounts::default();
        let compressed_hashes: Vec<_> = compressed_accounts
            .iter()
            .map(|acc| acc.account.hash)
            .collect();
        let rpc_result = rpc
            .get_validity_proof(compressed_hashes, vec![], None)
            .await
            .unwrap()
            .value;

        let packed_tree_info = rpc_result.pack_tree_infos(&mut remaining_accounts);
        let config = DecompressFullAccounts::new(None);
        remaining_accounts
            .add_custom_system_accounts(config)
            .unwrap();

        let token_data: Vec<_> = compressed_accounts
            .iter()
            .map(|acc| acc.token.clone())
            .collect();

        let indices: Vec<_> = token_data
            .iter()
            .zip(
                packed_tree_info
                    .state_trees
                    .as_ref()
                    .unwrap()
                    .packed_tree_infos
                    .iter(),
            )
            .zip(ctx.destination_accounts.iter())
            .map(|((token, tree_info), &dest_pubkey)| {
                light_compressed_token_sdk::instructions::decompress_full::pack_for_decompress_full(
                    token,
                    tree_info,
                    dest_pubkey,
                    &mut remaining_accounts,
                )
            })
            .collect();

        let validity_proof = rpc_result.proof;
        let (account_metas, _, _) = remaining_accounts.to_account_metas();
        let instruction_data = sdk_token_test::instruction::DecompressFullCpi {
            indices,
            validity_proof,
        };

        let instruction = Instruction {
            program_id: sdk_token_test::ID,
            accounts: [vec![AccountMeta::new(payer_pubkey, true)], account_metas].concat(),
            data: instruction_data.data(),
        };

        rpc.create_and_send_transaction(&[instruction], &payer_pubkey, &[&ctx.payer, &ctx.owner])
            .await
            .unwrap();

        let remaining_compressed = rpc
            .get_compressed_token_accounts_by_owner(&ctx.owner.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items;

        assert_eq!(
            remaining_compressed.len(),
            0,
            "All compressed accounts should be consumed"
        );

        for destination_account in &ctx.destination_accounts {
            let dest_account_after = rpc
                .get_account(*destination_account)
                .await
                .unwrap()
                .unwrap();
            use light_ctoken_types::state::CToken;
            use light_zero_copy::traits::ZeroCopyAt;
            let (dest_token_after, _) = CToken::zero_copy_at(&dest_account_after.data).unwrap();
            assert_eq!(
                *dest_token_after.amount, ctx.compressed_amount_per_account,
                "Each destination should have its decompressed amount"
            );
        }

        println!("Successfully decompressed {} inputs", num_inputs);
    }
}

/// Test decompress_full with CPI context for optimized multi-program transactions
/// This test uses CPI context to cache signer checks for potential cross-program operations
#[tokio::test]
async fn test_decompress_full_cpi_with_context() {
    for num_inputs in TEST_INPUT_RANGE {
        println!(
            "Testing decompress_full_cpi_with_context with {} inputs",
            num_inputs
        );
        let (mut rpc, ctx) = setup_decompress_full_test(num_inputs).await;
        let payer_pubkey = ctx.payer.pubkey();

        let initial_compressed_accounts = rpc
            .get_compressed_token_accounts_by_owner(&ctx.owner.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items;

        assert_eq!(
            initial_compressed_accounts.len(),
            num_inputs,
            "Should have {} compressed accounts initially",
            num_inputs
        );

        for destination_account in &ctx.destination_accounts {
            let dest_account_before = rpc
                .get_account(*destination_account)
                .await
                .unwrap()
                .unwrap();
            use light_ctoken_types::state::CToken;
            use light_zero_copy::traits::ZeroCopyAt;
            let (dest_token_before, _) = CToken::zero_copy_at(&dest_account_before.data).unwrap();
            assert_eq!(
                *dest_token_before.amount, 0,
                "Destination should be empty initially"
            );
        }

        let mut remaining_accounts = PackedAccounts::default();
        // let output_tree_info = rpc.get_random_state_tree_info().unwrap();

        let mint_recipients = vec![MintToRecipient {
            recipient: ctx.owner.pubkey(),
            amount: 500, // Mint some additional tokens
        }];

        let address_tree_info = rpc.get_address_tree_v2();
        let compressed_mint_address =
            light_compressed_token_sdk::instructions::derive_compressed_mint_address(
                &ctx.mint_seed.pubkey(),
                &address_tree_info.tree,
            );

        let compressed_mint_account = rpc
            .get_compressed_account(compressed_mint_address, None)
            .await
            .unwrap()
            .value
            .ok_or("Compressed mint account not found")
            .unwrap();
        println!(
            "compressed_mint_account
            .tree_info {:?}",
            compressed_mint_account.tree_info
        );
        let cpi_context_pubkey = compressed_mint_account
            .tree_info
            .cpi_context
            .expect("CPI context required for this test");

        let config = DecompressFullAccounts::new(Some(cpi_context_pubkey));
        remaining_accounts
            .add_custom_system_accounts(config)
            .unwrap();

        let compressed_hashes: Vec<_> = initial_compressed_accounts
            .iter()
            .map(|acc| acc.account.hash)
            .collect();
        let rpc_result = rpc
            .get_validity_proof(compressed_hashes, vec![], None)
            .await
            .unwrap()
            .value;

        use light_ctoken_types::state::CompressedMint;
        let compressed_mint =
            CompressedMint::deserialize(&mut compressed_mint_account.data.unwrap().data.as_slice())
                .unwrap();

        let compressed_mint_with_context = CompressedMintWithContext {
            prove_by_index: true,
            leaf_index: compressed_mint_account.leaf_index,
            root_index: 0,
            address: compressed_mint_address,
            mint: compressed_mint.try_into().unwrap(),
        };
        let packed_tree_info = rpc_result.pack_tree_infos(&mut remaining_accounts);
        let mint_params = MintCompressedTokensCpiWriteParams {
            compressed_mint_with_context,
            recipients: mint_recipients,
            cpi_context: light_ctoken_types::instructions::mint_action::CpiContext {
                set_context: false,
                first_set_context: true, // First operation sets the context
                in_tree_index: remaining_accounts
                    .insert_or_get(compressed_mint_account.tree_info.tree),
                in_queue_index: remaining_accounts
                    .insert_or_get(compressed_mint_account.tree_info.queue),
                out_queue_index: remaining_accounts
                    .insert_or_get(compressed_mint_account.tree_info.queue),
                token_out_queue_index: remaining_accounts
                    .insert_or_get(compressed_mint_account.tree_info.queue),
                assigned_account_index: 0,
                ..Default::default()
            },
            cpi_context_pubkey,
        };

        let token_data: Vec<_> = initial_compressed_accounts
            .iter()
            .map(|acc| acc.token.clone())
            .collect();

        let indices: Vec<_> = token_data
            .iter()
            .zip(
                packed_tree_info
                    .state_trees
                    .as_ref()
                    .unwrap()
                    .packed_tree_infos
                    .iter(),
            )
            .zip(ctx.destination_accounts.iter())
            .map(|((token, tree_info), &dest_pubkey)| {
                light_compressed_token_sdk::instructions::decompress_full::pack_for_decompress_full(
                    token,
                    tree_info,
                    dest_pubkey,
                    &mut remaining_accounts,
                )
            })
            .collect();

        let validity_proof = rpc_result.proof;

        let (account_metas, system_accounts_start_offset, _) =
            remaining_accounts.to_account_metas();

        println!("CPI Context test:");
        println!("  CPI context account: {:?}", cpi_context_pubkey);
        println!("  Destination accounts: {:?}", ctx.destination_accounts);
        println!(
            "  System accounts start offset: {}",
            system_accounts_start_offset
        );

        let instruction_data = sdk_token_test::instruction::DecompressFullCpiWithCpiContext {
            indices,
            validity_proof,
            params: Some(mint_params),
        };

        let instruction = Instruction {
            program_id: sdk_token_test::ID,
            accounts: [vec![AccountMeta::new(payer_pubkey, true)], account_metas].concat(),
            data: instruction_data.data(),
        };

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer_pubkey),
            &[&ctx.payer, &ctx.owner],
            rpc.get_latest_blockhash().await.unwrap().0,
        );

        rpc.process_transaction(transaction).await.unwrap();

        let final_compressed_accounts = rpc
            .get_compressed_token_accounts_by_owner(&ctx.owner.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items;

        assert_eq!(
            final_compressed_accounts.len(),
            1,
            "Should have 1 compressed account (newly minted 500 tokens)"
        );
        assert_eq!(
            final_compressed_accounts[0].token.amount, 500,
            "Newly minted compressed tokens"
        );
        assert_eq!(final_compressed_accounts[0].token.mint, ctx.mint_pubkey);

        for destination_account in &ctx.destination_accounts {
            let dest_account_after = rpc
                .get_account(*destination_account)
                .await
                .unwrap()
                .unwrap();
            use light_ctoken_types::state::CToken;
            use light_zero_copy::traits::ZeroCopyAt;
            let (dest_token_after, _) = CToken::zero_copy_at(&dest_account_after.data).unwrap();
            assert_eq!(
                *dest_token_after.amount, ctx.compressed_amount_per_account,
                "Each destination should have received its decompressed amount"
            );
        }

        println!(
            "✅ DecompressFull CPI with CPI context test passed with {} inputs!",
            num_inputs
        );
        println!(
            "  - Original {} tokens decompressed to {} destinations ({} each)",
            ctx.total_compressed_amount, num_inputs, ctx.compressed_amount_per_account
        );
        println!("  - Additional 500 tokens minted to compressed state");
    }
}
