// Tests for CloseCTokenAccountCpi invoke() and invoke_signed()

mod shared;

use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token::instruction::{rent_sponsor_pda, LIGHT_TOKEN_PROGRAM_ID};
use sdk_light_token_pinocchio_test::{InstructionType, TOKEN_ACCOUNT_SEED};
use shared::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
};

/// Test closing a compressible token account using CloseCTokenAccountCpi::invoke()
#[tokio::test]
async fn test_close_invoke() {
    let config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("sdk_light_token_pinocchio_test", PROGRAM_ID)]),
    );
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a compressed mint with an ATA for the payer
    let (_mint_pda, _compression_address, ata_pubkeys, _mint_seed) = setup_create_mint(
        &mut rpc,
        &payer,
        payer.pubkey(),
        9,
        vec![(0, payer.pubkey())],
    )
    .await;

    let ata = ata_pubkeys[0];

    // Verify the ATA exists
    let ata_account = rpc.get_account(ata).await.unwrap();
    assert!(ata_account.is_some(), "ATA should exist before close");

    // Get rent sponsor
    let rent_sponsor = rent_sponsor_pda();

    // Build instruction to close via wrapper program
    let instruction_data = vec![InstructionType::CloseAccountInvoke as u8];

    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false), // token_program
            AccountMeta::new(ata, false),                             // account to close
            AccountMeta::new(payer.pubkey(), false),                  // destination
            AccountMeta::new(payer.pubkey(), true),                   // owner (signer)
            AccountMeta::new(rent_sponsor, false),                    // rent_sponsor
        ],
        data: instruction_data,
    };

    // Execute the close instruction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify the ATA is closed
    let ata_account_after = rpc.get_account(ata).await.unwrap();
    assert!(
        ata_account_after.is_none(),
        "ATA should be closed after close instruction"
    );
}

/// Test closing a PDA-owned compressible token account using CloseCTokenAccountCpi::invoke_signed()
#[tokio::test]
async fn test_close_invoke_signed() {
    let config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("sdk_light_token_pinocchio_test", PROGRAM_ID)]),
    );
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive the PDA that will own the token account
    let (pda_owner, _bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &PROGRAM_ID);

    // Create a compressed mint with an ATA for the PDA owner
    let (_mint_pda, _compression_address, ata_pubkeys, _mint_seed) = setup_create_mint(
        &mut rpc,
        &payer,
        payer.pubkey(),
        9,
        vec![(0, pda_owner)], // PDA will own this ATA
    )
    .await;

    let ata = ata_pubkeys[0];

    // Verify the ATA exists
    let ata_account = rpc.get_account(ata).await.unwrap();
    assert!(ata_account.is_some(), "ATA should exist before close");

    // Get rent sponsor
    let rent_sponsor = rent_sponsor_pda();

    // Build instruction to close via wrapper program using invoke_signed
    let instruction_data = vec![InstructionType::CloseAccountInvokeSigned as u8];

    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false), // token_program
            AccountMeta::new(ata, false),                             // account to close
            AccountMeta::new(payer.pubkey(), false),                  // destination
            AccountMeta::new(pda_owner, false), // owner (PDA, mutable for write_top_up)
            AccountMeta::new(rent_sponsor, false), // rent_sponsor
        ],
        data: instruction_data,
    };

    // Execute the close instruction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify the ATA is closed
    let ata_account_after = rpc.get_account(ata).await.unwrap();
    assert!(
        ata_account_after.is_none(),
        "PDA-owned ATA should be closed after close instruction"
    );
}
