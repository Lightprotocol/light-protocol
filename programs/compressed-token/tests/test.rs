#![cfg(feature = "test-sbf")]

use account_compression::{
    initialize_nullifier_queue::NullifierQueueAccount,
    utils::constants::{STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT},
    StateMerkleTreeAccount,
};
use anchor_lang::AnchorSerialize;
use light_circuitlib_rs::{
    gnark::{
        constants::{PROVE_PATH, SERVER_ADDRESS},
        helpers::kill_gnark_server,
        proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    },
    inclusion::merkle_inclusion_proof_inputs::{InclusionMerkleProofInputs, InclusionProofInputs},
};
use light_compressed_pda::{
    invoke::processor::CompressedProof, sdk::compressed_account::MerkleContext,
};
use light_compressed_pda::{
    sdk::compressed_account::{PackedCompressedAccountWithMerkleContext, PackedMerkleContext},
    sdk::event::PublicTransactionEvent,
};
use light_compressed_token::{
    get_cpi_authority_pda, get_token_authority_pda, get_token_pool_pda,
    mint_sdk::{create_initialize_mint_instruction, create_mint_to_instruction},
    token_data::TokenData,
    transfer_sdk, ErrorCode, TokenTransferOutputData,
};
use light_hasher::Poseidon;
use light_test_utils::{
    airdrop_lamports, assert_custom_error_or_program_error, create_account_instruction,
    create_and_send_transaction, create_and_send_transaction_with_event, get_hash_set,
    test_env::setup_test_programs_with_accounts, AccountZeroCopy, FeeConfig, TransactionParams,
};
use light_verifier::VerifierError;
use num_bigint::{BigInt, BigUint};
use num_traits::ops::bytes::FromBytes;
use reqwest::Client;
use solana_program_test::{
    BanksClientError, BanksTransactionResultWithMetadata, ProgramTestContext,
};
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer,
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
        payer,
        anchor_spl::token::Mint::LEN,
        rent,
        &anchor_spl::token::ID,
        Some(mint_keypair),
    );

    let mint_pubkey = mint_keypair.pubkey();
    let mint_authority = get_token_authority_pda(authority, &mint_pubkey).0;
    let create_mint_instruction = initialize_mint(
        &anchor_spl::token::ID,
        &mint_keypair.pubkey(),
        &mint_authority,
        None,
        decimals,
    )
    .unwrap();
    let transfer_ix =
        anchor_lang::solana_program::system_instruction::transfer(payer, &mint_pubkey, rent);

    let instruction = create_initialize_mint_instruction(payer, authority, &mint_pubkey);
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
use light_circuitlib_rs::gnark::helpers::{spawn_gnark_server, ProofType};
use light_circuitlib_rs::gnark::inclusion_json_formatter::BatchInclusionJsonStruct;

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
    let mint_authority = get_token_authority_pda(authority, mint).0;
    assert_eq!(mint_account.supply, 0);
    assert_eq!(mint_account.decimals, 2);
    assert_eq!(mint_account.mint_authority.unwrap(), mint_authority);
    assert_eq!(mint_account.freeze_authority, None.into());
    assert!(mint_account.is_initialized);
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
    assert_eq!(mint_account.owner, get_cpi_authority_pda().0);
}

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
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;
    let mut mock_indexer = MockIndexer::new(
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        payer.insecure_clone(),
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
        let old_merkle_tree_account =
            AccountZeroCopy::<StateMerkleTreeAccount>::new(&mut context, env.merkle_tree_pubkey)
                .await;
        let old_merkle_tree = old_merkle_tree_account
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
                num_output_compressed_accounts: MINTS as u8,
                compress: 0,
                fee_config: FeeConfig::default(),
            }),
        )
        .await
        .unwrap()
        .unwrap();

        if i == 0 {
            mock_indexer.add_compressed_accounts_with_token_data(event);
            assert_mint_to(
                MINTS,
                &mut context,
                &mock_indexer,
                &recipient_keypair,
                mint,
                amount,
                &old_merkle_tree,
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
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;
    let mock_indexer = MockIndexer::new(
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        payer.insecure_clone(),
    );
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
    let old_merkle_tree_account =
        AccountZeroCopy::<StateMerkleTreeAccount>::new(&mut context, env.merkle_tree_pubkey).await;
    let old_merkle_tree = old_merkle_tree_account
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
            num_output_compressed_accounts: inputs as u8,
            compress: 0,
            fee_config: FeeConfig::default(),
        }),
    )
    .await
    .unwrap()
    .unwrap();
    let mut mock_indexer = mock_indexer.await;
    mock_indexer.add_compressed_accounts_with_token_data(event);
    assert_mint_to(
        inputs,
        &mut context,
        &mock_indexer,
        &recipient_keypair,
        mint,
        amount,
        &old_merkle_tree,
    )
    .await;

    let mut input_merkle_tree_context = Vec::new();
    let mut input_compressed_account_token_data = Vec::new();
    let mut input_compressed_account_hashes = Vec::new();
    for i in 0..inputs {
        let token_data: TokenDataWithContext = mock_indexer.token_compressed_accounts[i].clone();
        let leaf_index = token_data
            .compressed_account
            .merkle_context
            .leaf_index
            .clone();
        input_compressed_account_token_data.push(token_data.token_data);
        input_compressed_account_hashes.push(
            token_data
                .compressed_account
                .compressed_account
                .hash::<Poseidon>(&merkle_tree_pubkey, &leaf_index)
                .unwrap(),
        );
        input_merkle_tree_context.push(MerkleContext {
            merkle_tree_pubkey,
            nullifier_queue_pubkey,
            leaf_index,
        });
    }

    let equal_amount = (amount * inputs as u64) / outputs as u64;
    let rest_amount = (amount * inputs as u64) % outputs as u64;
    let mut output_compressed_accounts = Vec::new();

    let change_out_compressed_account = TokenTransferOutputData {
        amount: equal_amount + rest_amount,
        owner: recipient_keypair.pubkey(),
        lamports: None,
    };
    output_compressed_accounts.push(change_out_compressed_account);
    for _ in 1..outputs {
        let transfer_recipient_keypair = Keypair::new();
        let transfer_recipient_out_compressed_account = TokenTransferOutputData {
            amount: equal_amount,
            owner: transfer_recipient_keypair.pubkey(),
            lamports: None,
        };
        output_compressed_accounts.push(transfer_recipient_out_compressed_account);
    }
    let (root_indices, proof) = mock_indexer
        .create_proof_for_compressed_accounts(&input_compressed_account_hashes, &mut context)
        .await;

    let instruction = transfer_sdk::create_transfer_instruction(
        &payer_pubkey,
        &recipient_keypair.pubkey(), // authority
        &input_merkle_tree_context,
        &vec![merkle_tree_pubkey; outputs], // output_compressed_account_merkle_tree_pubkeys
        &output_compressed_accounts,        // output_compressed_accounts
        &root_indices,
        &Some(proof),
        input_compressed_account_token_data.as_slice(), // input_token_data
        mint,
        None,  // owner_if_delegate_is_signer
        false, // is_compress
        None,  // compression_amount
        None,  // token_pool_pda
        None,  // decompress_token_account
    )
    .unwrap();

    let old_merkle_tree_account =
        AccountZeroCopy::<StateMerkleTreeAccount>::new(&mut context, env.merkle_tree_pubkey).await;
    let old_merkle_tree = old_merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    let event = create_and_send_transaction_with_event(
        &mut context,
        &[instruction],
        &payer_pubkey,
        &[&payer, &recipient_keypair],
        Some(TransactionParams {
            num_new_addresses: 0,
            num_input_compressed_accounts: input_compressed_account_hashes.len() as u8,
            num_output_compressed_accounts: output_compressed_accounts.len() as u8,
            compress: 5000, // for second signer
            fee_config: FeeConfig::default(),
        }),
    )
    .await
    .unwrap()
    .unwrap();

    mock_indexer.add_compressed_accounts_with_token_data(event);

    assert_transfer(
        &mut context,
        &mock_indexer,
        &output_compressed_accounts,
        &old_merkle_tree,
        &input_compressed_account_hashes,
    )
    .await;
    kill_gnark_server();

    // TODO: fix nullify function
    // mock_indexer.nullify_compressed_accounts(&mut context).await;
}

#[tokio::test]
async fn test_decompression() {
    let (mut context, env) = setup_test_programs_with_accounts(None).await;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;
    let mock_indexer = MockIndexer::new(
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        payer.insecure_clone(),
        // Some(0), // TODO: check if required
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
    let old_merkle_tree_account =
        AccountZeroCopy::<StateMerkleTreeAccount>::new(&mut context, env.merkle_tree_pubkey).await;
    let old_merkle_tree = old_merkle_tree_account
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
    let mut mock_indexer = mock_indexer.await;
    mock_indexer.add_compressed_accounts_with_token_data(event);
    assert_mint_to(
        1,
        &mut context,
        &mock_indexer,
        &recipient_keypair,
        mint,
        amount,
        &old_merkle_tree,
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

    let input_compressed_account_token_data = mock_indexer.token_compressed_accounts[0].token_data;
    let input_compressed_accounts = vec![mock_indexer.token_compressed_accounts[0]
        .compressed_account
        .clone()];

    let change_out_compressed_account = TokenTransferOutputData {
        amount: input_compressed_account_token_data.amount - 1000,
        owner: recipient_keypair.pubkey(),
        lamports: None,
    };

    let (root_indices, proof) = mock_indexer
        .create_proof_for_compressed_accounts(
            &[input_compressed_accounts[0]
                .compressed_account
                .hash::<Poseidon>(
                    &merkle_tree_pubkey,
                    &input_compressed_accounts[0].merkle_context.leaf_index,
                )
                .unwrap()],
            &mut context,
        )
        .await;

    let instruction = transfer_sdk::create_transfer_instruction(
        &payer_pubkey,
        &recipient_keypair.pubkey(), // authority
        &[MerkleContext {
            merkle_tree_pubkey,
            nullifier_queue_pubkey,
            leaf_index: input_compressed_accounts[0].merkle_context.leaf_index,
        }], // input_compressed_account_merkle_context
        &[merkle_tree_pubkey],       // output_compressed_account_merkle_tree_pubkeys
        &[change_out_compressed_account], // output_compressed_accounts
        &root_indices,               // root_indices
        &Some(proof),
        [input_compressed_account_token_data].as_slice(), // input_token_data
        mint,                                             // mint
        None,                                             // owner_if_delegate_is_signer
        false,                                            // is_compress
        Some(1000u64),                                    // compression_amount
        Some(get_token_pool_pda(&mint)),                  // token_pool_pda
        Some(recipient_token_account_keypair.pubkey()),   // decompress_token_account
    )
    .unwrap();

    let event = create_and_send_transaction_with_event(
        &mut context,
        &[instruction],
        &payer_pubkey,
        &[&payer, &recipient_keypair],
        Some(TransactionParams {
            num_new_addresses: 0,
            num_input_compressed_accounts: 1,
            num_output_compressed_accounts: 1,
            compress: 5000, // for second signer
            fee_config: FeeConfig::default(),
        }),
    )
    .await
    .unwrap()
    .unwrap();

    mock_indexer.add_compressed_accounts_with_token_data(event);

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
    mock_indexer.add_compressed_accounts_with_token_data(event);
    assert!(mock_indexer
        .token_compressed_accounts
        .iter()
        .any(|x| x.token_data.amount == 1000));
    assert!(mock_indexer
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
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let nullifier_queue_pubkey = env.nullifier_queue_pubkey;
    let mock_indexer = MockIndexer::new(
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        payer.insecure_clone(),
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
    let old_merkle_tree_account =
        AccountZeroCopy::<StateMerkleTreeAccount>::new(&mut context, env.merkle_tree_pubkey).await;
    let old_merkle_tree = old_merkle_tree_account
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
    let mut mock_indexer = mock_indexer.await;
    mock_indexer.add_compressed_accounts_with_token_data(event);
    assert_mint_to(
        1,
        &mut context,
        &mock_indexer,
        &recipient_keypair,
        mint,
        amount,
        &old_merkle_tree,
    )
    .await;
    let transfer_recipient_keypair = Keypair::new();
    let input_compressed_account_token_data = mock_indexer.token_compressed_accounts[0].token_data;
    let input_compressed_accounts = vec![mock_indexer.token_compressed_accounts[0]
        .compressed_account
        .clone()];
    let (root_indices, proof) = mock_indexer
        .create_proof_for_compressed_accounts(
            &[input_compressed_accounts[0]
                .compressed_account
                .hash::<Poseidon>(
                    &merkle_tree_pubkey,
                    &input_compressed_accounts[0].merkle_context.leaf_index,
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
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof.clone()),
        &root_indices,
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
    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof.clone()),
        &root_indices,
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
    let res = create_transfer_out_utxo_test(
        &mut context,
        zero_amount,
        zero_amount,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof.clone()),
        &root_indices,
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
    let res = create_transfer_out_utxo_test(
        &mut context,
        double_amount,
        double_amount,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof.clone()),
        &root_indices,
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
    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        invalid_lamports_amount,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof.clone()),
        &root_indices,
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
        mock_indexer.token_compressed_accounts[0].token_data;
    input_compressed_account_token_data_invalid_amount.amount = 0;
    let mut input_compressed_accounts = vec![mock_indexer.token_compressed_accounts[0]
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

    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof.clone()),
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_custom_error_or_program_error(res, ErrorCode::ComputeOutputSumFailed.into()).unwrap();
    // invalid delegate and delegated amount
    let mut input_compressed_account_token_data =
        mock_indexer.token_compressed_accounts[0].token_data;
    input_compressed_account_token_data.delegate = Some(Pubkey::new_unique());
    input_compressed_account_token_data.delegated_amount = 1;
    let mut input_compressed_accounts = vec![mock_indexer.token_compressed_accounts[0]
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
    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof.clone()),
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_custom_error_or_program_error(res, VerifierError::ProofVerificationFailed.into())
        .unwrap();
    let input_compressed_accounts = vec![mock_indexer.token_compressed_accounts[0]
        .compressed_account
        .clone()];
    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &payer,
        &Some(proof.clone()),
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_custom_error_or_program_error(res, VerifierError::ProofVerificationFailed.into())
        .unwrap();
    let mut input_compressed_account_token_data =
        mock_indexer.token_compressed_accounts[0].token_data;
    input_compressed_account_token_data.is_native = Some(0);
    let mut input_compressed_accounts = vec![mock_indexer.token_compressed_accounts[0]
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
    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof.clone()),
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();

    assert_custom_error_or_program_error(res, VerifierError::ProofVerificationFailed.into())
        .unwrap();

    let mut input_compressed_account_token_data =
        mock_indexer.token_compressed_accounts[0].token_data;
    input_compressed_account_token_data.delegated_amount = 1;
    let mut input_compressed_accounts = vec![mock_indexer.token_compressed_accounts[0]
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
    let res = create_transfer_out_utxo_test(
        &mut context,
        change_out_compressed_account_0,
        transfer_recipient_out_compressed_account_0,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &recipient_keypair,
        &Some(proof.clone()),
        &root_indices,
        &input_compressed_accounts,
    )
    .await
    .unwrap();
    assert_custom_error_or_program_error(res, ErrorCode::DelegateUndefined.into()).unwrap();
    kill_gnark_server();
}

#[allow(clippy::too_many_arguments)]
async fn create_transfer_out_utxo_test(
    context: &mut ProgramTestContext,
    change_token_transfer_output: TokenTransferOutputData,
    transfer_recipient_token_transfer_output: TokenTransferOutputData,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    payer: &Keypair,
    proof: &Option<CompressedProof>,
    root_indices: &[u16],
    input_compressed_accounts: &[PackedCompressedAccountWithMerkleContext],
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

pub async fn create_token_account(
    context: &mut ProgramTestContext,
    mint: &Pubkey,
    account_keypair: &Keypair,
    owner: &Keypair,
) -> Result<(), BanksClientError> {
    let rent = context
        .banks_client
        .get_rent()
        .await
        .unwrap()
        .minimum_balance(anchor_spl::token::TokenAccount::LEN);
    let account_create_ix = create_account_instruction(
        &owner.pubkey(),
        anchor_spl::token::TokenAccount::LEN,
        rent,
        &anchor_spl::token::ID,
        Some(account_keypair),
    );
    let instruction = spl_token::instruction::initialize_account(
        &spl_token::ID,
        &account_keypair.pubkey(),
        mint,
        &owner.pubkey(),
    )
    .unwrap();
    create_and_send_transaction(
        context,
        &[account_create_ix, instruction],
        &owner.pubkey(),
        &[account_keypair, owner],
    )
    .await
    .unwrap();
    Ok(())
}

async fn assert_mint_to<'a>(
    num_mint_to: usize,
    context: &mut ProgramTestContext,
    mock_indexer: &MockIndexer,
    recipient_keypair: &Keypair,
    mint: Pubkey,
    amount: u64,
    old_merkle_tree: &light_concurrent_merkle_tree::ConcurrentMerkleTree26<'a, Poseidon>,
) {
    let token_compressed_account_data = mock_indexer.token_compressed_accounts[0].token_data;
    assert_eq!(token_compressed_account_data.amount, amount);
    assert_eq!(
        token_compressed_account_data.owner,
        recipient_keypair.pubkey()
    );
    assert_eq!(token_compressed_account_data.mint, mint);
    assert_eq!(token_compressed_account_data.delegate, None);
    assert_eq!(token_compressed_account_data.is_native, None);
    assert_eq!(token_compressed_account_data.delegated_amount, 0);

    let merkle_tree_account =
        AccountZeroCopy::<StateMerkleTreeAccount>::new(context, mock_indexer.merkle_tree_pubkey)
            .await;
    let merkle_tree = merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    assert_eq!(
        merkle_tree.root().unwrap(),
        mock_indexer.merkle_tree.root(),
        "merkle tree root update failed"
    );
    assert_eq!(merkle_tree.root_index(), num_mint_to);
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
    assert_eq!(mint_account.supply, amount * num_mint_to as u64);

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
    assert_eq!(pool_account.amount, amount * num_mint_to as u64);
}

async fn assert_transfer<'a>(
    context: &mut ProgramTestContext,
    mock_indexer: &MockIndexer,
    out_compressed_accounts: &[TokenTransferOutputData],
    old_merkle_tree: &light_concurrent_merkle_tree::ConcurrentMerkleTree26<'a, Poseidon>,
    input_compressed_account_hashes: &[[u8; 32]],
) {
    let merkle_tree_account =
        AccountZeroCopy::<StateMerkleTreeAccount>::new(context, mock_indexer.merkle_tree_pubkey)
            .await;
    let merkle_tree = merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    assert_eq!(
        merkle_tree.root_index(),
        old_merkle_tree.root_index() + out_compressed_accounts.len()
    );

    assert_eq!(
        merkle_tree.root().unwrap(),
        mock_indexer.merkle_tree.root(),
        "merkle tree root update failed"
    );
    assert_ne!(
        old_merkle_tree.root().unwrap(),
        merkle_tree.root().unwrap(),
        "merkle tree root update failed"
    );
    assert_eq!(
        merkle_tree.next_index(),
        old_merkle_tree.next_index() + out_compressed_accounts.len()
    );
    let next_index_old_mt = old_merkle_tree.next_index();
    for (i, out_compressed_account) in out_compressed_accounts.iter().enumerate() {
        let pos = mock_indexer
            .token_compressed_accounts
            .iter()
            .position(|x| {
                x.token_data.owner == out_compressed_account.owner
                    && x.token_data.amount == out_compressed_account.amount
            })
            .expect("transfer recipient compressed account not found in mock indexer");
        let transfer_recipient_token_compressed_account =
            mock_indexer.token_compressed_accounts[pos].clone();
        assert_eq!(
            transfer_recipient_token_compressed_account
                .token_data
                .amount,
            out_compressed_account.amount
        );
        assert_eq!(
            transfer_recipient_token_compressed_account.token_data.owner,
            out_compressed_account.owner
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

        let transfer_recipient_compressed_account = transfer_recipient_token_compressed_account
            .compressed_account
            .clone();
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
        assert_eq!(
            transfer_recipient_compressed_account
                .compressed_account
                .owner,
            light_compressed_token::ID
        );
        assert_eq!(
            transfer_recipient_compressed_account
                .merkle_context
                .leaf_index as usize,
            next_index_old_mt + i
        );
    }
    let nullifier_queue = unsafe {
        get_hash_set::<u16, NullifierQueueAccount>(context, mock_indexer.nullifier_queue_pubkey)
            .await
    };
    for hash in input_compressed_account_hashes.iter() {
        assert!(nullifier_queue
            .contains(&BigUint::from_be_bytes(hash), 0)
            .unwrap());
    }
}

#[derive(Debug)]
pub struct MockIndexer {
    pub merkle_tree_pubkey: Pubkey,
    pub nullifier_queue_pubkey: Pubkey,
    pub payer: Keypair,
    pub compressed_accounts: Vec<PackedCompressedAccountWithMerkleContext>,
    pub nullified_compressed_accounts: Vec<PackedCompressedAccountWithMerkleContext>,
    pub token_compressed_accounts: Vec<TokenDataWithContext>,
    pub token_nullified_compressed_accounts: Vec<TokenDataWithContext>,
    pub events: Vec<PublicTransactionEvent>,
    pub merkle_tree: light_merkle_tree_reference::MerkleTree<Poseidon>,
}

#[derive(Debug, Clone)]
pub struct TokenDataWithContext {
    pub token_data: TokenData,
    pub compressed_account: PackedCompressedAccountWithMerkleContext,
}

impl MockIndexer {
    async fn new(
        merkle_tree_pubkey: Pubkey,
        nullifier_queue_pubkey: Pubkey,
        payer: Keypair,
    ) -> Self {
        spawn_gnark_server(
            "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
            true,
            &[ProofType::Inclusion],
        )
        .await;
        let merkle_tree = light_merkle_tree_reference::MerkleTree::<Poseidon>::new(
            STATE_MERKLE_TREE_HEIGHT as usize,
            STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
        );

        Self {
            merkle_tree_pubkey,
            nullifier_queue_pubkey,
            payer,
            compressed_accounts: vec![],
            nullified_compressed_accounts: vec![],
            events: vec![],
            token_compressed_accounts: vec![],
            token_nullified_compressed_accounts: vec![],
            merkle_tree,
        }
    }

    pub async fn create_proof_for_compressed_accounts(
        &mut self,
        compressed_accounts: &[[u8; 32]],
        context: &mut ProgramTestContext,
    ) -> (Vec<u16>, CompressedProof) {
        let client = Client::new();

        let mut inclusion_proofs = Vec::<InclusionMerkleProofInputs>::new();
        for compressed_account in compressed_accounts.iter() {
            let leaf_index = self.merkle_tree.get_leaf_index(compressed_account).unwrap();
            let proof = self
                .merkle_tree
                .get_proof_of_leaf(leaf_index, true)
                .unwrap();
            inclusion_proofs.push(InclusionMerkleProofInputs {
                root: BigInt::from_be_bytes(self.merkle_tree.root().as_slice()),
                leaf: BigInt::from_be_bytes(compressed_account),
                path_index: BigInt::from_be_bytes(leaf_index.to_be_bytes().as_slice()), // leaf_index as u32,
                path_elements: proof.iter().map(|x| BigInt::from_be_bytes(x)).collect(),
            });
        }
        let inclusion_proof_inputs = InclusionProofInputs(inclusion_proofs.as_slice());
        let json_payload =
            BatchInclusionJsonStruct::from_inclusion_proof_inputs(&inclusion_proof_inputs)
                .to_string();

        let response_result = client
            .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
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

        let merkle_tree_account =
            AccountZeroCopy::<StateMerkleTreeAccount>::new(context, self.merkle_tree_pubkey).await;
        let merkle_tree = merkle_tree_account
            .deserialized()
            .copy_merkle_tree()
            .unwrap();
        assert_eq!(
            self.merkle_tree.root(),
            merkle_tree.root().unwrap(),
            "Local Merkle tree root is not equal to latest on-chain root"
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

    pub fn add_event_and_compressed_accounts(&mut self, event: PublicTransactionEvent) {
        for hash in event.input_compressed_account_hashes.iter() {
            let index = self.compressed_accounts.iter().position(|x| {
                x.compressed_account
                    .hash::<Poseidon>(&self.merkle_tree_pubkey, &x.merkle_context.leaf_index)
                    .unwrap()
                    == *hash
            });
            if let Some(index) = index {
                self.compressed_accounts.remove(index);
                continue;
            };
            if index.is_none() {
                let index = self
                    .token_compressed_accounts
                    .iter()
                    .position(|x| {
                        x.compressed_account
                            .compressed_account
                            .hash::<Poseidon>(
                                &self.merkle_tree_pubkey,
                                &x.compressed_account.merkle_context.leaf_index,
                            )
                            .unwrap()
                            == *hash
                    })
                    .expect("input compressed account not found");
                self.token_compressed_accounts.remove(index);
            }
        }

        for (i, compressed_account) in event.output_compressed_accounts.iter().enumerate() {
            let data = compressed_account.data.as_ref().unwrap();
            match TokenData::deserialize(&mut data.data.as_slice()) {
                Ok(token_data) => {
                    self.token_compressed_accounts.push(TokenDataWithContext {
                        token_data,
                        compressed_account: PackedCompressedAccountWithMerkleContext {
                            compressed_account: compressed_account.clone(),
                            merkle_context: PackedMerkleContext {
                                leaf_index: event.output_leaf_indices[i],
                                merkle_tree_pubkey_index: 0,
                                nullifier_queue_pubkey_index: 0,
                            },
                        },
                    });
                }
                Err(_) => {
                    self.compressed_accounts
                        .push(PackedCompressedAccountWithMerkleContext {
                            compressed_account: compressed_account.clone(),
                            merkle_context: PackedMerkleContext {
                                leaf_index: event.output_leaf_indices[i],
                                merkle_tree_pubkey_index: 0,
                                nullifier_queue_pubkey_index: 0,
                            },
                        });
                }
            };
            self.merkle_tree
                .append(
                    &compressed_account
                        .hash::<Poseidon>(&self.merkle_tree_pubkey, &event.output_leaf_indices[i])
                        .unwrap(),
                )
                .expect("insert failed");
        }

        self.events.push(event);
    }

    /// deserializes an event
    /// adds the output_compressed_accounts to the compressed_accounts
    /// removes the input_compressed_accounts from the compressed_accounts
    /// adds the input_compressed_accounts to the nullified_compressed_accounts
    /// deserializes token data from the output_compressed_accounts
    /// adds the token_compressed_accounts to the token_compressed_accounts
    pub fn add_compressed_accounts_with_token_data(&mut self, event: PublicTransactionEvent) {
        self.add_event_and_compressed_accounts(event);
    }

    /// Check compressed_accounts in the queue array which are not nullified yet
    /// Iterate over these compressed_accounts and nullify them
    pub async fn nullify_compressed_accounts(&mut self, context: &mut ProgramTestContext) {
        let nullifier_queue = unsafe {
            get_hash_set::<u16, NullifierQueueAccount>(context, self.nullifier_queue_pubkey).await
        };
        let merkle_tree_account =
            AccountZeroCopy::<StateMerkleTreeAccount>::new(context, self.merkle_tree_pubkey).await;
        let merkle_tree = merkle_tree_account
            .deserialized()
            .copy_merkle_tree()
            .unwrap();
        let change_log_index = merkle_tree.current_changelog_index as u64;

        let mut compressed_account_to_nullify = Vec::new();

        for (i, element) in nullifier_queue.iter() {
            if element.sequence_number().is_none() {
                compressed_account_to_nullify.push((i, element.value_bytes()));
            }
        }

        for (index_in_nullifier_queue, compressed_account) in compressed_account_to_nullify.iter() {
            let leaf_index = self.merkle_tree.get_leaf_index(compressed_account).unwrap();
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
                    vec![(*index_in_nullifier_queue) as u16].as_slice(),
                    vec![0u64].as_slice(),
                    vec![proof].as_slice(),
                    &context.payer.pubkey(),
                    &self.merkle_tree_pubkey,
                    &self.nullifier_queue_pubkey,
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

            let nullifier_queue = unsafe {
                get_hash_set::<u16, NullifierQueueAccount>(context, self.nullifier_queue_pubkey)
                    .await
            };
            let array_element = nullifier_queue
                .by_value_index(*index_in_nullifier_queue, Some(merkle_tree.sequence_number))
                .unwrap();
            assert_eq!(&array_element.value_bytes(), compressed_account);
            let merkle_tree_account =
                AccountZeroCopy::<StateMerkleTreeAccount>::new(context, self.merkle_tree_pubkey)
                    .await;
            assert_eq!(
                array_element.sequence_number(),
                Some(
                    merkle_tree_account
                        .deserialized()
                        .load_merkle_tree()
                        .unwrap()
                        .sequence_number
                        + account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS as usize
                )
            );
        }
    }
}
