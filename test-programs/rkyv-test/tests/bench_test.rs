#![cfg(feature = "test-sbf")]

use anchor_lang::InstructionData;
use light_test_utils::test_env::setup_test_programs_with_accounts;
use rkyv_test::instruction::InvokeTestRkyv;
use rkyv_test::ID;

use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::{signer::Signer, transaction::Transaction};
#[tokio::test]
async fn bench_rkyv() {
    let (mut context, env) =
        setup_test_programs_with_accounts(Some(vec![(String::from("rkyv_test"), ID)])).await;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();

    let instruction = InvokeTestRkyv {};
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![AccountMeta::new(payer_pubkey, true)],
        data: instruction.data(),
    };
    let block_hash = context.banks_client.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        block_hash,
    );
    let result = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}
