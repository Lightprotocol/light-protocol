#![cfg(feature = "test-sbf")]

use account_compression::StateMerkleTreeAccount;
use light_compressed_token::mint_sdk::create_mint_to_instruction;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::test_rpc::ProgramTestRpcConnection;
use light_test_utils::transaction_params::{FeeConfig, TransactionParams};
use light_test_utils::{
    assert_custom_error_or_program_error,
    test_env::setup_test_programs_with_accounts,
    test_indexer::{create_mint_helper, TestIndexer},
    AccountZeroCopy,
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};

#[tokio::test]
async fn test_program_owned_merkle_tree() {
    let (mut rpc, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("program_owned_account_test"),
        program_owned_account_test::ID,
    )]))
    .await;
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    let program_owned_merkle_tree_keypair = Keypair::new();
    let program_owned_merkle_tree_pubkey = program_owned_merkle_tree_keypair.pubkey();
    let program_owned_nullifier_queue_keypair = Keypair::new();
    let cpi_signature_keypair = Keypair::new();

    let mut test_indexer = TestIndexer::<200, ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        true,
        true,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    test_indexer
        .add_state_merkle_tree(
            &mut rpc,
            &program_owned_merkle_tree_keypair,
            &program_owned_nullifier_queue_keypair,
            &cpi_signature_keypair,
            Some(light_compressed_token::ID),
        )
        .await;

    let recipient_keypair = Keypair::new();
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let instruction = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &mint,
        &program_owned_merkle_tree_pubkey,
        vec![amount; 1],
        vec![recipient_keypair.pubkey(); 1],
    );
    let pre_merkle_tree_account =
        AccountZeroCopy::<StateMerkleTreeAccount>::new(&mut rpc, program_owned_merkle_tree_pubkey)
            .await;
    let pre_merkle_tree = pre_merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    let event = rpc
        .create_and_send_transaction_with_event(
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
    let post_merkle_tree_account =
        AccountZeroCopy::<StateMerkleTreeAccount>::new(&mut rpc, program_owned_merkle_tree_pubkey)
            .await;
    let post_merkle_tree = post_merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    test_indexer.add_compressed_accounts_with_token_data(&event);
    assert_ne!(post_merkle_tree.root(), pre_merkle_tree.root());
    assert_eq!(
        post_merkle_tree.root(),
        test_indexer.state_merkle_trees[1].merkle_tree.root()
    );

    let invalid_program_owned_merkle_tree_keypair = Keypair::new();
    let invalid_program_owned_merkle_tree_pubkey =
        invalid_program_owned_merkle_tree_keypair.pubkey();
    let invalid_program_owned_nullifier_queue_keypair = Keypair::new();
    let cpi_signature_keypair = Keypair::new();
    test_indexer
        .add_state_merkle_tree(
            &mut rpc,
            &invalid_program_owned_merkle_tree_keypair,
            &invalid_program_owned_nullifier_queue_keypair,
            &cpi_signature_keypair,
            Some(Pubkey::new_unique()),
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

    let latest_blockhash = rpc.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        latest_blockhash,
    );
    let res = rpc.process_transaction(transaction).await;
    assert_custom_error_or_program_error(
        res,
        light_system_program::errors::CompressedPdaError::InvalidMerkleTreeOwner.into(),
    )
    .unwrap();
}
