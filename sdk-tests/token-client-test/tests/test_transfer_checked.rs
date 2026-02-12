//! Tests for the transfer_checked action in light-token-client.

use borsh::BorshDeserialize;
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token::instruction::get_associated_token_address;
use light_token_client::actions::{CreateAta, CreateMint, MintTo, TransferChecked};
use light_token_interface::state::{AccountState, Token};
use solana_sdk::{pubkey::Pubkey, signer::Signer};

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

/// Test transfer_checked with correct decimals.
#[tokio::test]
async fn test_transfer_checked_basic() {
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

    // Create source and destination ATAs
    let source_owner = payer.pubkey();
    let dest_owner = Pubkey::new_unique();

    let source_ata = get_associated_token_address(&source_owner, &mint);
    let dest_ata = get_associated_token_address(&dest_owner, &mint);

    CreateAta {
        mint,
        owner: source_owner,
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    CreateAta {
        mint,
        owner: dest_owner,
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Mint tokens to source
    let mint_amount = 1000u64;
    MintTo {
        mint,
        destination: source_ata,
        amount: mint_amount,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Transfer with checked decimals
    let transfer_amount = 500u64;
    TransferChecked {
        source: source_ata,
        mint,
        destination: dest_ata,
        amount: transfer_amount,
        decimals,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Verify source account
    let source_data = rpc.get_account(source_ata).await.unwrap().unwrap();
    let source_state = Token::deserialize(&mut &source_data.data[..]).unwrap();
    let expected_source = get_expected_token(
        &source_state,
        mint,
        source_owner,
        mint_amount - transfer_amount,
        None,
        0,
    );
    assert_eq!(source_state, expected_source);

    // Verify destination account
    let dest_data = rpc.get_account(dest_ata).await.unwrap().unwrap();
    let dest_state = Token::deserialize(&mut &dest_data.data[..]).unwrap();
    let expected_dest = get_expected_token(&dest_state, mint, dest_owner, transfer_amount, None, 0);
    assert_eq!(dest_state, expected_dest);
}

/// Test transfer_checked with different decimals token.
#[tokio::test]
async fn test_transfer_checked_different_decimals() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Use 6 decimals (like USDC)
    let decimals = 6u8;

    // Create mint
    let (_, mint) = CreateMint {
        decimals,
        ..Default::default()
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Create source and destination ATAs
    let source_owner = payer.pubkey();
    let dest_owner = Pubkey::new_unique();

    let source_ata = get_associated_token_address(&source_owner, &mint);
    let dest_ata = get_associated_token_address(&dest_owner, &mint);

    CreateAta {
        mint,
        owner: source_owner,
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    CreateAta {
        mint,
        owner: dest_owner,
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Mint tokens to source (1000 tokens with 6 decimals = 1_000_000_000 base units)
    let mint_amount = 1_000_000_000u64;
    MintTo {
        mint,
        destination: source_ata,
        amount: mint_amount,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Transfer 500 tokens (500_000_000 base units)
    let transfer_amount = 500_000_000u64;
    TransferChecked {
        source: source_ata,
        mint,
        destination: dest_ata,
        amount: transfer_amount,
        decimals,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Verify source account
    let source_data = rpc.get_account(source_ata).await.unwrap().unwrap();
    let source_state = Token::deserialize(&mut &source_data.data[..]).unwrap();
    let expected_source = get_expected_token(
        &source_state,
        mint,
        source_owner,
        mint_amount - transfer_amount,
        None,
        0,
    );
    assert_eq!(source_state, expected_source);

    // Verify destination account
    let dest_data = rpc.get_account(dest_ata).await.unwrap().unwrap();
    let dest_state = Token::deserialize(&mut &dest_data.data[..]).unwrap();
    let expected_dest = get_expected_token(&dest_state, mint, dest_owner, transfer_amount, None, 0);
    assert_eq!(dest_state, expected_dest);
}
