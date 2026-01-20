//! Tests for DefaultAccountState extension behavior.
//!
//! This module tests the compress_only behavior with mints that have
//! the DefaultAccountState extension set to either Initialized or Frozen.

use borsh::BorshDeserialize;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    mint_2022::{create_mint_22_with_extension_types, create_mint_22_with_frozen_default_state},
    Rpc,
};
use light_token_interface::state::{AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT};
use serial_test::serial;
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token_2022::extension::ExtensionType;

/// Test creating a Light Token account for a mint with DefaultAccountState set to Frozen.
/// Verifies that the account is created with state = Frozen (2) at offset 108.
#[tokio::test]
#[serial]
async fn test_create_ctoken_with_frozen_default_state() {
    use light_token::instruction::{CompressibleParams, CreateTokenAccount};
    use light_token_interface::state::TokenDataVersion;

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

    // Create a compressible Light Token account for the frozen mint
    let account_keypair = Keypair::new();
    let account_pubkey = account_keypair.pubkey();

    let create_ix =
        CreateTokenAccount::new(payer.pubkey(), account_pubkey, mint_pubkey, payer.pubkey())
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

    // Verify account was created with correct size (274 bytes = 166 base + 7 metadata + 98 compressible + 3 markers)
    let account = rpc.get_account(account_pubkey).await.unwrap().unwrap();
    assert_eq!(
        account.data.len(),
        274,
        "Light Token account should be 274 bytes"
    );

    // Deserialize the Light Token account using borsh
    let ctoken =
        Token::deserialize(&mut &account.data[..]).expect("Failed to deserialize Token account");

    // Build expected Light Token account for comparison
    // Compression fields are now in the Compressible extension
    let expected_token = Token {
        mint: mint_pubkey.to_bytes().into(),
        owner: payer.pubkey().to_bytes().into(),
        amount: 0,
        delegate: None,
        state: AccountState::Frozen,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        // Extensions include Compressible, PausableAccount, PermanentDelegateAccount
        extensions: ctoken.extensions.clone(),
    };

    assert_eq!(
        ctoken, expected_token,
        "Light Token account should match expected"
    );

    println!(
        "Successfully created frozen Light Token account: state={:?}, extensions={}",
        ctoken.state,
        ctoken.extensions.as_ref().map(|e| e.len()).unwrap_or(0)
    );
}

/// Test creating a Light Token account for a mint with DefaultAccountState set to Initialized.
/// Verifies that the account is created with state = Initialized (1).
#[tokio::test]
#[serial]
async fn test_create_ctoken_with_initialized_default_state() {
    use light_token::instruction::{CompressibleParams, CreateTokenAccount};
    use light_token_interface::state::TokenDataVersion;

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create mint with DefaultAccountState = Initialized (non-frozen)
    let (mint_keypair, extension_config) = create_mint_22_with_extension_types(
        &mut rpc,
        &payer,
        9,
        &[ExtensionType::DefaultAccountState],
    )
    .await;
    let mint_pubkey = mint_keypair.pubkey();

    assert!(
        !extension_config.default_account_state_frozen,
        "Mint should have default_account_state_frozen = false"
    );

    // Create a compressible Light Token account
    let account_keypair = Keypair::new();
    let account_pubkey = account_keypair.pubkey();

    let create_ix =
        CreateTokenAccount::new(payer.pubkey(), account_pubkey, mint_pubkey, payer.pubkey())
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
                compression_only: true, // DefaultAccountState is a restricted extension
            })
            .instruction()
            .unwrap();

    rpc.create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await
        .unwrap();

    // Verify account was created
    let account = rpc.get_account(account_pubkey).await.unwrap().unwrap();

    // Deserialize the Light Token account using borsh
    let ctoken =
        Token::deserialize(&mut &account.data[..]).expect("Failed to deserialize Token account");

    // Build expected Light Token account for comparison
    // Extensions include Compressible (for compression fields)
    let expected_token = Token {
        mint: mint_pubkey.to_bytes().into(),
        owner: payer.pubkey().to_bytes().into(),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: ctoken.extensions.clone(),
    };

    assert_eq!(
        ctoken, expected_token,
        "Light Token account should match expected"
    );

    println!(
        "Successfully created initialized Light Token account: state={:?}",
        ctoken.state
    );
}
