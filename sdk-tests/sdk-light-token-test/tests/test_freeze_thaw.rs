// Tests for FreezeCTokenCpi and ThawCTokenCpi invoke() and invoke_signed()

mod shared;

use borsh::BorshDeserialize;
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token::LIGHT_TOKEN_PROGRAM_ID;
use light_token_interface::state::{AccountState, Token};
use sdk_light_token_test::{InstructionType, FREEZE_AUTHORITY_SEED, ID};
use shared::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

/// Test freezing a Light Token account using FreezeCTokenCpi::invoke()
#[tokio::test]
async fn test_freeze_invoke() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_light_token_test", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let freeze_authority = Keypair::new();

    // Create a compressed mint with freeze_authority and an ATA for the payer with 1000 tokens
    let (mint_pda, _compression_address, ata_pubkeys) = setup_create_mint_with_freeze_authority(
        &mut rpc,
        &payer,
        payer.pubkey(),
        Some(freeze_authority.pubkey()),
        9,
        vec![(1000, payer.pubkey())],
    )
    .await;

    let ata = ata_pubkeys[0];

    // Verify account is initially unfrozen
    let ata_account_before = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_before = Token::deserialize(&mut &ata_account_before.data[..]).unwrap();
    assert_eq!(
        ctoken_before.state,
        AccountState::Initialized,
        "Account should be initialized (unfrozen) before freeze"
    );

    // Build freeze instruction via wrapper program
    let instruction_data = vec![InstructionType::FreezeInvoke as u8];

    let light_token_program = LIGHT_TOKEN_PROGRAM_ID;
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(ata, false),               // token_account
            AccountMeta::new_readonly(mint_pda, false), // mint
            AccountMeta::new_readonly(freeze_authority.pubkey(), true), // freeze_authority (signer)
            AccountMeta::new_readonly(light_token_program, false), // light_token_program
        ],
        data: instruction_data,
    };

    // Execute the freeze instruction
    rpc.create_and_send_transaction(
        &[instruction],
        &payer.pubkey(),
        &[&payer, &freeze_authority],
    )
    .await
    .unwrap();

    // Verify the account is now frozen
    let ata_account_after = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_after = Token::deserialize(&mut &ata_account_after.data[..]).unwrap();

    assert_eq!(
        ctoken_after.state,
        AccountState::Frozen,
        "Account should be frozen after freeze"
    );
}

/// Test freezing a Light Token account with PDA freeze authority using FreezeCTokenCpi::invoke_signed()
#[tokio::test]
async fn test_freeze_invoke_signed() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_light_token_test", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive the PDA that will be the freeze authority
    let (pda_freeze_authority, _bump) = Pubkey::find_program_address(&[FREEZE_AUTHORITY_SEED], &ID);

    // Create a compressed mint with PDA freeze_authority and an ATA for the payer with 1000 tokens
    let (mint_pda, _compression_address, ata_pubkeys) = setup_create_mint_with_freeze_authority(
        &mut rpc,
        &payer,
        payer.pubkey(),
        Some(pda_freeze_authority),
        9,
        vec![(1000, payer.pubkey())],
    )
    .await;

    let ata = ata_pubkeys[0];

    // Build freeze instruction via wrapper program using invoke_signed
    let instruction_data = vec![InstructionType::FreezeInvokeSigned as u8];

    let light_token_program = LIGHT_TOKEN_PROGRAM_ID;
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(ata, false),                           // token_account
            AccountMeta::new_readonly(mint_pda, false),             // mint
            AccountMeta::new_readonly(pda_freeze_authority, false), // PDA freeze_authority (program signs)
            AccountMeta::new_readonly(light_token_program, false),  // light_token_program
        ],
        data: instruction_data,
    };

    // Execute the freeze instruction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify the account is now frozen
    let ata_account_after = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_after = Token::deserialize(&mut &ata_account_after.data[..]).unwrap();

    assert_eq!(
        ctoken_after.state,
        AccountState::Frozen,
        "Account should be frozen after freeze"
    );
}

/// Test thawing a frozen Light Token account using ThawCTokenCpi::invoke()
#[tokio::test]
async fn test_thaw_invoke() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_light_token_test", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let freeze_authority = Keypair::new();
    let light_token_program = LIGHT_TOKEN_PROGRAM_ID;

    // Create a compressed mint with freeze_authority and an ATA for the payer with 1000 tokens
    let (mint_pda, _compression_address, ata_pubkeys) = setup_create_mint_with_freeze_authority(
        &mut rpc,
        &payer,
        payer.pubkey(),
        Some(freeze_authority.pubkey()),
        9,
        vec![(1000, payer.pubkey())],
    )
    .await;

    let ata = ata_pubkeys[0];

    // First freeze the account
    let freeze_instruction_data = vec![InstructionType::FreezeInvoke as u8];
    let freeze_instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(ata, false),
            AccountMeta::new_readonly(mint_pda, false),
            AccountMeta::new_readonly(freeze_authority.pubkey(), true),
            AccountMeta::new_readonly(light_token_program, false),
        ],
        data: freeze_instruction_data,
    };

    rpc.create_and_send_transaction(
        &[freeze_instruction],
        &payer.pubkey(),
        &[&payer, &freeze_authority],
    )
    .await
    .unwrap();

    // Verify account is frozen
    let ata_account_after_freeze = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_after_freeze = Token::deserialize(&mut &ata_account_after_freeze.data[..]).unwrap();
    assert_eq!(
        ctoken_after_freeze.state,
        AccountState::Frozen,
        "Account should be frozen"
    );

    // Now thaw the account
    let thaw_instruction_data = vec![InstructionType::ThawInvoke as u8];
    let thaw_instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(ata, false),               // token_account
            AccountMeta::new_readonly(mint_pda, false), // mint
            AccountMeta::new_readonly(freeze_authority.pubkey(), true), // freeze_authority (signer)
            AccountMeta::new_readonly(light_token_program, false), // light_token_program
        ],
        data: thaw_instruction_data,
    };

    rpc.create_and_send_transaction(
        &[thaw_instruction],
        &payer.pubkey(),
        &[&payer, &freeze_authority],
    )
    .await
    .unwrap();

    // Verify the account is now thawed (initialized)
    let ata_account_after_thaw = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_after_thaw = Token::deserialize(&mut &ata_account_after_thaw.data[..]).unwrap();

    assert_eq!(
        ctoken_after_thaw.state,
        AccountState::Initialized,
        "Account should be initialized (thawed) after thaw"
    );
}

/// Test thawing a frozen Light Token account with PDA freeze authority using ThawCTokenCpi::invoke_signed()
#[tokio::test]
async fn test_thaw_invoke_signed() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("sdk_light_token_test", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive the PDA that will be the freeze authority
    let (pda_freeze_authority, _bump) = Pubkey::find_program_address(&[FREEZE_AUTHORITY_SEED], &ID);
    let light_token_program = LIGHT_TOKEN_PROGRAM_ID;

    // Create a compressed mint with PDA freeze_authority and an ATA for the payer with 1000 tokens
    let (mint_pda, _compression_address, ata_pubkeys) = setup_create_mint_with_freeze_authority(
        &mut rpc,
        &payer,
        payer.pubkey(),
        Some(pda_freeze_authority),
        9,
        vec![(1000, payer.pubkey())],
    )
    .await;

    let ata = ata_pubkeys[0];

    // First freeze the account using invoke_signed
    let freeze_instruction_data = vec![InstructionType::FreezeInvokeSigned as u8];
    let freeze_instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(ata, false),
            AccountMeta::new_readonly(mint_pda, false),
            AccountMeta::new_readonly(pda_freeze_authority, false),
            AccountMeta::new_readonly(light_token_program, false),
        ],
        data: freeze_instruction_data,
    };

    rpc.create_and_send_transaction(&[freeze_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify account is frozen
    let ata_account_after_freeze = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_after_freeze = Token::deserialize(&mut &ata_account_after_freeze.data[..]).unwrap();
    assert_eq!(
        ctoken_after_freeze.state,
        AccountState::Frozen,
        "Account should be frozen"
    );

    // Now thaw the account using invoke_signed
    let thaw_instruction_data = vec![InstructionType::ThawInvokeSigned as u8];
    let thaw_instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(ata, false),                           // token_account
            AccountMeta::new_readonly(mint_pda, false),             // mint
            AccountMeta::new_readonly(pda_freeze_authority, false), // PDA freeze_authority (program signs)
            AccountMeta::new_readonly(light_token_program, false),  // light_token_program
        ],
        data: thaw_instruction_data,
    };

    rpc.create_and_send_transaction(&[thaw_instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify the account is now thawed (initialized)
    let ata_account_after_thaw = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_after_thaw = Token::deserialize(&mut &ata_account_after_thaw.data[..]).unwrap();

    assert_eq!(
        ctoken_after_thaw.state,
        AccountState::Initialized,
        "Account should be initialized (thawed) after thaw"
    );
}
