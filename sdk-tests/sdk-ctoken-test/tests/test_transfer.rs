// Tests for TransferCtokenAccountInfos invoke() and invoke_signed()
// These tests focus on different transfer scenarios

mod shared;

use borsh::{BorshDeserialize, BorshSerialize};
use light_client::rpc::Rpc;
use light_compressed_token_sdk::ctoken::CTOKEN_PROGRAM_ID;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use native_ctoken_examples::{InstructionType, TransferData, ID, TOKEN_ACCOUNT_SEED};
use shared::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
};

/// Test basic transfer using TransferCtokenAccountInfos::invoke()
/// Tests a simple transfer from one account to another
#[tokio::test]
async fn test_transfer_basic() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a compressed mint with two ATAs - source with 1000 tokens, destination with 0
    let source_owner = payer.pubkey();
    let dest_owner = Pubkey::new_unique();

    let (_mint_pda, _compression_address, ata_pubkeys) = setup_create_compressed_mint(
        &mut rpc,
        &payer,
        payer.pubkey(),
        9,
        vec![(1000, source_owner), (0, dest_owner)],
    )
    .await;

    let source_ata = ata_pubkeys[0];
    let dest_ata = ata_pubkeys[1];

    // Verify initial balances
    use light_ctoken_types::state::CToken;
    let source_data = rpc.get_account(source_ata).await.unwrap().unwrap();
    let source_state = CToken::deserialize(&mut &source_data.data[..]).unwrap();
    assert_eq!(source_state.amount, 1000, "Source should have 1000 tokens");

    let dest_data = rpc.get_account(dest_ata).await.unwrap().unwrap();
    let dest_state = CToken::deserialize(&mut &dest_data.data[..]).unwrap();
    assert_eq!(dest_state.amount, 0, "Destination should have 0 tokens");

    // Transfer 500 tokens
    let transfer_data = TransferData { amount: 500 };
    let instruction_data = [
        vec![InstructionType::CTokenTransferInvoke as u8],
        transfer_data.try_to_vec().unwrap(),
    ]
    .concat();

    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(source_ata, false),
            AccountMeta::new(dest_ata, false),
            AccountMeta::new_readonly(source_owner, true),
            AccountMeta::new_readonly(CTOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify final balances
    let source_data_after = rpc.get_account(source_ata).await.unwrap().unwrap();
    let source_state_after = CToken::deserialize(&mut &source_data_after.data[..]).unwrap();
    assert_eq!(
        source_state_after.amount, 500,
        "Source should have 500 tokens after transfer"
    );

    let dest_data_after = rpc.get_account(dest_ata).await.unwrap().unwrap();
    let dest_state_after = CToken::deserialize(&mut &dest_data_after.data[..]).unwrap();
    assert_eq!(
        dest_state_after.amount, 500,
        "Destination should have 500 tokens after transfer"
    );
}

/// Test transfer that empties the source account completely
#[tokio::test]
async fn test_transfer_full_balance() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a compressed mint with two ATAs
    let source_owner = payer.pubkey();
    let dest_owner = Pubkey::new_unique();

    let (_mint_pda, _compression_address, ata_pubkeys) = setup_create_compressed_mint(
        &mut rpc,
        &payer,
        payer.pubkey(),
        9,
        vec![(1000, source_owner), (0, dest_owner)],
    )
    .await;

    let source_ata = ata_pubkeys[0];
    let dest_ata = ata_pubkeys[1];

    // Transfer all 1000 tokens
    let transfer_data = TransferData { amount: 1000 };
    let instruction_data = [
        vec![InstructionType::CTokenTransferInvoke as u8],
        transfer_data.try_to_vec().unwrap(),
    ]
    .concat();

    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(source_ata, false),
            AccountMeta::new(dest_ata, false),
            AccountMeta::new_readonly(source_owner, true),
            AccountMeta::new_readonly(CTOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify final balances
    use light_ctoken_types::state::CToken;
    let source_data_after = rpc.get_account(source_ata).await.unwrap().unwrap();
    let source_state_after = CToken::deserialize(&mut &source_data_after.data[..]).unwrap();
    assert_eq!(
        source_state_after.amount, 0,
        "Source should have 0 tokens after full transfer"
    );

    let dest_data_after = rpc.get_account(dest_ata).await.unwrap().unwrap();
    let dest_state_after = CToken::deserialize(&mut &dest_data_after.data[..]).unwrap();
    assert_eq!(
        dest_state_after.amount, 1000,
        "Destination should have 1000 tokens after full transfer"
    );
}

/// Test transfer from PDA-owned account using invoke_signed
#[tokio::test]
async fn test_transfer_pda_owned() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive the PDA that will own the source account
    let (pda_owner, _bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);
    let dest_owner = payer.pubkey();

    // Create a compressed mint with:
    // - PDA-owned source with 1000 tokens
    // - Regular destination with 0 tokens
    let (_mint_pda, _compression_address, ata_pubkeys) = setup_create_compressed_mint(
        &mut rpc,
        &payer,
        payer.pubkey(),
        9,
        vec![(1000, pda_owner), (0, dest_owner)],
    )
    .await;

    let source_ata = ata_pubkeys[0];
    let dest_ata = ata_pubkeys[1];

    // Transfer 300 tokens using invoke_signed
    let transfer_data = TransferData { amount: 300 };
    let instruction_data = [
        vec![InstructionType::CTokenTransferInvokeSigned as u8],
        transfer_data.try_to_vec().unwrap(),
    ]
    .concat();

    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(source_ata, false),
            AccountMeta::new(dest_ata, false),
            AccountMeta::new_readonly(pda_owner, false), // PDA authority, not signer
            AccountMeta::new_readonly(CTOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify final balances
    use light_ctoken_types::state::CToken;
    let source_data_after = rpc.get_account(source_ata).await.unwrap().unwrap();
    let source_state_after = CToken::deserialize(&mut &source_data_after.data[..]).unwrap();
    assert_eq!(
        source_state_after.amount, 700,
        "PDA source should have 700 tokens after transfer"
    );

    let dest_data_after = rpc.get_account(dest_ata).await.unwrap().unwrap();
    let dest_state_after = CToken::deserialize(&mut &dest_data_after.data[..]).unwrap();
    assert_eq!(
        dest_state_after.amount, 300,
        "Destination should have 300 tokens after transfer"
    );
}
