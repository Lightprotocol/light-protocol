//! Tests for Token 2022 mint with multiple extensions
//!
//! This module tests the creation and verification of Token 2022 mints
//! with all supported extensions.

use borsh::BorshDeserialize;
use light_ctoken_interface::state::{
    AccountState, CToken, PausableAccountExtension, PermanentDelegateAccountExtension,
    TransferFeeAccountExtension, TransferHookAccountExtension,
};
use light_program_test::{
    program_test::TestRpc, utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig,
};
use light_test_utils::{
    mint_2022::{
        create_mint_22_with_extensions, create_mint_22_with_frozen_default_state,
        create_token_22_account, mint_spl_tokens_22, verify_mint_extensions,
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
    let mut context = setup_extensions_test().await.unwrap();

    // Verify all extensions are present
    verify_mint_extensions(&mut context.rpc, &context.mint_pubkey)
        .await
        .unwrap();

    // Verify the extension config has correct values
    assert_eq!(context.extension_config.mint, context.mint_pubkey);

    // Verify token pool was created
    let token_pool_account = context
        .rpc
        .get_account(context.extension_config.token_pool)
        .await
        .unwrap();
    assert!(
        token_pool_account.is_some(),
        "Token pool account should exist"
    );

    assert_eq!(
        context.extension_config.close_authority,
        context.payer.pubkey()
    );
    assert_eq!(
        context.extension_config.transfer_fee_config_authority,
        context.payer.pubkey()
    );
    assert_eq!(
        context.extension_config.withdraw_withheld_authority,
        context.payer.pubkey()
    );
    assert_eq!(
        context.extension_config.permanent_delegate,
        context.payer.pubkey()
    );
    assert_eq!(
        context.extension_config.metadata_update_authority,
        context.payer.pubkey()
    );
    assert_eq!(
        context.extension_config.pause_authority,
        context.payer.pubkey()
    );
    assert_eq!(
        context.extension_config.confidential_transfer_authority,
        context.payer.pubkey()
    );
    assert_eq!(
        context.extension_config.confidential_transfer_fee_authority,
        context.payer.pubkey()
    );

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
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0);
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
    use borsh::BorshDeserialize;
    use light_ctoken_interface::state::{
        AccountState, CToken, ExtensionStruct, PausableAccountExtension,
        PermanentDelegateAccountExtension, TokenDataVersion, TransferFeeAccountExtension,
        TransferHookAccountExtension,
    };
    use light_ctoken_sdk::ctoken::{CompressibleParams, CreateCTokenAccount};

    let mut context = setup_extensions_test().await.unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Create a compressible CToken account for the Token-2022 mint
    let account_keypair = Keypair::new();
    let account_pubkey = account_keypair.pubkey();

    let create_ix =
        CreateCTokenAccount::new(payer.pubkey(), account_pubkey, mint_pubkey, payer.pubkey())
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

    // Verify account was created with correct size (273 bytes)
    let account = context
        .rpc
        .get_account(account_pubkey)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        account.data.len(),
        274,
        "CToken account should be 274 bytes (165 base + 7 metadata + 89 compressible + 1 pausable + 1 permanent_delegate + 9 transfer_fee + 2 transfer_hook)"
    );

    // Deserialize the CToken account
    let ctoken =
        CToken::deserialize(&mut &account.data[..]).expect("Failed to deserialize CToken account");

    // Extract CompressionInfo from the deserialized account (contains runtime-specific values)
    let compression_info = ctoken
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ExtensionStruct::Compressible(info) => Some(*info),
                _ => None,
            })
        })
        .expect("Should have Compressible extension");

    // Build expected CToken account for comparison
    let expected_ctoken = CToken {
        mint: mint_pubkey.to_bytes().into(),
        owner: payer.pubkey().to_bytes().into(),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        extensions: Some(vec![
            ExtensionStruct::Compressible(compression_info),
            ExtensionStruct::PausableAccount(PausableAccountExtension),
            ExtensionStruct::PermanentDelegateAccount(PermanentDelegateAccountExtension),
            ExtensionStruct::TransferFeeAccount(TransferFeeAccountExtension { withheld_amount: 0 }),
            ExtensionStruct::TransferHookAccount(TransferHookAccountExtension { transferring: 0 }),
        ]),
    };

    assert_eq!(
        ctoken, expected_ctoken,
        "CToken account should match expected with all 5 extensions"
    );

    println!(
        "Successfully created CToken account with all 5 extensions: compressible, pausable, permanent_delegate, transfer_fee, transfer_hook"
    );
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
        find_spl_interface_pda_with_index(&mint_pubkey, 0);

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
    let transfer_amount = 500_000_000u64;
    let mut data = vec![3]; // CTokenTransfer discriminator
    data.extend_from_slice(&transfer_amount.to_le_bytes());

    let transfer_ix = Instruction {
        program_id: light_compressed_token::ID,
        accounts: vec![
            AccountMeta::new(account_a_pubkey, false),
            AccountMeta::new(account_b_pubkey, false),
            AccountMeta::new(permanent_delegate, true), // Permanent delegate must sign
            AccountMeta::new_readonly(mint_pubkey, false), // Mint required for extension check
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

/// Test creating a CToken account for a mint with DefaultAccountState set to Frozen.
/// Verifies that the account is created with state = Frozen (2) at offset 108.
#[tokio::test]
#[serial]
async fn test_create_ctoken_with_frozen_default_state() {
    use light_ctoken_interface::state::TokenDataVersion;
    use light_ctoken_sdk::ctoken::{CompressibleParams, CreateCTokenAccount};

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create mint with DefaultAccountState = Frozen
    let (mint_keypair, extension_config) =
        create_mint_22_with_frozen_default_state(&mut rpc, &payer, 9).await;
    let mint_pubkey = mint_keypair.pubkey();

    assert!(
        extension_config.default_account_state_frozen,
        "Mint should have default_account_state_frozen = true"
    );

    // Create a compressible CToken account for the frozen mint
    let account_keypair = Keypair::new();
    let account_pubkey = account_keypair.pubkey();

    let create_ix =
        CreateCTokenAccount::new(payer.pubkey(), account_pubkey, mint_pubkey, payer.pubkey())
            .with_compressible(CompressibleParams {
                compressible_config: rpc
                    .test_accounts
                    .funding_pool_config
                    .compressible_config_pda,
                rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
                pre_pay_num_epochs: 2,
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .unwrap();

    rpc.create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await
        .unwrap();

    // Verify account was created with correct size (263 bytes = 165 base + 7 metadata + 88 compressible + 2 markers)
    let account = rpc.get_account(account_pubkey).await.unwrap().unwrap();
    assert_eq!(
        account.data.len(),
        263,
        "CToken account should be 263 bytes"
    );

    // Deserialize the CToken account using borsh
    use borsh::BorshDeserialize;
    use light_ctoken_interface::state::{
        AccountState, CToken, ExtensionStruct, PausableAccountExtension,
        PermanentDelegateAccountExtension,
    };

    let ctoken =
        CToken::deserialize(&mut &account.data[..]).expect("Failed to deserialize CToken account");

    // Extract CompressionInfo from the deserialized account (contains runtime-specific values)
    let compression_info = ctoken
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ExtensionStruct::Compressible(info) => Some(*info),
                _ => None,
            })
        })
        .expect("Should have Compressible extension");

    // Build expected CToken account for comparison
    let expected_ctoken = CToken {
        mint: mint_pubkey.to_bytes().into(),
        owner: payer.pubkey().to_bytes().into(),
        amount: 0,
        delegate: None,
        state: AccountState::Frozen,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        extensions: Some(vec![
            ExtensionStruct::Compressible(compression_info),
            ExtensionStruct::PausableAccount(PausableAccountExtension),
            ExtensionStruct::PermanentDelegateAccount(PermanentDelegateAccountExtension),
        ]),
    };

    assert_eq!(
        ctoken, expected_ctoken,
        "CToken account should match expected"
    );

    println!(
        "Successfully created frozen CToken account: state={:?}, extensions={}",
        ctoken.state,
        ctoken.extensions.as_ref().map(|e| e.len()).unwrap_or(0)
    );
}

/// Test complete flow with owner as transfer authority:
/// Create mint -> Create CToken accounts -> Transfer SPL to CToken (hot path) -> Transfer using owner
/// Verifies that transfer works with owner authority and all extensions are preserved
#[tokio::test]
#[serial]
async fn test_transfer_with_owner_authority() {
    use anchor_lang::prelude::AccountMeta;
    use anchor_spl::token_2022::spl_token_2022;
    use borsh::BorshDeserialize;
    use light_ctoken_interface::state::{
        AccountState, CToken, ExtensionStruct, PausableAccountExtension,
        PermanentDelegateAccountExtension, TokenDataVersion, TransferFeeAccountExtension,
        TransferHookAccountExtension,
    };
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
    assert_eq!(account_a_data.data.len(), 274);
    assert_eq!(account_b_data.data.len(), 274);

    // Step 3: Transfer SPL to CToken account A using hot path (compress + decompress in same tx)
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0);

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
    let transfer_amount = 500_000_000u64;
    let mut data = vec![3]; // CTokenTransfer discriminator
    data.extend_from_slice(&transfer_amount.to_le_bytes());

    let transfer_ix = Instruction {
        program_id: light_compressed_token::ID,
        accounts: vec![
            AccountMeta::new(account_a_pubkey, false),
            AccountMeta::new(account_b_pubkey, false),
            AccountMeta::new(owner.pubkey(), true), // Owner must sign
            AccountMeta::new_readonly(mint_pubkey, false), // Mint required for extension check
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

    // Extract CompressionInfo from account A
    let compression_info_a = ctoken_a
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ExtensionStruct::Compressible(info) => Some(*info),
                _ => None,
            })
        })
        .expect("Account A should have Compressible extension");

    // Extract CompressionInfo from account B
    let compression_info_b = ctoken_b
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ExtensionStruct::Compressible(info) => Some(*info),
                _ => None,
            })
        })
        .expect("Account B should have Compressible extension");

    // Build expected CToken accounts
    let expected_ctoken_a = CToken {
        mint: mint_pubkey.to_bytes().into(),
        owner: owner.pubkey().to_bytes().into(),
        amount: mint_amount - transfer_amount,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        extensions: Some(vec![
            ExtensionStruct::Compressible(compression_info_a),
            ExtensionStruct::PausableAccount(PausableAccountExtension),
            ExtensionStruct::PermanentDelegateAccount(PermanentDelegateAccountExtension),
            ExtensionStruct::TransferFeeAccount(TransferFeeAccountExtension { withheld_amount: 0 }),
            ExtensionStruct::TransferHookAccount(TransferHookAccountExtension { transferring: 0 }),
        ]),
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
        extensions: Some(vec![
            ExtensionStruct::Compressible(compression_info_b),
            ExtensionStruct::PausableAccount(PausableAccountExtension),
            ExtensionStruct::PermanentDelegateAccount(PermanentDelegateAccountExtension),
            ExtensionStruct::TransferFeeAccount(TransferFeeAccountExtension { withheld_amount: 0 }),
            ExtensionStruct::TransferHookAccount(TransferHookAccountExtension { transferring: 0 }),
        ]),
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

/// Test that forester can compress and close a CToken account with Token-2022 extensions
/// after prepaid epochs expire, and then decompress it back to a CToken account.
#[tokio::test]
#[serial]
async fn test_compress_and_close_ctoken_with_extensions() {
    #[allow(unused_imports)]
    use light_client::indexer::CompressedTokenAccount;
    use light_client::indexer::Indexer;
    use light_ctoken_interface::{
        instructions::extensions::{
            CompressedOnlyExtensionInstructionData, ExtensionInstructionData,
        },
        state::TokenDataVersion,
    };
    use light_ctoken_sdk::{
        ctoken::{CompressibleParams, CreateCTokenAccount, TransferSplToCtoken},
        spl_interface::find_spl_interface_pda_with_index,
    };
    use light_token_client::instructions::transfer2::{
        create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
    };

    let mut context = setup_extensions_test().await.unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // 1. Create SPL Token-2022 account and mint tokens
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

    // 2. Create CToken account with 0 prepaid epochs (immediately compressible)
    let owner = Keypair::new();
    let account_keypair = Keypair::new();
    let ctoken_account = account_keypair.pubkey();

    let create_ix =
        CreateCTokenAccount::new(payer.pubkey(), ctoken_account, mint_pubkey, owner.pubkey())
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
                pre_pay_num_epochs: 0, // Immediately compressible after 1 epoch
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

    // 3. Transfer tokens to CToken using hot path (required for mints with restricted extensions)
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0);
    let transfer_ix = TransferSplToCtoken {
        amount: mint_amount,
        spl_interface_pda_bump,
        decimals: 9,
        source_spl_token_account: spl_account,
        destination_ctoken_account: ctoken_account,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify tokens are in the CToken account
    let account_before = context
        .rpc
        .get_account(ctoken_account)
        .await
        .unwrap()
        .unwrap();
    assert!(
        account_before.lamports > 0,
        "Account should exist before compression"
    );

    // 4. Advance 2 epochs to trigger forester compression
    // Account created with 0 prepaid epochs needs time to become compressible
    context.rpc.warp_epoch_forward(30).await.unwrap();

    // 5. Assert the account has been compressed (closed) and compressed token account exists
    let account_after = context.rpc.get_account(ctoken_account).await.unwrap();
    assert!(
        account_after.is_none() || account_after.unwrap().lamports == 0,
        "CToken account should be closed"
    );

    let compressed_accounts = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        compressed_accounts.len(),
        1,
        "Should have exactly 1 compressed token account"
    );

    // Build expected TokenData with CompressedOnly extension
    // The CToken had marker extensions (PausableAccount, PermanentDelegateAccount),
    // so the compressed token should have CompressedOnly TLV extension
    use light_ctoken_interface::state::{
        CompressedOnlyExtension, CompressedTokenAccountState, ExtensionStruct, TokenData,
    };

    let expected_token_data = TokenData {
        mint: mint_pubkey.into(),
        owner: owner.pubkey().into(),
        amount: mint_amount,
        delegate: None,
        state: CompressedTokenAccountState::Initialized as u8,
        tlv: Some(vec![ExtensionStruct::CompressedOnly(
            CompressedOnlyExtension {
                delegated_amount: 0,
                withheld_transfer_fee: 0,
            },
        )]),
    };

    assert_eq!(
        compressed_accounts[0].token,
        expected_token_data.into(),
        "Compressed token account should match expected TokenData"
    );

    // 6. Create a new CToken account for decompress destination
    let decompress_dest_keypair = Keypair::new();
    let decompress_dest_account = decompress_dest_keypair.pubkey();

    let create_dest_ix = CreateCTokenAccount::new(
        payer.pubkey(),
        decompress_dest_account,
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
        pre_pay_num_epochs: 2, // More epochs so account won't be compressed again
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
            &[create_dest_ix],
            &payer.pubkey(),
            &[&payer, &decompress_dest_keypair],
        )
        .await
        .unwrap();

    println!(
        "Created decompress destination CToken account: {}",
        decompress_dest_account
    );

    // 7. Decompress the compressed account back to the new CToken account
    // Need to include in_tlv for the CompressedOnly extension
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
        },
    )]];

    let decompress_ix = create_generic_transfer2_instruction(
        &mut context.rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: vec![compressed_accounts[0].clone()],
            decompress_amount: mint_amount,
            solana_token_account: decompress_dest_account,
            amount: mint_amount,
            pool_index: None,
            decimals: 9,
            in_tlv: Some(in_tlv),
        })],
        payer.pubkey(),
        true,
    )
    .await
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();

    // 8. Verify the CToken account has the tokens and proper extension state

    let dest_account_data = context
        .rpc
        .get_account(decompress_dest_account)
        .await
        .unwrap()
        .unwrap();

    let dest_ctoken = CToken::deserialize(&mut &dest_account_data.data[..])
        .expect("Failed to deserialize destination CToken account");

    // Extract CompressionInfo for comparison (it has runtime values)
    let compression_info = dest_ctoken
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ExtensionStruct::Compressible(info) => Some(*info),
                _ => None,
            })
        })
        .expect("Should have Compressible extension");

    // Build expected CToken account
    let expected_dest_ctoken = CToken {
        mint: mint_pubkey.to_bytes().into(),
        owner: owner.pubkey().to_bytes().into(),
        amount: mint_amount,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        extensions: Some(vec![
            ExtensionStruct::Compressible(compression_info),
            ExtensionStruct::PausableAccount(PausableAccountExtension),
            ExtensionStruct::PermanentDelegateAccount(PermanentDelegateAccountExtension),
            ExtensionStruct::TransferFeeAccount(TransferFeeAccountExtension { withheld_amount: 0 }),
            ExtensionStruct::TransferHookAccount(TransferHookAccountExtension { transferring: 0 }),
        ]),
    };

    assert_eq!(
        dest_ctoken, expected_dest_ctoken,
        "Decompressed CToken account should match expected with all extensions"
    );

    // Verify no more compressed accounts for this owner
    let remaining_compressed = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        remaining_compressed.len(),
        0,
        "Should have no more compressed token accounts after full decompress"
    );

    println!(
        "Successfully completed compress-and-close -> decompress cycle with extension state transfer"
    );
}

/// Configuration for parameterized compress and close extension tests
#[derive(Debug, Clone)]
struct CompressAndCloseTestConfig {
    /// Set delegate and delegated_amount before compress (delegate pubkey, amount)
    delegate_config: Option<(Pubkey, u64)>,
    /// Set account state to frozen before compress
    is_frozen: bool,
    /// Use permanent delegate as authority for decompress (instead of owner)
    use_permanent_delegate_for_decompress: bool,
}

/// Helper to modify CToken account state for testing using set_account
/// Only modifies the SPL token portion (first 165 bytes) - CToken::deserialize reads from there
async fn set_ctoken_account_state(
    rpc: &mut LightProgramTest,
    account_pubkey: Pubkey,
    delegate: Option<Pubkey>,
    delegated_amount: u64,
    is_frozen: bool,
) -> Result<(), RpcError> {
    use anchor_spl::token_2022::spl_token_2022;
    use solana_sdk::{program_option::COption, program_pack::Pack};

    let mut account_info = rpc
        .get_account(account_pubkey)
        .await?
        .ok_or_else(|| RpcError::CustomError("Account not found".to_string()))?;

    // Update SPL token state (first 165 bytes)
    // CToken::deserialize reads delegate/delegated_amount/state from the SPL portion
    let mut spl_account =
        spl_token_2022::state::Account::unpack_unchecked(&account_info.data[..165])
            .map_err(|e| RpcError::CustomError(format!("Failed to unpack SPL account: {:?}", e)))?;

    spl_account.delegate = match delegate {
        Some(d) => COption::Some(d),
        None => COption::None,
    };
    spl_account.delegated_amount = delegated_amount;
    if is_frozen {
        spl_account.state = spl_token_2022::state::AccountState::Frozen;
    }

    spl_token_2022::state::Account::pack(spl_account, &mut account_info.data[..165])
        .map_err(|e| RpcError::CustomError(format!("Failed to pack SPL account: {:?}", e)))?;

    rpc.set_account(account_pubkey, account_info);
    Ok(())
}

/// Core parameterized test function for compress -> decompress cycle with configurable state
async fn run_compress_and_close_extension_test(
    config: CompressAndCloseTestConfig,
) -> Result<(), RpcError> {
    use light_client::indexer::Indexer;
    use light_ctoken_interface::{
        instructions::extensions::{
            CompressedOnlyExtensionInstructionData, ExtensionInstructionData,
        },
        state::{
            CompressedOnlyExtension, CompressedTokenAccountState, ExtensionStruct, TokenData,
            TokenDataVersion,
        },
    };
    use light_ctoken_sdk::{
        ctoken::{CompressibleParams, CreateCTokenAccount, TransferSplToCtoken},
        spl_interface::find_spl_interface_pda_with_index,
    };
    use light_token_client::instructions::transfer2::{
        create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
    };

    let mut context = setup_extensions_test().await?;
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;
    let _permanent_delegate = context.extension_config.permanent_delegate;

    // 1. Create SPL Token-2022 account and mint tokens
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

    // 2. Create CToken account with 0 prepaid epochs (immediately compressible)
    let owner = Keypair::new();
    let account_keypair = Keypair::new();
    let ctoken_account = account_keypair.pubkey();

    let create_ix =
        CreateCTokenAccount::new(payer.pubkey(), ctoken_account, mint_pubkey, owner.pubkey())
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
                pre_pay_num_epochs: 0,
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {:?}", e)))?;

    context
        .rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await?;

    // 3. Transfer tokens to CToken using hot path
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0);
    let transfer_ix = TransferSplToCtoken {
        amount: mint_amount,
        spl_interface_pda_bump,
        decimals: 9,
        source_spl_token_account: spl_account,
        destination_ctoken_account: ctoken_account,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .map_err(|e| {
        RpcError::CustomError(format!("Failed to create transfer instruction: {:?}", e))
    })?;

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await?;

    // 4. Modify CToken state based on config BEFORE warp
    let delegate_pubkey = config.delegate_config.map(|(d, _)| d);
    let delegated_amount = config.delegate_config.map(|(_, a)| a).unwrap_or(0);

    if config.delegate_config.is_some() || config.is_frozen {
        set_ctoken_account_state(
            &mut context.rpc,
            ctoken_account,
            delegate_pubkey,
            delegated_amount,
            config.is_frozen,
        )
        .await?;
    }

    // 5. Warp epoch to trigger forester compression
    context.rpc.warp_epoch_forward(30).await?;

    // 6. Assert the account has been compressed (closed)
    let account_after = context.rpc.get_account(ctoken_account).await?;
    assert!(
        account_after.is_none() || account_after.unwrap().lamports == 0,
        "CToken account should be closed after compression"
    );

    // 7. Get compressed accounts and verify state
    let compressed_accounts = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await?
        .value
        .items;

    assert_eq!(
        compressed_accounts.len(),
        1,
        "Should have exactly 1 compressed token account"
    );

    // Build expected TokenData based on config
    let expected_state = if config.is_frozen {
        CompressedTokenAccountState::Frozen as u8
    } else {
        CompressedTokenAccountState::Initialized as u8
    };

    let expected_token_data = TokenData {
        mint: mint_pubkey.into(),
        owner: owner.pubkey().into(),
        amount: mint_amount,
        delegate: delegate_pubkey.map(|d| d.into()),
        state: expected_state,
        tlv: Some(vec![ExtensionStruct::CompressedOnly(
            CompressedOnlyExtension {
                delegated_amount,
                withheld_transfer_fee: 0,
            },
        )]),
    };

    assert_eq!(
        compressed_accounts[0].token,
        expected_token_data.into(),
        "Compressed token account should match expected TokenData with config: {:?}",
        config
    );

    // 8. Create destination CToken account for decompress
    let decompress_dest_keypair = Keypair::new();
    let decompress_dest_account = decompress_dest_keypair.pubkey();

    let create_dest_ix = CreateCTokenAccount::new(
        payer.pubkey(),
        decompress_dest_account,
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
    .map_err(|e| RpcError::CustomError(format!("Failed to create dest instruction: {:?}", e)))?;

    context
        .rpc
        .create_and_send_transaction(
            &[create_dest_ix],
            &payer.pubkey(),
            &[&payer, &decompress_dest_keypair],
        )
        .await?;

    // 9. Decompress with correct in_tlv including is_frozen
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount,
            withheld_transfer_fee: 0,
            is_frozen: config.is_frozen,
        },
    )]];

    let mut decompress_ix = create_generic_transfer2_instruction(
        &mut context.rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: vec![compressed_accounts[0].clone()],
            decompress_amount: mint_amount,
            solana_token_account: decompress_dest_account,
            amount: mint_amount,
            pool_index: None,
            decimals: 9,
            in_tlv: Some(in_tlv),
        })],
        payer.pubkey(),
        true,
    )
    .await
    .map_err(|e| {
        RpcError::CustomError(format!("Failed to create decompress instruction: {:?}", e))
    })?;

    // 10. Sign with owner or permanent delegate based on config
    let signers: Vec<&Keypair> = if config.use_permanent_delegate_for_decompress {
        // Permanent delegate is the payer in this test setup.
        // Find owner in account metas and set is_signer = false since permanent delegate acts on behalf.
        let owner_pubkey = owner.pubkey();
        for account_meta in decompress_ix.accounts.iter_mut() {
            if account_meta.pubkey == owner_pubkey {
                account_meta.is_signer = false;
            }
        }
        vec![&payer]
    } else {
        vec![&payer, &owner]
    };

    context
        .rpc
        .create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &signers)
        .await?;

    // 11. Verify decompressed CToken state
    let dest_account_data = context
        .rpc
        .get_account(decompress_dest_account)
        .await?
        .ok_or_else(|| RpcError::CustomError("Dest account not found".to_string()))?;

    let dest_ctoken = CToken::deserialize(&mut &dest_account_data.data[..])
        .map_err(|e| RpcError::CustomError(format!("Failed to deserialize CToken: {:?}", e)))?;

    // Verify state matches config
    let expected_ctoken_state = if config.is_frozen {
        AccountState::Frozen
    } else {
        AccountState::Initialized
    };

    assert_eq!(
        dest_ctoken.state, expected_ctoken_state,
        "Decompressed CToken state should match config"
    );

    assert_eq!(
        dest_ctoken.delegated_amount, delegated_amount,
        "Decompressed CToken delegated_amount should match"
    );

    if let Some((delegate, _)) = config.delegate_config {
        assert_eq!(
            dest_ctoken.delegate,
            Some(delegate.to_bytes().into()),
            "Decompressed CToken delegate should match"
        );
    } else {
        assert!(
            dest_ctoken.delegate.is_none(),
            "Decompressed CToken should have no delegate"
        );
    }

    // 12. Verify no more compressed accounts
    let remaining_compressed = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await?
        .value
        .items;

    assert_eq!(
        remaining_compressed.len(),
        0,
        "Should have no more compressed token accounts after decompress"
    );

    println!(
        "Successfully completed compress-and-close -> decompress cycle with config: {:?}",
        config
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_compress_and_close_with_delegated_amount() {
    let delegate = Keypair::new();
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        delegate_config: Some((delegate.pubkey(), 500_000_000)),
        is_frozen: false,
        use_permanent_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}

#[tokio::test]
#[serial]
async fn test_compress_and_close_frozen() {
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        delegate_config: None,
        is_frozen: true,
        use_permanent_delegate_for_decompress: false,
    })
    .await
    .unwrap();
}

#[tokio::test]
#[serial]
async fn test_compress_and_close_with_permanent_delegate() {
    run_compress_and_close_extension_test(CompressAndCloseTestConfig {
        delegate_config: None,
        is_frozen: false,
        use_permanent_delegate_for_decompress: true,
    })
    .await
    .unwrap();
}
