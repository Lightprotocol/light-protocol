//#![cfg(feature = "test-sbf")]

// 2. escrow tokens with compressed pda
// create test env
// create mint and mint tokens
// escrow compressed tokens - with compressed pda
// release compressed tokens

// TODO: 3. escrow tokens by decompression with compressed pda
// this design pattern can be used to use compressed accounts with an AMMM
// create test env
// create mint and mint tokens
// decomcompress compressed tokens into program owned token account - with compressed pda
// release compressed tokens

use anchor_lang::{InstructionData, ToAccountMetas};
use light_test_utils::create_and_send_transaction;
use light_test_utils::test_env::setup_test_programs_with_accounts;
use light_test_utils::test_indexer::{create_mint_helper, mint_tokens_helper, TestIndexer};
use solana_program_test::ProgramTestContext;
use solana_sdk::instruction::Instruction;
use solana_sdk::system_program;
use solana_sdk::{pubkey::Pubkey, signer::Signer, transaction::Transaction};
use token_escrow::compressed_pda_sdk::create_escrow_instruction;

pub async fn init_cpi_signature_account(context: &mut ProgramTestContext) -> Pubkey {
    let payer = context.payer.insecure_clone();
    let compressed_sol_pda = Pubkey::find_program_address(
        &[psp_compressed_pda::instructions::CPI_SEED],
        &psp_compressed_pda::id(),
    )
    .0;
    let data = psp_compressed_pda::instruction::InitCpiSignatureAccount {};
    let accounts = psp_compressed_pda::accounts::InitializeCpiSignatureAccount {
        fee_payer: payer.insecure_clone().pubkey(),
        compressed_sol_pda,
        system_program: system_program::ID,
    };
    let instruction = Instruction {
        program_id: psp_compressed_pda::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: data.data(),
    };
    create_and_send_transaction(context, &[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    compressed_sol_pda
}

/// Steps:
/// 1. create test env
/// 2. create mint and mint tokens
/// 3. escrow compressed tokens
/// 4. withdraw compressed tokens
#[tokio::test]
async fn test_escrow_with_compressed_pda() {
    let env: light_test_utils::test_env::EnvWithAccounts = setup_test_programs_with_accounts(Some(
        vec![(String::from("token_escrow"), token_escrow::ID)],
    ))
    .await;
    let mut context = env.context;
    let cpi_signature_account = init_cpi_signature_account(&mut context).await;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let indexed_array_pubkey = env.indexed_array_pubkey;
    let address_merkle_tree_pubkey = env.address_merkle_tree_pubkey;
    let test_indexer = TestIndexer::new(
        merkle_tree_pubkey,
        indexed_array_pubkey,
        address_merkle_tree_pubkey,
        payer.insecure_clone(),
        true,
        true,
        false,
    );
    let mint = create_mint_helper(&mut context, &payer).await;
    let mut test_indexer = test_indexer.await;
    // big footgun signer check of token account is done with zkp onchain thus no conclusive error message
    // let recipient_keypair = Keypair::new();
    let amount = 10000u64;
    mint_tokens_helper(
        &mut context,
        &mut test_indexer,
        &merkle_tree_pubkey,
        &payer,
        &mint,
        vec![amount],
        vec![payer.pubkey()],
    )
    .await;
    let input_compressed_token_account_data = test_indexer.token_compressed_accounts[0].clone();

    let compressed_input_account_with_context =
        test_indexer.compressed_accounts[input_compressed_token_account_data.index].clone();
    let input_compressed_account_hash = test_indexer.compressed_accounts
        [input_compressed_token_account_data.index]
        .compressed_account
        .hash(
            &merkle_tree_pubkey,
            &compressed_input_account_with_context.leaf_index,
        )
        .unwrap();
    let seed = [1u8; 32];
    let address = psp_compressed_pda::compressed_account::derive_address(
        &env.address_merkle_tree_pubkey,
        &seed,
    )
    .unwrap();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[input_compressed_account_hash]),
            Some(&[address]),
            &mut context,
        )
        .await;

    let escrow_amount = 100u64;
    let new_address_params: psp_compressed_pda::NewAddressParams =
        psp_compressed_pda::NewAddressParams {
            seed: [1u8; 32],
            address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
            address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
            address_merkle_tree_root_index: rpc_result.address_root_indices[0],
        };
    let create_ix_inputs =
        token_escrow::compressed_pda_sdk::CreateCompressedPdaEscrowInstructionInputs {
            input_token_data: &vec![input_compressed_token_account_data.token_data],
            lock_up_time: 0,
            signer: &payer_pubkey,
            input_compressed_account_merkle_tree_pubkeys: &[merkle_tree_pubkey],
            nullifier_array_pubkeys: &[indexed_array_pubkey],
            output_compressed_account_merkle_tree_pubkeys: &[
                merkle_tree_pubkey,
                merkle_tree_pubkey,
            ],
            output_compressed_accounts: &Vec::new(),
            root_indices: &rpc_result.root_indices,
            proof: &rpc_result.proof,
            leaf_indices: &[compressed_input_account_with_context.leaf_index],
            mint: &input_compressed_token_account_data.token_data.mint,
            new_address_params,
        };
    let instruction = create_escrow_instruction(
        create_ix_inputs.clone(),
        escrow_amount,
        cpi_signature_account,
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_new_latest_blockhash().await.unwrap(),
    );
    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await;
    test_indexer.add_compressed_accounts_with_token_data(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );

    let token_data_escrow = test_indexer.token_compressed_accounts[1].token_data.clone();
    assert_eq!(token_data_escrow.amount, escrow_amount);
    let cpi_signer = Pubkey::find_program_address(
        &[b"escrow".as_ref(), payer_pubkey.as_ref()],
        &token_escrow::id(),
    )
    .0;
    assert_eq!(token_data_escrow.owner, cpi_signer);

    let token_data_change_compressed_token_account =
        test_indexer.token_compressed_accounts[2].token_data.clone();
    assert_eq!(
        token_data_change_compressed_token_account.amount,
        amount - escrow_amount
    );
    assert_eq!(
        token_data_change_compressed_token_account.owner,
        payer_pubkey
    );

    // TODO: assert compressed pda creation
}
