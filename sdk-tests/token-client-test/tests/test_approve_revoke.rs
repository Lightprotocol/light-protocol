//! Tests for the approve and revoke actions in light-token-client.

use borsh::BorshDeserialize;
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token::instruction::derive_token_ata;
use light_token_client::actions::{Approve, CreateAta, CreateMint, MintTo, Revoke, Transfer};
use light_token_interface::state::{AccountState, Token};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

fn get_expected_token(
    actual: &Token,
    mint: Pubkey,
    owner: Pubkey,
    amount: u64,
    delegate: Option<Pubkey>,
    delegated_amount: u64,
) -> Token {
    Token {
        mint: mint.to_bytes().into(),
        owner: owner.to_bytes().into(),
        amount,
        delegate: delegate.map(|d| d.to_bytes().into()),
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount,
        close_authority: None,
        account_type: actual.account_type,
        extensions: actual.extensions.clone(),
    }
}

/// Test approving a delegate for a token account.
#[tokio::test]
async fn test_approve_basic() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let decimals = 9u8;

    // Create mint
    let (_, mint) = CreateMint {
        decimals,
        ..Default::default()
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Create ATA for payer
    let owner = payer.pubkey();
    let (token_account, _) = derive_token_ata(&owner, &mint);

    CreateAta {
        mint,
        owner,
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Mint tokens
    let mint_amount = 1000u64;
    MintTo {
        mint,
        destination: token_account,
        amount: mint_amount,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Approve delegate
    let delegate = Pubkey::new_unique();
    let delegate_amount = 500u64;

    Approve {
        token_account,
        delegate,
        amount: delegate_amount,
        owner: None,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Verify delegation
    let account_data = rpc.get_account(token_account).await.unwrap().unwrap();
    let token_state = Token::deserialize(&mut &account_data.data[..]).unwrap();
    let expected = get_expected_token(
        &token_state,
        mint,
        owner,
        mint_amount,
        Some(delegate),
        delegate_amount,
    );
    assert_eq!(token_state, expected);
}

/// Test approving with a separate owner keypair.
#[tokio::test]
async fn test_approve_with_separate_owner() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let decimals = 9u8;

    // Create mint
    let (_, mint) = CreateMint {
        decimals,
        ..Default::default()
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Create ATA for a different owner
    let owner = Keypair::new();
    let (token_account, _) = derive_token_ata(&owner.pubkey(), &mint);

    CreateAta {
        mint,
        owner: owner.pubkey(),
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Mint tokens
    let mint_amount = 1000u64;
    MintTo {
        mint,
        destination: token_account,
        amount: mint_amount,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Approve delegate with separate owner
    let delegate = Pubkey::new_unique();
    let delegate_amount = 300u64;

    Approve {
        token_account,
        delegate,
        amount: delegate_amount,
        owner: Some(owner.pubkey()),
    }
    .execute_with_owner(&mut rpc, &payer, &owner)
    .await
    .unwrap();

    // Verify delegation
    let account_data = rpc.get_account(token_account).await.unwrap().unwrap();
    let token_state = Token::deserialize(&mut &account_data.data[..]).unwrap();
    let expected = get_expected_token(
        &token_state,
        mint,
        owner.pubkey(),
        mint_amount,
        Some(delegate),
        delegate_amount,
    );
    assert_eq!(token_state, expected);
}

/// Test revoking a delegate.
#[tokio::test]
async fn test_revoke_basic() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let decimals = 9u8;

    // Create mint
    let (_, mint) = CreateMint {
        decimals,
        ..Default::default()
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Create ATA
    let owner = payer.pubkey();
    let (token_account, _) = derive_token_ata(&owner, &mint);

    CreateAta {
        mint,
        owner,
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Mint tokens
    let mint_amount = 1000u64;
    MintTo {
        mint,
        destination: token_account,
        amount: mint_amount,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Approve delegate first
    let delegate = Pubkey::new_unique();
    let delegate_amount = 500u64;

    Approve {
        token_account,
        delegate,
        amount: delegate_amount,
        owner: None,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Verify delegation is set
    let account_data = rpc.get_account(token_account).await.unwrap().unwrap();
    let token_state = Token::deserialize(&mut &account_data.data[..]).unwrap();
    let expected_with_delegate = get_expected_token(
        &token_state,
        mint,
        owner,
        mint_amount,
        Some(delegate),
        delegate_amount,
    );
    assert_eq!(token_state, expected_with_delegate);

    // Revoke delegate
    Revoke {
        token_account,
        owner: None,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Verify delegation is revoked
    let account_data = rpc.get_account(token_account).await.unwrap().unwrap();
    let token_state = Token::deserialize(&mut &account_data.data[..]).unwrap();
    let expected_revoked = get_expected_token(&token_state, mint, owner, mint_amount, None, 0);
    assert_eq!(token_state, expected_revoked);
}

/// Test revoking with a separate owner keypair.
#[tokio::test]
async fn test_revoke_with_separate_owner() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let decimals = 9u8;

    // Create mint
    let (_, mint) = CreateMint {
        decimals,
        ..Default::default()
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Create ATA for a different owner
    let owner = Keypair::new();
    let (token_account, _) = derive_token_ata(&owner.pubkey(), &mint);

    CreateAta {
        mint,
        owner: owner.pubkey(),
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Mint tokens
    let mint_amount = 1000u64;
    MintTo {
        mint,
        destination: token_account,
        amount: mint_amount,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Approve delegate
    let delegate = Pubkey::new_unique();
    Approve {
        token_account,
        delegate,
        amount: 500,
        owner: Some(owner.pubkey()),
    }
    .execute_with_owner(&mut rpc, &payer, &owner)
    .await
    .unwrap();

    // Revoke with separate owner
    Revoke {
        token_account,
        owner: Some(owner.pubkey()),
    }
    .execute_with_owner(&mut rpc, &payer, &owner)
    .await
    .unwrap();

    // Verify delegation is revoked
    let account_data = rpc.get_account(token_account).await.unwrap().unwrap();
    let token_state = Token::deserialize(&mut &account_data.data[..]).unwrap();
    let expected = get_expected_token(&token_state, mint, owner.pubkey(), mint_amount, None, 0);
    assert_eq!(token_state, expected);
}

/// Test delegate transfer using approved amount.
#[tokio::test]
async fn test_approve_and_delegate_transfer() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let decimals = 9u8;

    // Create mint
    let (_, mint) = CreateMint {
        decimals,
        ..Default::default()
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Create source ATA
    let source_owner = payer.pubkey();
    let (source_ata, _) = derive_token_ata(&source_owner, &mint);

    CreateAta {
        mint,
        owner: source_owner,
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Create destination ATA
    let dest_owner = Pubkey::new_unique();
    let (dest_ata, _) = derive_token_ata(&dest_owner, &mint);

    CreateAta {
        mint,
        owner: dest_owner,
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Mint tokens
    let mint_amount = 1000u64;
    MintTo {
        mint,
        destination: source_ata,
        amount: mint_amount,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Create a delegate keypair
    let delegate = Keypair::new();
    let delegate_amount = 500u64;

    // Approve the delegate
    Approve {
        token_account: source_ata,
        delegate: delegate.pubkey(),
        amount: delegate_amount,
        owner: None,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Transfer using delegate authority
    let transfer_amount = 300u64;
    Transfer {
        source: source_ata,
        destination: dest_ata,
        amount: transfer_amount,
    }
    .execute(&mut rpc, &payer, &delegate)
    .await
    .unwrap();

    // Verify source account (delegated amount should be reduced)
    let source_data = rpc.get_account(source_ata).await.unwrap().unwrap();
    let source_state = Token::deserialize(&mut &source_data.data[..]).unwrap();
    let expected_source = get_expected_token(
        &source_state,
        mint,
        source_owner,
        mint_amount - transfer_amount,
        Some(delegate.pubkey()),
        delegate_amount - transfer_amount,
    );
    assert_eq!(source_state, expected_source);

    // Verify destination account
    let dest_data = rpc.get_account(dest_ata).await.unwrap().unwrap();
    let dest_state = Token::deserialize(&mut &dest_data.data[..]).unwrap();
    let expected_dest = get_expected_token(&dest_state, mint, dest_owner, transfer_amount, None, 0);
    assert_eq!(dest_state, expected_dest);
}
