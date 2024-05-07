#![cfg(feature = "test-sbf")]

use anchor_lang::context;
use anchor_lang::AnchorDeserialize;
use anchor_lang::AnchorSerialize;
use light_circuitlib_rs::gnark::helpers::kill_gnark_server;
use light_compressed_pda::{
    invoke::processor::CompressedProof,
    sdk::compressed_account::{CompressedAccountWithMerkleContext, MerkleContext},
};
use light_compressed_token::{
    get_cpi_authority_pda, get_token_pool_pda, mint_sdk::create_mint_to_instruction,
    token_data::TokenData, transfer_sdk, ErrorCode, TokenTransferOutputData,
};
use light_hasher::Poseidon;
use light_test_utils::spl::create_token_account;
use light_test_utils::spl::decompress_test;
use light_test_utils::spl::get_merkle_tree_snapshots;
use light_test_utils::spl::mint_tokens_helper;
use light_test_utils::spl::perform_compressed_transfer_test;
use light_test_utils::spl::{assert_create_mint, assert_mint_to};
use light_test_utils::test_indexer::create_initialize_mint_instructions;
use light_test_utils::test_indexer::TokenDataWithContext;
use light_test_utils::{
    airdrop_lamports, assert_custom_error_or_program_error, create_and_send_transaction,
    create_and_send_transaction_with_event, test_env::setup_test_programs_with_accounts,
    test_indexer::TestIndexer, FeeConfig, TransactionParams,
};
use light_verifier::VerifierError;
use solana_program_test::{
    BanksClientError, BanksTransactionResultWithMetadata, ProgramTestContext,
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};
use std::vec;

#[tokio::test]
async fn test_create_mint() {
    let (mut context, _) = setup_test_programs_with_accounts(None).await;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let rent = context
        .banks_client
        .get_rent()
        .await
        .unwrap()
        .minimum_balance(anchor_spl::token::Mint::LEN);
    let mint = Keypair::new();
    let (instructions, pool) =
        create_initialize_mint_instructions(&payer_pubkey, &payer_pubkey, rent, 2, &mint);

    create_and_send_transaction(&mut context, &instructions, &payer_pubkey, &[&payer, &mint])
        .await
        .unwrap();
    assert_create_mint(&mut context, &payer_pubkey, &mint.pubkey(), &pool).await;
}

async fn create_mint_helper(context: &mut ProgramTestContext, payer: &Keypair) -> Pubkey {
    let payer_pubkey = payer.pubkey();
    let rent = context
        .banks_client
        .get_rent()
        .await
        .unwrap()
        .minimum_balance(anchor_spl::token::Mint::LEN);
    let mint = Keypair::new();

    let (instructions, pool) =
        create_initialize_mint_instructions(&payer_pubkey, &payer_pubkey, rent, 2, &mint);

    create_and_send_transaction(context, &instructions, &payer_pubkey, &[&payer, &mint])
        .await
        .unwrap();
    assert_create_mint(context, &payer_pubkey, &mint.pubkey(), &pool).await;
    mint.pubkey()
}

async fn test_mint_to<const MINTS: usize, const ITER: usize>() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let mut test_indexer = TestIndexer::init_from_env(
        &payer.insecure_clone(),
        &env,
        true,
        false,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;

    let recipient_keypair = Keypair::new();
    let mint = create_mint_helper(&mut context, &payer).await;
    for i in 0..ITER {
        let amount = 10000u64;
        let instruction = create_mint_to_instruction(
            &payer_pubkey,
            &payer_pubkey,
            &mint,
            &merkle_tree_pubkey,
            vec![amount; MINTS],
            vec![recipient_keypair.pubkey(); MINTS],
        );
        let snapshots = get_merkle_tree_snapshots(
            &mut context,
            &test_indexer,
            &vec![merkle_tree_pubkey; MINTS],
        )
        .await;
        let event = create_and_send_transaction_with_event(
            &mut context,
            &[instruction],
            &payer_pubkey,
            &[&payer],
            Some(TransactionParams {
                num_new_addresses: 0,
                num_input_compressed_accounts: 0,
                num_output_compressed_accounts: MINTS as u8,
                compress: 0,
                fee_config: FeeConfig::default(),
            }),
        )
        .await
        .unwrap()
        .unwrap();

        if i == 0 {
            test_indexer.add_compressed_accounts_with_token_data(event);
            assert_mint_to(
                MINTS,
                &mut context,
                &test_indexer,
                &recipient_keypair,
                mint,
                amount,
                &snapshots,
            )
            .await;
        }
    }
    kill_gnark_server();
}

#[tokio::test]
async fn test_1_mint_to() {
    test_mint_to::<1, 1>().await
}

#[tokio::test]
async fn test_5_mint_to() {
    test_mint_to::<5, 1>().await
}

#[tokio::test]
async fn test_10_mint_to() {
    test_mint_to::<10, 1>().await
}

#[tokio::test]
async fn test_20_mint_to() {
    test_mint_to::<20, 1>().await
}

#[tokio::test]
async fn test_25_mint_to() {
    test_mint_to::<25, 10>().await
}

#[tokio::test]
async fn test_transfers() {
    let possible_inputs = [1, 2, 3, 4, 8];
    for input_num in possible_inputs {
        for output_num in 1..11 {
            if input_num == 8 && output_num > 7 {
                // 8 inputs and 7 outputs is the max we can do
                break;
            }
            println!(
                "\n\ninput num: {}, output num: {}\n\n",
                input_num, output_num
            );
            test_transfer(input_num, output_num, 10_000).await
        }
    }
}

#[tokio::test]
async fn test_1_transfer() {
    let possible_inputs = [1];
    for input_num in possible_inputs {
        for output_num in 1..2 {
            if input_num == 8 && output_num > 7 {
                // 8 inputs and 7 outputs is the max we can do
                break;
            }
            println!(
                "\n\ninput num: {}, output num: {}\n\n",
                input_num, output_num
            );
            test_transfer(input_num, output_num, 10_000).await
        }
    }
}

#[tokio::test]
async fn test_2_transfer() {
    let possible_inputs = [2];
    for input_num in possible_inputs {
        for output_num in 2..3 {
            if input_num == 8 && output_num > 7 {
                // 8 inputs and 7 outputs is the max we can do
                break;
            }
            println!(
                "\n\ninput num: {}, output num: {}\n\n",
                input_num, output_num
            );
            test_transfer(input_num, output_num, 10_000).await
        }
    }
}

#[tokio::test]
async fn test_8_transfer() {
    let possible_inputs = [8];
    for input_num in possible_inputs {
        for output_num in 2..3 {
            if input_num == 8 && output_num > 7 {
                // 8 inputs and 7 outputs is the max we can do
                break;
            }
            println!(
                "\n\ninput num: {}, output num: {}\n\n",
                input_num, output_num
            );
            test_transfer(input_num, output_num, 10_000).await
        }
    }
}

/// Creates inputs compressed accounts with amount tokens each
/// Transfers all tokens from inputs compressed accounts evenly distributed to outputs compressed accounts
async fn test_transfer(inputs: usize, outputs: usize, amount: u64) {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let mut test_indexer = TestIndexer::init_from_env(
        &payer.insecure_clone(),
        &env,
        true,
        false,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    let recipient_keypair = Keypair::new();
    let mint = create_mint_helper(&mut context, &payer).await;
    let instruction = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &mint,
        &merkle_tree_pubkey,
        vec![amount; inputs],
        vec![recipient_keypair.pubkey(); inputs],
    );

    let snapshots = get_merkle_tree_snapshots(
        &mut context,
        &test_indexer,
        &vec![merkle_tree_pubkey; inputs],
    )
    .await;
    let event = create_and_send_transaction_with_event(
        &mut context,
        &[instruction],
        &payer_pubkey,
        &[&payer],
        Some(TransactionParams {
            num_new_addresses: 0,
            num_input_compressed_accounts: 0,
            num_output_compressed_accounts: inputs as u8,
            compress: 0,
            fee_config: FeeConfig::default(),
        }),
    )
    .await
    .unwrap()
    .unwrap();

    test_indexer.add_compressed_accounts_with_token_data(event);
    assert_mint_to(
        inputs,
        &mut context,
        &test_indexer,
        &recipient_keypair,
        mint,
        amount,
        &snapshots,
    )
    .await;

    let equal_amount = (amount * inputs as u64) / outputs as u64;
    let amounts = vec![equal_amount; outputs];
    let keypairs = amounts
        .iter()
        .map(|_| Keypair::new().pubkey())
        .collect::<Vec<_>>();

    let input_compressed_accounts = test_indexer.token_compressed_accounts[0..inputs]
        .iter()
        .map(|x| x.clone())
        .collect::<Vec<_>>();
    perform_compressed_transfer_test(
        &payer,
        &mut context,
        &mut test_indexer,
        &mint,
        &recipient_keypair,
        &keypairs,
        &amounts,
        &input_compressed_accounts,
        &vec![merkle_tree_pubkey; amounts.len()],
    )
    .await;
    kill_gnark_server();

    // TODO: fix nullify function
    // test_indexer.nullify_compressed_accounts(&mut context).await;
}

#[tokio::test]
async fn test_decompression() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;
    let mut test_indexer = TestIndexer::init_from_env(
        &payer.insecure_clone(),
        &env,
        true,
        false,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    let recipient_keypair = Keypair::new();
    airdrop_lamports(&mut context, &recipient_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let mint = create_mint_helper(&mut context, &payer).await;
    let amount = 10000u64;
    mint_tokens_helper(
        &mut context,
        &mut test_indexer,
        &merkle_tree_pubkey,
        &payer,
        &mint,
        vec![amount],
        vec![recipient_keypair.pubkey()],
    )
    .await;
    let recipient_token_account_keypair = Keypair::new();

    create_token_account(
        &mut context,
        &mint,
        &recipient_token_account_keypair,
        &recipient_keypair,
    )
    .await
    .unwrap();

    let input_compressed_account_token_data = test_indexer.token_compressed_accounts[0].token_data;
    let input_compressed_accounts = vec![test_indexer.token_compressed_accounts[0].clone()];
    let transaction_params = Some(TransactionParams {
        num_new_addresses: 0,
        num_input_compressed_accounts: 1,
        num_output_compressed_accounts: 1,
        compress: 5000, // for second signer
        fee_config: FeeConfig::default(),
    });
    decompress_test(
        &payer,
        &mut context,
        &mut test_indexer,
        input_compressed_accounts,
        1000,
        &merkle_tree_pubkey,
        &recipient_token_account_keypair.pubkey(),
        transaction_params,
    )
    .await;

    let compress_out_compressed_account = TokenTransferOutputData {
        amount: 1000,
        owner: recipient_keypair.pubkey(),
        lamports: None,
    };
    let approve_instruction = spl_token::instruction::approve(
        &anchor_spl::token::ID,
        &recipient_token_account_keypair.pubkey(),
        &get_cpi_authority_pda().0,
        &recipient_keypair.pubkey(),
        &[&recipient_keypair.pubkey()],
        amount,
    )
    .unwrap();
    // Compression
    let instruction = transfer_sdk::create_transfer_instruction(
        &payer_pubkey,
        &recipient_keypair.pubkey(),        // authority
        &[],                                // input_compressed_account_merkle_tree_pubkeys
        &[merkle_tree_pubkey],              // output_compressed_account_merkle_tree_pubkeys
        &[compress_out_compressed_account], // output_compressed_accounts
        &Vec::new(),                        // root_indices
        &None,
        &Vec::new(),                                    // input_token_data
        mint,                                           // mint
        None,                                           // owner_if_delegate_is_signer
        true,                                           // is_compress
        Some(1000u64),                                  // compression_amount
        Some(get_token_pool_pda(&mint)),                // token_pool_pda
        Some(recipient_token_account_keypair.pubkey()), // decompress_token_account
    )
    .unwrap();

    let event = create_and_send_transaction_with_event(
        &mut context,
        &[approve_instruction, instruction],
        &payer_pubkey,
        &[&payer, &recipient_keypair],
        Some(TransactionParams {
            num_new_addresses: 0,
            num_input_compressed_accounts: 0,
            num_output_compressed_accounts: 1,
            compress: 5000, // for second signer
            fee_config: FeeConfig::default(),
        }),
    )
    .await
    .unwrap()
    .unwrap();
    test_indexer.add_compressed_accounts_with_token_data(event);
    assert!(test_indexer
        .token_compressed_accounts
        .iter()
        .any(|x| x.token_data.amount == 1000));
    assert!(test_indexer
        .token_compressed_accounts
        .iter()
        .any(|x| x.token_data.owner == recipient_keypair.pubkey()));
    kill_gnark_server();
}

/// Failing security tests:
/// Out utxo tests:
/// 1. Invalid token data amount (+ 1)
/// 2. Invalid token data amount (- 1)
/// 3. Invalid token data zero out amount
/// 4. Invalid double token data amount
/// In utxo tests:
/// 1. Invalid delegate
/// 2. Invalid owner
/// 3. Invalid is native (deactivated, revisit)
/// 4. Invalid account state
/// 5. Invalid delegated amount
#[tokio::test]
async fn test_invalid_inputs() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.payer.insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;
    let mut test_indexer = TestIndexer::init_from_env(
        &payer.insecure_clone(),
        &env,
        true,
        false,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    let recipient_keypair = Keypair::new();
    airdrop_lamports(&mut context, &recipient_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let mint = create_mint_helper(&mut context, &payer).await;
    let amount = 10000u64;
    mint_tokens_helper(
        &mut context,
        &mut test_indexer,
        &merkle_tree_pubkey,
        &payer,
        &mint,
        vec![amount],
        vec![recipient_keypair.pubkey()],
    )
    .await;
    let transfer_recipient_keypair = Keypair::new();
    let input_compressed_account_token_data = test_indexer.token_compressed_accounts[0].token_data;
    let input_compressed_accounts = vec![test_indexer.token_compressed_accounts[0]
        .compressed_account
        .clone()];
    let proof_rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[input_compressed_accounts[0]
                .compressed_account
                .hash::<Poseidon>(
                    &merkle_tree_pubkey,
                    &input_compressed_accounts[0].merkle_context.leaf_index,
                )
                .unwrap()]),
            &vec![merkle_tree_pubkey],
            None,
            &[],
            &mut context,
        )
        .await;
    let change_out_compressed_account_0 = TokenTransferOutputData {
        amount: input_compressed_account_token_data.amount - 1000,
        owner: recipient_keypair.pubkey(),
        lamports: None,
    };
    let transfer_recipient_out_compressed_account_0 = TokenTransferOutputData {
        amount: 1000 + 1,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
    };
    // invalid token data amount (+ 1)
    let res = create_transfer_out_compressed_account_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_custom_error_or_program_error(res, ErrorCode::ComputeOutputSumFailed.into()).unwrap();
    let transfer_recipient_out_compressed_account_0 = TokenTransferOutputData {
        amount: 1000 - 1,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
    };
    // invalid token data amount (- 1)
    let res = create_transfer_out_compressed_account_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();

    assert_custom_error_or_program_error(res, ErrorCode::SumCheckFailed.into()).unwrap();

    let zero_amount = TokenTransferOutputData {
        amount: 0,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
    };
    // invalid token data zero out amount
    let res = create_transfer_out_compressed_account_test(
        &mut context,
        zero_amount,
        zero_amount,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_custom_error_or_program_error(res, ErrorCode::SumCheckFailed.into()).unwrap();

    let double_amount = TokenTransferOutputData {
        amount: input_compressed_account_token_data.amount,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
    };
    // invalid double token data  amount
    let res = create_transfer_out_compressed_account_test(
        &mut context,
        double_amount,
        double_amount,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();

    assert_custom_error_or_program_error(res, ErrorCode::ComputeOutputSumFailed.into()).unwrap();

    let invalid_lamports_amount = TokenTransferOutputData {
        amount: 1000,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: Some(1),
    };

    // invalid_lamports_amount
    let res = create_transfer_out_compressed_account_test(
        &mut context,
        change_out_compressed_account_0,
        invalid_lamports_amount,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_custom_error_or_program_error(
        res,
        light_compressed_pda::errors::CompressedPdaError::ComputeOutputSumFailed.into(),
    )
    .unwrap();

    let mut input_compressed_account_token_data_invalid_amount =
        test_indexer.token_compressed_accounts[0].token_data;
    input_compressed_account_token_data_invalid_amount.amount = 0;
    let mut input_compressed_accounts = vec![test_indexer.token_compressed_accounts[0]
        .compressed_account
        .clone()];
    crate::TokenData::serialize(
        &input_compressed_account_token_data_invalid_amount,
        &mut input_compressed_accounts[0]
            .compressed_account
            .data
            .as_mut()
            .unwrap()
            .data
            .as_mut_slice(),
    )
    .unwrap();
    let change_out_compressed_account_0 = TokenTransferOutputData {
        amount: input_compressed_account_token_data.amount - 1000,
        owner: recipient_keypair.pubkey(),
        lamports: None,
    };
    let transfer_recipient_out_compressed_account_0 = TokenTransferOutputData {
        amount: 1000,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
    };

    let res = create_transfer_out_compressed_account_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_custom_error_or_program_error(res, ErrorCode::ComputeOutputSumFailed.into()).unwrap();

    let mut input_compressed_account_token_data =
        test_indexer.token_compressed_accounts[0].token_data;
    input_compressed_account_token_data.delegate = Some(Pubkey::new_unique());
    input_compressed_account_token_data.delegated_amount = 1;
    let mut input_compressed_accounts = vec![test_indexer.token_compressed_accounts[0]
        .compressed_account
        .clone()];
    let mut vec = Vec::new();
    crate::TokenData::serialize(&input_compressed_account_token_data, &mut vec).unwrap();
    input_compressed_accounts[0]
        .compressed_account
        .data
        .as_mut()
        .unwrap()
        .data = vec;
    let res = create_transfer_out_compressed_account_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_custom_error_or_program_error(res, VerifierError::ProofVerificationFailed.into())
        .unwrap();
    let input_compressed_accounts = vec![test_indexer.token_compressed_accounts[0]
        .compressed_account
        .clone()];
    let res = create_transfer_out_compressed_account_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &payer,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_custom_error_or_program_error(res, VerifierError::ProofVerificationFailed.into())
        .unwrap();
    let mut input_compressed_account_token_data =
        test_indexer.token_compressed_accounts[0].token_data;
    input_compressed_account_token_data.is_native = Some(0);
    let mut input_compressed_accounts = vec![test_indexer.token_compressed_accounts[0]
        .compressed_account
        .clone()];
    let mut vec = Vec::new();
    crate::TokenData::serialize(&input_compressed_account_token_data, &mut vec).unwrap();
    input_compressed_accounts[0]
        .compressed_account
        .data
        .as_mut()
        .unwrap()
        .data = vec;
    let res = create_transfer_out_compressed_account_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();

    assert_custom_error_or_program_error(res, VerifierError::ProofVerificationFailed.into())
        .unwrap();

    let mut input_compressed_account_token_data =
        test_indexer.token_compressed_accounts[0].token_data;
    input_compressed_account_token_data.delegated_amount = 1;
    let mut input_compressed_accounts = vec![test_indexer.token_compressed_accounts[0]
        .compressed_account
        .clone()];
    let mut vec = Vec::new();
    crate::TokenData::serialize(&input_compressed_account_token_data, &mut vec).unwrap();
    input_compressed_accounts[0]
        .compressed_account
        .data
        .as_mut()
        .unwrap()
        .data = vec;
    let res = create_transfer_out_compressed_account_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_custom_error_or_program_error(res, ErrorCode::DelegateUndefined.into()).unwrap();
    kill_gnark_server();
}

/// Helper function to create failing tests
#[allow(clippy::too_many_arguments)]
async fn create_transfer_out_compressed_account_test(
    context: &mut ProgramTestContext,
    change_token_transfer_output: TokenTransferOutputData,
    transfer_recipient_token_transfer_output: TokenTransferOutputData,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    payer: &Keypair,
    proof: &Option<CompressedProof>,
    root_indices: &[u16],
    input_compressed_accounts: &[CompressedAccountWithMerkleContext],
) -> Result<BanksTransactionResultWithMetadata, BanksClientError> {
    let input_compressed_account_token_data: Vec<TokenData> = input_compressed_accounts
        .iter()
        .map(|x| {
            TokenData::deserialize(&mut &x.compressed_account.data.as_ref().unwrap().data[..])
                .unwrap()
        })
        .collect();
    let instruction = transfer_sdk::create_transfer_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &input_compressed_accounts
            .iter()
            .map(|x| MerkleContext {
                merkle_tree_pubkey: *merkle_tree_pubkey,
                nullifier_queue_pubkey: *nullifier_queue_pubkey,
                leaf_index: x.merkle_context.leaf_index,
            })
            .collect::<Vec<MerkleContext>>(),
        &[*merkle_tree_pubkey, *merkle_tree_pubkey], // output compressed account Merkle trees
        &[
            change_token_transfer_output,
            transfer_recipient_token_transfer_output,
        ],
        root_indices,
        &proof,
        input_compressed_account_token_data.as_slice(),
        input_compressed_account_token_data[0].mint,
        None,
        false,
        None,
        None,
        None,
    )
    .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        [&payer].as_slice(),
        context.get_new_latest_blockhash().await.unwrap(),
    );
    solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await
}
