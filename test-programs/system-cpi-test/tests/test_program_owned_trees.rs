#![cfg(feature = "test-sbf")]

use account_compression::sdk::create_insert_leaves_instruction;
use account_compression::StateMerkleTreeAccount;
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use light_compressed_token::mint_sdk::create_mint_to_instruction;

use light_test_utils::rpc::errors::{assert_rpc_error, RpcError};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::test_rpc::ProgramTestRpcConnection;
use light_test_utils::test_env::{
    initialize_new_group, register_program_with_registry_program,
    COMPRESSED_TOKEN_PROGRAM_PROGRAM_ID,
};
use light_test_utils::transaction_params::{FeeConfig, TransactionParams};
use light_test_utils::{
    assert_custom_error_or_program_error,
    test_env::setup_test_programs_with_accounts,
    test_indexer::{create_mint_helper, TestIndexer},
    AccountZeroCopy,
};
use solana_sdk::instruction::Instruction;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};

// TODO: move to token tests
#[tokio::test]
async fn test_program_owned_merkle_tree() {
    let (mut rpc, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("system_cpi_test"),
        system_cpi_test::ID,
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

/// Test:
/// - Register the test program
/// - failing test registered program signer check
/// 1. FAIL: try to append leaves to the merkle tree from test program with invalid registered program account
/// 2. try to append leaves to the merkle tree from account compression program
/// - register the test program to the correct group
/// 3. SUCCEED: append leaves to the merkle tree from test program
/// - register the token program to the correct group
/// 4. FAIL: try to append leaves to the merkle tree from test program with invalid registered program account
#[tokio::test]
async fn test_invalid_registered_program() {
    let (mut rpc, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("system_cpi_test"),
        system_cpi_test::ID,
    )]))
    .await;
    let payer = rpc.get_payer().insecure_clone();
    let group_seed_keypair = Keypair::new();
    let invalid_group_pda =
        initialize_new_group(&group_seed_keypair, &payer, &mut rpc, payer.pubkey()).await;
    let invalid_group_registered_program_pda =
        register_program(&mut rpc, &payer, &system_cpi_test::ID, &invalid_group_pda)
            .await
            .unwrap();

    let merkle_tree_pubkey = env.merkle_tree_pubkey;

    // invoke account compression program through system cpi test
    // 1. the program is registered with a different group than the Merkle tree
    {
        let derived_address =
            Pubkey::find_program_address(&[b"cpi_authority"], &system_cpi_test::ID).0;
        let accounts = system_cpi_test::accounts::AppendLeavesAccountCompressionProgram {
            signer: payer.pubkey(),
            registered_program_pda: invalid_group_registered_program_pda,
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_program: account_compression::ID,
            cpi_signer: derived_address,
            system_program: solana_sdk::system_program::ID,
            merkle_tree: merkle_tree_pubkey,
        };

        let instruction_data =
            system_cpi_test::instruction::AppendLeavesAccountCompressionProgram {};
        let instruction = Instruction {
            program_id: system_cpi_test::ID,
            accounts: [accounts.to_account_metas(Some(true))].concat(),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        let expected_error_code =
            account_compression::errors::AccountCompressionErrorCode::InvalidAuthority.into();

        let result = match result {
            Ok(_) => {
                println!("expected_error_code: {}", expected_error_code);
                panic!("Transaction should have failed");
            }
            Err(e) => e,
        };
        assert_rpc_error(Err(result), 0, expected_error_code).unwrap();
    }
    // 2. directly invoke account compression program
    {
        let instruction = create_insert_leaves_instruction(
            vec![(0, [1u8; 32])],
            payer.pubkey(),
            payer.pubkey(),
            vec![merkle_tree_pubkey],
        );
        let expected_error_code =
            account_compression::errors::AccountCompressionErrorCode::InvalidAuthority.into();

        let result = match rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await
        {
            Ok(_) => {
                println!("expected_error_code: {}", expected_error_code);
                panic!("Transaction should have failed");
            }
            Err(e) => e,
        };
        assert_rpc_error(Err(result), 0, expected_error_code).unwrap();
    }

    let token_program_registered_program_pda = register_program_with_registry_program(
        &mut rpc,
        &env,
        &COMPRESSED_TOKEN_PROGRAM_PROGRAM_ID,
    )
    .await
    .unwrap();
    // 4. use registered_program_pda of other program
    {
        let derived_address =
            Pubkey::find_program_address(&[b"cpi_authority"], &system_cpi_test::ID).0;
        let accounts = system_cpi_test::accounts::AppendLeavesAccountCompressionProgram {
            signer: payer.pubkey(),
            registered_program_pda: token_program_registered_program_pda,
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_program: account_compression::ID,
            cpi_signer: derived_address,
            system_program: solana_sdk::system_program::ID,
            merkle_tree: merkle_tree_pubkey,
        };

        let instruction_data =
            system_cpi_test::instruction::AppendLeavesAccountCompressionProgram {};
        let instruction = Instruction {
            program_id: system_cpi_test::ID,
            accounts: [accounts.to_account_metas(Some(true))].concat(),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        let expected_error_code =
            account_compression::errors::AccountCompressionErrorCode::InvalidAuthority.into();

        let result = match result {
            Ok(_) => {
                println!("expected_error_code: {}", expected_error_code);
                panic!("Transaction should have failed");
            }
            Err(e) => e,
        };
        assert_rpc_error(Err(result), 0, expected_error_code).unwrap();
    }
}
pub async fn register_program(
    rpc: &mut ProgramTestRpcConnection,
    authority: &Keypair,
    program_id: &Pubkey,
    group_account: &Pubkey,
) -> Result<Pubkey, RpcError> {
    let registered_program_pda = Pubkey::find_program_address(
        &[program_id.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;

    let accounts = account_compression::accounts::RegisterProgramToGroup {
        authority: authority.pubkey(),
        system_program: system_program::ID,
        registered_program_pda,
        group_authority_pda: *group_account,
    };
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: account_compression::instruction::RegisterProgramToGroup {
            program_id: *program_id,
        }
        .data(),
    };

    rpc.create_and_send_transaction(&[instruction], &authority.pubkey(), &[authority])
        .await?;

    Ok(registered_program_pda)
}
