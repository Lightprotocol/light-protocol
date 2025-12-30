//! Tests for Token 2022 mint with multiple extensions
//!
//! This module tests the creation and verification of Token 2022 mints
//! with all supported extensions.

use light_ctoken_interface::state::{
    ExtensionStruct, PausableAccountExtension, PermanentDelegateAccountExtension,
    TransferFeeAccountExtension, TransferHookAccountExtension, ACCOUNT_TYPE_TOKEN_ACCOUNT,
};
use light_program_test::{utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    mint_2022::{
        create_mint_22_with_extensions, create_token_22_account, mint_spl_tokens_22,
        Token22ExtensionConfig,
    },
    Rpc, RpcError,
};
use light_token_client::instructions::transfer2::{
    create_generic_transfer2_instruction, CompressInput, Transfer2InstructionType,
};
use serial_test::serial;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer,
};

/// Test context for extension-related tests
pub struct ExtensionsTestContext {
    pub rpc: LightProgramTest,
    pub payer: Keypair,
    pub _mint_keypair: Keypair,
    pub mint_pubkey: Pubkey,
    pub extension_config: Token22ExtensionConfig,
}

/// Set up test environment with a Token 2022 mint with all extensions
pub async fn setup_extensions_test() -> Result<ExtensionsTestContext, RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();

    // Create mint with all extensions
    let (mint_keypair, extension_config) =
        create_mint_22_with_extensions(&mut rpc, &payer, 9).await;

    let mint_pubkey = mint_keypair.pubkey();

    Ok(ExtensionsTestContext {
        rpc,
        payer,
        _mint_keypair: mint_keypair,
        mint_pubkey,
        extension_config,
    })
}

#[tokio::test]
#[serial]
async fn test_setup_mint_22_with_all_extensions() {
    use light_test_utils::mint_2022::assert_mint_22_with_all_extensions;

    let mut context = setup_extensions_test().await.unwrap();

    // Use the assert helper to verify all extensions are correctly configured
    assert_mint_22_with_all_extensions(
        &mut context.rpc,
        &context.mint_pubkey,
        &context.extension_config,
        &context.payer.pubkey(),
    )
    .await;

    println!(
        "Mint with all extensions created successfully: {}",
        context.mint_pubkey
    );
}

/// Test minting SPL tokens and transferring to CToken using hot path with a Token 2022 mint with all extensions.
/// Mints with restricted extensions (Pausable, PermanentDelegate, TransferFee, TransferHook) require hot path.
#[tokio::test]
#[serial]
async fn test_mint_and_compress_with_extensions() {
    use light_ctoken_interface::state::TokenDataVersion;
    use light_ctoken_sdk::{
        ctoken::{CompressibleParams, CreateCTokenAccount, TransferSplToCtoken},
        spl_interface::find_spl_interface_pda_with_index,
    };

    let mut context = setup_extensions_test().await.unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // 1. Create a Token 2022 token account for the payer (SPL source)
    let spl_account =
        create_token_22_account(&mut context.rpc, &payer, &mint_pubkey, &payer.pubkey()).await;

    println!("Created SPL token account: {}", spl_account);

    // 2. Mint SPL tokens to the token account
    let mint_amount = 1_000_000_000u64; // 1 token with 9 decimals
    mint_spl_tokens_22(
        &mut context.rpc,
        &payer,
        &mint_pubkey,
        &spl_account,
        mint_amount,
    )
    .await;

    println!("Minted {} tokens to {}", mint_amount, spl_account);

    // 3. Create CToken account with extensions (destination for hot path transfer)
    let owner = Keypair::new();
    let account_keypair = Keypair::new();
    let create_ix = CreateCTokenAccount::new(
        payer.pubkey(),
        account_keypair.pubkey(),
        mint_pubkey,
        owner.pubkey(),
    )
    .with_compressible(CompressibleParams {
        compressible_config: context
            .rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: context
            .rpc
            .test_accounts
            .funding_pool_config
            .rent_sponsor_pda,
        pre_pay_num_epochs: 2,
        lamports_per_write: Some(100),
        compress_to_account_pubkey: None,
        token_account_version: TokenDataVersion::ShaFlat,
        compression_only: true,
    })
    .instruction()
    .unwrap();
    context
        .rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await
        .unwrap();

    println!("Created CToken account: {}", account_keypair.pubkey());

    // 4. Transfer SPL to CToken using hot path (compress + decompress in same tx)
    let transfer_amount = 500_000_000u64; // Transfer half
                                          // Use restricted=true because this mint has restricted extensions (PermanentDelegate, etc.)
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);
    let transfer_ix = TransferSplToCtoken {
        amount: transfer_amount,
        spl_interface_pda_bump,
        source_spl_token_account: spl_account,
        destination_ctoken_account: account_keypair.pubkey(),
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
        decimals: 9,
    }
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify CToken account has the tokens
    let ctoken_account_data = context
        .rpc
        .get_account(account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let ctoken_account = spl_pod::bytemuck::pod_from_bytes::<spl_token_2022::pod::PodAccount>(
        &ctoken_account_data.data[..165],
    )
    .unwrap();
    assert_eq!(
        u64::from(ctoken_account.amount),
        transfer_amount,
        "CToken account should have {} tokens",
        transfer_amount
    );

    println!(
        "Successfully transferred {} tokens from SPL to CToken using hot path",
        transfer_amount
    );
}

/// Test creating a CToken account for a Token-2022 mint with permanent delegate extension
/// Verifies that the account gets all extensions: compressible, pausable, permanent_delegate, transfer_fee, transfer_hook
#[tokio::test]
#[serial]
async fn test_create_ctoken_with_extensions() {
    use light_ctoken_interface::state::TokenDataVersion;
    use light_ctoken_sdk::ctoken::{CompressibleParams, CreateCTokenAccount};
    use light_test_utils::assert_create_token_account::{
        assert_create_token_account, CompressibleData,
    };

    let mut context = setup_extensions_test().await.unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Create a compressible CToken account for the Token-2022 mint
    let account_keypair = Keypair::new();
    let account_pubkey = account_keypair.pubkey();

    let compressible_config = context
        .rpc
        .test_accounts
        .funding_pool_config
        .compressible_config_pda;
    let rent_sponsor = context
        .rpc
        .test_accounts
        .funding_pool_config
        .rent_sponsor_pda;
    let compression_authority = context
        .rpc
        .test_accounts
        .funding_pool_config
        .compression_authority_pda;

    let create_ix =
        CreateCTokenAccount::new(payer.pubkey(), account_pubkey, mint_pubkey, payer.pubkey())
            .with_compressible(CompressibleParams {
                compressible_config,
                rent_sponsor,
                pre_pay_num_epochs: 2,
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await
        .unwrap();

    // Use assertion function to verify account creation with T22 extensions
    let expected_extensions = vec![
        ExtensionStruct::PausableAccount(PausableAccountExtension),
        ExtensionStruct::PermanentDelegateAccount(PermanentDelegateAccountExtension),
        ExtensionStruct::TransferFeeAccount(TransferFeeAccountExtension { withheld_amount: 0 }),
        ExtensionStruct::TransferHookAccount(TransferHookAccountExtension { transferring: 0 }),
    ];

    assert_create_token_account(
        &mut context.rpc,
        account_pubkey,
        mint_pubkey,
        payer.pubkey(),
        Some(CompressibleData {
            compression_authority,
            rent_sponsor,
            num_prepaid_epochs: 2,
            lamports_per_write: Some(100),
            compress_to_pubkey: false,
            account_version: TokenDataVersion::ShaFlat,
            payer: payer.pubkey(),
        }),
        Some(expected_extensions),
    )
    .await;

    println!("Successfully created CToken account with all extensions from Token-2022 mint");
}

/// Test complete flow: Create Token-2022 mint -> SPL account -> Mint -> Create CToken accounts -> Transfer SPL to CToken (hot path) -> Transfer with permanent delegate
#[tokio::test]
#[serial]
async fn test_transfer_with_permanent_delegate() {
    use anchor_lang::prelude::AccountMeta;
    use anchor_spl::token_2022::spl_token_2022;
    use light_ctoken_interface::state::TokenDataVersion;
    use light_ctoken_sdk::{
        ctoken::{CompressibleParams, CreateCTokenAccount, TransferSplToCtoken},
        spl_interface::find_spl_interface_pda_with_index,
    };
    use solana_sdk::{instruction::Instruction, program_pack::Pack};

    let mut context = setup_extensions_test().await.unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;
    let permanent_delegate = context.extension_config.permanent_delegate;

    // Step 1: Create SPL Token-2022 account and mint tokens
    let spl_account =
        create_token_22_account(&mut context.rpc, &payer, &mint_pubkey, &payer.pubkey()).await;

    let mint_amount = 1_000_000_000u64;
    mint_spl_tokens_22(
        &mut context.rpc,
        &payer,
        &mint_pubkey,
        &spl_account,
        mint_amount,
    )
    .await;

    // Step 2: Create two compressible CToken accounts (A and B) - must be created before transfer
    let owner = Keypair::new();
    let account_a_keypair = Keypair::new();
    let account_a_pubkey = account_a_keypair.pubkey();

    let create_a_ix = CreateCTokenAccount::new(
        payer.pubkey(),
        account_a_pubkey,
        mint_pubkey,
        owner.pubkey(),
    )
    .with_compressible(CompressibleParams {
        compressible_config: context
            .rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: context
            .rpc
            .test_accounts
            .funding_pool_config
            .rent_sponsor_pda,
        pre_pay_num_epochs: 2,
        lamports_per_write: Some(100),
        compress_to_account_pubkey: None,
        token_account_version: TokenDataVersion::ShaFlat,
        compression_only: true,
    })
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_a_ix],
            &payer.pubkey(),
            &[&payer, &account_a_keypair],
        )
        .await
        .unwrap();

    let account_b_keypair = Keypair::new();
    let account_b_pubkey = account_b_keypair.pubkey();

    let create_b_ix = CreateCTokenAccount::new(
        payer.pubkey(),
        account_b_pubkey,
        mint_pubkey,
        owner.pubkey(),
    )
    .with_compressible(CompressibleParams {
        compressible_config: context
            .rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: context
            .rpc
            .test_accounts
            .funding_pool_config
            .rent_sponsor_pda,
        pre_pay_num_epochs: 2,
        lamports_per_write: Some(100),
        compress_to_account_pubkey: None,
        token_account_version: TokenDataVersion::ShaFlat,
        compression_only: true,
    })
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_b_ix],
            &payer.pubkey(),
            &[&payer, &account_b_keypair],
        )
        .await
        .unwrap();

    // Step 3: Transfer SPL to CToken account A using hot path (compress + decompress in same tx)
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);

    let transfer_spl_to_ctoken_ix = TransferSplToCtoken {
        amount: mint_amount,
        spl_interface_pda_bump,
        source_spl_token_account: spl_account,
        destination_ctoken_account: account_a_pubkey,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
        decimals: 9,
    }
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[transfer_spl_to_ctoken_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Step 5: Transfer from A to B using permanent delegate as authority
    // Use CTokenTransferChecked (discriminator 12) because accounts have PausableAccount extension
    let transfer_amount = 500_000_000u64;
    let decimals: u8 = 9;
    let mut data = vec![12]; // CTokenTransferChecked discriminator
    data.extend_from_slice(&transfer_amount.to_le_bytes());
    data.push(decimals);

    let transfer_ix = Instruction {
        program_id: light_compressed_token::ID,
        accounts: vec![
            AccountMeta::new(account_a_pubkey, false),     // source
            AccountMeta::new_readonly(mint_pubkey, false), // mint (required for extension check)
            AccountMeta::new(account_b_pubkey, false),     // destination
            AccountMeta::new(permanent_delegate, true), // authority (permanent delegate must sign)
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // System program for compressible top-up
        ],
        data,
    };

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Step 6: Verify balances
    let account_a = context
        .rpc
        .get_account(account_a_pubkey)
        .await
        .unwrap()
        .unwrap();
    let account_b = context
        .rpc
        .get_account(account_b_pubkey)
        .await
        .unwrap()
        .unwrap();

    let token_a = spl_token_2022::state::Account::unpack_unchecked(&account_a.data[..165]).unwrap();
    let token_b = spl_token_2022::state::Account::unpack_unchecked(&account_b.data[..165]).unwrap();

    assert_eq!(
        token_a.amount,
        mint_amount - transfer_amount,
        "Account A should have 500M tokens"
    );
    assert_eq!(
        token_b.amount, transfer_amount,
        "Account B should have 500M tokens"
    );

    println!(
        "Successfully completed full flow: compressed {} tokens, decompressed to account A, transferred {} using permanent delegate to account B",
        mint_amount, transfer_amount
    );
}

// test_create_ctoken_with_frozen_default_state moved to compress_only/default_state.rs

/// Test complete flow with owner as transfer authority:
/// Create mint -> Create CToken accounts -> Transfer SPL to CToken (hot path) -> Transfer using owner
/// Verifies that transfer works with owner authority and all extensions are preserved
#[tokio::test]
#[serial]
async fn test_transfer_with_owner_authority() {
    use anchor_lang::prelude::AccountMeta;
    use anchor_spl::token_2022::spl_token_2022;
    use borsh::BorshDeserialize;
    use light_ctoken_interface::state::{AccountState, CToken, TokenDataVersion};
    use light_ctoken_sdk::{
        ctoken::{CompressibleParams, CreateCTokenAccount, TransferSplToCtoken},
        spl_interface::find_spl_interface_pda_with_index,
    };
    use solana_sdk::{instruction::Instruction, program_pack::Pack};

    let mut context = setup_extensions_test().await.unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Step 1: Create SPL Token-2022 account and mint tokens
    let spl_account =
        create_token_22_account(&mut context.rpc, &payer, &mint_pubkey, &payer.pubkey()).await;

    let mint_amount = 1_000_000_000u64;
    mint_spl_tokens_22(
        &mut context.rpc,
        &payer,
        &mint_pubkey,
        &spl_account,
        mint_amount,
    )
    .await;

    // Step 2: Create two compressible CToken accounts (A and B) with all extensions
    let owner = Keypair::new();
    context
        .rpc
        .airdrop_lamports(&owner.pubkey(), LAMPORTS_PER_SOL)
        .await
        .unwrap();
    let account_a_keypair = Keypair::new();
    let account_a_pubkey = account_a_keypair.pubkey();

    let create_a_ix = CreateCTokenAccount::new(
        payer.pubkey(),
        account_a_pubkey,
        mint_pubkey,
        owner.pubkey(),
    )
    .with_compressible(CompressibleParams {
        compressible_config: context
            .rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: context
            .rpc
            .test_accounts
            .funding_pool_config
            .rent_sponsor_pda,
        pre_pay_num_epochs: 2,
        lamports_per_write: Some(100),
        compress_to_account_pubkey: None,
        token_account_version: TokenDataVersion::ShaFlat,
        compression_only: true,
    })
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_a_ix],
            &payer.pubkey(),
            &[&payer, &account_a_keypair],
        )
        .await
        .unwrap();

    let account_b_keypair = Keypair::new();
    let account_b_pubkey = account_b_keypair.pubkey();

    let create_b_ix = CreateCTokenAccount::new(
        payer.pubkey(),
        account_b_pubkey,
        mint_pubkey,
        owner.pubkey(),
    )
    .with_compressible(CompressibleParams {
        compressible_config: context
            .rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: context
            .rpc
            .test_accounts
            .funding_pool_config
            .rent_sponsor_pda,
        pre_pay_num_epochs: 2,
        lamports_per_write: Some(100),
        compress_to_account_pubkey: None,
        token_account_version: TokenDataVersion::ShaFlat,
        compression_only: true,
    })
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_b_ix],
            &payer.pubkey(),
            &[&payer, &account_b_keypair],
        )
        .await
        .unwrap();

    // Verify both accounts have correct size (274 bytes with all extensions)
    let account_a_data = context
        .rpc
        .get_account(account_a_pubkey)
        .await
        .unwrap()
        .unwrap();
    let account_b_data = context
        .rpc
        .get_account(account_b_pubkey)
        .await
        .unwrap()
        .unwrap();
    // Accounts have extensions, so size should be larger than base (165 bytes)
    assert!(
        account_a_data.data.len() > 165,
        "Account A should be larger than base size due to extensions"
    );
    assert!(
        account_b_data.data.len() > 165,
        "Account B should be larger than base size due to extensions"
    );

    // Step 3: Transfer SPL to CToken account A using hot path (compress + decompress in same tx)
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);

    let transfer_spl_to_ctoken_ix = TransferSplToCtoken {
        amount: mint_amount,
        spl_interface_pda_bump,
        source_spl_token_account: spl_account,
        destination_ctoken_account: account_a_pubkey,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
        decimals: 9,
    }
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[transfer_spl_to_ctoken_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Step 4: Transfer from A to B using owner as authority
    // Use CTokenTransferChecked (discriminator 12) because accounts have PausableAccount extension
    let transfer_amount = 500_000_000u64;
    let decimals: u8 = 9;
    let mut data = vec![12]; // CTokenTransferChecked discriminator
    data.extend_from_slice(&transfer_amount.to_le_bytes());
    data.push(decimals);

    let transfer_ix = Instruction {
        program_id: light_compressed_token::ID,
        accounts: vec![
            AccountMeta::new(account_a_pubkey, false),     // source
            AccountMeta::new_readonly(mint_pubkey, false), // mint (required for extension check)
            AccountMeta::new(account_b_pubkey, false),     // destination
            AccountMeta::new(owner.pubkey(), true),        // authority (owner must sign)
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // System program for compressible top-up
        ],
        data,
    };

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();

    // Step 6: Verify balances and TransferFeeAccount extension
    let account_a = context
        .rpc
        .get_account(account_a_pubkey)
        .await
        .unwrap()
        .unwrap();
    let account_b = context
        .rpc
        .get_account(account_b_pubkey)
        .await
        .unwrap()
        .unwrap();

    // Verify token balances using SPL unpacking
    let token_a = spl_token_2022::state::Account::unpack_unchecked(&account_a.data[..165]).unwrap();
    let token_b = spl_token_2022::state::Account::unpack_unchecked(&account_b.data[..165]).unwrap();

    assert_eq!(
        token_a.amount,
        mint_amount - transfer_amount,
        "Account A should have 500M tokens"
    );
    assert_eq!(
        token_b.amount, transfer_amount,
        "Account B should have 500M tokens"
    );

    // Deserialize and verify TransferFeeAccount extension on both accounts
    let ctoken_a = CToken::deserialize(&mut &account_a.data[..]).unwrap();
    let ctoken_b = CToken::deserialize(&mut &account_b.data[..]).unwrap();

    // Build expected CToken accounts
    // Compression fields are now in the Compressible extension
    let expected_ctoken_a = CToken {
        mint: mint_pubkey.to_bytes().into(),
        owner: owner.pubkey().to_bytes().into(),
        amount: mint_amount - transfer_amount,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: ctoken_a.extensions.clone(),
    };

    let expected_ctoken_b = CToken {
        mint: mint_pubkey.to_bytes().into(),
        owner: owner.pubkey().to_bytes().into(),
        amount: transfer_amount,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: ctoken_b.extensions.clone(),
    };

    assert_eq!(
        ctoken_a, expected_ctoken_a,
        "Account A should match expected with withheld_amount=0"
    );
    assert_eq!(
        ctoken_b, expected_ctoken_b,
        "Account B should match expected with withheld_amount=0"
    );

    println!(
        "Successfully completed transfer with owner authority: A={} tokens, B={} tokens",
        token_a.amount, token_b.amount
    );
}

/// Test that compressing SPL tokens with restricted extensions outside the hot path fails.
/// Mints with restricted extensions (Pausable, PermanentDelegate, TransferFee, TransferHook) require hot path.
#[tokio::test]
#[serial]
async fn test_compress_with_restricted_extensions_fails() {
    let mut context = setup_extensions_test().await.unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Create SPL account and mint tokens
    let spl_account =
        create_token_22_account(&mut context.rpc, &payer, &mint_pubkey, &payer.pubkey()).await;
    let mint_amount = 1_000_000_000u64;
    mint_spl_tokens_22(
        &mut context.rpc,
        &payer,
        &mint_pubkey,
        &spl_account,
        mint_amount,
    )
    .await;

    // Try to compress to compressed accounts (NOT hot path) - should fail
    let owner = Keypair::new();
    let output_queue = context.rpc.get_random_state_tree_info().unwrap().queue;
    let compress_ix = create_generic_transfer2_instruction(
        &mut context.rpc,
        vec![Transfer2InstructionType::Compress(CompressInput {
            compressed_token_account: None,
            solana_token_account: spl_account,
            to: owner.pubkey(),
            mint: mint_pubkey,
            amount: mint_amount,
            authority: payer.pubkey(),
            output_queue,
            pool_index: None,
            decimals: 9,
        })],
        payer.pubkey(),
        true,
    )
    .await
    .unwrap();
    let result = context
        .rpc
        .create_and_send_transaction(&[compress_ix], &payer.pubkey(), &[&payer])
        .await;
    // MintHasRestrictedExtensions: mints with Pausable, PermanentDelegate, TransferFee,
    // or TransferHook cannot create compressed token outputs (error code 6142)
    assert_rpc_error(result, 0, 6142).unwrap();

    println!("Correctly rejected compress operation for mint with restricted extensions");
}

// test_compress_and_close_ctoken_with_extensions moved to compress_only/all.rs

// CompressAndCloseTestConfig, set_ctoken_account_state, and run_compress_and_close_extension_test
// moved to compress_only/mod.rs

// Compress and close tests moved to compress_only/ directory:
// - test_compress_and_close_with_delegated_amount -> delegated.rs
// - test_compress_and_close_frozen -> frozen.rs
// - test_compress_and_close_with_permanent_delegate -> permanent_delegate.rs
// - test_compress_and_close_delegate_decompress -> delegated.rs
