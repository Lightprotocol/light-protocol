//#![cfg(feature = "test-sbf")]

use anchor_lang::InstructionData;
use light_compressed_token_sdk::instructions::{
    compress_and_close::{pack_for_compress_and_close, CompressAndCloseAccounts},
    find_spl_mint_address,
};
use light_ctoken_types::{instructions::mint_action::Recipient, COMPRESSIBLE_TOKEN_ACCOUNT_SIZE};
use light_program_test::{Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_sdk::instruction::PackedAccounts;
use light_test_utils::airdrop_lamports;
use light_token_client::{actions::mint_action_comprehensive, instructions::mint_action::NewMint};
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
    rent_authority: Keypair,
    rent_recipient: Pubkey,
    mint_seed: Keypair,
    mint_pubkey: Pubkey,
    token_account_pubkeys: Vec<Pubkey>,
    mint_amount: u64,
    rent_exemption: u64,
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
    let (rent_authority, rent_recipient) = if with_compressible_extension {
        // Separate rent authority and recipient
        let rent_auth = Keypair::new();
        let rent_recip = Pubkey::new_unique();
        airdrop_lamports(&mut rpc, &rent_auth.pubkey(), 10_000_000_000)
            .await
            .unwrap();
        (rent_auth, rent_recip)
    } else {
        // Use first owner as both rent authority and recipient
        (owners[0].insecure_clone(), owners[0].pubkey())
    };

    // Get rent exemption
    let rent_exemption = rpc
        .get_minimum_balance_for_rent_exemption(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize)
        .await
        .unwrap();

    // Create ATA accounts for each owner
    let mut token_account_pubkeys = Vec::with_capacity(num_ctoken_accounts);

    use light_compressed_token_sdk::instructions::{
        create_compressible_associated_token_account, derive_ctoken_ata,
        CreateCompressibleAssociatedTokenAccountInputs,
    };

    for owner in &owners {
        let (token_account_pubkey, _) = derive_ctoken_ata(&owner.pubkey(), &mint_pubkey);

        // Create the ATA account with compressible extension if needed
        let create_token_account_ix = create_compressible_associated_token_account(
            CreateCompressibleAssociatedTokenAccountInputs {
                payer: payer.pubkey(),
                mint: mint_pubkey,
                owner: owner.pubkey(),
                rent_authority: if with_compressible_extension {
                    rent_authority.pubkey()
                } else {
                    owner.pubkey()
                },
                rent_recipient: if with_compressible_extension {
                    rent_recipient
                } else {
                    owner.pubkey()
                },
                slots_until_compression: 0,
            },
        )
        .unwrap();

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
        false,
        Vec::new(),              // No compressed recipients
        decompressed_recipients, // Mint to owners - ATAs already exist
        None,
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
            rent_authority,
            rent_recipient,
            mint_seed,
            mint_pubkey,
            token_account_pubkeys,
            mint_amount,
            rent_exemption,
            with_compressible_extension,
        },
    )
}

/// Test the original compress_and_close_cpi_indices instruction with manual indices
/// This test verifies that CompressAndClose mode works correctly through CPI with manual index management
#[tokio::test]
async fn test_compress_and_close_cpi_indices() {
    let (mut rpc, ctx) = setup_compress_and_close_test(1, true).await;
    let payer_pubkey = ctx.payer.pubkey();
    let token_account_pubkey = ctx.token_account_pubkeys[0];
    // Get initial rent recipient balance
    let initial_recipient_balance = rpc
        .get_account(ctx.rent_recipient)
        .await
        .unwrap()
        .map(|acc| acc.lamports)
        .unwrap_or(0);

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

    // Use pack_for_compress_and_close to pack all required accounts
    let indices = pack_for_compress_and_close(
        token_account_pubkey,
        ctoken_solana_account.data.as_slice(),
        output_tree_info.queue,
        &mut remaining_accounts,
        ctx.with_compressible_extension, // true since we have a separate rent authority
    )
    .unwrap();

    // Add light system program accounts
    let config = CompressAndCloseAccounts::new(sdk_token_test::ID, None);
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

    // Sign with payer and rent_authority (which is owner when no extension)
    let signers = if ctx.with_compressible_extension {
        vec![&ctx.payer, &ctx.rent_authority]
    } else {
        vec![&ctx.payer, &ctx.owners[0]]
    };

    rpc.create_and_send_transaction(&[instruction], &payer_pubkey, &signers)
        .await
        .unwrap();

    // Verify compressed account was created for the first owner
    let compressed_accounts = rpc
        .get_compressed_token_accounts_by_owner(&ctx.owners[0].pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(compressed_accounts.len(), 1);
    assert_eq!(compressed_accounts[0].token.amount, ctx.mint_amount);
    assert_eq!(compressed_accounts[0].token.mint, ctx.mint_pubkey);

    // Verify source account is closed
    let closed_account = rpc.get_account(token_account_pubkey).await.unwrap();
    if let Some(acc) = closed_account {
        assert_eq!(
            acc.lamports, 0,
            "Account should have 0 lamports after closing"
        );
    }

    // Verify rent was transferred to recipient
    let final_recipient_balance = rpc
        .get_account(ctx.rent_recipient)
        .await
        .unwrap()
        .map(|acc| acc.lamports)
        .unwrap_or(0);

    assert_eq!(
        final_recipient_balance,
        initial_recipient_balance + ctx.rent_exemption,
        "Rent recipient should receive exact rent exemption amount"
    );

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
    // DON'T pack the output tree - it's passed separately as output_queue account
    let ctoken_solana_account = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .unwrap();

    pack_for_compress_and_close(
        token_account_pubkey,
        ctoken_solana_account.data.as_slice(),
        output_tree_info.queue,
        &mut remaining_accounts,
        ctx.with_compressible_extension, // false - using owner as authority
    )
    .unwrap();

    let config = CompressAndCloseAccounts::new(sdk_token_test::ID, None);
    remaining_accounts
        .add_custom_system_accounts(config)
        .unwrap();
    // Add accounts to instruction
    let (account_metas, system_accounts_start_offset, _) = remaining_accounts.to_account_metas();

    // Create the compress_and_close_cpi instruction data for high-level function
    let instruction_data = sdk_token_test::instruction::CompressAndCloseCpi {
        with_rent_authority: false, // Don't use rent authority from extension
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

    // Collect indices for all 4 accounts
    let mut indices_vec = Vec::with_capacity(ctx.token_account_pubkeys.len());

    for (i, token_account_pubkey) in ctx.token_account_pubkeys.iter().enumerate() {
        let ctoken_solana_account = rpc
            .get_account(*token_account_pubkey)
            .await
            .unwrap()
            .unwrap();
        println!("packing token_account_pubkey {:?}", token_account_pubkey);
        let indices = pack_for_compress_and_close(
            *token_account_pubkey,
            ctoken_solana_account.data.as_slice(),
            output_tree_info.queue,
            &mut remaining_accounts,
            ctx.with_compressible_extension,
        )
        .unwrap();
        indices_vec.push(indices);
    }

    // Add light system program accounts
    let config = CompressAndCloseAccounts::new(sdk_token_test::ID, None);
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

/// Test compressing 4 token accounts with rent authority as signer
/// This test uses compress_and_close_cpi_indices with compressible extension and separate rent authority
#[tokio::test]
async fn test_compress_and_close_cpi_multiple_with_rent_authority() {
    let (mut rpc, ctx) = setup_compress_and_close_test(4, true).await;
    let payer_pubkey = ctx.payer.pubkey();

    // Prepare accounts for CPI instruction
    let mut remaining_accounts = PackedAccounts::default();

    // Get output tree for compression
    let output_tree_info = rpc.get_random_state_tree_info().unwrap();

    // Collect indices for all 4 accounts
    let mut indices_vec = Vec::with_capacity(ctx.token_account_pubkeys.len());

    for (_i, token_account_pubkey) in ctx.token_account_pubkeys.iter().enumerate() {
        let ctoken_solana_account = rpc
            .get_account(*token_account_pubkey)
            .await
            .unwrap()
            .unwrap();
        println!("packing token_account_pubkey {:?}", token_account_pubkey);
        let indices = pack_for_compress_and_close(
            *token_account_pubkey,
            ctoken_solana_account.data.as_slice(),
            output_tree_info.queue,
            &mut remaining_accounts,
            ctx.with_compressible_extension,
        )
        .unwrap();
        indices_vec.push(indices);
    }

    // Add light system program accounts
    let config = CompressAndCloseAccounts::new(sdk_token_test::ID, None);
    remaining_accounts
        .add_custom_system_accounts(config)
        .unwrap();

    let (account_metas, system_accounts_start_offset, _) = remaining_accounts.to_account_metas();

    println!("Total account_metas: {}", account_metas.len());
    println!(
        "System accounts start offset: {}",
        system_accounts_start_offset
    );
    println!("indices_vec {:?}", indices_vec);
    println!(
        "owners {:?}",
        ctx.owners.iter().map(|x| x.pubkey()).collect::<Vec<_>>()
    );
    println!("rent_authority {:?}", ctx.rent_authority.pubkey());
    
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
    // Need to sign with payer and rent_authority (since with_compressible_extension is true)
    let signers = vec![&ctx.payer, &ctx.rent_authority];

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

    // Verify rent was transferred to recipient (not owners)
    let final_recipient_balance = rpc
        .get_account(ctx.rent_recipient)
        .await
        .unwrap()
        .map(|acc| acc.lamports)
        .unwrap_or(0);

    // With 4 accounts, rent should be 4 times the single account rent
    assert!(
        final_recipient_balance >= ctx.rent_exemption * 4,
        "Rent recipient should receive rent from all 4 accounts"
    );

    println!("✅ CompressAndClose CPI multiple accounts with rent authority test passed!");
}

/// Test compressing 4 token accounts: 2 with owner as signer, 2 with rent authority as signer
/// This test mixes both signing modes in a single transaction
#[tokio::test]
async fn test_compress_and_close_cpi_mixed_signers() {
    // Create test setup with 4 accounts
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_token_test", sdk_token_test::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Create compressed mint
    let mint_seed = Keypair::new();
    let mint_pubkey = find_spl_mint_address(&mint_seed.pubkey()).0;
    let mint_authority = payer.pubkey();
    let decimals = 9u8;

    // Create 4 owners
    let mut owners = Vec::with_capacity(4);
    for _ in 0..4 {
        let owner = Keypair::new();
        airdrop_lamports(&mut rpc, &owner.pubkey(), 10_000_000_000)
            .await
            .unwrap();
        owners.push(owner);
    }

    // Create rent authority for accounts 2 and 3
    let rent_authority = Keypair::new();
    let rent_recipient = Pubkey::new_unique();
    airdrop_lamports(&mut rpc, &rent_authority.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    // Get rent exemption
    let rent_exemption = rpc
        .get_minimum_balance_for_rent_exemption(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize)
        .await
        .unwrap();

    // Create ATA accounts - first 2 without extension, last 2 with extension
    let mut token_account_pubkeys = Vec::with_capacity(4);
    
    use light_compressed_token_sdk::instructions::{
        create_compressible_associated_token_account, derive_ctoken_ata,
        CreateCompressibleAssociatedTokenAccountInputs,
    };

    for (i, owner) in owners.iter().enumerate() {
        let (token_account_pubkey, _) = derive_ctoken_ata(&owner.pubkey(), &mint_pubkey);

        // First 2 accounts: owner is authority (no extension)
        // Last 2 accounts: rent_authority is authority (with extension)
        let with_extension = i >= 2;
        
        let create_token_account_ix = create_compressible_associated_token_account(
            CreateCompressibleAssociatedTokenAccountInputs {
                payer: payer.pubkey(),
                mint: mint_pubkey,
                owner: owner.pubkey(),
                rent_authority: if with_extension {
                    rent_authority.pubkey()
                } else {
                    owner.pubkey()
                },
                rent_recipient: if with_extension {
                    rent_recipient
                } else {
                    owner.pubkey()
                },
                slots_until_compression: 0,
            },
        )
        .unwrap();

        rpc.create_and_send_transaction(&[create_token_account_ix], &payer.pubkey(), &[&payer])
            .await
            .unwrap();

        token_account_pubkeys.push(token_account_pubkey);
    }

    // Mint to all 4 accounts
    let mint_amount = 1000;
    let decompressed_recipients = owners
        .iter()
        .map(|owner| Recipient {
            recipient: owner.pubkey().into(),
            amount: mint_amount,
        })
        .collect::<Vec<_>>();

    mint_action_comprehensive(
        &mut rpc,
        &mint_seed,
        &payer,
        &payer,
        false,
        Vec::new(),
        decompressed_recipients,
        None,
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

    // Now compress all 4 accounts in a single transaction
    let mut remaining_accounts = PackedAccounts::default();
    let output_tree_info = rpc.get_random_state_tree_info().unwrap();
    let mut indices_vec = Vec::with_capacity(4);

    for (i, token_account_pubkey) in token_account_pubkeys.iter().enumerate() {
        let ctoken_solana_account = rpc
            .get_account(*token_account_pubkey)
            .await
            .unwrap()
            .unwrap();
        
        // First 2 accounts don't have extension, last 2 have extension
        let with_extension = i >= 2;
        
        let indices = pack_for_compress_and_close(
            *token_account_pubkey,
            ctoken_solana_account.data.as_slice(),
            output_tree_info.queue,
            &mut remaining_accounts,
            with_extension,
        )
        .unwrap();
        indices_vec.push(indices);
    }

    // Add light system program accounts
    let config = CompressAndCloseAccounts::new(sdk_token_test::ID, None);
    remaining_accounts
        .add_custom_system_accounts(config)
        .unwrap();

    let (account_metas, system_accounts_start_offset, _) = remaining_accounts.to_account_metas();

    println!("Mixed signers test:");
    println!("  Accounts 0-1: owner as signer");
    println!("  Accounts 2-3: rent_authority as signer");
    println!("  Total account_metas: {}", account_metas.len());
    
    // Create the instruction
    let instruction_data = sdk_token_test::instruction::CompressAndCloseCpiIndices {
        indices: indices_vec,
        system_accounts_offset: system_accounts_start_offset as u8,
    };

    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: [vec![AccountMeta::new(payer_pubkey, true)], account_metas].concat(),
        data: instruction_data.data(),
    };

    // Sign with payer, first 2 owners, and rent authority
    let signers = vec![
        &payer,
        &owners[0],  // Signer for account 0
        &owners[1],  // Signer for account 1
        &rent_authority,  // Signer for accounts 2 and 3
    ];

    rpc.create_and_send_transaction(&[instruction], &payer_pubkey, &signers)
        .await
        .unwrap();

    // Verify all compressed accounts were created
    let mut total_compressed_accounts = 0;
    for owner in &owners {
        let compressed_accounts = rpc
            .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items;

        assert_eq!(compressed_accounts.len(), 1);
        assert_eq!(compressed_accounts[0].token.amount, mint_amount);
        assert_eq!(compressed_accounts[0].token.mint, mint_pubkey);
        total_compressed_accounts += compressed_accounts.len();
    }
    assert_eq!(total_compressed_accounts, 4);

    // Verify all source accounts are closed
    for token_account_pubkey in &token_account_pubkeys {
        let closed_account = rpc.get_account(*token_account_pubkey).await.unwrap();
        if let Some(acc) = closed_account {
            assert_eq!(
                acc.lamports, 0,
                "Account should have 0 lamports after closing"
            );
        }
    }

    // Verify rent distribution
    // First 2 accounts: rent goes to owners
    for i in 0..2 {
        let owner_balance = rpc
            .get_account(owners[i].pubkey())
            .await
            .unwrap()
            .map(|acc| acc.lamports)
            .unwrap_or(0);
        
        assert!(
            owner_balance >= 10_000_000_000 + rent_exemption,
            "Owner {} should have received rent", i
        );
    }

    // Last 2 accounts: rent goes to rent_recipient
    let recipient_balance = rpc
        .get_account(rent_recipient)
        .await
        .unwrap()
        .map(|acc| acc.lamports)
        .unwrap_or(0);
    
    assert!(
        recipient_balance >= rent_exemption * 2,
        "Rent recipient should have received rent from 2 accounts"
    );

    println!("✅ CompressAndClose CPI mixed signers test passed!");
}
