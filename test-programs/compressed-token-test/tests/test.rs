#![cfg(feature = "test-sbf")]

use anchor_lang::{
    system_program, AnchorDeserialize, AnchorSerialize, InstructionData, ToAccountMetas,
};
use anchor_spl::token::Mint;
use light_compressed_token::mint_sdk::create_create_token_pool_instruction;
use light_compressed_token::mint_sdk::create_mint_to_instruction;
use light_compressed_token::token_data::AccountState;
use light_test_utils::rpc::errors::assert_rpc_error;
use light_test_utils::spl::approve_test;
use light_test_utils::spl::burn_test;
use light_test_utils::spl::create_burn_test_instruction;
use light_test_utils::spl::freeze_test;
use light_test_utils::spl::mint_tokens_helper_with_lamports;
use light_test_utils::spl::mint_wrapped_sol;
use light_test_utils::spl::revoke_test;
use light_test_utils::spl::thaw_test;
use light_test_utils::spl::BurnInstructionMode;
use solana_sdk::{
    instruction::{Instruction, InstructionError},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::{Transaction, TransactionError},
};

use light_compressed_token::delegation::sdk::{
    create_approve_instruction, create_revoke_instruction, CreateApproveInstructionInputs,
    CreateRevokeInstructionInputs,
};
use light_compressed_token::freeze::sdk::{create_instruction, CreateInstructionInputs};
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
    airdrop_lamports, assert_custom_error_or_program_error, create_account_instruction,
    indexer::TestIndexer, test_env::setup_test_programs_with_accounts,
};
use light_verifier::VerifierError;
use rand::Rng;
use spl_token::{error::TokenError, instruction::initialize_mint};

#[tokio::test]
async fn test_create_mint() {
    let (mut rpc, _) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    create_mint_helper(&mut rpc, &payer).await;
}

#[tokio::test]
async fn test_failing_create_token_pool() {
    let (mut rpc, _) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();

    let rent = rpc
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await
        .unwrap();

    let mint_1_keypair = Keypair::new();
    let mint_1_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        Mint::LEN,
        rent,
        &spl_token::ID,
        Some(&mint_1_keypair),
    );
    let create_mint_1_ix = initialize_mint(
        &spl_token::ID,
        &mint_1_keypair.pubkey(),
        &payer.pubkey(),
        Some(&payer.pubkey()),
        2,
    )
    .unwrap();
    rpc.create_and_send_transaction(
        &[mint_1_account_create_ix, create_mint_1_ix],
        &payer.pubkey(),
        &[&payer, &mint_1_keypair],
    )
    .await
    .unwrap();
    let mint_1_pool_pda = get_token_pool_pda(&mint_1_keypair.pubkey());

    let mint_2_keypair = Keypair::new();
    let mint_2_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        Mint::LEN,
        rent,
        &spl_token::ID,
        Some(&mint_2_keypair),
    );
    let create_mint_2_ix = initialize_mint(
        &spl_token::ID,
        &mint_2_keypair.pubkey(),
        &payer.pubkey(),
        Some(&payer.pubkey()),
        2,
    )
    .unwrap();
    rpc.create_and_send_transaction(
        &[mint_2_account_create_ix, create_mint_2_ix],
        &payer.pubkey(),
        &[&payer, &mint_2_keypair],
    )
    .await
    .unwrap();
    let mint_2_pool_pda = get_token_pool_pda(&mint_2_keypair.pubkey());

    // Try to create pool for `mint_1` while using seeds of `mint_2` for PDAs.
    {
        let instruction_data = light_compressed_token::instruction::CreateTokenPool {};
        let accounts = light_compressed_token::accounts::CreateTokenPoolInstruction {
            fee_payer: payer.pubkey(),
            token_pool_pda: mint_2_pool_pda,
            system_program: system_program::ID,
            mint: mint_1_keypair.pubkey(),
            token_program: anchor_spl::token::ID,
            cpi_authority_pda: get_cpi_authority_pda().0,
        };
        let instruction = Instruction {
            program_id: light_compressed_token::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::ConstraintSeeds.into(),
        )
        .unwrap();
    }
    // Try to create pool for `mint_2` while using seeds of `mint_1` for PDAs.
    {
        let instruction_data = light_compressed_token::instruction::CreateTokenPool {};
        let accounts = light_compressed_token::accounts::CreateTokenPoolInstruction {
            fee_payer: payer.pubkey(),
            token_pool_pda: mint_1_pool_pda,
            system_program: system_program::ID,
            mint: mint_2_keypair.pubkey(),
            token_program: anchor_spl::token::ID,
            cpi_authority_pda: get_cpi_authority_pda().0,
        };
        let instruction = Instruction {
            program_id: light_compressed_token::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::ConstraintSeeds.into(),
        )
        .unwrap();
    }
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

async fn test_mint_to(amounts: Vec<u64>, iterations: usize, lamports: Option<u64>) {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer = rpc.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let mut test_indexer =
        TestIndexer::<ProgramTestRpcConnection>::init_from_env(&payer, &env, false, false).await;

    let recipients = amounts
        .iter()
        .map(|_| Keypair::new().pubkey())
        .collect::<Vec<_>>();
    let mint = create_mint_helper(&mut rpc, &payer).await;

    for _ in 0..iterations {
        mint_tokens_helper_with_lamports(
            &mut rpc,
            &mut test_indexer,
            &merkle_tree_pubkey,
            &payer,
            &mint,
            amounts.clone(),
            recipients.clone(),
            lamports,
        )
        .await;
    }
    kill_prover();
}

#[tokio::test]
async fn test_1_mint_to() {
    test_mint_to(vec![10000], 1, Some(1_000_000)).await
}

#[tokio::test]
async fn test_1_max_mint_to() {
    test_mint_to(vec![u64::MAX], 1, Some(1_000_000)).await
}

#[tokio::test]
async fn test_5_mint_to() {
    test_mint_to(vec![0, 10000, 10000, 10000, 10000], 1, Some(1_000_000)).await
}

#[tokio::test]
async fn test_10_mint_to() {
    let mut rng = rand::thread_rng();
    // Make sure that the tokal token supply does not exceed `u64::MAX`.
    let amounts: Vec<u64> = (0..10).map(|_| rng.gen_range(0..(u64::MAX / 10))).collect();
    test_mint_to(amounts, 1, Some(1_000_000)).await
}

#[tokio::test]
async fn test_20_mint_to() {
    let mut rng = rand::thread_rng();
    // Make sure that the total token supply does not exceed `u64::MAX`.
    let amounts: Vec<u64> = (0..20).map(|_| rng.gen_range(0..(u64::MAX / 20))).collect();
    test_mint_to(amounts, 1, Some(1_000_000)).await
}

#[tokio::test]
async fn test_25_mint_to() {
    let mut rng = rand::thread_rng();
    // Make sure that the total token supply does not exceed `u64::MAX`.
    let amounts: Vec<u64> = (0..25)
        .map(|_| rng.gen_range(0..(u64::MAX / (25 * 10))))
        .collect();
    test_mint_to(amounts, 10, Some(1_000_000)).await
}

#[tokio::test]
async fn test_25_mint_to_zeros() {
    let amounts = vec![0; 25];
    test_mint_to(amounts, 1, Some(1_000_000)).await
}

/// Failing tests:
/// 1. Try to mint token from `mint_1` and sign the transaction with `mint_2`
///    authority.
/// 2. Try to mint token from `mint_2` and sign the transaction with `mint_1`
///    authority.
/// 3. Try to mint token from `mint_1` while using `mint_2` pool.
/// 4. Try to mint token from `mint_2` while using `mint_1` pool.
/// 5. Invalid CPI authority.
/// 6. Invalid registered program.
/// 7. Invalid noop program.
/// 8. Invalid account compression authority.
/// 9. Invalid Merkle tree.
/// 10. Mint more than `u64::MAX` tokens.
/// 11. Multiple mints which overflow the token supply over `u64::MAX`.
#[tokio::test]
async fn test_mint_to_failing() {
    const MINTS: usize = 10;

    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let payer_1 = rpc.get_payer().insecure_clone();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;

    let mut rng = rand::thread_rng();

    let payer_2 = Keypair::new();
    airdrop_lamports(&mut rpc, &payer_2.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let mint_1 = create_mint_helper(&mut rpc, &payer_1).await;
    let mint_pool_1 = get_token_pool_pda(&mint_1);

    let mint_2 = create_mint_helper(&mut rpc, &payer_2).await;

    // Make sure that the tokal token supply does not exceed `u64::MAX`.
    let amounts: Vec<u64> = (0..MINTS)
        .map(|_| rng.gen_range(0..(u64::MAX / MINTS as u64)))
        .collect();
    let recipients = amounts
        .iter()
        .map(|_| Keypair::new().pubkey())
        .collect::<Vec<_>>();

    let instruction_data = light_compressed_token::instruction::MintTo {
        amounts: amounts.clone(),
        public_keys: recipients.clone(),
        lamports: None,
    };

    // 1. Try to mint token from `mint_1` and sign the transaction with `mint_2`
    //    authority.
    {
        let instruction = create_mint_to_instruction(
            &payer_2.pubkey(),
            &payer_2.pubkey(),
            &mint_1,
            &merkle_tree_pubkey,
            amounts.clone(),
            recipients.clone(),
            None,
        );
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer_2.pubkey(), &[&payer_2])
            .await;
        assert_rpc_error(
            result,
            0,
            light_compressed_token::ErrorCode::InvalidAuthorityMint.into(),
        )
        .unwrap();
    }
    // 2. Try to mint token from `mint_2` and sign the transaction with `mint_1`
    //    authority.
    {
        let instruction = create_mint_to_instruction(
            &payer_1.pubkey(),
            &payer_1.pubkey(),
            &mint_2,
            &merkle_tree_pubkey,
            amounts.clone(),
            recipients.clone(),
            None,
        );
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer_1.pubkey(), &[&payer_1])
            .await;
        assert_rpc_error(
            result,
            0,
            light_compressed_token::ErrorCode::InvalidAuthorityMint.into(),
        )
        .unwrap();
    }
    // 3. Try to mint token to random token account.
    {
        let token_account_keypair = Keypair::new();
        create_token_account(&mut rpc, &mint_1, &token_account_keypair, &payer_1)
            .await
            .unwrap();
        let accounts = light_compressed_token::accounts::MintToInstruction {
            fee_payer: payer_1.pubkey(),
            authority: payer_1.pubkey(),
            cpi_authority_pda: get_cpi_authority_pda().0,
            mint: mint_1.clone(),
            token_pool_pda: token_account_keypair.pubkey(),
            token_program: anchor_spl::token::ID,
            light_system_program: light_system_program::ID,
            registered_program_pda: light_system_program::utils::get_registered_program_pda(
                &light_system_program::ID,
            ),
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_authority: light_system_program::utils::get_cpi_authority_pda(
                &light_system_program::ID,
            ),
            account_compression_program: account_compression::ID,
            merkle_tree: merkle_tree_pubkey,
            self_program: light_compressed_token::ID,
            system_program: system_program::ID,
            sol_pool_pda: None,
        };
        let instruction = Instruction {
            program_id: light_compressed_token::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer_1.pubkey(), &[&payer_1])
            .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::ConstraintSeeds.into(),
        )
        .unwrap();
    }
    // 4. Try to mint token from `mint_2` while using `mint_1` pool.
    {
        let accounts = light_compressed_token::accounts::MintToInstruction {
            fee_payer: payer_2.pubkey(),
            authority: payer_2.pubkey(),
            cpi_authority_pda: get_cpi_authority_pda().0,
            mint: mint_2.clone(),
            token_pool_pda: mint_pool_1,
            token_program: anchor_spl::token::ID,
            light_system_program: light_system_program::ID,
            registered_program_pda: light_system_program::utils::get_registered_program_pda(
                &light_system_program::ID,
            ),
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_authority: light_system_program::utils::get_cpi_authority_pda(
                &light_system_program::ID,
            ),
            account_compression_program: account_compression::ID,
            merkle_tree: merkle_tree_pubkey,
            self_program: light_compressed_token::ID,
            system_program: system_program::ID,
            sol_pool_pda: None,
        };
        let instruction = Instruction {
            program_id: light_compressed_token::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer_2.pubkey(), &[&payer_2])
            .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::ConstraintSeeds.into(),
        )
        .unwrap();
    }
    // 5. Invalid CPI authority.
    {
        let invalid_cpi_authority_pda = Keypair::new();
        let accounts = light_compressed_token::accounts::MintToInstruction {
            fee_payer: payer_2.pubkey(),
            authority: payer_2.pubkey(),
            cpi_authority_pda: invalid_cpi_authority_pda.pubkey(),
            mint: mint_1.clone(),
            token_pool_pda: mint_pool_1,
            token_program: anchor_spl::token::ID,
            light_system_program: light_system_program::ID,
            registered_program_pda: light_system_program::utils::get_registered_program_pda(
                &light_system_program::ID,
            ),
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_authority: light_system_program::utils::get_cpi_authority_pda(
                &light_system_program::ID,
            ),
            account_compression_program: account_compression::ID,
            merkle_tree: merkle_tree_pubkey,
            self_program: light_compressed_token::ID,
            system_program: system_program::ID,
            sol_pool_pda: None,
        };
        let instruction = Instruction {
            program_id: light_compressed_token::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer_2.pubkey(), &[&payer_2])
            .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::ConstraintSeeds.into(),
        )
        .unwrap();
    }
    // 6. Invalid registered program.
    {
        let invalid_registered_program = Keypair::new();
        let accounts = light_compressed_token::accounts::MintToInstruction {
            fee_payer: payer_1.pubkey(),
            authority: payer_1.pubkey(),
            cpi_authority_pda: get_cpi_authority_pda().0,
            mint: mint_1.clone(),
            token_pool_pda: mint_pool_1,
            token_program: anchor_spl::token::ID,
            light_system_program: light_system_program::ID,
            registered_program_pda: invalid_registered_program.pubkey(),
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_authority: light_system_program::utils::get_cpi_authority_pda(
                &light_system_program::ID,
            ),
            account_compression_program: account_compression::ID,
            merkle_tree: merkle_tree_pubkey,
            self_program: light_compressed_token::ID,
            system_program: system_program::ID,
            sol_pool_pda: None,
        };
        let instruction = Instruction {
            program_id: light_compressed_token::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer_1.pubkey(), &[&payer_1])
            .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::ConstraintSeeds.into(),
        )
        .unwrap();
    }
    // 7. Invalid noop program.
    {
        let invalid_noop_program = Keypair::new();
        let accounts = light_compressed_token::accounts::MintToInstruction {
            fee_payer: payer_1.pubkey(),
            authority: payer_1.pubkey(),
            cpi_authority_pda: get_cpi_authority_pda().0,
            mint: mint_1.clone(),
            token_pool_pda: mint_pool_1,
            token_program: anchor_spl::token::ID,
            light_system_program: light_system_program::ID,
            registered_program_pda: light_system_program::utils::get_registered_program_pda(
                &light_system_program::ID,
            ),
            noop_program: invalid_noop_program.pubkey(),
            account_compression_authority: light_system_program::utils::get_cpi_authority_pda(
                &light_system_program::ID,
            ),
            account_compression_program: account_compression::ID,
            merkle_tree: merkle_tree_pubkey,
            self_program: light_compressed_token::ID,
            system_program: system_program::ID,
            sol_pool_pda: None,
        };
        let instruction = Instruction {
            program_id: light_compressed_token::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer_1.pubkey(), &[&payer_1])
            .await;
        assert_rpc_error(
            result,
            0,
            light_system_program::errors::SystemProgramError::InvalidNoopPubkey.into(),
        )
        .unwrap();
    }
    // 8. Invalid account compression authority.
    {
        let invalid_account_compression_authority = Keypair::new();
        let accounts = light_compressed_token::accounts::MintToInstruction {
            fee_payer: payer_1.pubkey(),
            authority: payer_1.pubkey(),
            cpi_authority_pda: get_cpi_authority_pda().0,
            mint: mint_1.clone(),
            token_pool_pda: mint_pool_1,
            token_program: anchor_spl::token::ID,
            light_system_program: light_system_program::ID,
            registered_program_pda: light_system_program::utils::get_registered_program_pda(
                &light_system_program::ID,
            ),
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_authority: invalid_account_compression_authority.pubkey(),
            account_compression_program: account_compression::ID,
            merkle_tree: merkle_tree_pubkey,
            self_program: light_compressed_token::ID,
            system_program: system_program::ID,
            sol_pool_pda: None,
        };
        let instruction = Instruction {
            program_id: light_compressed_token::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        };
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer_1.pubkey(), &[&payer_1])
            .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::ConstraintSeeds.into(),
        )
        .unwrap();
    }
    // 9. Invalid Merkle tree.
    {
        let invalid_merkle_tree = Keypair::new();
        let instruction = create_mint_to_instruction(
            &payer_1.pubkey(),
            &payer_1.pubkey(),
            &mint_1,
            &invalid_merkle_tree.pubkey(),
            amounts.clone(),
            recipients.clone(),
            None,
        );
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer_1.pubkey(), &[&payer_1])
            .await;
        assert!(matches!(
            result,
            Err(RpcError::TransactionError(
                TransactionError::InstructionError(0, InstructionError::ProgramFailedToComplete)
            ))
        ));
    }
    // 10. Mint more than `u64::MAX` tokens.
    {
        // Overall sum greater than `u64::MAX`
        let amounts = vec![u64::MAX / 5; MINTS];
        let instruction = create_mint_to_instruction(
            &payer_1.pubkey(),
            &payer_1.pubkey(),
            &mint_1,
            &merkle_tree_pubkey,
            amounts,
            recipients.clone(),
            None,
        );
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer_1.pubkey(), &[&payer_1])
            .await;
        assert_rpc_error(
            result,
            0,
            light_compressed_token::ErrorCode::MintTooLarge.into(),
        )
        .unwrap();
    }
    // 11. Multiple mints which overflow the token supply over `u64::MAX`.
    {
        let amounts = vec![u64::MAX / 10; MINTS];
        let instruction = create_mint_to_instruction(
            &payer_1.pubkey(),
            &payer_1.pubkey(),
            &mint_1,
            &merkle_tree_pubkey,
            amounts,
            recipients.clone(),
            None,
        );
        // The first mint is still below `u64::MAX`.
        rpc.create_and_send_transaction(&[instruction.clone()], &payer_1.pubkey(), &[&payer_1])
            .await
            .unwrap();
        // The second mint should overflow.
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer_1.pubkey(), &[&payer_1])
            .await;
        assert_rpc_error(result, 0, TokenError::Overflow as u32).unwrap();
    }
}

#[tokio::test]
async fn test_transfers() {
    let possible_inputs = [1, 2, 3, 4, 8];
    for input_num in possible_inputs {
        for output_num in 1..8 {
            if input_num == 8 && output_num > 5 {
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
            if input_num == 8 && output_num > 5 {
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
            if input_num == 8 && output_num > 5 {
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
        let output_num = 5;
        println!(
            "\n\ninput num: {}, output num: {}\n\n",
            input_num, output_num
        );
        perform_transfer_test(input_num, output_num, 10_000).await
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
    mint_tokens_helper_with_lamports(
        &mut rpc,
        &mut test_indexer,
        &merkle_tree_pubkey,
        &payer,
        &mint,
        vec![amount; inputs],
        vec![sender.pubkey(); inputs],
        Some(1_000_000),
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
        None,
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
    num_inputs: usize,
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
    mint_tokens_helper_with_lamports(
        &mut rpc,
        &mut test_indexer,
        &merkle_tree_pubkey,
        &payer,
        &mint,
        vec![mint_amount; num_inputs],
        vec![sender.pubkey(); num_inputs],
        Some(1_000_000),
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
            Some(100),
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
            Some(vec![Some(90), Some(10)]),
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
            None,
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
    let num_inputs = 1;
    test_delegation(0, num_inputs, 0, vec![0, 0], vec![0]).await
}

#[tokio::test]
async fn test_delegation_10000() {
    let num_inputs = 1;
    test_delegation(10000, num_inputs, 1000, vec![900, 100], vec![100]).await
}
#[tokio::test]
async fn test_delegation_8_inputs() {
    let num_inputs = 8;
    test_delegation(10000, num_inputs, 1000, vec![900, 100], vec![100]).await
}

#[tokio::test]
async fn test_delegation_max() {
    let num_inputs = 1;
    test_delegation(
        u64::MAX,
        num_inputs,
        u64::MAX,
        vec![u64::MAX - 100, 100],
        vec![100],
    )
    .await
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
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            mint,
            delegated_amount,
            delegate_lamports: None,
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
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            mint,
            delegated_amount,
            delegate_lamports: None,
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
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            mint,
            delegated_amount,
            delegate_lamports: None,
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
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            mint: invalid_mint.pubkey(),
            delegated_amount,
            delegate_lamports: None,
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
    // 5. Invalid delegate amount (too high)
    {
        let sum_inputs = input_compressed_accounts
            .iter()
            .map(|x| x.token_data.amount)
            .sum::<u64>();
        let delegated_amount = sum_inputs + 1;
        let inputs = CreateApproveInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: sender.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            mint,
            delegated_amount,
            delegate_lamports: None,
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
        assert_rpc_error(result, 0, ErrorCode::ArithmeticUnderflow.into()).unwrap();
    }
}

/// Test revoke:
/// 1. Delegate tokens with approve
/// 2. Revoke
async fn test_revoke(num_inputs: usize, mint_amount: u64, delegated_amount: u64) {
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
    mint_tokens_helper_with_lamports(
        &mut rpc,
        &mut test_indexer,
        &merkle_tree_pubkey,
        &payer,
        &mint,
        vec![mint_amount; num_inputs],
        vec![sender.pubkey(); num_inputs],
        Some(1_000_000),
    )
    .await;
    // 1. Delegate tokens
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        for input in input_compressed_accounts.iter() {
            let input_compressed_accounts = vec![input.clone()];
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
                Some(1000),
                &delegate.pubkey(),
                &delegated_compressed_account_merkle_tree,
                &delegated_compressed_account_merkle_tree,
                None,
            )
            .await;
        }
    }
    // 2. Revoke
    {
        let input_compressed_accounts = test_indexer
            .get_compressed_token_accounts_by_owner(&sender.pubkey())
            .iter()
            .filter(|x| x.token_data.delegate.is_some())
            .cloned()
            .collect::<Vec<TokenDataWithContext>>();
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

#[tokio::test]
async fn test_revoke_0() {
    let num_inputs = 1;
    test_revoke(num_inputs, 0, 0).await
}

#[tokio::test]
async fn test_revoke_10000() {
    let num_inputs = 1;
    test_revoke(num_inputs, 10000, 1000).await
}

#[tokio::test]
async fn test_revoke_8_inputs() {
    let num_inputs = 8;
    test_revoke(num_inputs, 10000, 1000).await
}
#[tokio::test]
async fn test_revoke_max() {
    let num_inputs = 1;
    test_revoke(num_inputs, u64::MAX, u64::MAX).await
}

/// Failing tests:
/// 1. Invalid root indices.
/// 2. Invalid Merkle tree.
/// 3. Invalid mint.
#[tokio::test]
async fn test_revoke_failing() {
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
    // Delegate tokens
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
            None,
            &delegate.pubkey(),
            &delegated_compressed_account_merkle_tree,
            &delegated_compressed_account_merkle_tree,
            None,
        )
        .await;
    }

    let input_compressed_accounts =
        test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
    let input_compressed_accounts = input_compressed_accounts
        .iter()
        .filter(|x| x.token_data.delegate.is_some())
        .cloned()
        .collect::<Vec<TokenDataWithContext>>();

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

    // 1. Invalid root indices.
    {
        let invalid_root_indices = vec![0];

        let inputs = CreateRevokeInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: sender.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            mint,
            output_account_merkle_tree: merkle_tree_pubkey,
            root_indices: invalid_root_indices,
            proof: proof_rpc_result.proof.clone(),
        };
        let instruction = create_revoke_instruction(inputs).unwrap();
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
    // 2. Invalid Merkle tree.
    {
        let invalid_merkle_tree = Keypair::new();

        let inputs = CreateRevokeInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: sender.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            mint,
            output_account_merkle_tree: invalid_merkle_tree.pubkey(),
            root_indices: proof_rpc_result.root_indices.clone(),
            proof: proof_rpc_result.proof.clone(),
        };
        let instruction = create_revoke_instruction(inputs).unwrap();
        let context_payer = rpc.get_payer().insecure_clone();

        let result = rpc
            .create_and_send_transaction(
                &[instruction],
                &sender.pubkey(),
                &[&context_payer, &sender],
            )
            .await;
        // Anchor panics when trying to deserialize the account, the
        // instruction returns `ProgramFailedToComplete`. No specific error
        // to assert.
        assert!(matches!(
            result,
            Err(RpcError::TransactionError(
                TransactionError::InstructionError(0, InstructionError::ProgramFailedToComplete)
            ))
        ));
    }
    // 3. Invalid mint.
    {
        let invalid_mint = Keypair::new();

        let inputs = CreateRevokeInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: sender.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            mint: invalid_mint.pubkey(),
            output_account_merkle_tree: merkle_tree_pubkey,
            root_indices: proof_rpc_result.root_indices,
            proof: proof_rpc_result.proof,
        };
        let instruction = create_revoke_instruction(inputs).unwrap();
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
    mint_tokens_helper_with_lamports(
        &mut rpc,
        &mut test_indexer,
        &merkle_tree_pubkey,
        &payer,
        &mint,
        vec![amount],
        vec![sender.pubkey()],
        Some(1_000_000),
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
            None,
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

#[tokio::test]
async fn failing_tests_burn() {
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
    // Delegate tokens
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
            None,
            &delegate.pubkey(),
            &delegated_compressed_account_merkle_tree,
            &delegated_compressed_account_merkle_tree,
            None,
        )
        .await;
    }
    // 1. invalid proof
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let burn_amount = 1;
        let change_account_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;
        let (_, _, _, _, instruction) = create_burn_test_instruction(
            &sender,
            &mut rpc,
            &mut test_indexer,
            &input_compressed_accounts,
            &change_account_merkle_tree,
            burn_amount,
            false,
            BurnInstructionMode::InvalidProof,
        )
        .await;
        let res = rpc
            .create_and_send_transaction(&[instruction], &sender.pubkey(), &[&payer, &sender])
            .await;
        assert_rpc_error(res, 0, VerifierError::ProofVerificationFailed.into()).unwrap();
    }
    // 2. Signer is delegate but token data has no delegate.
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let burn_amount = 1;
        let change_account_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;
        let (_, _, _, _, instruction) = create_burn_test_instruction(
            &delegate,
            &mut rpc,
            &mut test_indexer,
            &input_compressed_accounts,
            &change_account_merkle_tree,
            burn_amount,
            true,
            BurnInstructionMode::Normal,
        )
        .await;
        let res = rpc
            .create_and_send_transaction(&[instruction], &delegate.pubkey(), &[&payer, &delegate])
            .await;
        assert_rpc_error(res, 0, ErrorCode::DelegateSignerCheckFailed.into()).unwrap();
    }
    // 3. Signer is delegate but token data has no delegate.
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let input_compressed_accounts = input_compressed_accounts
            .iter()
            .filter(|x| x.token_data.delegate.is_some())
            .cloned()
            .collect::<Vec<TokenDataWithContext>>();
        let burn_amount = 1;
        let change_account_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;
        let (_, _, _, _, instruction) = create_burn_test_instruction(
            &sender,
            &mut rpc,
            &mut test_indexer,
            &input_compressed_accounts,
            &change_account_merkle_tree,
            burn_amount,
            true,
            BurnInstructionMode::Normal,
        )
        .await;
        let res = rpc
            .create_and_send_transaction(&[instruction], &sender.pubkey(), &[&payer, &sender])
            .await;
        assert_rpc_error(res, 0, ErrorCode::DelegateSignerCheckFailed.into()).unwrap();
    }
    // 4. invalid authority (use delegate as authority)
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let burn_amount = 1;
        let change_account_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;
        let (_, _, _, _, instruction) = create_burn_test_instruction(
            &delegate,
            &mut rpc,
            &mut test_indexer,
            &input_compressed_accounts,
            &change_account_merkle_tree,
            burn_amount,
            false,
            BurnInstructionMode::Normal,
        )
        .await;
        let res = rpc
            .create_and_send_transaction(&[instruction], &delegate.pubkey(), &[&payer, &delegate])
            .await;
        assert_rpc_error(res, 0, VerifierError::ProofVerificationFailed.into()).unwrap();
    }
    // 5. invalid mint
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let burn_amount = 1;
        let change_account_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .merkle_tree_pubkey;
        let (_, _, _, _, instruction) = create_burn_test_instruction(
            &sender,
            &mut rpc,
            &mut test_indexer,
            &input_compressed_accounts,
            &change_account_merkle_tree,
            burn_amount,
            false,
            BurnInstructionMode::InvalidMint,
        )
        .await;
        let res = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &sender])
            .await;
        assert_rpc_error(
            res,
            0,
            anchor_lang::error::ErrorCode::AccountNotInitialized.into(),
        )
        .unwrap();
    }
    // 6. invalid change merkle tree
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let burn_amount = 1;
        let invalid_change_account_merkle_tree = input_compressed_accounts[0]
            .compressed_account
            .merkle_context
            .nullifier_queue_pubkey;
        let (_, _, _, _, instruction) = create_burn_test_instruction(
            &sender,
            &mut rpc,
            &mut test_indexer,
            &input_compressed_accounts,
            &invalid_change_account_merkle_tree,
            burn_amount,
            false,
            BurnInstructionMode::Normal,
        )
        .await;
        let res = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &sender])
            .await;
        assert_rpc_error(
            res,
            0,
            anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into(),
        )
        .unwrap();
    }
}

/// Test freeze and thaw:
/// 1. Freeze tokens
/// 2. Thaw tokens
/// 3. Delegate tokens
/// 4. Freeze delegated tokens
/// 5. Thaw delegated tokens
async fn test_freeze_and_thaw(mint_amount: u64, delegated_amount: u64) {
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
    mint_tokens_helper_with_lamports(
        &mut rpc,
        &mut test_indexer,
        &merkle_tree_pubkey,
        &payer,
        &mint,
        vec![mint_amount],
        vec![sender.pubkey()],
        Some(1_000_000),
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
            None,
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

#[tokio::test]
async fn test_freeze_and_thaw_0() {
    test_freeze_and_thaw(0, 0).await
}

#[tokio::test]
async fn test_freeze_and_thaw_10000() {
    test_freeze_and_thaw(10000, 1000).await
}

/// Failing tests:
/// 1. Invalid authority.
/// 2. Invalid Merkle tree.
/// 3. Invalid proof.
/// 4. Freeze frozen compressed account.
#[tokio::test]
async fn test_failing_freeze() {
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
        vec![amount; 3],
        vec![sender.pubkey(); 3],
    )
    .await;

    let input_compressed_accounts =
        vec![test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey())[0].clone()];
    let outputs_merkle_tree = input_compressed_accounts[0]
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
    let context_payer = rpc.get_payer().insecure_clone();

    // 1. Invalid authority.
    {
        let invalid_authority = Keypair::new();

        let inputs = CreateInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: invalid_authority.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            outputs_merkle_tree,
            root_indices: proof_rpc_result.root_indices.clone(),
            proof: proof_rpc_result.proof.clone(),
        };
        let instruction = create_instruction::<true>(inputs).unwrap();
        let result = rpc
            .create_and_send_transaction(
                &[instruction],
                &payer.pubkey(),
                &[&context_payer, &invalid_authority],
            )
            .await;
        assert_rpc_error(
            result,
            0,
            light_compressed_token::ErrorCode::InvalidFreezeAuthority.into(),
        )
        .unwrap();
    }
    // 2. Invalid Merkle tree.
    {
        let invalid_merkle_tree = Keypair::new();

        let inputs = CreateInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: payer.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            outputs_merkle_tree: invalid_merkle_tree.pubkey(),
            root_indices: proof_rpc_result.root_indices.clone(),
            proof: proof_rpc_result.proof.clone(),
        };
        let instruction = create_instruction::<true>(inputs).unwrap();
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&context_payer, &payer])
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

        let inputs = CreateInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: payer.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            outputs_merkle_tree,
            root_indices: proof_rpc_result.root_indices.clone(),
            proof: invalid_proof,
        };
        let instruction = create_instruction::<true>(inputs).unwrap();
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&context_payer, &payer])
            .await;
        assert_rpc_error(result, 0, VerifierError::ProofVerificationFailed.into()).unwrap();
    }
    // 4. Freeze frozen compressed account
    {
        freeze_test(
            &payer,
            &mut rpc,
            &mut test_indexer,
            input_compressed_accounts,
            &outputs_merkle_tree,
            None,
        )
        .await;
        let input_compressed_accounts = vec![test_indexer
            .get_compressed_token_accounts_by_owner(&sender.pubkey())
            .iter()
            .filter(|x| x.token_data.state == AccountState::Frozen)
            .cloned()
            .collect::<Vec<TokenDataWithContext>>()[0]
            .clone()];
        let outputs_merkle_tree = input_compressed_accounts[0]
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
        let inputs = CreateInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: payer.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            outputs_merkle_tree,
            root_indices: proof_rpc_result.root_indices.clone(),
            proof: proof_rpc_result.proof.clone(),
        };
        let instruction = create_instruction::<true>(inputs).unwrap();
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&context_payer, &payer])
            .await;
        assert_rpc_error(result, 0, VerifierError::ProofVerificationFailed.into()).unwrap();
    }
}

/// Failing tests:
/// 1. Invalid authority.
/// 2. Invalid Merkle tree.
/// 3. Invalid proof.
/// 4. thaw compressed account which is not frozen
#[tokio::test]
async fn test_failing_thaw() {
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
        vec![amount; 2],
        vec![sender.pubkey(); 2],
    )
    .await;

    // Freeze tokens
    {
        let input_compressed_accounts =
            vec![test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey())[0].clone()];
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

    let input_compressed_accounts =
        test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
    let input_compressed_accounts = input_compressed_accounts
        .iter()
        .filter(|x| x.token_data.state == AccountState::Frozen)
        .cloned()
        .collect::<Vec<TokenDataWithContext>>();
    let outputs_merkle_tree = input_compressed_accounts[0]
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
    let context_payer = rpc.get_payer().insecure_clone();

    // 1. Invalid authority.
    {
        let invalid_authority = Keypair::new();

        let inputs = CreateInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: invalid_authority.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            outputs_merkle_tree,
            root_indices: proof_rpc_result.root_indices.clone(),
            proof: proof_rpc_result.proof.clone(),
        };
        let instruction = create_instruction::<false>(inputs).unwrap();
        let result = rpc
            .create_and_send_transaction(
                &[instruction],
                &payer.pubkey(),
                &[&context_payer, &invalid_authority],
            )
            .await;
        assert_rpc_error(
            result,
            0,
            light_compressed_token::ErrorCode::InvalidFreezeAuthority.into(),
        )
        .unwrap();
    }
    // 2. Invalid Merkle tree.
    {
        let invalid_merkle_tree = Keypair::new();

        let inputs = CreateInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: payer.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            outputs_merkle_tree: invalid_merkle_tree.pubkey(),
            root_indices: proof_rpc_result.root_indices.clone(),
            proof: proof_rpc_result.proof.clone(),
        };
        let instruction = create_instruction::<false>(inputs).unwrap();
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&context_payer, &payer])
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

        let inputs = CreateInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: payer.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            outputs_merkle_tree,
            root_indices: proof_rpc_result.root_indices.clone(),
            proof: invalid_proof,
        };
        let instruction = create_instruction::<false>(inputs).unwrap();
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&context_payer, &payer])
            .await;
        assert_rpc_error(result, 0, VerifierError::ProofVerificationFailed.into()).unwrap();
    }
    // 4. thaw compressed account which is not frozen
    {
        let input_compressed_accounts =
            test_indexer.get_compressed_token_accounts_by_owner(&sender.pubkey());
        let input_compressed_accounts = input_compressed_accounts
            .iter()
            .filter(|x| x.token_data.state == AccountState::Initialized)
            .cloned()
            .collect::<Vec<TokenDataWithContext>>();
        let outputs_merkle_tree = input_compressed_accounts[0]
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
        let inputs = CreateInstructionInputs {
            fee_payer: rpc.get_payer().pubkey(),
            authority: payer.pubkey(),
            input_merkle_contexts: input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.merkle_context)
                .collect(),
            input_token_data: input_compressed_accounts
                .iter()
                .map(|x| x.token_data.clone())
                .collect(),
            input_compressed_accounts: input_compressed_accounts
                .iter()
                .map(|x| &x.compressed_account.compressed_account)
                .cloned()
                .collect::<Vec<_>>(),
            outputs_merkle_tree,
            root_indices: proof_rpc_result.root_indices.clone(),
            proof: proof_rpc_result.proof.clone(),
        };
        let instruction = create_instruction::<false>(inputs).unwrap();
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&context_payer, &payer])
            .await;
        assert_rpc_error(result, 0, VerifierError::ProofVerificationFailed.into()).unwrap();
    }
}

/// Failing tests:
/// 1. Invalid decompress account
/// 2. Invalid token pool pda
/// 3. Invalid decompression amount -1
/// 4. Invalid decompression amount +1
/// 5. Invalid decompression amount 0
/// 6: invalid token recipient
/// 7. Invalid compression amount -1
/// 8. Invalid compression amount +1
/// 9. Invalid compression amount 0
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
            decompress_amount, // needs to be consistent with compression amount
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
    // Test 2: invalid token pool pda (compress and decompress)
    {
        let invalid_token_account_keypair = Keypair::new();
        create_token_account(&mut context, &mint, &invalid_token_account_keypair, &payer)
            .await
            .unwrap();
        failing_compress_decompress(
            &sender,
            &mut context,
            &mut test_indexer,
            input_compressed_account.clone(),
            decompress_amount, // needs to be consistent with compression amount
            &merkle_tree_pubkey,
            decompress_amount,
            false,
            &token_account_keypair.pubkey(),
            Some(invalid_token_account_keypair.pubkey()),
            &mint,
            ErrorCode::InvalidTokenPoolPda.into(),
        )
        .await
        .unwrap();

        let invalid_token_account_keypair = Keypair::new();
        create_token_account(&mut context, &mint, &invalid_token_account_keypair, &payer)
            .await
            .unwrap();
        failing_compress_decompress(
            &sender,
            &mut context,
            &mut test_indexer,
            input_compressed_account.clone(),
            0, // needs to be consistent with compression amount
            &merkle_tree_pubkey,
            0,
            true,
            &token_account_keypair.pubkey(),
            Some(invalid_token_account_keypair.pubkey()),
            &mint,
            ErrorCode::InvalidTokenPoolPda.into(),
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
            decompress_amount, // needs to be consistent with compression amount
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
            decompress_amount, // needs to be consistent with compression amount
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
            decompress_amount, // needs to be consistent with compression amount
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
    // Test 6: invalid token recipient
    {
        failing_compress_decompress(
            &sender,
            &mut context,
            &mut test_indexer,
            input_compressed_account.clone(),
            decompress_amount, // needs to be consistent with compression amount
            &merkle_tree_pubkey,
            decompress_amount,
            false,
            &get_token_pool_pda(&mint),
            Some(get_token_pool_pda(&mint)),
            &mint,
            light_compressed_token::ErrorCode::IsTokenPoolPda.into(),
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
    // Test 7: invalid compression amount -1
    {
        failing_compress_decompress(
            &sender,
            &mut context,
            &mut test_indexer,
            Vec::new(),
            compress_amount, // needs to be consistent with compression amount
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
    // Test 8: invalid compression amount +1
    {
        failing_compress_decompress(
            &sender,
            &mut context,
            &mut test_indexer,
            Vec::new(),
            compress_amount, // needs to be consistent with compression amount
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
    // Test 9: invalid compression amount 0
    {
        failing_compress_decompress(
            &sender,
            &mut context,
            &mut test_indexer,
            Vec::new(),
            compress_amount, // needs to be consistent with compression amount
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
            .map(|x| x.token_data.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        &input_compressed_accounts
            .iter()
            .map(|x| &x.compressed_account.compressed_account)
            .cloned()
            .collect::<Vec<_>>(),
        *mint,
        None,
        is_compress,
        Some(compression_amount),
        token_pool_pda,
        Some(*compress_or_decompress_token_account),
        true,
        None,
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
    let input_compressed_account_token_data =
        test_indexer.token_compressed_accounts[0].token_data.clone();
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
            test_indexer.token_compressed_accounts[0].token_data.clone();
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
            test_indexer.token_compressed_accounts[0].token_data.clone();
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
        &input_compressed_accounts
            .iter()
            .map(|x| &x.compressed_account)
            .cloned()
            .collect::<Vec<_>>(),
        mint,
        None,
        false,
        None,
        None,
        None,
        true,
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
    rpc.process_transaction(transaction).await
}
