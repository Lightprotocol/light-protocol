#![cfg(feature = "test-sbf")]

use anchor_lang::AnchorDeserialize;
use anchor_lang::AnchorSerialize;
use light_compressed_token::mint_sdk::create_create_token_pool_instruction;
use light_compressed_token::token_data::AccountState;
use light_test_utils::rpc::errors::assert_rpc_error;
use light_test_utils::spl::approve_test;
use light_test_utils::spl::burn_test;
use light_test_utils::spl::freeze_test;
use light_test_utils::spl::mint_wrapped_sol;
use light_test_utils::spl::revoke_test;
use light_test_utils::spl::thaw_test;
use solana_sdk::{
    instruction::InstructionError,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::{Transaction, TransactionError},
};

use light_compressed_token::delegation::sdk::{
    create_approve_instruction, CreateApproveInstructionInputs,
};
use light_compressed_token::get_token_pool_pda;
use light_compressed_token::process_transfer::transfer_sdk::create_transfer_instruction;
use light_compressed_token::process_transfer::{get_cpi_authority_pda, TokenTransferOutputData};
use light_compressed_token::{token_data::TokenData, ErrorCode};
use light_prover_client::gnark::helpers::kill_prover;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::compressed_account::{CompressedAccountWithMerkleContext, MerkleContext},
};
use light_test_utils::indexer::{Indexer, TokenDataWithContext};
use light_test_utils::rpc::errors::RpcError;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::test_rpc::ProgramTestRpcConnection;
use light_test_utils::spl::{
    compress_test, compressed_transfer_test, create_mint_helper, create_token_account,
    decompress_test, mint_tokens_helper,
};
use light_test_utils::{
    airdrop_lamports, assert_custom_error_or_program_error, indexer::TestIndexer,
    test_env::setup_test_programs_with_accounts,
};
use light_verifier::VerifierError;

#[tokio::test]
async fn test_create_mint() {
    let (mut rpc, _) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    create_mint_helper(&mut rpc, &payer).await;
}

#[tokio::test]
async fn test_wrapped_sol() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    let mut test_indexer =
        TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, true, false).await;
    let native_mint = spl_token::native_mint::ID;
    let token_account_keypair = Keypair::new();
    create_token_account(&mut rpc, &native_mint, &token_account_keypair, &payer)
        .await
        .unwrap();
    let amount = 1_000_000_000u64;
    mint_wrapped_sol(&mut rpc, &payer, &token_account_keypair.pubkey(), amount)
        .await
        .unwrap();
    let fetched_token_account = rpc
        .get_account(token_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    use anchor_lang::solana_program::program_pack::Pack;
    let unpacked_token_account: spl_token::state::Account =
        spl_token::state::Account::unpack(&fetched_token_account.data).unwrap();
    assert_eq!(unpacked_token_account.amount, amount);
    assert_eq!(unpacked_token_account.owner, payer.pubkey());
    assert_eq!(unpacked_token_account.mint, native_mint);
    assert!(unpacked_token_account.is_native.is_some());
    let instruction = create_create_token_pool_instruction(&payer.pubkey(), &native_mint);
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    compress_test(
        &payer,
        &mut rpc,
        &mut test_indexer,
        amount,
        &native_mint,
        &env.merkle_tree_pubkey,
        &token_account_keypair.pubkey(),
        None,
    )
    .await;
    let input_compressed_accounts =
        test_indexer.get_compressed_token_accounts_by_owner(&payer.pubkey());
    decompress_test(
        &payer,
        &mut rpc,
        &mut test_indexer,
        input_compressed_accounts,
        amount,
        &env.merkle_tree_pubkey,
        &token_account_keypair.pubkey(),
        None,
    )
    .await;
    kill_prover();
}

async fn test_mint_to<const MINTS: usize, const ITER: usize>() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let mut test_indexer =
        TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, false, false).await;

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
    kill_prover();
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
    let mut test_indexer =
        TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, true, false).await;
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
        None,
    )
    .await;
}

#[tokio::test]
async fn test_decompression() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let mut test_indexer =
        TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, true, false).await;
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
    kill_prover();
}

/// Test delegation:
/// 1. Delegate tokens with approve
/// 2. Delegate transfers a part of the delegated tokens
/// 3. Delegate transfers all the remaining delegated tokens
async fn test_delegation(
    mint_amount: u64,
    delegated_amount: u64,
    output_amounts_1: Vec<u64>,
    output_amounts_2: Vec<u64>,
) {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let mut test_indexer =
        TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, true, false).await;
    let sender = Keypair::new();
    airdrop_lamports(&mut rpc, &sender.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let delegate = Keypair::new();
    airdrop_lamports(&mut rpc, &delegate.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let mint = create_mint_helper(&mut rpc, &payer).await;
    mint_tokens_helper(
        &mut rpc,
        &mut test_indexer,
        &merkle_tree_pubkey,
        &payer,
        &mint,
        vec![mint_amount],
        vec![sender.pubkey()],
    )
    .await;
    // 1. Delegate tokens
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let delegated_compressed_account_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;
        approve_test(
            &sender,
            &mut rpc,
            &mut test_indexer,
            input_compressed_accounts,
            delegated_amount,
            &delegate.pubkey(),
            &delegated_compressed_account_merkle_tree,
            &delegated_compressed_account_merkle_tree,
            None,
        )
        .await;
    }

    let recipient = Pubkey::new_unique();
    // 2. Transfer partial delegated amount
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let input_compressed_accounts = input_compressed_accounts
            .iter()
            .filter(|x| x.token_data.delegate.is_some())
            .cloned()
            .collect::<Vec<TokenDataWithContext>>();
        compressed_transfer_test(
            &delegate,
            &mut rpc,
            &mut test_indexer,
            &mint,
            &sender,
            &[recipient, sender.pubkey()],
            &output_amounts_1,
            input_compressed_accounts.as_slice(),
            &[env.merkle_tree_pubkey; 2],
            Some(1),
            None,
        )
        .await;
    }
    // 3. Transfer full delegated amount
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let input_compressed_accounts = input_compressed_accounts
            .iter()
            .filter(|x| x.token_data.delegate.is_some())
            .cloned()
            .collect::<Vec<TokenDataWithContext>>();
        compressed_transfer_test(
            &delegate,
            &mut rpc,
            &mut test_indexer,
            &mint,
            &sender,
            &[recipient],
            &output_amounts_2,
            input_compressed_accounts.as_slice(),
            &[env.merkle_tree_pubkey; 1],
            None,
            None,
        )
        .await;
    }
    kill_prover();
}

#[tokio::test]
async fn test_delegation_0() {
    test_delegation(0, 0, vec![0, 0], vec![0]).await
}

#[tokio::test]
async fn test_delegation_10000() {
    test_delegation(10000, 1000, vec![900, 100], vec![100]).await
}

#[tokio::test]
async fn test_delegation_max() {
    test_delegation(u64::MAX, u64::MAX, vec![u64::MAX - 100, 100], vec![100]).await
}

/// Failing tests:
/// 1. Invalid delegated compressed account Merkle tree.
/// 2. Invalid change compressed account Merkle tree.
/// 3. Invalid proof.
/// 4. Invalid mint.
#[tokio::test]
async fn test_approve_failing() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let mut test_indexer =
        TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, true, false).await;
    let sender = Keypair::new();
    airdrop_lamports(&mut rpc, &sender.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let delegate = Keypair::new();
    airdrop_lamports(&mut rpc, &delegate.pubkey(), 1_000_000_000)
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
        vec![sender.pubkey()],
    )
    .await;

    let input_compressed_accounts =
        test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
    let delegated_amount = 1000u64;
    let delegated_compressed_account_merkle_tree = input_compressed_accounts[0]
        .compressed_account
        .merkle_context
        .merkle_tree_pubkey;

    let input_compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.hash().unwrap())
        .collect::<Vec<_>>();
    let input_merkle_tree_pubkeys = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.merkle_context.merkle_tree_pubkey)
        .collect::<Vec<_>>();
    let proof_rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&input_compressed_account_hashes),
            Some(&input_merkle_tree_pubkeys),
            None,
            None,
            &mut rpc,
        )
        .await;
    let mint = input_compressed_accounts[0].token_data.mint;

    // 1. Invalid delegated compressed account Merkle tree.
    {
        let invalid_delegated_merkle_tree = Keypair::new();

        let inputs = CreateApproveInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: sender.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data)
                .collect(),
            mint,
            delegated_amount,
            delegated_compressed_account_merkle_tree: invalid_delegated_merkle_tree.pubkey(),
            change_compressed_account_merkle_tree: delegated_compressed_account_merkle_tree,
            delegate: delegate.pubkey(),
            root_indices: proof_rpc_result.root_indices.clone(),
            proof: proof_rpc_result.proof.clone(),
        };
        let instruction = create_approve_instruction(inputs).unwrap();
        let context_payer = rpc.get_payer().insecure_clone();
        let result = rpc
            .create_and_send_transaction(
                &[instruction],
                &sender.pubkey(),
                &[&context_payer, &sender],
            )
            .await;
        // Anchor panics when trying to read the MT account. Unfortunately
        // there is no specific error code to assert.
        assert!(matches!(
            result,
            Err(RpcError::TransactionError(
                TransactionError::InstructionError(0, InstructionError::ProgramFailedToComplete)
            ))
        ));
    }
    // 2. Invalid change compressed account Merkle tree.
    {
        let invalid_change_merkle_tree = Keypair::new();

        let inputs = CreateApproveInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: sender.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data)
                .collect(),
            mint,
            delegated_amount,
            delegated_compressed_account_merkle_tree,
            change_compressed_account_merkle_tree: invalid_change_merkle_tree.pubkey(),
            delegate: delegate.pubkey(),
            root_indices: proof_rpc_result.root_indices.clone(),
            proof: proof_rpc_result.proof.clone(),
        };
        let instruction = create_approve_instruction(inputs).unwrap();
        let context_payer = rpc.get_payer().insecure_clone();
        let result = rpc
            .create_and_send_transaction(
                &[instruction],
                &sender.pubkey(),
                &[&context_payer, &sender],
            )
            .await;
        // Anchor panics when trying to read the MT account. Unfortunately
        // there is no specific error code to assert.
        assert!(matches!(
            result,
            Err(RpcError::TransactionError(
                TransactionError::InstructionError(0, InstructionError::ProgramFailedToComplete)
            ))
        ));
    }
    // 3. Invalid proof.
    {
        let invalid_proof = CompressedProof {
            a: [0; 32],
            b: [0; 64],
            c: [0; 32],
        };

        let inputs = CreateApproveInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: sender.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data)
                .collect(),
            mint,
            delegated_amount,
            delegated_compressed_account_merkle_tree,
            change_compressed_account_merkle_tree: delegated_compressed_account_merkle_tree,
            delegate: delegate.pubkey(),
            root_indices: proof_rpc_result.root_indices.clone(),
            proof: invalid_proof,
        };
        let instruction = create_approve_instruction(inputs).unwrap();
        let context_payer = rpc.get_payer().insecure_clone();
        let result = rpc
            .create_and_send_transaction(
                &[instruction],
                &sender.pubkey(),
                &[&context_payer, &sender],
            )
            .await;
        assert_rpc_error(result, 0, VerifierError::ProofVerificationFailed.into()).unwrap();
    }
    // 4. Invalid mint.
    {
        let invalid_mint = Keypair::new();

        let inputs = CreateApproveInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: sender.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data)
                .collect(),
            mint: invalid_mint.pubkey(),
            delegated_amount,
            delegated_compressed_account_merkle_tree,
            change_compressed_account_merkle_tree: delegated_compressed_account_merkle_tree,
            delegate: delegate.pubkey(),
            root_indices: proof_rpc_result.root_indices.clone(),
            proof: proof_rpc_result.proof.clone(),
        };
        let instruction = create_approve_instruction(inputs).unwrap();
        let context_payer = rpc.get_payer().insecure_clone();
        let result = rpc
            .create_and_send_transaction(
                &[instruction],
                &sender.pubkey(),
                &[&context_payer, &sender],
            )
            .await;
        assert_rpc_error(result, 0, VerifierError::ProofVerificationFailed.into()).unwrap();
    }
}

/// Test revoke:
/// 1. Delegate tokens with approve
/// 2. Revoke
#[tokio::test]
async fn test_revoke() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let mut test_indexer =
        TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, true, false).await;
    let sender = Keypair::new();
    airdrop_lamports(&mut rpc, &sender.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let delegate = Keypair::new();
    airdrop_lamports(&mut rpc, &delegate.pubkey(), 1_000_000_000)
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
        vec![sender.pubkey()],
    )
    .await;
    // 1. Delegate tokens
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let delegated_amount = 1000u64;
        let delegated_compressed_account_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;
        approve_test(
            &sender,
            &mut rpc,
            &mut test_indexer,
            input_compressed_accounts,
            delegated_amount,
            &delegate.pubkey(),
            &delegated_compressed_account_merkle_tree,
            &delegated_compressed_account_merkle_tree,
            None,
        )
        .await;
    }
    // 2. Revoke
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let input_compressed_accounts = input_compressed_accounts
            .iter()
            .filter(|x| x.token_data.delegate.is_some())
            .cloned()
            .collect::<Vec<TokenDataWithContext>>();
        let delegated_compressed_account_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;
        revoke_test(
            &sender,
            &mut rpc,
            &mut test_indexer,
            input_compressed_accounts,
            &delegated_compressed_account_merkle_tree,
            None,
        )
        .await;
    }
}

/// Test Burn:
/// 1. Burn tokens
/// 1. Delegate tokens with approve
/// 2. Burn delegated tokens
#[tokio::test]
async fn test_burn() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let mut test_indexer =
        TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, true, false).await;
    let sender = Keypair::new();
    airdrop_lamports(&mut rpc, &sender.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let delegate = Keypair::new();
    airdrop_lamports(&mut rpc, &delegate.pubkey(), 1_000_000_000)
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
        vec![sender.pubkey()],
    )
    .await;
    // 1. Burn tokens
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let burn_amount = 1000u64;
        let change_account_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;
        burn_test(
            &sender,
            &mut rpc,
            &mut test_indexer,
            input_compressed_accounts,
            &change_account_merkle_tree,
            burn_amount,
            false,
            None,
        )
        .await;
    }
    // 2. Delegate tokens
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let delegated_amount = 1000u64;
        let delegated_compressed_account_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;
        approve_test(
            &sender,
            &mut rpc,
            &mut test_indexer,
            input_compressed_accounts,
            delegated_amount,
            &delegate.pubkey(),
            &delegated_compressed_account_merkle_tree,
            &delegated_compressed_account_merkle_tree,
            None,
        )
        .await;
    }
    // 3. Burn delegated tokens
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let input_compressed_accounts = input_compressed_accounts
            .iter()
            .filter(|x| x.token_data.delegate.is_some())
            .cloned()
            .collect::<Vec<TokenDataWithContext>>();
        let burn_amount = 100;
        let change_account_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;
        burn_test(
            &delegate,
            &mut rpc,
            &mut test_indexer,
            input_compressed_accounts,
            &change_account_merkle_tree,
            burn_amount,
            true,
            None,
        )
        .await;
    }
    // 3. Burn all delegated tokens
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let input_compressed_accounts = input_compressed_accounts
            .iter()
            .filter(|x| x.token_data.delegate.is_some())
            .cloned()
            .collect::<Vec<TokenDataWithContext>>();
        let burn_amount = input_compressed_accounts
            .iter()
            .map(|x| x.token_data.amount)
            .sum::<u64>();
        let change_account_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;
        burn_test(
            &delegate,
            &mut rpc,
            &mut test_indexer,
            input_compressed_accounts,
            &change_account_merkle_tree,
            burn_amount,
            true,
            None,
        )
        .await;
    }
}

/// Test freeze and thaw:
/// 1. Freeze tokens
/// 2. Thaw tokens
/// 3. Delegate tokens
/// 4. Freeze delegated tokens
/// 5. Thaw delegated tokens
#[tokio::test]
async fn test_freeze_and_thaw() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let mut test_indexer =
        TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, true, false).await;
    let sender = Keypair::new();
    airdrop_lamports(&mut rpc, &sender.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let delegate = Keypair::new();
    airdrop_lamports(&mut rpc, &delegate.pubkey(), 1_000_000_000)
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
        vec![sender.pubkey()],
    )
    .await;
    // 1. Freeze tokens
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let output_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;

        freeze_test(
            &payer,
            &mut rpc,
            &mut test_indexer,
            input_compressed_accounts,
            &output_merkle_tree,
            None,
        )
        .await;
    }
    // 2. Thaw tokens
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let input_compressed_accounts = input_compressed_accounts
            .iter()
            .filter(|x| x.token_data.state == AccountState::Frozen)
            .cloned()
            .collect::<Vec<TokenDataWithContext>>();
        let output_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;
        thaw_test(
            &payer,
            &mut rpc,
            &mut test_indexer,
            input_compressed_accounts,
            &output_merkle_tree,
            None,
        )
        .await;
    }
    // 3. Delegate tokens
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let delegated_amount = 1000u64;
        let delegated_compressed_account_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;
        approve_test(
            &sender,
            &mut rpc,
            &mut test_indexer,
            input_compressed_accounts,
            delegated_amount,
            &delegate.pubkey(),
            &delegated_compressed_account_merkle_tree,
            &delegated_compressed_account_merkle_tree,
            None,
        )
        .await;
    }
    // 4. Freeze delegated tokens
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let output_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;

        freeze_test(
            &payer,
            &mut rpc,
            &mut test_indexer,
            input_compressed_accounts,
            &output_merkle_tree,
            None,
        )
        .await;
    }
    // 5. Thaw delegated tokens
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let input_compressed_accounts = input_compressed_accounts
            .iter()
            .filter(|x| x.token_data.state == AccountState::Frozen)
            .cloned()
            .collect::<Vec<TokenDataWithContext>>();
        let output_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;

        thaw_test(
            &payer,
            &mut rpc,
            &mut test_indexer,
            input_compressed_accounts,
            &output_merkle_tree,
            None,
        )
        .await;
    }
}

/// Failing tests:
/// 1. Invalid decompress account
/// 2. Invalid token pool pda
/// 3. Invalid decompression amount -1
/// 4. Invalid decompression amount +1
/// 5. Invalid decompression amount 0
/// 6. Invalid compression amount -1
/// 7. Invalid compression amount +1
/// 8. Invalid compression amount 0
#[tokio::test]
async fn test_failing_decompression() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let mut test_indexer =
        TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, true, false).await;
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
    let decompress_amount = amount - 1000;
    // Test 1: invalid decompress account
    {
        let invalid_token_account = mint;
        failing_compress_decompress(
            &sender,
            &mut context,
            &mut test_indexer,
            input_compressed_account.clone(),
            decompress_amount, // need to be consistent with compression amount
            &merkle_tree_pubkey,
            decompress_amount,
            false,
            &invalid_token_account,
            Some(get_token_pool_pda(&mint)),
            &mint,
            0, //ProgramError::InvalidAccountData.into(), error code 17179869184 does not fit u32
        )
        .await
        .unwrap_err();
    }
    // Test 2: invalid token pool pda
    {
        failing_compress_decompress(
            &sender,
            &mut context,
            &mut test_indexer,
            input_compressed_account.clone(),
            decompress_amount, // need to be consistent with compression amount
            &merkle_tree_pubkey,
            decompress_amount - 1,
            false,
            &token_account_keypair.pubkey(),
            Some(get_token_pool_pda(&Pubkey::new_unique())),
            &mint,
            anchor_lang::error::ErrorCode::AccountNotInitialized.into(),
        )
        .await
        .unwrap();
    }
    // Test 3: invalid compression amount -1
    {
        failing_compress_decompress(
            &sender,
            &mut context,
            &mut test_indexer,
            input_compressed_account.clone(),
            decompress_amount, // need to be consistent with compression amount
            &merkle_tree_pubkey,
            decompress_amount - 1,
            false,
            &token_account_keypair.pubkey(),
            Some(get_token_pool_pda(&mint)),
            &mint,
            ErrorCode::SumCheckFailed.into(),
        )
        .await
        .unwrap();
    }
    // Test 4: invalid compression amount + 1
    {
        failing_compress_decompress(
            &sender,
            &mut context,
            &mut test_indexer,
            input_compressed_account.clone(),
            decompress_amount, // need to be consistent with compression amount
            &merkle_tree_pubkey,
            decompress_amount + 1,
            false,
            &token_account_keypair.pubkey(),
            Some(get_token_pool_pda(&mint)),
            &mint,
            ErrorCode::ComputeOutputSumFailed.into(),
        )
        .await
        .unwrap();
    }
    // Test 5: invalid compression amount 0
    {
        failing_compress_decompress(
            &sender,
            &mut context,
            &mut test_indexer,
            input_compressed_account.clone(),
            decompress_amount, // need to be consistent with compression amount
            &merkle_tree_pubkey,
            0,
            false,
            &token_account_keypair.pubkey(),
            Some(get_token_pool_pda(&mint)),
            &mint,
            ErrorCode::SumCheckFailed.into(),
        )
        .await
        .unwrap();
    }

    // functional so that we have tokens to compress
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
    let compress_amount = decompress_amount - 100;
    // Test 6: invalid compression amount -1
    {
        failing_compress_decompress(
            &sender,
            &mut context,
            &mut test_indexer,
            Vec::new(),
            compress_amount, // need to be consistent with compression amount
            &merkle_tree_pubkey,
            compress_amount - 1,
            true,
            &token_account_keypair.pubkey(),
            Some(get_token_pool_pda(&mint)),
            &mint,
            ErrorCode::ComputeOutputSumFailed.into(),
        )
        .await
        .unwrap();
    }
    // Test 7: invalid compression amount +1
    {
        failing_compress_decompress(
            &sender,
            &mut context,
            &mut test_indexer,
            Vec::new(),
            compress_amount, // need to be consistent with compression amount
            &merkle_tree_pubkey,
            compress_amount + 1,
            true,
            &token_account_keypair.pubkey(),
            Some(get_token_pool_pda(&mint)),
            &mint,
            ErrorCode::SumCheckFailed.into(),
        )
        .await
        .unwrap();
    }
    // Test 7: invalid compression amount 0
    {
        failing_compress_decompress(
            &sender,
            &mut context,
            &mut test_indexer,
            Vec::new(),
            compress_amount, // need to be consistent with compression amount
            &merkle_tree_pubkey,
            0,
            true,
            &token_account_keypair.pubkey(),
            Some(get_token_pool_pda(&mint)),
            &mint,
            ErrorCode::ComputeOutputSumFailed.into(),
        )
        .await
        .unwrap();
    }
    // functional
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
    kill_prover();
}

#[allow(clippy::too_many_arguments)]
pub async fn failing_compress_decompress<R: RpcConnection>(
    payer: &Keypair,
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    input_compressed_accounts: Vec<TokenDataWithContext>,
    amount: u64,
    output_merkle_tree_pubkey: &Pubkey,
    compression_amount: u64,
    is_compress: bool,
    compress_or_decompress_token_account: &Pubkey,
    token_pool_pda: Option<Pubkey>,
    mint: &Pubkey,
    error_code: u32,
) -> Result<(), RpcError> {
    let max_amount: u64 = input_compressed_accounts
        .iter()
        .map(|x| x.token_data.amount)
        .sum();
    let change_out_compressed_account = if !is_compress {
        TokenTransferOutputData {
            amount: max_amount - amount,
            owner: payer.pubkey(),
            lamports: None,
            merkle_tree: *output_merkle_tree_pubkey,
        }
    } else {
        TokenTransferOutputData {
            amount: max_amount + amount,
            owner: payer.pubkey(),
            lamports: None,
            merkle_tree: *output_merkle_tree_pubkey,
        }
    };

    let input_compressed_account_hashes = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.hash().unwrap())
        .collect::<Vec<_>>();
    let input_merkle_tree_pubkeys = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.merkle_context.merkle_tree_pubkey)
        .collect::<Vec<_>>();
    let (root_indices, proof) = if !input_compressed_account_hashes.is_empty() {
        let proof_rpc_result = test_indexer
            .create_proof_for_compressed_accounts(
                Some(&input_compressed_account_hashes),
                Some(&input_merkle_tree_pubkeys),
                None,
                None,
                rpc,
            )
            .await;
        (proof_rpc_result.root_indices, Some(proof_rpc_result.proof))
    } else {
        (Vec::new(), None)
    };
    let instruction = create_transfer_instruction(
        &rpc.get_payer().pubkey(),
        &payer.pubkey(),
        &input_compressed_accounts
            .iter()
            .map(|x| x.compressed_account.merkle_context)
            .collect::<Vec<_>>(),
        &[change_out_compressed_account],
        &root_indices,
        &proof,
        input_compressed_accounts
            .iter()
            .map(|x| x.token_data)
            .collect::<Vec<_>>()
            .as_slice(),
        *mint,
        None,
        is_compress,
        Some(compression_amount),
        token_pool_pda,
        Some(*compress_or_decompress_token_account),
        true,
        None,
    )
    .unwrap();
    let instructions = if !is_compress {
        vec![instruction]
    } else {
        vec![
            spl_token::instruction::approve(
                &anchor_spl::token::ID,
                compress_or_decompress_token_account,
                &get_cpi_authority_pda().0,
                &payer.pubkey(),
                &[&payer.pubkey()],
                amount,
            )
            .unwrap(),
            instruction,
        ]
    };

    let context_payer = rpc.get_payer().insecure_clone();
    let result = rpc
        .create_and_send_transaction(
            &instructions,
            &context_payer.pubkey(),
            &[&context_payer, payer],
        )
        .await;
    assert_rpc_error(
        result,
        instructions.len().saturating_sub(1) as u8,
        error_code,
    )
}

/// Failing tests:
/// Out utxo tests:
/// 1. Invalid token data amount (+ 1)
/// 2. Invalid token data amount (- 1)
/// 3. Invalid token data zero out amount
/// 4. Invalid double token data amount
/// In utxo tests:
/// 5. Invalid input token data amount (0)
/// 6. Invalid delegate
/// 7. Invalid owner
/// 8. Invalid is native (deactivated, revisit)
/// 9. DelegateUndefined
/// Invalid account state (Frozen is only hashed if frozen thus failing test is not possible)
/// 10. invalid root indices (ProofVerificationFailed)
/// 11. invalid mint (ProofVerificationFailed)
/// 12. invalid Merkle tree pubkey (ProofVerificationFailed)
#[tokio::test]
async fn test_invalid_inputs() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;
    let mut test_indexer =
        TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, true, false).await;
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
    let payer = recipient_keypair.insecure_clone();
    let transfer_recipient_keypair = Keypair::new();
    let input_compressed_account_token_data = test_indexer.token_compressed_accounts[0].token_data;
    let input_compressed_accounts = vec![test_indexer.token_compressed_accounts[0]
        .compressed_account
        .clone()];
    let proof_rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[input_compressed_accounts[0].hash().unwrap()]),
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
        amount: 1000,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
        merkle_tree: merkle_tree_pubkey,
    };
    {
        let mut transfer_recipient_out_compressed_account_0 =
            transfer_recipient_out_compressed_account_0;
        transfer_recipient_out_compressed_account_0.amount += 1;
        // Test 1: invalid token data amount (+ 1)
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
            false,
        )
        .await;
        assert_custom_error_or_program_error(res, ErrorCode::ComputeOutputSumFailed.into())
            .unwrap();
    }
    // Test 2: invalid token data amount (- 1)
    {
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
            false,
        )
        .await;

        assert_custom_error_or_program_error(res, ErrorCode::SumCheckFailed.into()).unwrap();
    }
    // Test 3: invalid token data amount (0)
    {
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
            false,
        )
        .await;
        assert_custom_error_or_program_error(res, ErrorCode::SumCheckFailed.into()).unwrap();
    }
    // Test 4: invalid token data amount (2x)
    {
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
            false,
        )
        .await;
        assert_custom_error_or_program_error(res, ErrorCode::ComputeOutputSumFailed.into())
            .unwrap();
    }
    // Test 4: invalid token data amount (2x)
    {
        let double_amount = TokenTransferOutputData {
            amount: input_compressed_account_token_data.amount,
            owner: transfer_recipient_keypair.pubkey(),
            lamports: None,
            merkle_tree: merkle_tree_pubkey,
        };
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
            false,
        )
        .await;
        assert_custom_error_or_program_error(res, ErrorCode::ComputeOutputSumFailed.into())
            .unwrap();
    }
    // Test 5: invalid input token data amount (0)
    {
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
            false,
        )
        .await;
        assert_custom_error_or_program_error(res, ErrorCode::ComputeOutputSumFailed.into())
            .unwrap();
    }
    // Test 6: invalid delegate
    {
        let mut input_compressed_account_token_data =
            test_indexer.token_compressed_accounts[0].token_data;
        input_compressed_account_token_data.delegate = Some(Pubkey::new_unique());
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
            false,
        )
        .await;
        assert_custom_error_or_program_error(res, VerifierError::ProofVerificationFailed.into())
            .unwrap();
    }
    // Test 7: invalid owner
    {
        let invalid_payer = rpc.get_payer().insecure_clone();
        let res = perform_transfer_failing_test(
            &mut rpc,
            change_out_compressed_account_0,
            transfer_recipient_out_compressed_account_0,
            &merkle_tree_pubkey,
            &nullifier_queue_pubkey,
            &invalid_payer,
            &Some(proof_rpc_result.proof.clone()),
            &proof_rpc_result.root_indices,
            &input_compressed_accounts,
            false,
        )
        .await;
        assert_custom_error_or_program_error(res, VerifierError::ProofVerificationFailed.into())
            .unwrap();
    }
    // Test 10: invalid root indices
    {
        let mut root_indices = proof_rpc_result.root_indices.clone();
        root_indices[0] += 1;
        let res = perform_transfer_failing_test(
            &mut rpc,
            change_out_compressed_account_0,
            transfer_recipient_out_compressed_account_0,
            &merkle_tree_pubkey,
            &nullifier_queue_pubkey,
            &payer,
            &Some(proof_rpc_result.proof.clone()),
            &root_indices,
            &input_compressed_accounts,
            false,
        )
        .await;
        assert_custom_error_or_program_error(res, VerifierError::ProofVerificationFailed.into())
            .unwrap();
    }
    // Test 11: invalid mint
    {
        let res = perform_transfer_failing_test(
            &mut rpc,
            change_out_compressed_account_0,
            transfer_recipient_out_compressed_account_0,
            &nullifier_queue_pubkey,
            &nullifier_queue_pubkey,
            &payer,
            &Some(proof_rpc_result.proof.clone()),
            &proof_rpc_result.root_indices,
            &input_compressed_accounts,
            true,
        )
        .await;
        assert_custom_error_or_program_error(
            res,
            anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into(),
        )
        .unwrap();
    }
    // Test 12: invalid Merkle tree pubkey
    {
        let res = perform_transfer_failing_test(
            &mut rpc,
            change_out_compressed_account_0,
            transfer_recipient_out_compressed_account_0,
            &nullifier_queue_pubkey,
            &nullifier_queue_pubkey,
            &payer,
            &Some(proof_rpc_result.proof.clone()),
            &proof_rpc_result.root_indices,
            &input_compressed_accounts,
            false,
        )
        .await;

        assert_custom_error_or_program_error(
            res,
            anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into(),
        )
        .unwrap();
    }
    kill_prover();
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
    invalid_mint: bool,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let input_compressed_account_token_data: Vec<TokenData> = input_compressed_accounts
        .iter()
        .map(|x| {
            TokenData::deserialize(&mut &x.compressed_account.data.as_ref().unwrap().data[..])
                .unwrap()
        })
        .collect();
    let mint = if invalid_mint {
        Pubkey::new_unique()
    } else {
        input_compressed_account_token_data[0].mint
    };
    let instruction = create_transfer_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &input_compressed_accounts
            .iter()
            .map(|x| MerkleContext {
                merkle_tree_pubkey: *merkle_tree_pubkey,
                nullifier_queue_pubkey: *nullifier_queue_pubkey,
                leaf_index: x.merkle_context.leaf_index,
                queue_index: None,
            })
            .collect::<Vec<MerkleContext>>(),
        &[
            change_token_transfer_output,
            transfer_recipient_token_transfer_output,
        ],
        root_indices,
        proof,
        input_compressed_account_token_data.as_slice(),
        mint,
        None,
        false,
        None,
        None,
        None,
        true,
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
    rpc.process_transaction(transaction).await
}
