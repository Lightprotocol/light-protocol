#![cfg(feature = "test-sbf")]
use anchor_lang::AnchorDeserialize;
use anchor_lang::AnchorSerialize;
use light_circuitlib_rs::gnark::helpers::kill_gnark_server;
use light_compressed_token::{
    token_data::TokenData, transfer_sdk, ErrorCode, TokenTransferOutputData,
};
use light_hasher::Poseidon;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::compressed_account::{CompressedAccountWithMerkleContext, MerkleContext},
};
use light_test_utils::rpc::errors::RpcError;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::test_rpc::ProgramTestRpcConnection;
use light_test_utils::spl::{
    compress_test, compressed_transfer_test, create_mint_helper, create_token_account,
    decompress_test, mint_tokens_helper,
};
use light_test_utils::{
    airdrop_lamports, assert_custom_error_or_program_error,
    test_env::setup_test_programs_with_accounts, test_indexer::TestIndexer,
};
use light_verifier::VerifierError;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};

#[tokio::test]
async fn test_create_mint() {
    let (mut rpc, _) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    create_mint_helper(&mut rpc, &payer).await;
}

async fn test_mint_to<const MINTS: usize, const ITER: usize>() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let mut test_indexer = TestIndexer::<200, ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        true,
        false,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;

    let recipient_keypair = Keypair::new();
    let mint = create_mint_helper(&mut rpc, &payer).await;
    for _ in 0..ITER {
        let amount = 10000u64;
        mint_tokens_helper(
            &mut rpc,
            &mut test_indexer,
            &merkle_tree_pubkey,
            &payer,
            &mint,
            vec![amount; MINTS],
            vec![recipient_keypair.pubkey(); MINTS],
        )
        .await;
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
            perform_transfer_test(input_num, output_num, 10_000).await
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
            perform_transfer_test(input_num, output_num, 10_000).await
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
            perform_transfer_test(input_num, output_num, 10_000).await
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
            perform_transfer_test(input_num, output_num, 10_000).await
        }
    }
}

/// Creates inputs compressed accounts with amount tokens each
/// Transfers all tokens from inputs compressed accounts evenly distributed to outputs compressed accounts
async fn perform_transfer_test(inputs: usize, outputs: usize, amount: u64) {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let mut test_indexer = TestIndexer::<200, ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        true,
        false,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let sender = Keypair::new();
    mint_tokens_helper(
        &mut rpc,
        &mut test_indexer,
        &merkle_tree_pubkey,
        &payer,
        &mint,
        vec![amount; inputs],
        vec![sender.pubkey(); inputs],
    )
    .await;
    let mut recipients = Vec::new();
    for _ in 0..outputs {
        recipients.push(Pubkey::new_unique());
    }
    let input_compressed_accounts =
        test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
    let equal_amount = (amount * inputs as u64) / outputs as u64;
    let rest_amount = (amount * inputs as u64) % outputs as u64;
    let mut output_amounts = vec![equal_amount; outputs - 1];
    output_amounts.push(equal_amount + rest_amount);
    compressed_transfer_test(
        &payer,
        &mut rpc,
        &mut test_indexer,
        &mint,
        &sender,
        &recipients,
        &output_amounts,
        input_compressed_accounts.as_slice(),
        &vec![env.merkle_tree_pubkey; outputs],
        None,
    )
    .await;
}

#[tokio::test]
async fn test_decompression() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let mut test_indexer = TestIndexer::<200, ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        true,
        false,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    let sender = Keypair::new();
    airdrop_lamports(&mut context, &sender.pubkey(), 1_000_000_000)
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
        vec![sender.pubkey()],
    )
    .await;
    let token_account_keypair = Keypair::new();
    create_token_account(&mut context, &mint, &token_account_keypair, &sender)
        .await
        .unwrap();
    let input_compressed_account =
        test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
    decompress_test(
        &sender,
        &mut context,
        &mut test_indexer,
        input_compressed_account,
        amount,
        &merkle_tree_pubkey,
        &token_account_keypair.pubkey(),
        None,
    )
    .await;

    compress_test(
        &sender,
        &mut context,
        &mut test_indexer,
        amount,
        &mint,
        &merkle_tree_pubkey,
        &token_account_keypair.pubkey(),
        None,
    )
    .await;
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
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;
    let mut test_indexer = TestIndexer::<200, ProgramTestRpcConnection>::init_from_env(
        &payer,
        &env,
        true,
        false,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    let recipient_keypair = Keypair::new();
    airdrop_lamports(&mut rpc, &recipient_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    mint_tokens_helper(
        &mut rpc,
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
            Some(&[input_compressed_accounts[0]
                .merkle_context
                .merkle_tree_pubkey]),
            None,
            None,
            &mut rpc,
        )
        .await;
    let change_out_compressed_account_0 = TokenTransferOutputData {
        amount: input_compressed_account_token_data.amount - 1000,
        owner: recipient_keypair.pubkey(),
        lamports: None,
        merkle_tree: merkle_tree_pubkey,
    };
    let transfer_recipient_out_compressed_account_0 = TokenTransferOutputData {
        amount: 1000 + 1,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
        merkle_tree: merkle_tree_pubkey,
    };
    // invalid token data amount (+ 1)
    let res = perform_transfer_failing_test(
        &mut rpc,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await;
    assert_custom_error_or_program_error(res, ErrorCode::ComputeOutputSumFailed.into()).unwrap();
    let transfer_recipient_out_compressed_account_0 = TokenTransferOutputData {
        amount: 1000 - 1,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
        merkle_tree: merkle_tree_pubkey,
    };
    // invalid token data amount (- 1)
    let res = perform_transfer_failing_test(
        &mut rpc,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await;

    assert_custom_error_or_program_error(res, ErrorCode::SumCheckFailed.into()).unwrap();

    let zero_amount = TokenTransferOutputData {
        amount: 0,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
        merkle_tree: merkle_tree_pubkey,
    };
    // invalid token data zero out amount
    let res = perform_transfer_failing_test(
        &mut rpc,
        zero_amount,
        zero_amount,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await;
    assert_custom_error_or_program_error(res, ErrorCode::SumCheckFailed.into()).unwrap();

    let double_amount = TokenTransferOutputData {
        amount: input_compressed_account_token_data.amount,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
        merkle_tree: merkle_tree_pubkey,
    };
    // invalid double token data  amount
    let res = perform_transfer_failing_test(
        &mut rpc,
        double_amount,
        double_amount,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await;
    assert_custom_error_or_program_error(res, ErrorCode::ComputeOutputSumFailed.into()).unwrap();

    let invalid_lamports_amount = TokenTransferOutputData {
        amount: 1000,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: Some(1),
        merkle_tree: merkle_tree_pubkey,
    };

    // invalid_lamports_amount
    let res = perform_transfer_failing_test(
        &mut rpc,
        change_out_compressed_account_0,
        invalid_lamports_amount,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await;
    assert_custom_error_or_program_error(
        res,
        light_system_program::errors::CompressedPdaError::ComputeOutputSumFailed.into(),
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
        merkle_tree: merkle_tree_pubkey,
    };
    let transfer_recipient_out_compressed_account_0 = TokenTransferOutputData {
        amount: 1000,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
        merkle_tree: merkle_tree_pubkey,
    };

    let res = perform_transfer_failing_test(
        &mut rpc,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await;
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
    let res = perform_transfer_failing_test(
        &mut rpc,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await;
    assert_custom_error_or_program_error(res, VerifierError::ProofVerificationFailed.into())
        .unwrap();
    let input_compressed_accounts = vec![test_indexer.token_compressed_accounts[0]
        .compressed_account
        .clone()];
    let res = perform_transfer_failing_test(
        &mut rpc,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &payer,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await;
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
    let res = perform_transfer_failing_test(
        &mut rpc,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await;
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
    let res = perform_transfer_failing_test(
        &mut rpc,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof_rpc_result.proof.clone()),
        &proof_rpc_result.root_indices,
        &input_compressed_accounts,
    )
    .await;
    assert_custom_error_or_program_error(res, ErrorCode::DelegateUndefined.into()).unwrap();
    kill_gnark_server();
}

#[allow(clippy::too_many_arguments)]
async fn perform_transfer_failing_test<R: RpcConnection>(
    rpc: &mut R,
    change_token_transfer_output: TokenTransferOutputData,
    transfer_recipient_token_transfer_output: TokenTransferOutputData,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    payer: &Keypair,
    proof: &Option<CompressedProof>,
    root_indices: &[u16],
    input_compressed_accounts: &[CompressedAccountWithMerkleContext],
) -> Result<(), RpcError> {
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
        &[
            change_token_transfer_output,
            transfer_recipient_token_transfer_output,
        ],
        root_indices,
        proof,
        input_compressed_account_token_data.as_slice(),
        input_compressed_account_token_data[0].mint,
        None,
        false,
        None,
        None,
        None,
    )
    .unwrap();

    let latest_blockhash = rpc.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        [&payer].as_slice(),
        latest_blockhash,
    );
    rpc.process_transaction_with_metadata(transaction).await
}
