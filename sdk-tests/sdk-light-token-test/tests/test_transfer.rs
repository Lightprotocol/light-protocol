// Tests for CTokenTransfer invoke() and invoke_signed()

mod shared;

use borsh::{BorshDeserialize, BorshSerialize};
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token::token::LIGHT_TOKEN_PROGRAM_ID;
use native_ctoken_examples::{InstructionType, TransferData, ID, TOKEN_ACCOUNT_SEED};
use shared::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
};

/// Test CTokenTransfer using invoke()
#[tokio::test]
async fn test_ctoken_transfer_invoke() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let source_owner = payer.pubkey();
    let dest_owner = Pubkey::new_unique();

    let (_mint_pda, _compression_address, ata_pubkeys, _mint_seed) = setup_create_mint(
        &mut rpc,
        &payer,
        payer.pubkey(),
        9,
        vec![(1000, source_owner), (0, dest_owner)],
    )
    .await;

    let source_ata = ata_pubkeys[0];
    let dest_ata = ata_pubkeys[1];

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
            AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify final balances
    use light_token_interface::state::Token;
    let source_data_after = rpc.get_account(source_ata).await.unwrap().unwrap();
    let source_state_after = Token::deserialize(&mut &source_data_after.data[..]).unwrap();
    assert_eq!(source_state_after.amount, 500);

    let dest_data_after = rpc.get_account(dest_ata).await.unwrap().unwrap();
    let dest_state_after = Token::deserialize(&mut &dest_data_after.data[..]).unwrap();
    assert_eq!(dest_state_after.amount, 500);
}

/// Test CTokenTransfer using invoke_signed() with PDA authority
#[tokio::test]
async fn test_ctoken_transfer_invoke_signed() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive the PDA that will own the source account
    let (pda_owner, _bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);
    let dest_owner = payer.pubkey();

    let (_mint_pda, _compression_address, ata_pubkeys, _mint_seed) = setup_create_mint(
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
            AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify final balances
    use light_token_interface::state::Token;
    let source_data_after = rpc.get_account(source_ata).await.unwrap().unwrap();
    let source_state_after = Token::deserialize(&mut &source_data_after.data[..]).unwrap();
    assert_eq!(source_state_after.amount, 700);

    let dest_data_after = rpc.get_account(dest_ata).await.unwrap().unwrap();
    let dest_state_after = Token::deserialize(&mut &dest_data_after.data[..]).unwrap();
    assert_eq!(dest_state_after.amount, 300);
}
