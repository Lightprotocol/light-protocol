// Tests for BurnCTokenCpi invoke() and invoke_signed()

mod shared;

use borsh::{BorshDeserialize, BorshSerialize};
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token_interface::state::Token;
use native_ctoken_examples::{BurnData, InstructionType, ID, TOKEN_ACCOUNT_SEED};
use shared::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
};

/// Test burning CTokens using BurnCTokenCpi::invoke()
#[tokio::test]
async fn test_burn_invoke() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a decompressed mint (required for burn) with an ATA for the payer with 1000 tokens
    let (mint_pda, _compression_address, ata_pubkeys) =
        setup_create_compressed_mint_with_freeze_authority(
            &mut rpc,
            &payer,
            payer.pubkey(),
            None, // No freeze authority needed for burn test
            9,
            vec![(1000, payer.pubkey())],
        )
        .await;

    let ata = ata_pubkeys[0];
    let burn_amount = 300u64;

    // Get initial state
    let ata_account_before = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_before = Token::deserialize(&mut &ata_account_before.data[..]).unwrap();

    // Build burn instruction via wrapper program
    let mut instruction_data = vec![InstructionType::BurnInvoke as u8];
    let burn_data = BurnData {
        amount: burn_amount,
    };
    burn_data.serialize(&mut instruction_data).unwrap();

    let ctoken_program = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(ata, false),                     // source
            AccountMeta::new(mint_pda, false),                // cmint
            AccountMeta::new_readonly(payer.pubkey(), true),  // authority (signer)
            AccountMeta::new_readonly(ctoken_program, false), // ctoken_program
        ],
        data: instruction_data,
    };

    // Execute the burn instruction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify with single assert_eq
    let ata_account_after = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_after = Token::deserialize(&mut &ata_account_after.data[..]).unwrap();

    let mut expected_ctoken = ctoken_before;
    expected_ctoken.amount = 700; // 1000 - 300

    assert_eq!(
        ctoken_after, expected_ctoken,
        "Light Token should match expected state after burn"
    );
}

/// Test burning CTokens with PDA authority using BurnCTokenCpi::invoke_signed()
#[tokio::test]
async fn test_burn_invoke_signed() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive the PDA that will own the token account
    let (pda_owner, _bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

    // Create a decompressed mint with an ATA for the PDA owner with 1000 tokens
    let (mint_pda, _compression_address, ata_pubkeys) =
        setup_create_compressed_mint_with_freeze_authority(
            &mut rpc,
            &payer,
            payer.pubkey(),
            None, // No freeze authority needed for burn test
            9,
            vec![(1000, pda_owner)],
        )
        .await;

    let ata = ata_pubkeys[0];
    let burn_amount = 500u64;

    // Get initial state
    let ata_account_before = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_before = Token::deserialize(&mut &ata_account_before.data[..]).unwrap();

    // Build burn instruction via wrapper program using invoke_signed
    let mut instruction_data = vec![InstructionType::BurnInvokeSigned as u8];
    let burn_data = BurnData {
        amount: burn_amount,
    };
    burn_data.serialize(&mut instruction_data).unwrap();

    let ctoken_program = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(ata, false),                     // source
            AccountMeta::new(mint_pda, false),                // cmint
            AccountMeta::new_readonly(pda_owner, false),      // PDA authority (program signs)
            AccountMeta::new_readonly(ctoken_program, false), // ctoken_program
        ],
        data: instruction_data,
    };

    // Execute the burn instruction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify with single assert_eq
    let ata_account_after = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_after = Token::deserialize(&mut &ata_account_after.data[..]).unwrap();

    let mut expected_ctoken = ctoken_before;
    expected_ctoken.amount = 500; // 1000 - 500

    assert_eq!(
        ctoken_after, expected_ctoken,
        "Light Token should match expected state after burn"
    );
}
