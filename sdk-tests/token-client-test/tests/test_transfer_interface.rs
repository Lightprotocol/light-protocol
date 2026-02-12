//! Tests for the transfer_interface action in light-token-client.

use borsh::BorshDeserialize;
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token::instruction::get_associated_token_address;
use light_token_client::actions::{CreateAta, CreateMint, MintTo, TransferInterface};
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

/// Test transfer_interface for Light -> Light transfer.
#[tokio::test]
async fn test_transfer_interface_light_to_light() {
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

    // Transfer using interface (Light -> Light, no SPL token program needed)
    let transfer_amount = 500u64;
    TransferInterface {
        source: source_ata,
        mint,
        destination: dest_ata,
        amount: transfer_amount,
        decimals,
        spl_token_program: None, // No SPL token program needed for Light -> Light
        restricted: false,
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

/// Test transfer_interface for multiple Light -> Light transfers.
#[tokio::test]
async fn test_transfer_interface_multiple_transfers() {
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

    // Transfer 300
    TransferInterface {
        source: source_ata,
        mint,
        destination: dest_ata,
        amount: 300,
        decimals,
        spl_token_program: None,
        restricted: false,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Transfer 200
    TransferInterface {
        source: source_ata,
        mint,
        destination: dest_ata,
        amount: 200,
        decimals,
        spl_token_program: None,
        restricted: false,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    // Verify source account (1000 - 300 - 200 = 500)
    let source_data = rpc.get_account(source_ata).await.unwrap().unwrap();
    let source_state = Token::deserialize(&mut &source_data.data[..]).unwrap();
    let expected_source = get_expected_token(&source_state, mint, source_owner, 500, None, 0);
    assert_eq!(source_state, expected_source);

    // Verify destination account (300 + 200 = 500)
    let dest_data = rpc.get_account(dest_ata).await.unwrap().unwrap();
    let dest_state = Token::deserialize(&mut &dest_data.data[..]).unwrap();
    let expected_dest = get_expected_token(&dest_state, mint, dest_owner, 500, None, 0);
    assert_eq!(dest_state, expected_dest);
}
