#![cfg(feature = "test-sbf")]

use account_compression::StateMerkleTreeAccount;
use light_compressed_token::mint_sdk::create_mint_to_instruction;
use light_test_utils::{
    assert_custom_error_or_program_error, create_and_send_transaction_with_event,
    test_env::{create_state_merkle_tree_and_queue_account, setup_test_programs_with_accounts},
    test_indexer::{create_mint_helper, TestIndexer},
    AccountZeroCopy, FeeConfig, TransactionParams,
};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

#[tokio::test]
async fn test_program_owned_merkle_tree() {
    let (mut context, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("program_owned_account_test"),
        program_owned_account_test::ID,
    )]))
    .await;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let address_merkle_tree_pubkey = env.address_merkle_tree_pubkey;

    let program_owned_merkle_tree_keypair = Keypair::new();
    let program_owned_merkle_tree_pubkey = program_owned_merkle_tree_keypair.pubkey();
    let program_owned_nullifier_queue_keypair = Keypair::new();
    let program_owned_nullifier_queue_pubkey = program_owned_nullifier_queue_keypair.pubkey();
    create_state_merkle_tree_and_queue_account(
        &payer,
        &mut context,
        &program_owned_merkle_tree_keypair,
        &program_owned_nullifier_queue_keypair,
        Some(light_compressed_token::ID),
        1,
    )
    .await;

    let test_indexer = TestIndexer::new(
        program_owned_merkle_tree_pubkey,
        program_owned_nullifier_queue_pubkey,
        address_merkle_tree_pubkey,
        payer.insecure_clone(),
        true,
        true,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    );

    let mut test_indexer = test_indexer.await;
    let recipient_keypair = Keypair::new();
    let mint = create_mint_helper(&mut context, &payer).await;
    let amount = 10000u64;
    let instruction = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &mint,
        &program_owned_merkle_tree_pubkey,
        vec![amount; 1],
        vec![recipient_keypair.pubkey(); 1],
    );
    let pre_merkle_tree_account = AccountZeroCopy::<StateMerkleTreeAccount>::new(
        &mut context,
        program_owned_merkle_tree_pubkey,
    )
    .await;
    let pre_merkle_tree = pre_merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    let event = create_and_send_transaction_with_event(
        &mut context,
        &[instruction],
        &payer_pubkey,
        &[&payer],
        Some(TransactionParams {
            num_new_addresses: 0,
            num_input_compressed_accounts: 0,
            num_output_compressed_accounts: 1,
            compress: 0,
            fee_config: FeeConfig::default(),
        }),
    )
    .await
    .unwrap()
    .unwrap();
    let post_merkle_tree_account = AccountZeroCopy::<StateMerkleTreeAccount>::new(
        &mut context,
        program_owned_merkle_tree_pubkey,
    )
    .await;
    let post_merkle_tree = post_merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    test_indexer.add_compressed_accounts_with_token_data(event);
    assert_ne!(
        post_merkle_tree.root().unwrap(),
        pre_merkle_tree.root().unwrap()
    );
    assert_eq!(
        post_merkle_tree.root().unwrap(),
        test_indexer.merkle_tree.root()
    );

    let invalid_program_owned_merkle_tree_keypair = Keypair::new();
    let invalid_program_owned_merkle_tree_pubkey =
        invalid_program_owned_merkle_tree_keypair.pubkey();
    let invalid_program_owned_nullifier_queue_keypair = Keypair::new();
    create_state_merkle_tree_and_queue_account(
        &payer,
        &mut context,
        &invalid_program_owned_merkle_tree_keypair,
        &invalid_program_owned_nullifier_queue_keypair,
        Some(program_owned_account_test::ID),
        2,
    )
    .await;
    let recipient_keypair = Keypair::new();
    let instruction = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &mint,
        &invalid_program_owned_merkle_tree_pubkey,
        vec![amount + 1; 1],
        vec![recipient_keypair.pubkey(); 1],
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.last_blockhash,
    );
    let res = context
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .unwrap();

    assert_custom_error_or_program_error(
        res,
        light_compressed_pda::errors::CompressedPdaError::InvalidMerkleTreeOwner.into(),
    )
    .unwrap();
}
