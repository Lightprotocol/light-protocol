#![cfg(feature = "test-sbf")]

use account_compression::{
    utils::constants::{
        STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT, STATE_MERKLE_TREE_ROOTS,
    },
    StateMerkleTreeAccount,
};
use anchor_lang::AnchorSerialize;
use circuitlib_rs::{
    gnark::{
        constants::{INCLUSION_PATH, SERVER_ADDRESS},
        helpers::{health_check, spawn_gnark_server},
        inclusion_json_formatter::InclusionJsonStruct,
        proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    },
    inclusion::merkle_inclusion_proof_inputs::{InclusionMerkleProofInputs, InclusionProofInputs},
};
use light_hasher::Poseidon;
use light_test_utils::{
    airdrop_lamports, create_account_instruction, create_and_send_transaction,
    test_env::setup_test_programs_with_accounts, AccountZeroCopy,
};
use num_bigint::BigInt;
use num_traits::ops::bytes::FromBytes;
use psp_compressed_pda::{
    compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext},
    event::PublicTransactionEvent,
    utils::CompressedProof,
};
use psp_compressed_token::{
    get_token_authority_pda, get_token_pool_pda,
    mint_sdk::{create_initialize_mint_instruction, create_mint_to_instruction},
    transfer_sdk, AccountState, ErrorCode, TokenData, TokenTransferOutputData,
};
use reqwest::Client;
use solana_program_test::{
    BanksClientError, BanksTransactionResultWithMetadata, ProgramTestContext,
};
use solana_sdk::{
    instruction::{Instruction, InstructionError},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use spl_token::instruction::initialize_mint;

pub fn create_initialize_mint_instructions(
    payer: &Pubkey,
    authority: &Pubkey,
    rent: u64,
    decimals: u8,
    mint_keypair: &Keypair,
) -> ([Instruction; 4], Pubkey) {
    let account_create_ix = create_account_instruction(
        &payer,
        anchor_spl::token::Mint::LEN,
        rent,
        &anchor_spl::token::ID,
        Some(mint_keypair),
    );

    let mint_pubkey = mint_keypair.pubkey();
    let mint_authority = get_token_authority_pda(authority, &mint_pubkey);
    let create_mint_instruction = initialize_mint(
        &anchor_spl::token::ID,
        &mint_keypair.pubkey(),
        &mint_authority,
        None,
        decimals,
    )
    .unwrap();
    let transfer_ix =
        anchor_lang::solana_program::system_instruction::transfer(&payer, &mint_pubkey, rent);

    let instruction = create_initialize_mint_instruction(&payer, &authority, &mint_pubkey);
    let pool_pubkey = get_token_pool_pda(&mint_pubkey);
    (
        [
            account_create_ix,
            create_mint_instruction,
            transfer_ix,
            instruction,
        ],
        pool_pubkey,
    )
}

use anchor_lang::{solana_program::program_pack::Pack, AnchorDeserialize};
async fn assert_create_mint(
    context: &mut ProgramTestContext,
    authority: &Pubkey,
    mint: &Pubkey,
    pool: &Pubkey,
) {
    let mint_account: spl_token::state::Mint = spl_token::state::Mint::unpack(
        &context
            .banks_client
            .get_account(*mint)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    let mint_authority = get_token_authority_pda(authority, mint);
    assert_eq!(mint_account.supply, 0);
    assert_eq!(mint_account.decimals, 2);
    assert_eq!(mint_account.mint_authority.unwrap(), mint_authority);
    assert_eq!(mint_account.freeze_authority, None.into());
    assert_eq!(mint_account.is_initialized, true);
    let mint_account: spl_token::state::Account = spl_token::state::Account::unpack(
        &context
            .banks_client
            .get_account(*pool)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();

    assert_eq!(mint_account.amount, 0);
    assert_eq!(mint_account.delegate, None.into());
    assert_eq!(mint_account.mint, *mint);
    assert_eq!(mint_account.owner, mint_authority);
}

#[tokio::test]
async fn test_create_mint() {
    let env: light_test_utils::test_env::EnvWithAccounts =
        setup_test_programs_with_accounts().await;
    let mut context = env.context;
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

#[tokio::test]
async fn test_mint_to() {
    let env: light_test_utils::test_env::EnvWithAccounts =
        setup_test_programs_with_accounts().await;
    let mut context = env.context;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let indexed_array_pubkey = env.indexed_array_pubkey;
    let mock_indexer = MockIndexer::new(
        merkle_tree_pubkey,
        indexed_array_pubkey,
        payer.insecure_clone(),
        None,
    );
    let recipient_keypair = Keypair::new();
    let mint = create_mint_helper(&mut context, &payer).await;
    let amount = 10000u64;
    let instruction = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &mint,
        &merkle_tree_pubkey,
        vec![amount; 1],
        vec![recipient_keypair.pubkey(); 1],
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.last_blockhash,
    );
    let old_merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
        &mut context,
        env.merkle_tree_pubkey,
    )
    .await;
    let old_merkle_tree = old_merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await;

    let mut mock_indexer = mock_indexer.await;
    mock_indexer.add_compressed_accounts_with_token_data(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );
    assert_mint_to(
        &mut context,
        &mock_indexer,
        &recipient_keypair,
        mint,
        amount,
        &old_merkle_tree,
    )
    .await;
}

#[tokio::test]
async fn test_transfer() {
    let env: light_test_utils::test_env::EnvWithAccounts =
        setup_test_programs_with_accounts().await;
    let mut context = env.context;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let indexed_array_pubkey = env.indexed_array_pubkey;
    let mock_indexer = MockIndexer::new(
        merkle_tree_pubkey,
        indexed_array_pubkey,
        payer.insecure_clone(),
        Some(0),
    );
    let recipient_keypair = Keypair::new();
    let mint = create_mint_helper(&mut context, &payer).await;
    let amount = 10000u64;
    let instruction = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &mint,
        &merkle_tree_pubkey,
        vec![amount],
        vec![recipient_keypair.pubkey()],
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.last_blockhash,
    );
    let old_merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
        &mut context,
        env.merkle_tree_pubkey,
    )
    .await;
    let old_merkle_tree = old_merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await;
    let mut mock_indexer = mock_indexer.await;
    mock_indexer.add_compressed_accounts_with_token_data(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );
    assert_mint_to(
        &mut context,
        &mock_indexer,
        &recipient_keypair,
        mint,
        amount,
        &old_merkle_tree,
    )
    .await;
    let transfer_recipient_keypair = Keypair::new();
    let input_compressed_account_token_data =
        mock_indexer.token_compressed_accounts[0].token_data.clone();
    let input_compressed_accounts = vec![mock_indexer.compressed_accounts
        [mock_indexer.token_compressed_accounts[0].index]
        .clone()];
    let input_compressed_account_indices: Vec<u32> = input_compressed_accounts
        .iter()
        .map(|x| x.leaf_index)
        .collect();

    let change_out_compressed_account = TokenTransferOutputData {
        amount: input_compressed_account_token_data.amount - 1000,
        owner: recipient_keypair.pubkey(),
        lamports: None,
    };
    let transfer_recipient_out_compressed_account = TokenTransferOutputData {
        amount: 1000,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
    };
    let (root_indices, proof) = mock_indexer
        .create_proof_for_compressed_accounts(
            &[input_compressed_accounts[0]
                .compressed_account
                .hash(
                    &merkle_tree_pubkey,
                    &input_compressed_accounts[0].leaf_index,
                )
                .unwrap()],
            &mut context,
        )
        .await;
    let input_compressed_accounts: Vec<CompressedAccount> = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.clone())
        .collect();
    let instruction = transfer_sdk::create_transfer_instruction(
        &payer_pubkey,
        &recipient_keypair.pubkey(),
        &vec![merkle_tree_pubkey], // input compressed account Merkle trees
        &vec![indexed_array_pubkey], // input compressed account indexed arrays
        &vec![merkle_tree_pubkey, merkle_tree_pubkey], // output compressed account Merkle trees
        input_compressed_accounts.as_slice(), // input compressed_accounts
        &vec![
            change_out_compressed_account,
            transfer_recipient_out_compressed_account,
        ],
        &root_indices,
        &input_compressed_account_indices,
        &proof,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        [&payer, &recipient_keypair].as_slice(),
        context.last_blockhash,
    );
    let old_merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
        &mut context,
        env.merkle_tree_pubkey,
    )
    .await;
    let old_merkle_tree = old_merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await;

    mock_indexer.add_compressed_accounts_with_token_data(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );

    assert_transfer(
        &mut context,
        &mock_indexer,
        &transfer_recipient_out_compressed_account,
        &change_out_compressed_account,
        &old_merkle_tree,
        &input_compressed_accounts,
    )
    .await;
    mock_indexer.nullify_compressed_accounts(&mut context).await;
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
    let env: light_test_utils::test_env::EnvWithAccounts =
        setup_test_programs_with_accounts().await;
    let mut context = env.context;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let indexed_array_pubkey = env.indexed_array_pubkey;
    let mock_indexer = MockIndexer::new(
        merkle_tree_pubkey,
        indexed_array_pubkey,
        payer.insecure_clone(),
        Some(0),
    );
    let recipient_keypair = Keypair::new();
    airdrop_lamports(&mut context, &recipient_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let mint = create_mint_helper(&mut context, &payer).await;
    let amount = 10000u64;
    let instruction = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        &mint,
        &merkle_tree_pubkey,
        vec![amount],
        vec![recipient_keypair.pubkey()],
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.last_blockhash,
    );
    let old_merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
        &mut context,
        env.merkle_tree_pubkey,
    )
    .await;
    let old_merkle_tree = old_merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await;
    let mut mock_indexer = mock_indexer.await;
    mock_indexer.add_compressed_accounts_with_token_data(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );
    assert_mint_to(
        &mut context,
        &mock_indexer,
        &recipient_keypair,
        mint,
        amount,
        &old_merkle_tree,
    )
    .await;
    let transfer_recipient_keypair = Keypair::new();
    let input_compressed_account_token_data =
        mock_indexer.token_compressed_accounts[0].token_data.clone();
    let input_compressed_accounts = vec![mock_indexer.compressed_accounts
        [mock_indexer.token_compressed_accounts[0].index]
        .clone()];
    let (root_indices, proof) = mock_indexer
        .create_proof_for_compressed_accounts(
            &[input_compressed_accounts[0]
                .compressed_account
                .hash(
                    &merkle_tree_pubkey,
                    &input_compressed_accounts[0].leaf_index,
                )
                .unwrap()],
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
    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &recipient_keypair,
        &proof,
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(ErrorCode::ComputeOutputSumFailed.into())
        ))
    );

    let transfer_recipient_out_compressed_account_0 = TokenTransferOutputData {
        amount: 1000 - 1,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
    };
    // invalid token data amount (- 1)
    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &recipient_keypair,
        &proof,
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(ErrorCode::SumCheckFailed.into())
        ))
    );

    let zero_amount = TokenTransferOutputData {
        amount: 0,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
    };
    // invalid token data zero out amount
    let res = create_transfer_out_utxo_test(
        &mut context,
        zero_amount,
        zero_amount,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &recipient_keypair,
        &proof,
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(ErrorCode::SumCheckFailed.into())
        ))
    );
    let double_amount = TokenTransferOutputData {
        amount: input_compressed_account_token_data.amount,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: None,
    };
    // invalid double token data  amount
    let res = create_transfer_out_utxo_test(
        &mut context,
        double_amount,
        double_amount,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &recipient_keypair,
        &proof,
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(ErrorCode::ComputeOutputSumFailed.into())
        ))
    );

    let invalid_lamports_amount = TokenTransferOutputData {
        amount: 1000,
        owner: transfer_recipient_keypair.pubkey(),
        lamports: Some(1),
    };

    // invalid_lamports_amount
    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        invalid_lamports_amount,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &recipient_keypair,
        &proof,
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();

    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(psp_compressed_pda::ErrorCode::ComputeOutputSumFailed.into())
        ))
    );

    let mut input_compressed_account_token_data_invalid_amount =
        mock_indexer.token_compressed_accounts[0].token_data.clone();
    input_compressed_account_token_data_invalid_amount.amount = 0;
    let mut input_compressed_accounts = vec![mock_indexer.compressed_accounts
        [mock_indexer.token_compressed_accounts[0].index]
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

    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &recipient_keypair,
        &proof,
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();

    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(ErrorCode::ComputeOutputSumFailed.into())
        ))
    );
    let mut input_compressed_account_token_data =
        mock_indexer.token_compressed_accounts[0].token_data.clone();
    input_compressed_account_token_data.delegate = Some(Pubkey::new_unique());
    let mut input_compressed_accounts = vec![mock_indexer.compressed_accounts
        [mock_indexer.token_compressed_accounts[0].index]
        .clone()];
    let mut vec = Vec::new();
    crate::TokenData::serialize(&input_compressed_account_token_data, &mut vec).unwrap();
    input_compressed_accounts[0]
        .compressed_account
        .data
        .as_mut()
        .unwrap()
        .data = vec;
    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &recipient_keypair,
        &proof,
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();

    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(psp_compressed_pda::ErrorCode::ProofVerificationFailed.into())
        ))
    );
    let mut input_compressed_account_token_data =
        mock_indexer.token_compressed_accounts[0].token_data.clone();
    input_compressed_account_token_data.owner = Pubkey::new_unique();
    let mut input_compressed_accounts = vec![mock_indexer.compressed_accounts
        [mock_indexer.token_compressed_accounts[0].index]
        .clone()];
    let mut vec = Vec::new();
    crate::TokenData::serialize(&input_compressed_account_token_data, &mut vec).unwrap();
    input_compressed_accounts[0]
        .compressed_account
        .data
        .as_mut()
        .unwrap()
        .data = vec;
    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &recipient_keypair,
        &proof,
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();

    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(ErrorCode::SignerCheckFailed.into())
        ))
    );

    let mut input_compressed_account_token_data =
        mock_indexer.token_compressed_accounts[0].token_data.clone();
    input_compressed_account_token_data.is_native = Some(0);
    let mut input_compressed_accounts = vec![mock_indexer.compressed_accounts
        [mock_indexer.token_compressed_accounts[0].index]
        .clone()];
    let mut vec = Vec::new();
    crate::TokenData::serialize(&input_compressed_account_token_data, &mut vec).unwrap();
    input_compressed_accounts[0]
        .compressed_account
        .data
        .as_mut()
        .unwrap()
        .data = vec;
    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &recipient_keypair,
        &proof,
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();

    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(psp_compressed_pda::ErrorCode::ProofVerificationFailed.into())
        ))
    );

    let mut input_compressed_account_token_data =
        mock_indexer.token_compressed_accounts[0].token_data.clone();
    input_compressed_account_token_data.state = AccountState::Uninitialized;
    let mut input_compressed_accounts = vec![mock_indexer.compressed_accounts
        [mock_indexer.token_compressed_accounts[0].index]
        .clone()];
    let mut vec = Vec::new();
    crate::TokenData::serialize(&input_compressed_account_token_data, &mut vec).unwrap();
    input_compressed_accounts[0]
        .compressed_account
        .data
        .as_mut()
        .unwrap()
        .data = vec;
    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &recipient_keypair,
        &proof,
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();

    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(psp_compressed_pda::ErrorCode::ProofVerificationFailed.into())
        ))
    );
    let mut input_compressed_account_token_data =
        mock_indexer.token_compressed_accounts[0].token_data.clone();
    input_compressed_account_token_data.delegated_amount = 1;
    let mut input_compressed_accounts = vec![mock_indexer.compressed_accounts
        [mock_indexer.token_compressed_accounts[0].index]
        .clone()];
    let mut vec = Vec::new();
    crate::TokenData::serialize(&input_compressed_account_token_data, &mut vec).unwrap();
    input_compressed_accounts[0]
        .compressed_account
        .data
        .as_mut()
        .unwrap()
        .data = vec;
    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &indexed_array_pubkey,
        &recipient_keypair,
        &proof,
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();

    assert_eq!(
        res.result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(psp_compressed_pda::ErrorCode::ProofVerificationFailed.into())
        ))
    );
}

async fn create_transfer_out_utxo_test(
    context: &mut ProgramTestContext,
    change_token_transfer_output: TokenTransferOutputData,
    transfer_recipient_token_transfer_output: TokenTransferOutputData,
    merkle_tree_pubkey: &Pubkey,
    indexed_array_pubkey: &Pubkey,
    payer: &Keypair,
    proof: &CompressedProof,
    root_indices: &Vec<u16>,
    input_compressed_accounts: &Vec<CompressedAccountWithMerkleContext>,
) -> Result<BanksTransactionResultWithMetadata, BanksClientError> {
    let input_compressed_account_indices: Vec<u32> = input_compressed_accounts
        .iter()
        .map(|x| x.leaf_index)
        .collect();

    let input_compressed_accounts: Vec<CompressedAccount> = input_compressed_accounts
        .iter()
        .map(|x| x.compressed_account.clone())
        .collect();
    let instruction = transfer_sdk::create_transfer_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &vec![*merkle_tree_pubkey], // input compressed account Merkle trees
        &vec![*indexed_array_pubkey], // input compressed account indexed arrays
        &vec![*merkle_tree_pubkey, *merkle_tree_pubkey], // output compressed account Merkle trees
        input_compressed_accounts.as_slice(), // input compressed_accounts
        &vec![
            change_token_transfer_output,
            transfer_recipient_token_transfer_output,
        ],
        root_indices,
        &input_compressed_account_indices,
        &proof,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        [&payer].as_slice(),
        context.last_blockhash,
    );
    solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await
}

async fn assert_mint_to<'a>(
    context: &mut ProgramTestContext,
    mock_indexer: &MockIndexer,
    recipient_keypair: &Keypair,
    mint: Pubkey,
    amount: u64,
    old_merkle_tree: &light_concurrent_merkle_tree::ConcurrentMerkleTree26<'a, Poseidon>,
) {
    let token_compressed_account_data =
        mock_indexer.token_compressed_accounts[0].token_data.clone();
    assert_eq!(token_compressed_account_data.amount, amount);
    assert_eq!(
        token_compressed_account_data.owner,
        recipient_keypair.pubkey()
    );
    assert_eq!(token_compressed_account_data.mint, mint);
    assert_eq!(token_compressed_account_data.delegate, None);
    assert_eq!(token_compressed_account_data.is_native, None);
    assert_eq!(token_compressed_account_data.delegated_amount, 0);

    let merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
        context,
        mock_indexer.merkle_tree_pubkey,
    )
    .await;
    // let merkle_tree =
    //     state_merkle_tree_from_bytes(&merkle_tree_account.deserialized.state_merkle_tree);
    let merkle_tree = merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    assert_eq!(
        merkle_tree.root().unwrap(),
        mock_indexer.merkle_tree.root().unwrap(),
        "merkle tree root update failed"
    );
    assert_eq!(merkle_tree.root_index(), 1);
    assert_ne!(
        old_merkle_tree.root().unwrap(),
        merkle_tree.root().unwrap(),
        "merkle tree root update failed"
    );
    let mint_account: spl_token::state::Mint = spl_token::state::Mint::unpack(
        &context
            .banks_client
            .get_account(mint)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    assert_eq!(mint_account.supply, amount);

    let pool = get_token_pool_pda(&mint);
    let pool_account = spl_token::state::Account::unpack(
        &context
            .banks_client
            .get_account(pool)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();
    assert_eq!(pool_account.amount, amount);
}

async fn assert_transfer<'a>(
    context: &mut ProgramTestContext,
    mock_indexer: &MockIndexer,
    recipient_out_compressed_account: &TokenTransferOutputData,
    change_out_compressed_account: &TokenTransferOutputData,
    old_merkle_tree: &light_concurrent_merkle_tree::ConcurrentMerkleTree26<'a, Poseidon>,
    input_compressed_accounts: &Vec<CompressedAccount>,
) {
    let merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
        context,
        mock_indexer.merkle_tree_pubkey,
    )
    .await;
    let merkle_tree = merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    assert_eq!(merkle_tree.root_index(), 3);

    assert_eq!(
        merkle_tree.root().unwrap(),
        mock_indexer.merkle_tree.root().unwrap(),
        "merkle tree root update failed"
    );
    assert_ne!(
        old_merkle_tree.root().unwrap(),
        merkle_tree.root().unwrap(),
        "merkle tree root update failed"
    );
    let pos = mock_indexer
        .token_compressed_accounts
        .iter()
        .position(|x| x.token_data.owner == recipient_out_compressed_account.owner)
        .expect("transfer recipient compressed account not found in mock indexer");
    let transfer_recipient_token_compressed_account =
        mock_indexer.token_compressed_accounts[pos].clone();
    assert_eq!(
        transfer_recipient_token_compressed_account
            .token_data
            .amount,
        recipient_out_compressed_account.amount
    );
    assert_eq!(
        transfer_recipient_token_compressed_account.token_data.mint,
        transfer_recipient_token_compressed_account.token_data.mint
    );
    assert_eq!(
        transfer_recipient_token_compressed_account.token_data.owner,
        recipient_out_compressed_account.owner
    );
    assert_eq!(
        transfer_recipient_token_compressed_account
            .token_data
            .delegate,
        None
    );
    assert_eq!(
        transfer_recipient_token_compressed_account
            .token_data
            .is_native,
        None
    );
    assert_eq!(
        transfer_recipient_token_compressed_account
            .token_data
            .delegated_amount,
        0
    );
    let transfer_recipient_compressed_account =
        mock_indexer.compressed_accounts[transfer_recipient_token_compressed_account.index].clone();
    assert_eq!(
        transfer_recipient_compressed_account
            .compressed_account
            .lamports,
        0
    );
    assert!(transfer_recipient_compressed_account
        .compressed_account
        .data
        .is_some());
    let mut data = Vec::new();
    transfer_recipient_token_compressed_account
        .token_data
        .serialize(&mut data)
        .unwrap();
    assert_eq!(
        transfer_recipient_compressed_account
            .compressed_account
            .data
            .as_ref()
            .unwrap()
            .data,
        data
    );

    let pos = mock_indexer
        .token_compressed_accounts
        .iter()
        .position(|x| {
            x.token_data.owner == change_out_compressed_account.owner
                && x.token_data.amount == change_out_compressed_account.amount
        })
        .expect("transfer recipient compressed account not found in mock indexer");
    let change_token_compressed_account = mock_indexer.token_compressed_accounts[pos].clone();
    assert_eq!(
        change_token_compressed_account.token_data.amount,
        change_out_compressed_account.amount
    );
    assert_eq!(
        change_token_compressed_account.token_data.mint,
        transfer_recipient_token_compressed_account.token_data.mint
    );
    assert_eq!(
        change_token_compressed_account.token_data.owner,
        change_out_compressed_account.owner
    );
    assert_eq!(change_token_compressed_account.token_data.delegate, None);
    assert_eq!(change_token_compressed_account.token_data.is_native, None);
    assert_eq!(
        change_token_compressed_account.token_data.delegated_amount,
        0
    );

    let change_compressed_account =
        mock_indexer.compressed_accounts[change_token_compressed_account.index].clone();
    assert_eq!(change_compressed_account.compressed_account.lamports, 0);
    assert!(change_compressed_account.compressed_account.data.is_some());
    let mut data = Vec::new();
    change_token_compressed_account
        .token_data
        .serialize(&mut data)
        .unwrap();
    assert_eq!(
        change_compressed_account
            .compressed_account
            .data
            .as_ref()
            .unwrap()
            .data,
        data
    );
    assert_eq!(
        change_compressed_account.compressed_account.owner,
        psp_compressed_token::ID
    );

    // assert in compressed_accounts are nullified
    for compressed_account in input_compressed_accounts.iter() {
        let _nullified_compressed_account = mock_indexer
            .nullified_compressed_accounts
            .iter()
            .find(|x| x.compressed_account == *compressed_account)
            .expect("compressed_account not nullified");
    }
}

#[derive(Debug)]
pub struct MockIndexer {
    pub merkle_tree_pubkey: Pubkey,
    pub indexed_array_pubkey: Pubkey,
    pub payer: Keypair,
    pub compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub nullified_compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub token_compressed_accounts: Vec<TokenDataWithContext>,
    pub token_nullified_compressed_accounts: Vec<TokenDataWithContext>,
    pub events: Vec<PublicTransactionEvent>,
    pub merkle_tree: light_merkle_tree_reference::MerkleTree<light_hasher::Poseidon>,
    pub gnark_server: Option<std::process::Child>,
}

#[derive(Debug, Clone)]
pub struct TokenDataWithContext {
    pub index: usize,
    pub token_data: TokenData,
}

impl MockIndexer {
    async fn new(
        merkle_tree_pubkey: Pubkey,
        indexed_array_pubkey: Pubkey,
        payer: Keypair,
        startup_time: Option<u64>,
    ) -> Self {
        let gnark_server = if startup_time.is_some() {
            Some(spawn_gnark_server(
                "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
                0,
            ))
        } else {
            None
        };

        let merkle_tree = light_merkle_tree_reference::MerkleTree::<light_hasher::Poseidon>::new(
            STATE_MERKLE_TREE_HEIGHT,
            STATE_MERKLE_TREE_ROOTS,
            STATE_MERKLE_TREE_CANOPY_DEPTH,
        )
        .unwrap();
        if startup_time.is_some() {
            tokio::time::sleep(tokio::time::Duration::from_secs(startup_time.unwrap())).await;
        }
        Self {
            merkle_tree_pubkey,
            indexed_array_pubkey,
            payer,
            compressed_accounts: vec![],
            nullified_compressed_accounts: vec![],
            events: vec![],
            token_compressed_accounts: vec![],
            token_nullified_compressed_accounts: vec![],
            merkle_tree,
            gnark_server,
        }
    }

    pub async fn create_proof_for_compressed_accounts(
        &mut self,
        compressed_accounts: &[[u8; 32]],
        context: &mut ProgramTestContext,
    ) -> (Vec<u16>, CompressedProof) {
        self.gnark_server = if self.gnark_server.is_none() {
            Some(spawn_gnark_server(
                "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
                4,
            ))
        } else {
            None
        };
        // waiting for server to start
        health_check().await;

        let client = Client::new();

        let mut inclusion_proofs = Vec::<InclusionMerkleProofInputs>::new();
        for compressed_account in compressed_accounts.iter() {
            let leaf_index = self.merkle_tree.get_leaf_index(compressed_account).unwrap();
            let proof = self
                .merkle_tree
                .get_proof_of_leaf(leaf_index, true)
                .unwrap();
            inclusion_proofs.push(InclusionMerkleProofInputs {
                root: BigInt::from_be_bytes(self.merkle_tree.root().unwrap().as_slice()),
                leaf: BigInt::from_be_bytes(compressed_account),
                in_path_indices: BigInt::from_be_bytes(leaf_index.to_be_bytes().as_slice()), // leaf_index as u32,
                in_path_elements: proof.iter().map(|x| BigInt::from_be_bytes(x)).collect(),
            });
        }
        let inclusion_proof_inputs = InclusionProofInputs(inclusion_proofs.as_slice());
        let json_payload =
            InclusionJsonStruct::from_inclusion_proof_inputs(&inclusion_proof_inputs).to_string();

        let response_result = client
            .post(&format!("{}{}", SERVER_ADDRESS, INCLUSION_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(json_payload)
            .send()
            .await
            .expect("Failed to execute request.");
        assert!(response_result.status().is_success());
        let body = response_result.text().await.unwrap();
        let proof_json = deserialize_gnark_proof_json(&body).unwrap();
        let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
        let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);

        let merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
            context,
            self.merkle_tree_pubkey,
        )
        .await;
        let merkle_tree = merkle_tree_account
            .deserialized()
            .copy_merkle_tree()
            .unwrap();
        assert_eq!(
            self.merkle_tree.root().unwrap(),
            merkle_tree.root().unwrap(),
            "Local Merkle tree root is not equal to latest onchain root"
        );

        let root_indices: Vec<u16> =
            vec![merkle_tree.current_root_index as u16; compressed_accounts.len()];
        (
            root_indices,
            CompressedProof {
                a: proof_a,
                b: proof_b,
                c: proof_c,
            },
        )
    }

    /// deserializes an event
    /// adds the output_compressed_accounts to the compressed_accounts
    /// removes the input_compressed_accounts from the compressed_accounts
    /// adds the input_compressed_accounts to the nullified_compressed_accounts
    pub fn add_lamport_compressed_accounts(&mut self, event_bytes: Vec<u8>) {
        let event_bytes = event_bytes.clone();
        let event = PublicTransactionEvent::deserialize(&mut event_bytes.as_slice()).unwrap();
        self.add_event_and_compressed_accounts(event);
    }

    pub fn add_event_and_compressed_accounts(
        &mut self,
        event: PublicTransactionEvent,
    ) -> Vec<usize> {
        for compressed_account in event.input_compressed_accounts.iter() {
            let index = self
                .compressed_accounts
                .iter()
                .position(|x| x.compressed_account == compressed_account.compressed_account)
                .expect("compressed_account not found");
            self.compressed_accounts.remove(index);
            // TODO: nullify compressed_account in Merkle tree, not implemented yet
            self.nullified_compressed_accounts
                .push(compressed_account.clone());
            let index = self
                .compressed_accounts
                .iter()
                .position(|x| x == compressed_account);
            match index {
                Some(index) => {
                    let token_compressed_account_element =
                        self.token_compressed_accounts[index].clone();
                    self.token_compressed_accounts.remove(index);
                    self.token_nullified_compressed_accounts
                        .push(token_compressed_account_element);
                }
                None => {}
            }
        }
        let mut indices = Vec::with_capacity(event.output_compressed_accounts.len());
        for (i, compressed_account) in event.output_compressed_accounts.iter().enumerate() {
            self.compressed_accounts
                .push(CompressedAccountWithMerkleContext {
                    compressed_account: compressed_account.clone(),
                    leaf_index: event.output_leaf_indices[i as usize],
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                });
            indices.push(self.compressed_accounts.len() - 1);
            self.merkle_tree
                .append(
                    &compressed_account
                        .hash(
                            &self.merkle_tree_pubkey,
                            &event.output_leaf_indices[i as usize],
                        )
                        .unwrap(),
                )
                .expect("insert failed");
        }

        self.events.push(event);
        indices
    }

    /// deserializes an event
    /// adds the output_compressed_accounts to the compressed_accounts
    /// removes the input_compressed_accounts from the compressed_accounts
    /// adds the input_compressed_accounts to the nullified_compressed_accounts
    /// deserialiazes token data from the output_compressed_accounts
    /// adds the token_compressed_accounts to the token_compressed_accounts
    pub fn add_compressed_accounts_with_token_data(&mut self, event_bytes: Vec<u8>) {
        let event_bytes = event_bytes.clone();
        let event = PublicTransactionEvent::deserialize(&mut event_bytes.as_slice()).unwrap();
        let indices = self.add_event_and_compressed_accounts(event);
        for index in indices.iter() {
            let data = self.compressed_accounts[*index]
                .compressed_account
                .data
                .as_ref()
                .unwrap();
            let token_data = TokenData::deserialize(&mut data.data.as_slice()).unwrap();
            self.token_compressed_accounts.push(TokenDataWithContext {
                index: *index,
                token_data,
            });
        }
    }

    /// Check compressed_accounts in the queue array which are not nullified yet
    /// Iterate over these compressed_accounts and nullify them
    pub async fn nullify_compressed_accounts(&mut self, context: &mut ProgramTestContext) {
        let array = AccountZeroCopy::<account_compression::IndexedArrayAccount>::new(
            context,
            self.indexed_array_pubkey,
        )
        .await;
        let indexed_array = array.deserialized().indexed_array;
        let merkle_tree_account = light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
            context,
            self.merkle_tree_pubkey,
        )
        .await;
        let merkle_tree = merkle_tree_account
            .deserialized()
            .copy_merkle_tree()
            .unwrap();
        let change_log_index = merkle_tree.current_changelog_index as u64;

        let mut compressed_account_to_nullify = Vec::new();

        for (i, element) in indexed_array.iter().enumerate() {
            if element.merkle_tree_overwrite_sequence_number == 0 && element.element != [0u8; 32] {
                compressed_account_to_nullify.push((i, element));
            }
        }

        for (index_in_indexed_array, compressed_account) in compressed_account_to_nullify.iter() {
            let leaf_index = self
                .merkle_tree
                .get_leaf_index(&compressed_account.element)
                .unwrap();
            let proof: Vec<[u8; 32]> = self
                .merkle_tree
                .get_proof_of_leaf(leaf_index, false)
                .unwrap()
                .to_array::<16>()
                .unwrap()
                .to_vec();

            let instructions = [
                account_compression::nullify_leaves::sdk_nullify::create_nullify_instruction(
                    vec![change_log_index].as_slice(),
                    vec![(*index_in_indexed_array) as u16].as_slice(),
                    vec![0u64].as_slice(),
                    vec![proof].as_slice(),
                    &context.payer.pubkey(),
                    &self.merkle_tree_pubkey,
                    &self.indexed_array_pubkey,
                ),
            ];

            create_and_send_transaction(
                context,
                &instructions,
                &self.payer.pubkey(),
                &[&self.payer],
            )
            .await
            .unwrap();
            let array = AccountZeroCopy::<account_compression::IndexedArrayAccount>::new(
                context,
                self.indexed_array_pubkey,
            )
            .await;
            let indexed_array = array.deserialized().indexed_array;
            assert_eq!(
                indexed_array[*index_in_indexed_array].element,
                compressed_account.element
            );
            let merkle_tree_account =
                light_test_utils::AccountZeroCopy::<StateMerkleTreeAccount>::new(
                    context,
                    self.merkle_tree_pubkey,
                )
                .await;
            assert_eq!(
                indexed_array[*index_in_indexed_array].merkle_tree_overwrite_sequence_number,
                merkle_tree_account
                    .deserialized()
                    .load_merkle_tree()
                    .unwrap()
                    .sequence_number as u64
                    + account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS as u64
            );
        }
    }
}
