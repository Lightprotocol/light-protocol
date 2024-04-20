#![cfg(feature = "test-sbf")]

use account_compression::Pubkey;
use anchor_lang::AnchorDeserialize;
use light_hasher::{Hasher, Poseidon};
use light_test_utils::test_env::{setup_test_programs_with_accounts, EnvAccounts};
use light_test_utils::test_indexer::{create_mint_helper, mint_tokens_helper, TestIndexer};
use light_utils::hash_to_bn254_field_size_le;
use program_owned_account_test::sdk::{
    create_pda_instruction, CreateCompressedPdaInstructionInputs,
};
use program_owned_account_test::{self, RegisteredUser};

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
        program_owned_account_test::ID,
    )]))
    .await;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    println!("payer_pubkey {:?}", payer_pubkey);

    let address_merkle_tree_pubkey = env.address_merkle_tree_pubkey;
    let test_indexer = TestIndexer::new(
        env.merkle_tree_pubkey,
        env.indexed_array_pubkey,
        address_merkle_tree_pubkey,
        payer.insecure_clone(),
        true,
        true,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    );

    let mut test_indexer = test_indexer.await;

    // let mint = create_mint_helper(&mut context, &payer).await;

    // let amount = 10000u64;
    // mint_tokens_helper(
    //     &mut context,
    //     &mut test_indexer,
    //     &env.merkle_tree_pubkey,
    //     &payer,
    //     &mint,
    //     vec![amount],
    //     vec![payer.pubkey()],
    // )
    // .await;

    let seed = [1u8; 32];
    let data = [2u8; 31];

    let res = perform_create_pda(
        &mut test_indexer,
        &mut context,
        &env,
        &payer,
        seed,
        &data,
        &ID,
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
    assert_created_pda(&mut test_indexer, &env, &payer, &seed, &data).await;

    let seed = [2u8; 32];
    let data = [3u8; 31];
    let invalid_owner_program = Pubkey::new_unique();
    let res = perform_create_pda(
        &mut test_indexer,
        &mut context,
        &env,
        &payer,
        seed,
        &data,
        &invalid_owner_program,
    )
    .await;
    assert_eq!(
        res.unwrap().result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(psp_compressed_pda::ErrorCode::WriteAccessCheckFailed.into())
        ))
    );
    // assert_escrow(
    //     &mut test_indexer,
    //     &env,
    //     &payer,
    //     &escrow_amount,
    //     &amount,
    //     &seed,
    //     &lockup_end,
    // )
    // .await;
}

pub async fn perform_create_pda(
    test_indexer: &mut TestIndexer,
    context: &mut ProgramTestContext,
    env: &EnvAccounts,
    payer: &Keypair,
    seed: [u8; 32],
    data: &[u8; 31],
    owner_program: &Pubkey,
) -> Result<BanksTransactionResultWithMetadata, BanksClientError> {
    let payer_pubkey = payer.pubkey();
    let address = psp_compressed_pda::compressed_account::derive_address(
        &env.address_merkle_tree_pubkey,
        &seed,
    )
    .unwrap();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(None, Some(&[address]), context)
        .await;

    let new_address_params: psp_compressed_pda::NewAddressParams =
        psp_compressed_pda::NewAddressParams {
            seed,
            address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
            address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
            address_merkle_tree_root_index: rpc_result.address_root_indices[0],
        };
    let create_ix_inputs = CreateCompressedPdaInstructionInputs {
        data: *data,
        signer: &payer_pubkey,
        output_compressed_account_merkle_tree_pubkey: &env.merkle_tree_pubkey,
        root_indices: &rpc_result.root_indices,
        proof: &rpc_result.proof,
        new_address_params,
        cpi_signature_account: &env.cpi_signature_account_pubkey,
        owner_program,
    };
    let instruction = create_pda_instruction(create_ix_inputs.clone());
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
use program_owned_account_test::ID;

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
    let address = psp_compressed_pda::compressed_account::derive_address(
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
    println!(
        "compressed_escrow_pda_data {:?}",
        compressed_escrow_pda_data
    );
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
