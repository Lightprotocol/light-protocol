//! Test ATA pattern - Associated Token Account with rent-free CPI.

mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use anchor_manual_test::CreateAtaParams;
use borsh::BorshDeserialize;
use light_program_test::Rpc;
use light_token::instruction::{
    config_pda, derive_associated_token_account, rent_sponsor_pda, LIGHT_TOKEN_PROGRAM_ID,
};
use light_token_interface::state::{AccountState, Token};
use solana_sdk::{
    instruction::Instruction,
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
    let (user_ata, _) = derive_associated_token_account(&ata_owner.pubkey(), &mint);

    let params = CreateAtaParams::default();

    let accounts = anchor_manual_test::accounts::CreateAtaAccounts {
        payer: payer.pubkey(),
        mint,
        ata_owner: ata_owner.pubkey(),
        user_ata,
        compressible_config: config_pda(),
        rent_sponsor: rent_sponsor_pda(),
        light_token_program: LIGHT_TOKEN_PROGRAM_ID,
        system_program: solana_sdk::system_program::ID,
    };

    let ix = Instruction {
        program_id: anchor_manual_test::ID,
        accounts: accounts.to_account_metas(None),
        data: anchor_manual_test::instruction::CreateAta { params }.data(),
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
    let (user_ata, _) = derive_associated_token_account(&ata_owner.pubkey(), &mint);

    let params = CreateAtaParams::default();

    let accounts = anchor_manual_test::accounts::CreateAtaAccounts {
        payer: payer.pubkey(),
        mint,
        ata_owner: ata_owner.pubkey(),
        user_ata,
        compressible_config: config_pda(),
        rent_sponsor: rent_sponsor_pda(),
        light_token_program: LIGHT_TOKEN_PROGRAM_ID,
        system_program: solana_sdk::system_program::ID,
    };

    let ix = Instruction {
        program_id: anchor_manual_test::ID,
        accounts: accounts.to_account_metas(None),
        data: anchor_manual_test::instruction::CreateAta {
            params: params.clone(),
        }
        .data(),
    };

    // First creation
    rpc.create_and_send_transaction(std::slice::from_ref(&ix), &payer.pubkey(), &[&payer])
        .await
        .expect("First CreateAta should succeed");

    // Second creation (idempotent) - should NOT fail
    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Second CreateAta should succeed (idempotent)");
}
