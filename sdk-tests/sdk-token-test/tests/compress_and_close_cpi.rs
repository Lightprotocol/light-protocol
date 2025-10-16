//#![cfg(feature = "test-sbf")]

use anchor_lang::InstructionData;
use light_compressed_token_sdk::instructions::{
    compress_and_close::{pack_for_compress_and_close, CompressAndCloseAccounts},
    find_spl_mint_address,
};
use light_ctoken_types::instructions::mint_action::Recipient;
use light_program_test::{Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_sdk::instruction::PackedAccounts;
use light_test_utils::{airdrop_lamports, assert_transfer2::assert_transfer2_compress_and_close};
use light_token_client::{
    actions::mint_action_comprehensive,
    instructions::{mint_action::NewMint, transfer2::CompressAndCloseInput},
};
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
    owners: Vec<Keypair>,
    mint_seed: Keypair,
    mint_pubkey: Pubkey,
    token_account_pubkeys: Vec<Pubkey>,
    mint_amount: u64,
    with_compressible_extension: bool,
}

/// Shared setup function for compress_and_close tests
async fn setup_compress_and_close_test(
    num_ctoken_accounts: usize,
    with_compressible_extension: bool,
) -> (LightProgramTest, TestContext) {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_token_test", sdk_token_test::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // Create compressed mint
    let mint_seed = Keypair::new();
    let mint_pubkey = find_spl_mint_address(&mint_seed.pubkey()).0;
    let mint_authority = payer.pubkey();
    let decimals = 9u8;

    // Create owners - one for each token account
    let mut owners = Vec::with_capacity(num_ctoken_accounts);
    for _ in 0..num_ctoken_accounts {
        let owner = Keypair::new();
        // Fund each owner
        airdrop_lamports(&mut rpc, &owner.pubkey(), 10_000_000_000)
            .await
            .unwrap();
        owners.push(owner);
    }

    // Set up rent authority using the first owner
    let rent_sponsor = if with_compressible_extension {
        rpc.test_accounts.funding_pool_config.rent_sponsor_pda
    } else {
        // Use first owner as both rent authority and recipient
        owners[0].pubkey()
    };
    let pre_pay_num_epochs = 0;
    // Create ATA accounts for each owner
    let mut token_account_pubkeys = Vec::with_capacity(num_ctoken_accounts);

    use light_compressed_token_sdk::instructions::{
        create_associated_token_account, create_compressible_associated_token_account,
        derive_ctoken_ata, CreateCompressibleAssociatedTokenAccountInputs,
    };

    for owner in &owners {
        let (token_account_pubkey, _) = derive_ctoken_ata(&owner.pubkey(), &mint_pubkey);

        // Create the ATA account with compressible extension if needed
        let create_token_account_ix = if with_compressible_extension {
            create_compressible_associated_token_account(
                CreateCompressibleAssociatedTokenAccountInputs {
                    payer: payer.pubkey(),
                    mint: mint_pubkey,
                    owner: owner.pubkey(),
                    rent_sponsor,
                    pre_pay_num_epochs,
                    lamports_per_write: None,
                    compressible_config: rpc
                        .test_accounts
                        .funding_pool_config
                        .compressible_config_pda,
                    token_account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
                },
            )
            .unwrap()
        } else {
            // Create regular ATA without compressible extension
            create_associated_token_account(payer.pubkey(), owner.pubkey(), mint_pubkey).unwrap()
        };

        rpc.create_and_send_transaction(&[create_token_account_ix], &payer.pubkey(), &[&payer])
            .await
            .unwrap();

        token_account_pubkeys.push(token_account_pubkey);
    }

    // Now create mint and mint to the decompressed token accounts
    let mint_amount = 1000;

    let decompressed_recipients = owners
        .iter()
        .map(|owner| Recipient {
            recipient: owner.pubkey().into(),
            amount: mint_amount,
        })
        .collect::<Vec<_>>();
    println!("decompressed_recipients {:?}", decompressed_recipients);
    // Create the mint and mint to the existing ATAs
    mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &payer,
        &payer,
        Vec::new(),              // No compressed recipients
        decompressed_recipients, // Mint to owners - ATAs already exist
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
            owners,
            mint_seed,
            mint_pubkey,
            token_account_pubkeys,
            mint_amount,
            with_compressible_extension,
        },
    )
}

#[tokio::test]
async fn test_compress_and_close_cpi_indices_owner() {
    let (mut rpc, ctx) = setup_compress_and_close_test(1, true).await;
    let payer_pubkey = ctx.payer.pubkey();
    let token_account_pubkey = ctx.token_account_pubkeys[0];

    // Prepare accounts for CPI instruction
    let mut remaining_accounts = PackedAccounts::default();

    // Get output tree for compression
    let output_tree_info = rpc.get_random_state_tree_info().unwrap();

    // Get the ctoken account data
    let ctoken_solana_account = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap();
    // Add output queue first so it's at index 0
    remaining_accounts.insert_or_get(output_tree_info.queue);
    // Use pack_for_compress_and_close to pack all required accounts
    let indices = pack_for_compress_and_close(
        token_account_pubkey,
        ctoken_solana_account.data.as_slice(),
        &mut remaining_accounts,
        false,
    )
    .unwrap();

    // Add light system program accounts
    let config = CompressAndCloseAccounts::default();
    remaining_accounts
        .add_custom_system_accounts(config)
        .unwrap();

    let (account_metas, system_accounts_start_offset, _) = remaining_accounts.to_account_metas();

    // Create the compress_and_close_cpi_indices instruction data
    let indices_vec = vec![indices];

    let instruction_data = sdk_token_test::instruction::CompressAndCloseCpiIndices {
        indices: indices_vec,
        system_accounts_offset: system_accounts_start_offset as u8,
    };

    // Create the instruction
    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: [vec![AccountMeta::new(payer_pubkey, true)], account_metas].concat(),
        data: instruction_data.data(),
    };

    // Sign with payer and compression_authority (which is owner when no extension)
    let signers = vec![&ctx.payer, &ctx.owners[0]];

    rpc.create_and_send_transaction(&[instruction], &payer_pubkey, &signers)
        .await
        .unwrap();

    let compress_and_close_input = CompressAndCloseInput {
        solana_ctoken_account: token_account_pubkey,
        authority: ctx.owners[0].pubkey(), // Owner is the authority in this test
        output_queue: output_tree_info.queue,
        destination: None, // Owner is the authority and destination in this test
        is_compressible: false,
    };

    assert_transfer2_compress_and_close(&mut rpc, compress_and_close_input).await;

    println!("✅ CompressAndClose CPI test passed!");
}
/// Test the high-level compress_and_close_cpi function
/// This test uses the SDK's compress_and_close_ctoken_accounts which handles all index discovery
#[tokio::test]
async fn test_compress_and_close_cpi_high_level() {
    let (mut rpc, ctx) = setup_compress_and_close_test(1, false).await;
    let payer_pubkey = ctx.payer.pubkey();
    let token_account_pubkey = ctx.token_account_pubkeys[0];

    // Prepare accounts for CPI instruction - using high-level function
    // Mirror the exact setup from test_compress_and_close_cpi_indices
    let mut remaining_accounts = PackedAccounts::default();

    // Get output tree for compression
    let output_tree_info = rpc.get_random_state_tree_info().unwrap();
    remaining_accounts.insert_or_get(output_tree_info.queue);
    // DON'T pack the output tree - it's passed separately as output_queue account
    let ctoken_solana_account = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap();

    pack_for_compress_and_close(
        token_account_pubkey,
        ctoken_solana_account.data.as_slice(),
        &mut remaining_accounts,
        ctx.with_compressible_extension, // false - using owner as authority
    )
    .unwrap();

    let config = CompressAndCloseAccounts::default();
    remaining_accounts
        .add_custom_system_accounts(config)
        .unwrap();
    // Add accounts to instruction
    let (account_metas, system_accounts_start_offset, _) = remaining_accounts.to_account_metas();

    // Create the compress_and_close_cpi instruction data for high-level function
    let instruction_data = sdk_token_test::instruction::CompressAndCloseCpi {
        with_compression_authority: false, // Don't use rent authority from extension
        system_accounts_offset: system_accounts_start_offset as u8, // No accounts before system accounts in remaining_accounts
    };

    // Create the instruction - OneCTokenAccount expects [signer, ctoken_account, ...remaining]
    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: [
            vec![
                AccountMeta::new(payer_pubkey, true), // signer (mutable)
                AccountMeta::new(token_account_pubkey, false), // ctoken_account (mutable)
                AccountMeta::new(output_tree_info.queue, false), // ctoken_account (mutable)
            ],
            account_metas, // remaining accounts (trees, mint, owner, etc.)
        ]
        .concat(),
        data: instruction_data.data(),
    };

    // Execute transaction - sign with payer and owner
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&ctx.payer, &ctx.owners[0]],
        rpc.get_latest_blockhash().await.unwrap().0,
    );

    // Check if there are any compressed accounts BEFORE compress_and_close
    let pre_compress_accounts = rpc
        .get_compressed_token_accounts_by_owner(&ctx.owners[0].pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;
    println!(
        "Compressed accounts BEFORE compress_and_close: {}",
        pre_compress_accounts.len()
    );
    for (i, acc) in pre_compress_accounts.iter().enumerate() {
        println!(
            "  Pre-compress Account {}: amount={}, mint={}",
            i, acc.token.amount, acc.token.mint
        );
    }

    rpc.process_transaction(transaction).await.unwrap();

    // Verify compressed account was created for the first owner
    let compressed_accounts = rpc
        .get_compressed_token_accounts_by_owner(&ctx.owners[0].pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    println!("Compressed accounts found: {:?}", compressed_accounts);
    assert_eq!(compressed_accounts[0].token.amount, ctx.mint_amount);
    assert_eq!(compressed_accounts[0].token.mint, ctx.mint_pubkey);
    assert_eq!(compressed_accounts.len(), 1);

    // Verify source account is closed
    let closed_account = rpc.get_account(token_account_pubkey).await.unwrap();
    if let Some(acc) = closed_account {
        assert_eq!(
            acc.lamports, 0,
            "Account should have 0 lamports after closing"
        );
    }

    println!("✅ CompressAndClose CPI high-level test passed!");
}

/// Test compressing 4 token accounts in a single instruction
/// This test uses compress_and_close_cpi_indices which supports multiple accounts
#[tokio::test]
async fn test_compress_and_close_cpi_multiple() {
    let (mut rpc, ctx) = setup_compress_and_close_test(4, false).await;
    let payer_pubkey = ctx.payer.pubkey();

    // Prepare accounts for CPI instruction
    let mut remaining_accounts = PackedAccounts::default();

    // Get output tree for compression
    let output_tree_info = rpc.get_random_state_tree_info().unwrap();
    remaining_accounts.insert_or_get(output_tree_info.queue);

    // Collect indices for all 4 accounts
    let mut indices_vec = Vec::with_capacity(ctx.token_account_pubkeys.len());

    for token_account_pubkey in ctx.token_account_pubkeys.iter() {
        let ctoken_solana_account = rpc
            .get_account(*token_account_pubkey)
            .await
            .unwrap()
            .unwrap();
        println!("packing token_account_pubkey {:?}", token_account_pubkey);
        let indices = pack_for_compress_and_close(
            *token_account_pubkey,
            ctoken_solana_account.data.as_slice(),
            &mut remaining_accounts,
            ctx.with_compressible_extension,
        )
        .unwrap();
        indices_vec.push(indices);
    }

    // Add light system program accounts
    let config = CompressAndCloseAccounts::default();
    remaining_accounts
        .add_custom_system_accounts(config)
        .unwrap();

    let (account_metas, system_accounts_start_offset, _) = remaining_accounts.to_account_metas();

    println!("Total account_metas: {}", account_metas.len());
    for (i, meta) in account_metas.iter().enumerate() {
        println!(
            "  [{}] {:?} (signer: {}, writable: {})",
            i, meta.pubkey, meta.is_signer, meta.is_writable
        );
    }
    println!(
        "System accounts start offset: {}",
        system_accounts_start_offset
    );
    println!("indices_vec {:?}", indices_vec);
    println!(
        "owners {:?}",
        ctx.owners.iter().map(|x| x.pubkey()).collect::<Vec<_>>()
    );
    // Create the compress_and_close_cpi_indices instruction data
    let instruction_data = sdk_token_test::instruction::CompressAndCloseCpiIndices {
        indices: indices_vec,
        system_accounts_offset: system_accounts_start_offset as u8,
    };

    // Create the instruction
    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: [vec![AccountMeta::new(payer_pubkey, true)], account_metas].concat(),
        data: instruction_data.data(),
    };

    // Execute transaction with all 4 accounts compressed in a single instruction
    // Need to sign with all owners since we're compressing their accounts
    let mut signers = vec![&ctx.payer];
    for owner in &ctx.owners {
        signers.push(owner);
    }

    rpc.create_and_send_transaction(&[instruction], &payer_pubkey, &signers)
        .await
        .unwrap();

    // Verify compressed accounts were created - one for each owner
    let mut total_compressed_accounts = 0;
    for owner in &ctx.owners {
        let compressed_accounts = rpc
            .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items;

        assert_eq!(compressed_accounts.len(), 1);
        assert_eq!(compressed_accounts[0].token.amount, ctx.mint_amount);
        assert_eq!(compressed_accounts[0].token.mint, ctx.mint_pubkey);
        total_compressed_accounts += compressed_accounts.len();
    }
    assert_eq!(total_compressed_accounts, 4);

    // Verify all source accounts are closed
    for token_account_pubkey in &ctx.token_account_pubkeys {
        let closed_account = rpc.get_account(*token_account_pubkey).await.unwrap();
        if let Some(acc) = closed_account {
            assert_eq!(
                acc.lamports, 0,
                "Account should have 0 lamports after closing"
            );
        }
    }

    println!("✅ CompressAndClose CPI multiple accounts test passed!");
}

/// Test compress_and_close with CPI context for optimized multi-program transactions
/// This test uses CPI context to cache signer checks for potential cross-program operations
#[tokio::test]
async fn test_compress_and_close_cpi_with_context() {
    let (mut rpc, ctx) = setup_compress_and_close_test(1, false).await;
    let payer_pubkey = ctx.payer.pubkey();
    let token_account_pubkey = ctx.token_account_pubkeys[0];

    // Import required types for minting
    use anchor_lang::AnchorDeserialize;
    use light_compressed_token_sdk::instructions::MintToRecipient;
    use light_ctoken_types::instructions::mint_action::CompressedMintWithContext;
    use sdk_token_test::mint_compressed_tokens_cpi_write::MintCompressedTokensCpiWriteParams;

    // Get initial rent recipient balance (owner in this case since no extension)
    let initial_recipient_balance = rpc
        .get_account(ctx.owners[0].pubkey())
        .await
        .unwrap()
        .map(|acc| acc.lamports)
        .unwrap_or(0);

    // Prepare accounts for CPI instruction with CPI context
    let mut remaining_accounts = PackedAccounts::default();
    // Derive compressed mint address using utility function
    let address_tree_info = rpc.get_address_tree_v2();
    let compressed_mint_address =
        light_compressed_token_sdk::instructions::derive_compressed_mint_address(
            &ctx.mint_seed.pubkey(),
            &address_tree_info.tree,
        );

    // Get the compressed mint account
    let compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value
        .ok_or("Compressed mint account not found")
        .unwrap();

    let cpi_context_pubkey = compressed_mint_account
        .tree_info
        .cpi_context
        .expect("CPI context required for this test");
    // Add light system program accounts (following the pattern from other tests)
    use light_compressed_token_sdk::instructions::compress_and_close::CompressAndCloseAccounts;
    let config = CompressAndCloseAccounts::new_with_cpi_context(Some(cpi_context_pubkey), None);
    remaining_accounts
        .add_custom_system_accounts(config)
        .unwrap();

    // Create mint params to populate CPI context
    let mint_recipients = vec![MintToRecipient {
        recipient: ctx.owners[0].pubkey(),
        amount: 500, // Mint some additional tokens
    }];

    // Deserialize the mint data
    use light_ctoken_types::state::CompressedMint;
    let compressed_mint =
        CompressedMint::deserialize(&mut compressed_mint_account.data.unwrap().data.as_slice())
            .unwrap();
    remaining_accounts.insert_or_get(compressed_mint_account.tree_info.queue);

    // Create CompressedMintWithContext for minting to populate CPI context
    let compressed_mint_with_context = CompressedMintWithContext {
        prove_by_index: true,
        leaf_index: compressed_mint_account.leaf_index,
        root_index: 0,
        address: compressed_mint_address,
        mint: compressed_mint.try_into().unwrap(),
    };
    let mint_params = MintCompressedTokensCpiWriteParams {
        compressed_mint_with_context,
        recipients: mint_recipients,
        cpi_context: light_ctoken_types::instructions::mint_action::CpiContext {
            set_context: false,
            first_set_context: true, // First operation sets the context
            in_tree_index: remaining_accounts.insert_or_get(compressed_mint_account.tree_info.tree),
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
    // Get the ctoken account data
    let ctoken_solana_account = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap();

    // Debug: Check the actual token account balance
    use light_ctoken_types::state::CToken;
    use light_zero_copy::traits::ZeroCopyAt;
    let (ctoken_account, _) = CToken::zero_copy_at(ctoken_solana_account.data.as_slice()).unwrap();
    println!(
        "DEBUG: Token account balance before compress_and_close: {}",
        ctoken_account.amount
    );
    println!("DEBUG: Expected balance: {}", ctx.mint_amount);

    // Generate indices for compress and close operation (following the pattern from test_compress_and_close_cpi_indices)
    let indices = pack_for_compress_and_close(
        token_account_pubkey,
        ctoken_solana_account.data.as_slice(),
        &mut remaining_accounts,
        ctx.with_compressible_extension, // false - using owner as authority
    )
    .unwrap();

    let (account_metas, system_accounts_start_offset, _) = remaining_accounts.to_account_metas();

    println!("CPI Context test:");
    println!("  CPI context account: {:?}", cpi_context_pubkey);
    println!("  Token account: {:?}", token_account_pubkey);
    println!(
        "  Output queue: {:?}",
        compressed_mint_account.tree_info.queue
    );
    println!(
        "  System accounts start offset: {}",
        system_accounts_start_offset
    );
    println!("account_metas: {:?}", account_metas);

    // Create the compress_and_close_cpi_with_cpi_context instruction
    let instruction_data = sdk_token_test::instruction::CompressAndCloseCpiWithCpiContext {
        indices: vec![indices], // Use generated indices like CompressAndCloseCpiIndices pattern
        params: mint_params,
    };

    // Create the instruction - TwoCTokenAccounts expects signer, ctoken_account1, ctoken_account2, output_queue
    // But we're only using one account, so we'll pass the same account twice (second one won't be used)
    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: [
            vec![
                AccountMeta::new(payer_pubkey, true), // signer
            ],
            account_metas, // remaining accounts (trees, system accounts, etc.)
        ]
        .concat(),
        data: instruction_data.data(),
    };

    // Execute transaction - sign with payer and owner
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&ctx.payer, &ctx.owners[0]],
        rpc.get_latest_blockhash().await.unwrap().0,
    );

    rpc.process_transaction(transaction).await.unwrap();

    // Verify compressed account was created
    let compressed_accounts = rpc
        .get_compressed_token_accounts_by_owner(&ctx.owners[0].pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(compressed_accounts.len(), 2);
    assert_eq!(compressed_accounts[0].token.amount, ctx.mint_amount);
    assert_eq!(compressed_accounts[0].token.mint, ctx.mint_pubkey);
    assert_eq!(compressed_accounts[1].token.amount, 500);
    assert_eq!(compressed_accounts[1].token.mint, ctx.mint_pubkey);

    // Verify source account is closed
    let closed_account = rpc.get_account(token_account_pubkey).await.unwrap();
    if let Some(acc) = closed_account {
        assert_eq!(
            acc.lamports, 0,
            "Account should have 0 lamports after closing"
        );
    }

    // Verify rent was transferred to owner (no extension, so owner gets rent)
    let final_recipient_balance = rpc
        .get_account(ctx.owners[0].pubkey())
        .await
        .unwrap()
        .map(|acc| acc.lamports)
        .unwrap_or(0);

    assert!(
        final_recipient_balance > initial_recipient_balance,
        "Owner should receive rent when no extension is present"
    );

    println!("✅ CompressAndClose CPI with CPI context test passed!");
}
