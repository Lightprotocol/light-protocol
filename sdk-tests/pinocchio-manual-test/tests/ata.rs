//! Test ATA pattern - Associated Token Account with rent-free CPI.

mod shared;

use borsh::BorshDeserialize;
use light_program_test::Rpc;
use light_token::instruction::{
    config_pda, derive_associated_token_account, rent_sponsor_pda, LIGHT_TOKEN_PROGRAM_ID,
};
use light_token_interface::state::{AccountState, Token};
use pinocchio_manual_test::CreateAtaParams;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

/// Test creating an ATA using CreateTokenAtaCpi.
#[tokio::test]
async fn test_create_ata() {
    let (mut rpc, payer, _) = shared::setup_test_env().await;

    // Create a mint to use for the ATA
    let mint = shared::create_test_mint(&mut rpc, &payer).await;

    // ATA owner - typically a user wallet
    let ata_owner = Keypair::new();

    // Derive ATA address using light-token's standard derivation
    let user_ata = derive_associated_token_account(&ata_owner.pubkey(), &mint);

    let params = CreateAtaParams::default();

    let program_id = Pubkey::new_from_array(pinocchio_manual_test::ID);

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(ata_owner.pubkey(), false),
        AccountMeta::new(user_ata, false),
        AccountMeta::new_readonly(config_pda(), false),
        AccountMeta::new(rent_sponsor_pda(), false),
        AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let data = [
        pinocchio_manual_test::discriminators::CREATE_ATA.as_slice(),
        &borsh::to_vec(&params).unwrap(),
    ]
    .concat();

    let ix = Instruction {
        program_id,
        accounts,
        data,
    };

    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
        .await
        .expect("CreateAta should succeed");

    // Verify ATA exists and has correct state
    let ata_account = rpc
        .get_account(user_ata)
        .await
        .unwrap()
        .expect("ATA should exist");

    let token =
        Token::deserialize(&mut &ata_account.data[..]).expect("Should deserialize as Token");

    assert_eq!(token.mint.to_bytes(), mint.to_bytes());
    assert_eq!(token.owner.to_bytes(), ata_owner.pubkey().to_bytes());
    assert_eq!(token.amount, 0);
    assert_eq!(token.state, AccountState::Initialized);
}

/// Test idempotent ATA creation - should not fail if ATA already exists.
#[tokio::test]
async fn test_create_ata_idempotent() {
    let (mut rpc, payer, _) = shared::setup_test_env().await;

    let mint = shared::create_test_mint(&mut rpc, &payer).await;
    let ata_owner = Keypair::new();
    let user_ata = derive_associated_token_account(&ata_owner.pubkey(), &mint);

    let params = CreateAtaParams::default();

    let program_id = Pubkey::new_from_array(pinocchio_manual_test::ID);

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(ata_owner.pubkey(), false),
        AccountMeta::new(user_ata, false),
        AccountMeta::new_readonly(config_pda(), false),
        AccountMeta::new(rent_sponsor_pda(), false),
        AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let data = [
        pinocchio_manual_test::discriminators::CREATE_ATA.as_slice(),
        &borsh::to_vec(&params).unwrap(),
    ]
    .concat();

    let ix = Instruction {
        program_id,
        accounts: accounts.clone(),
        data: data.clone(),
    };

    // First creation
    rpc.create_and_send_transaction(std::slice::from_ref(&ix), &payer.pubkey(), &[&payer])
        .await
        .expect("First CreateAta should succeed");

    // Second creation (idempotent) - should NOT fail
    let ix2 = Instruction {
        program_id,
        accounts,
        data,
    };

    rpc.create_and_send_transaction(&[ix2], &payer.pubkey(), &[&payer])
        .await
        .expect("Second CreateAta should succeed (idempotent)");
}
