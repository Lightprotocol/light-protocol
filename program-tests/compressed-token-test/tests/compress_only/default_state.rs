//! Tests for DefaultAccountState extension behavior.
//!
//! This module tests the compress_only behavior with mints that have
//! the DefaultAccountState extension.

use borsh::BorshDeserialize;
use light_ctoken_interface::state::{
    AccountState, CToken, ExtensionStruct, PausableAccountExtension,
    PermanentDelegateAccountExtension, ACCOUNT_TYPE_TOKEN_ACCOUNT,
};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{mint_2022::create_mint_22_with_frozen_default_state, Rpc};
use serial_test::serial;
use solana_sdk::{signature::Keypair, signer::Signer};

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

    // Verify account was created with correct size (264 bytes = 166 base + 7 metadata + 88 compressible + 2 markers)
    let account = rpc.get_account(account_pubkey).await.unwrap().unwrap();
    assert_eq!(
        account.data.len(),
        264,
        "CToken account should be 264 bytes"
    );

    // Deserialize the CToken account using borsh
    let ctoken =
        CToken::deserialize(&mut &account.data[..]).expect("Failed to deserialize CToken account");

    // Build expected CToken account for comparison
    // compression is now a direct field on CToken
    let expected_ctoken = CToken {
        mint: mint_pubkey.to_bytes().into(),
        owner: payer.pubkey().to_bytes().into(),
        amount: 0,
        delegate: None,
        state: AccountState::Frozen,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        decimals: ctoken.decimals,
        compression_only: ctoken.compression_only,
        compression: ctoken.compression,
        extensions: Some(vec![
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
