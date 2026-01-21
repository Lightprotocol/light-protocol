//! Tests for the wrap and unwrap actions in light-token-client.
//!
//! These tests verify:
//! - Wrapping SPL tokens into Light Token accounts
//! - Unwrapping Light Tokens back to SPL token accounts

use borsh::BorshDeserialize;
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::spl::{
    create_mint_helper, create_token_account, mint_spl_tokens, CREATE_MINT_HELPER_DECIMALS,
};
use light_token::instruction::derive_token_ata;
use light_token_client::actions::{CreateAta, Unwrap, Wrap};
use light_token_interface::state::Token;
use solana_sdk::{program_pack::Pack, signature::Keypair, signer::Signer};
use spl_token::state::Account as SplTokenAccount;

/// Test wrapping SPL tokens into a Light Token account.
#[tokio::test]
async fn test_wrap_basic() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let decimals = CREATE_MINT_HELPER_DECIMALS;

    // Create SPL mint
    let mint = create_mint_helper(&mut rpc, &payer).await;

    // Create SPL token account for payer
    let spl_token_account = Keypair::new();
    create_token_account(&mut rpc, &mint, &spl_token_account, &payer)
        .await
        .unwrap();

    // Mint SPL tokens
    let mint_amount = 1000u64;
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_token_account.pubkey(),
        &payer.pubkey(),
        &payer,
        mint_amount,
        false,
    )
    .await
    .unwrap();

    // Create Light Token ATA for destination
    let owner = payer.pubkey();
    let (light_token_ata, _) = derive_token_ata(&owner, &mint);

    CreateAta {
        mint,
        owner,
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Wrap SPL tokens to Light Token
    let wrap_amount = 500u64;
    Wrap {
        source_spl_ata: spl_token_account.pubkey(),
        destination: light_token_ata,
        mint,
        amount: wrap_amount,
        decimals,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Verify Light Token balance
    let light_token_data = rpc.get_account(light_token_ata).await.unwrap().unwrap();
    let light_token_state = Token::deserialize(&mut &light_token_data.data[..]).unwrap();
    assert_eq!(light_token_state.amount, wrap_amount);

    // Verify SPL token balance decreased
    let spl_token_data = rpc
        .get_account(spl_token_account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let spl_state = SplTokenAccount::unpack(&spl_token_data.data).unwrap();
    assert_eq!(spl_state.amount, mint_amount - wrap_amount);
}

/// Test unwrapping Light Tokens back to SPL tokens.
#[tokio::test]
async fn test_unwrap_basic() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let decimals = CREATE_MINT_HELPER_DECIMALS;

    // Create SPL mint
    let mint = create_mint_helper(&mut rpc, &payer).await;

    // Create SPL token account
    let spl_token_account = Keypair::new();
    create_token_account(&mut rpc, &mint, &spl_token_account, &payer)
        .await
        .unwrap();

    // Mint SPL tokens
    let mint_amount = 1000u64;
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_token_account.pubkey(),
        &payer.pubkey(),
        &payer,
        mint_amount,
        false,
    )
    .await
    .unwrap();

    // Create Light Token ATA
    let owner = payer.pubkey();
    let (light_token_ata, _) = derive_token_ata(&owner, &mint);

    CreateAta {
        mint,
        owner,
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Wrap all SPL tokens to Light Token first
    Wrap {
        source_spl_ata: spl_token_account.pubkey(),
        destination: light_token_ata,
        mint,
        amount: mint_amount,
        decimals,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Verify Light Token has all tokens
    let light_token_data = rpc.get_account(light_token_ata).await.unwrap().unwrap();
    let light_token_state = Token::deserialize(&mut &light_token_data.data[..]).unwrap();
    assert_eq!(light_token_state.amount, mint_amount);

    // Unwrap some tokens back to SPL
    let unwrap_amount = 300u64;
    Unwrap {
        source: light_token_ata,
        destination_spl_ata: spl_token_account.pubkey(),
        mint,
        amount: unwrap_amount,
        decimals,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Verify Light Token balance decreased
    let light_token_data = rpc.get_account(light_token_ata).await.unwrap().unwrap();
    let light_token_state = Token::deserialize(&mut &light_token_data.data[..]).unwrap();
    assert_eq!(light_token_state.amount, mint_amount - unwrap_amount);

    // Verify SPL token balance increased
    let spl_token_data = rpc
        .get_account(spl_token_account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let spl_state = SplTokenAccount::unpack(&spl_token_data.data).unwrap();
    assert_eq!(spl_state.amount, unwrap_amount);
}

/// Test wrap and unwrap round trip.
#[tokio::test]
async fn test_wrap_unwrap_round_trip() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let decimals = CREATE_MINT_HELPER_DECIMALS;

    // Create SPL mint
    let mint = create_mint_helper(&mut rpc, &payer).await;

    // Create SPL token account
    let spl_token_account = Keypair::new();
    create_token_account(&mut rpc, &mint, &spl_token_account, &payer)
        .await
        .unwrap();

    // Mint SPL tokens
    let mint_amount = 1000u64;
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_token_account.pubkey(),
        &payer.pubkey(),
        &payer,
        mint_amount,
        false,
    )
    .await
    .unwrap();

    // Create Light Token ATA
    let owner = payer.pubkey();
    let (light_token_ata, _) = derive_token_ata(&owner, &mint);

    CreateAta {
        mint,
        owner,
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Wrap all tokens
    Wrap {
        source_spl_ata: spl_token_account.pubkey(),
        destination: light_token_ata,
        mint,
        amount: mint_amount,
        decimals,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Unwrap all tokens back
    Unwrap {
        source: light_token_ata,
        destination_spl_ata: spl_token_account.pubkey(),
        mint,
        amount: mint_amount,
        decimals,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Verify Light Token has 0 balance
    let light_token_data = rpc.get_account(light_token_ata).await.unwrap().unwrap();
    let light_token_state = Token::deserialize(&mut &light_token_data.data[..]).unwrap();
    assert_eq!(light_token_state.amount, 0);

    // Verify SPL token has original balance
    let spl_token_data = rpc
        .get_account(spl_token_account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let spl_state = SplTokenAccount::unpack(&spl_token_data.data).unwrap();
    assert_eq!(spl_state.amount, mint_amount);
}

/// Test wrapping with large amounts.
#[tokio::test]
async fn test_wrap_large_amount() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let decimals = CREATE_MINT_HELPER_DECIMALS;

    // Create SPL mint
    let mint = create_mint_helper(&mut rpc, &payer).await;

    // Create SPL token account
    let spl_token_account = Keypair::new();
    create_token_account(&mut rpc, &mint, &spl_token_account, &payer)
        .await
        .unwrap();

    // Mint large amount of SPL tokens
    let mint_amount = 1_000_000_000u64;
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_token_account.pubkey(),
        &payer.pubkey(),
        &payer,
        mint_amount,
        false,
    )
    .await
    .unwrap();

    // Create Light Token ATA
    let owner = payer.pubkey();
    let (light_token_ata, _) = derive_token_ata(&owner, &mint);

    CreateAta {
        mint,
        owner,
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Wrap half the tokens
    let wrap_amount = 500_000_000u64;
    Wrap {
        source_spl_ata: spl_token_account.pubkey(),
        destination: light_token_ata,
        mint,
        amount: wrap_amount,
        decimals,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Verify Light Token balance
    let light_token_data = rpc.get_account(light_token_ata).await.unwrap().unwrap();
    let light_token_state = Token::deserialize(&mut &light_token_data.data[..]).unwrap();
    assert_eq!(light_token_state.amount, wrap_amount);
}
