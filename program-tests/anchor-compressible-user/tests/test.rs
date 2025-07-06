#![cfg(feature = "test-sbf")]

use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use anchor_lang::ToAccountMetas;
use solana_program_test::*;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

#[tokio::test]
async fn test_user_record() {
    let program_id = anchor_compressible_user::ID;
    let mut program_test = ProgramTest::new(
        "anchor_compressible_user",
        program_id,
        processor!(anchor_compressible_user::entry),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Test create_record
    let user = payer;
    let (user_record_pda, _bump) = Pubkey::find_program_address(
        &[b"user_record", user.pubkey().as_ref()],
        &program_id,
    );

    let accounts = anchor_compressible_user::accounts::CreateRecord {
        user: user.pubkey(),
        user_record: user_record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = anchor_compressible_user::instruction::CreateRecord {
        name: "Alice".to_string(),
    };

    let instruction = Instruction {
        program_id,
        accounts: accounts.to_account_metas(None),
        data: instruction_data.data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&user.pubkey()),
        &[&user],
        recent_blockhash,
    );

    banks_client.process_transaction(transaction).await.unwrap();

    // Test update_record
    let accounts = anchor_compressible_user::accounts::UpdateRecord {
        user: user.pubkey(),
        user_record: user_record_pda,
    };

    let instruction_data = anchor_compressible_user::instruction::UpdateRecord {
        name: "Alice Updated".to_string(),
        score: 100,
    };

    let instruction = Instruction {
        program_id,
        accounts: accounts.to_account_metas(None),
        data: instruction_data.data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&user.pubkey()),
        &[&user],
        recent_blockhash,
    );

    banks_client.process_transaction(transaction).await.unwrap();
} 