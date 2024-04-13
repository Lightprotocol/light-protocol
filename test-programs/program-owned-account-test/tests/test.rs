#![cfg(feature = "test-sbf")]

use account_compression::Pubkey;
use anchor_lang::AnchorDeserialize;
use light_compressed_pda::compressed_account::CompressedAccountWithMerkleContext;
use light_hasher::{Hasher, Poseidon};
use light_test_utils::create_and_send_transaction_with_event;
use light_test_utils::test_env::{setup_test_programs_with_accounts, EnvAccounts};
use light_test_utils::test_indexer::{create_mint_helper, mint_tokens_helper, TestIndexer};
use light_utils::hash_to_bn254_field_size_le;
use program_owned_account_test::sdk::{
    create_invalidate_not_owned_account_instruction, create_pda_instruction,
    CreateCompressedPdaInstructionInputs, InvalidateNotOwnedCompressedAccountInstructionInputs,
};
use program_owned_account_test::{self, RegisteredUser};
use program_owned_account_test::{CreatePdaMode, ID};
use solana_program_test::{
    BanksClientError, BanksTransactionResultWithMetadata, ProgramTestContext,
};
use solana_sdk::instruction::InstructionError;
use solana_sdk::signature::Keypair;
use solana_sdk::{signer::Signer, transaction::Transaction};

#[tokio::test]
async fn test_create_pda() {
    let (mut context, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("program_owned_account_test"),
        ID,
    )]))
    .await;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    println!("payer_pubkey {:?}", payer_pubkey);

    let address_merkle_tree_pubkey = env.address_merkle_tree_pubkey;
    let test_indexer = TestIndexer::new(
        env.merkle_tree_pubkey,
        env.nullifier_queue_pubkey,
        address_merkle_tree_pubkey,
        payer.insecure_clone(),
    );

    let mut test_indexer = test_indexer.await;

    let seed = [1u8; 32];
    let data = [2u8; 31];

    perform_create_pda_with_event(
        &mut test_indexer,
        &mut context,
        &env,
        &payer,
        seed,
        &data,
        &ID,
        CreatePdaMode::ProgramIsSigner,
    )
    .await
    .unwrap();

    assert_created_pda(&mut test_indexer, &env, &payer, &seed, &data).await;

    let seed = [2u8; 32];
    let data = [3u8; 31];
    let invalid_owner_program = Pubkey::new_unique();
    let res = perform_create_pda_failing(
        &mut test_indexer,
        &mut context,
        &env,
        &payer,
        seed,
        &data,
        &invalid_owner_program,
        CreatePdaMode::ProgramIsSigner,
    )
    .await;
    assert_eq!(
        res.unwrap().result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(
                light_compressed_pda::ErrorCode::WriteAccessCheckFailed.into()
            )
        ))
    );
    let res = perform_create_pda_failing(
        &mut test_indexer,
        &mut context,
        &env,
        &payer,
        seed,
        &data,
        &invalid_owner_program,
        CreatePdaMode::ProgramIsNotSigner,
    )
    .await;
    assert_eq!(
        res.unwrap().result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(
                light_compressed_pda::ErrorCode::SignerSeedsNotProvided.into()
            )
        ))
    );
    let res = perform_create_pda_failing(
        &mut test_indexer,
        &mut context,
        &env,
        &payer,
        seed,
        &data,
        &ID,
        CreatePdaMode::InvalidSignerSeeds,
    )
    .await;
    assert_eq!(
        res.unwrap().result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(light_compressed_pda::ErrorCode::SignerCheckFailed.into())
        ))
    );
    let mint = create_mint_helper(&mut context, &payer).await;

    let amount = 10000u64;
    mint_tokens_helper(
        &mut context,
        &mut test_indexer,
        &env.merkle_tree_pubkey,
        &payer,
        &mint,
        vec![amount],
        vec![payer.pubkey()],
    )
    .await;
    let compressed_token_account = test_indexer
        .compressed_accounts
        .iter()
        .find(|x| x.compressed_account.owner == light_compressed_token::ID)
        .unwrap()
        .clone();
    let res = perform_invalidate_not_owned_compressed_account(
        &mut test_indexer,
        &mut context,
        &env,
        &payer,
        &compressed_token_account,
    )
    .await;
    assert_eq!(
        res.unwrap().result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(light_compressed_pda::ErrorCode::SignerCheckFailed.into())
        ))
    );
}

pub async fn perform_create_pda_failing(
    test_indexer: &mut TestIndexer,
    context: &mut ProgramTestContext,
    env: &EnvAccounts,
    payer: &Keypair,
    seed: [u8; 32],
    data: &[u8; 31],
    owner_program: &Pubkey,
    signer_is_program: CreatePdaMode,
) -> Result<BanksTransactionResultWithMetadata, BanksClientError> {
    let payer_pubkey = payer.pubkey();
    let instruction = perform_create_pda(
        env,
        seed,
        test_indexer,
        context,
        data,
        payer_pubkey,
        owner_program,
        signer_is_program,
    )
    .await;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_new_latest_blockhash().await.unwrap(),
    );
    solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await
}
pub async fn perform_create_pda_with_event(
    test_indexer: &mut TestIndexer,
    context: &mut ProgramTestContext,
    env: &EnvAccounts,
    payer: &Keypair,
    seed: [u8; 32],
    data: &[u8; 31],
    owner_program: &Pubkey,
    signer_is_program: CreatePdaMode,
) -> Result<(), BanksClientError> {
    let payer_pubkey = payer.pubkey();
    let instruction = perform_create_pda(
        env,
        seed,
        test_indexer,
        context,
        data,
        payer_pubkey,
        owner_program,
        signer_is_program,
    )
    .await;
    let event =
        create_and_send_transaction_with_event(context, &[instruction], &payer_pubkey, &[payer])
            .await?;
    test_indexer.add_compressed_accounts_with_token_data(event.unwrap());
    Ok(())
}

async fn perform_create_pda(
    env: &EnvAccounts,
    seed: [u8; 32],
    test_indexer: &mut TestIndexer,
    context: &mut ProgramTestContext,
    data: &[u8; 31],
    payer_pubkey: Pubkey,
    owner_program: &Pubkey,
    signer_is_program: CreatePdaMode,
) -> solana_sdk::instruction::Instruction {
    let address = light_compressed_pda::compressed_account::derive_address(
        &env.address_merkle_tree_pubkey,
        &seed,
    )
    .unwrap();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(None, Some(&[address]), context)
        .await;

    let new_address_params: light_compressed_pda::NewAddressParams =
        light_compressed_pda::NewAddressParams {
            seed,
            address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
            address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
            address_merkle_tree_root_index: rpc_result.address_root_indices[0],
        };
    let create_ix_inputs = CreateCompressedPdaInstructionInputs {
        data: *data,
        signer: &payer_pubkey,
        output_compressed_account_merkle_tree_pubkey: &env.merkle_tree_pubkey,
        proof: &rpc_result.proof,
        new_address_params,
        cpi_signature_account: &env.cpi_signature_account_pubkey,
        owner_program,
        signer_is_program,
    };
    let instruction = create_pda_instruction(create_ix_inputs.clone());
    instruction
}

pub async fn assert_created_pda(
    test_indexer: &mut TestIndexer,
    env: &EnvAccounts,
    payer: &Keypair,
    seed: &[u8; 32],
    data: &[u8; 31],
) {
    let compressed_escrow_pda = test_indexer
        .compressed_accounts
        .iter()
        .find(|x| x.compressed_account.owner == ID)
        .unwrap()
        .clone();
    let address = light_compressed_pda::compressed_account::derive_address(
        &env.address_merkle_tree_pubkey,
        &seed,
    )
    .unwrap();
    assert_eq!(
        compressed_escrow_pda.compressed_account.address.unwrap(),
        address
    );
    assert_eq!(compressed_escrow_pda.compressed_account.owner, ID);
    let compressed_escrow_pda_deserialized = compressed_escrow_pda
        .compressed_account
        .data
        .as_ref()
        .unwrap();
    let compressed_escrow_pda_data =
        RegisteredUser::deserialize_reader(&mut &compressed_escrow_pda_deserialized.data[..])
            .unwrap();
    assert_eq!(compressed_escrow_pda_data.user_pubkey, payer.pubkey());
    assert_eq!(compressed_escrow_pda_data.data, *data);

    assert_eq!(
        compressed_escrow_pda_deserialized.discriminator,
        1u64.to_le_bytes(),
    );
    let truncated_user_pubkey =
        hash_to_bn254_field_size_le(&compressed_escrow_pda_data.user_pubkey.to_bytes())
            .unwrap()
            .0;
    assert_eq!(
        compressed_escrow_pda_deserialized.data_hash,
        Poseidon::hashv(&[truncated_user_pubkey.as_slice(), data.as_slice()]).unwrap(),
    );
}

pub async fn perform_invalidate_not_owned_compressed_account(
    test_indexer: &mut TestIndexer,
    context: &mut ProgramTestContext,
    env: &EnvAccounts,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
) -> Result<BanksTransactionResultWithMetadata, BanksClientError> {
    let payer_pubkey = payer.pubkey();
    let hash = compressed_account
        .compressed_account
        .hash(&env.merkle_tree_pubkey, &compressed_account.leaf_index)
        .unwrap();
    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(Some(&[hash]), None, context)
        .await;
    let create_ix_inputs = InvalidateNotOwnedCompressedAccountInstructionInputs {
        signer: &payer_pubkey,
        input_merkle_tree_pubkey: &env.merkle_tree_pubkey,
        root_indices: &rpc_result.root_indices,
        proof: &rpc_result.proof,
        compressed_account,
    };
    let instruction = create_invalidate_not_owned_account_instruction(create_ix_inputs.clone());
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_new_latest_blockhash().await.unwrap(),
    );
    solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await
}
