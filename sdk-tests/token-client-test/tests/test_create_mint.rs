//! Tests for the create_mint action in light-token-client.

use borsh::BorshDeserialize;
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token::instruction::find_mint_address;
use light_token_client::actions::{CreateMint, TokenMetadata};
use light_token_interface::state::Mint;
use solana_sdk::{signature::Keypair, signer::Signer};

/// Test creating a new mint using the create_mint action with all fields.
#[tokio::test]
async fn test_create_mint_with_metadata() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let decimals = 9u8;
    let freeze_authority = payer.pubkey();
    let seed = Keypair::new();

    // Create mint with all fields
    let (_, mint) = CreateMint {
        decimals,
        freeze_authority: Some(freeze_authority),
        token_metadata: Some(TokenMetadata {
            name: "Test Token".to_string(),
            symbol: "TEST".to_string(),
            uri: "https://example.com/metadata.json".to_string(),
            update_authority: Some(payer.pubkey()),
            additional_metadata: Some(vec![
                ("key1".to_string(), "value1".to_string()),
                ("key2".to_string(), "value2".to_string()),
            ]),
        }),
        seed: Some(seed),
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Verify the mint was created
    let mint_account = rpc.get_account(mint).await.unwrap();
    assert!(mint_account.is_some(), "Mint account should exist");

    let mint_data = mint_account.unwrap();
    let mint_state = Mint::deserialize(&mut &mint_data.data[..]).unwrap();

    // Verify mint fields
    assert_eq!(mint_state.base.decimals, decimals);
    assert_eq!(
        mint_state.base.mint_authority,
        Some(payer.pubkey().to_bytes().into())
    );
    assert_eq!(
        mint_state.base.freeze_authority,
        Some(freeze_authority.to_bytes().into())
    );
    assert_eq!(mint_state.base.supply, 0);
    assert!(mint_state.base.is_initialized);

    // Verify metadata
    assert_eq!(mint_state.metadata.mint.to_bytes(), mint.to_bytes());
    assert!(mint_state.metadata.mint_decompressed);
}

/// Test creating a mint with freeze authority.
#[tokio::test]
async fn test_create_mint_with_freeze_authority() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let decimals = 6u8;
    let freeze_authority = payer.pubkey();

    // Create mint with freeze authority
    let (_, mint) = CreateMint {
        decimals,
        freeze_authority: Some(freeze_authority),
        ..Default::default()
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Verify the mint was created
    let mint_account = rpc.get_account(mint).await.unwrap();
    assert!(mint_account.is_some(), "Mint account should exist");

    let mint_data = mint_account.unwrap();
    let mint_state = Mint::deserialize(&mut &mint_data.data[..]).unwrap();

    // Verify freeze authority
    assert_eq!(
        mint_state.base.freeze_authority,
        Some(freeze_authority.to_bytes().into())
    );
}

/// Test creating a mint with deterministic seed.
#[tokio::test]
async fn test_create_mint_with_seed() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let seed = Keypair::new();
    let expected_mint = find_mint_address(&seed.pubkey()).0;

    // Create mint with explicit seed
    let (_, mint) = CreateMint {
        decimals: 9,
        seed: Some(seed),
        ..Default::default()
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Verify the mint address matches the expected derived address
    assert_eq!(mint, expected_mint);

    // Verify the mint was created
    let mint_account = rpc.get_account(mint).await.unwrap();
    assert!(mint_account.is_some(), "Mint account should exist");
}

/// Test creating multiple mints.
#[tokio::test]
async fn test_create_multiple_mints() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create first mint
    let (_, mint1) = CreateMint {
        decimals: 9,
        ..Default::default()
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Create second mint
    let (_, mint2) = CreateMint {
        decimals: 6,
        ..Default::default()
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Verify both mints are different
    assert_ne!(mint1, mint2, "Mints should be different");

    // Verify both mints exist
    assert!(rpc.get_account(mint1).await.unwrap().is_some());
    assert!(rpc.get_account(mint2).await.unwrap().is_some());
}
