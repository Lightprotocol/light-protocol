/*

use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{account::Account, signature::Signer, transaction::Transaction},
    spl_example_transfer_lamports::processor::process_instruction,
    std::str::FromStr,
};

#[tokio::test]
async fn test_lamport_transfer() {
    let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();
    let user_pubkey = Pubkey::new_unique();
    let destination_pubkey = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "spl_example_transfer_lamports",
        program_id,
        processor!(process_instruction),
    );
    program_test.add_account(
        user_pubkey,
        Account {
            lamports: 10000000000,//10 sol
            owner: program_id, // Can only withdraw lamports from accounts owned by the program
            ..Account::default()
        },
    );
    program_test.add_account(
        destination_pubkey,
        Account {
            lamports: 10000000000,//10 sol
            ..Account::default()
        },
    );
    program_test.add_account(
        destination_pubkey,
        Account {
            lamports: 10000000000,//10 sol
            ..Account::default()
        },
    );
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &(),
            vec![
                AccountMeta::new(user_pubkey, false),
                AccountMeta::new(destination_pubkey, false),
            ],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}
*/
