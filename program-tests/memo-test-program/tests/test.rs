// #![cfg(feature = "test-sbf")]
use light_client::indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts};
use light_client::rpc::rpc_connection::RpcConnection;
use light_program_test::indexer::{TestIndexer, TestIndexerExtensions};
use light_program_test::test_env::setup_test_programs_with_accounts_v2;
use light_program_test::test_rpc::ProgramTestRpcConnection;
use light_prover_client::gnark::helpers::{ProofType, ProverConfig};
use litesvm::LiteSVM;
use memo_test_program::process_instruction;
use solana_program_test::processor;
use solana_program_test::ProgramTest;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::message::Message;
use solana_sdk::system_instruction::transfer;
use solana_sdk::{
    account::Account, instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};

#[tokio::test]
async fn test_memo_program() {
    // Setup the light test environment
    let (mut rpc, env) = setup_test_programs_with_accounts_v2(Some(vec![(
        String::from("memo_test_program"),
        memo_test_program::ID,
    )]))
    .await;
    let payer = rpc.get_payer().insecure_clone();

    // Setup the zk compression test indexer
    let mut test_indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::new(
        vec![StateMerkleTreeAccounts {
            merkle_tree: env.merkle_tree_pubkey,
            nullifier_queue: env.nullifier_queue_pubkey,
            cpi_context: env.cpi_context_account_pubkey,
        }],
        vec![AddressMerkleTreeAccounts {
            merkle_tree: env.address_merkle_tree_pubkey,
            queue: env.address_merkle_tree_queue_pubkey,
        }],
        payer.insecure_clone(),
        env.group_pda,
        None,
        // Some(ProverConfig {
        //     circuits: vec![ProofType::Inclusion, ProofType::NonInclusion],
        //     run_mode: None,
        // }),
    )
    .await;

    // Create a memo instruction
    let memo_data = b"Test memo data";
    let memo_instruction = Instruction {
        program_id: memo_test_program::ID,
        accounts: vec![AccountMeta::new(payer.pubkey(), true)],
        data: memo_data.to_vec(),
    };

    println!("memo_instruction: {:?}", memo_instruction);

    // Emit event
    let event = rpc
        .create_and_send_transaction_with_event(
            &[memo_instruction],
            &payer.pubkey(),
            &[&payer],
            None,
        )
        .await
        .unwrap();
    let slot = rpc.get_slot().await.unwrap();

    // Index the event
    test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);

    // Assert that the memo was processed correctly
    let processed_memo = std::str::from_utf8(&memo_data[..]).unwrap();
    assert_eq!(processed_memo, "Test memo data");
}
