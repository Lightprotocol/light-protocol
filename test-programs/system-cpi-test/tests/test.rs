#![cfg(feature = "test-sbf")]

use anchor_lang::AnchorDeserialize;
use light_compressed_token::InputTokenDataWithContext;
use light_hasher::{Hasher, Poseidon};
use light_system_program::sdk::address::derive_address;
use light_system_program::sdk::compressed_account::{
    CompressedAccountWithMerkleContext, PackedCompressedAccountWithMerkleContext,
    PackedMerkleContext,
};

use light_system_program::sdk::CompressedCpiContext;
use light_system_program::NewAddressParams;
use light_test_utils::rpc::errors::{assert_rpc_error, RpcError};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::spl::mint_tokens_helper;
use light_test_utils::test_env::{setup_test_programs_with_accounts, EnvAccounts};
use light_test_utils::test_indexer::{create_mint_helper, TestIndexer, TokenDataWithContext};
use light_utils::hash_to_bn254_field_size_be;
use solana_sdk::signature::Keypair;
use solana_sdk::{pubkey::Pubkey, signer::Signer, transaction::Transaction};
use system_cpi_test::sdk::{
    create_invalidate_not_owned_account_instruction, create_pda_instruction,
    CreateCompressedPdaInstructionInputs, InvalidateNotOwnedCompressedAccountInstructionInputs,
};
use system_cpi_test::{self, RegisteredUser, TokenTransferData, WithInputAccountsMode};
use system_cpi_test::{CreatePdaMode, ID};

/// Test:
/// Functional:
/// 1. Create pda
/// Failing tests To add:
/// 1. invalid signer seeds (CpiSignerCheckFailed)
/// 2. invalid invoking program (CpiSignerCheckFailed)
/// 3. write data to an account that it doesn't own (WriteAccessCheckFailed)
/// 4. input account that is not owned by signer(SignerCheckFailed)
/// Failing tests with cpi context:
/// 5. provide cpi context but no cpi context account (CpiContextMissing)
/// 6. provide cpi context account but no cpi context (CpiContextAccountUndefined)
/// 7. provide cpi context account but cpi context is empty (CpiContextEmpty)
/// 8. test signer checks trying to insert into cpi context account (invalid invoking program)
/// 9. test signer checks trying to insert into cpi context account (invalid signer seeds)
/// 10. provide cpi context account but cpi context has a different proof (CpiContextProofMismatch)
/// 11. write data to an account that it doesn't own (WriteAccessCheckFailed)
#[tokio::test]
async fn only_test_create_pda() {
    let (mut rpc, env) =
        setup_test_programs_with_accounts(Some(vec![(String::from("system_cpi_test"), ID)])).await;
    let payer = rpc.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::init_from_env(
        &payer,
        &env,
        true,
        true,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;

    let seed = vec![1u8; 32];
    let data = [2u8; 31];

    // Functional test 1 ----------------------------------------------
    perform_create_pda_with_event(
        &mut test_indexer,
        &mut rpc,
        &env,
        &payer,
        seed.to_vec(),
        &data,
        &ID,
        CreatePdaMode::ProgramIsSigner,
    )
    .await
    .unwrap();

    assert_created_pda(&mut test_indexer, &env, &payer, seed, &data).await;

    let seed = vec![2u8; 32];
    let data = [3u8; 31];

    // Failing 1 invalid signer seeds ----------------------------------------------
    perform_create_pda_failing(
        &mut test_indexer,
        &mut rpc,
        &env,
        &payer,
        seed.to_vec(),
        &data,
        &ID,
        CreatePdaMode::InvalidSignerSeeds,
        light_system_program::errors::CompressedPdaError::CpiSignerCheckFailed.into(),
    )
    .await
    .unwrap();

    // Failing 2 invoking program ----------------------------------------------
    perform_create_pda_failing(
        &mut test_indexer,
        &mut rpc,
        &env,
        &payer,
        seed.to_vec(),
        &data,
        &ID,
        CreatePdaMode::InvalidInvokingProgram,
        light_system_program::errors::CompressedPdaError::CpiSignerCheckFailed.into(),
    )
    .await
    .unwrap();

    // Failing 3 write to account not owned ----------------------------------------------
    perform_create_pda_failing(
        &mut test_indexer,
        &mut rpc,
        &env,
        &payer,
        seed,
        &data,
        &ID,
        CreatePdaMode::WriteToAccountNotOwned,
        light_system_program::errors::CompressedPdaError::WriteAccessCheckFailed.into(),
    )
    .await
    .unwrap();

    // create a token program owned Merkle tree
    // mint tokens to that tree
    let program_owned_merkle_tree_keypair = Keypair::new();
    let program_owned_queue_keypair = Keypair::new();
    let program_owned_cpi_context_keypair = Keypair::new();

    test_indexer
        .add_state_merkle_tree(
            &mut rpc,
            &program_owned_merkle_tree_keypair,
            &program_owned_queue_keypair,
            &program_owned_cpi_context_keypair,
            Some(light_compressed_token::ID),
        )
        .await;
    let mint = create_mint_helper(&mut rpc, &payer).await;

    let amount = 10000u64;
    mint_tokens_helper(
        &mut rpc,
        &mut test_indexer,
        &program_owned_merkle_tree_keypair.pubkey(),
        &payer,
        &mint,
        vec![amount],
        vec![payer.pubkey()],
    )
    .await;

    let compressed_account = test_indexer.get_compressed_token_accounts_by_owner(&payer.pubkey())
        [0]
    .compressed_account
    .clone();
    // Failing 4 input account that is not owned by signer ----------------------------------------------
    perform_with_input_accounts(
        &mut test_indexer,
        &mut rpc,
        &payer,
        &compressed_account,
        None,
        light_system_program::errors::CompressedPdaError::SignerCheckFailed.into(),
        WithInputAccountsMode::NotOwnedCompressedAccount,
    )
    .await
    .unwrap();
    let compressed_account =
        test_indexer.get_compressed_accounts_by_owner(&system_cpi_test::ID)[0].clone();
    // Failing 5 provide cpi context but no cpi context account ----------------------------------------------
    perform_with_input_accounts(
        &mut test_indexer,
        &mut rpc,
        &payer,
        &compressed_account,
        None,
        light_system_program::errors::CompressedPdaError::CpiContextMissing.into(),
        WithInputAccountsMode::CpiContextMissing,
    )
    .await
    .unwrap();
    // Failing 6 provide cpi context account but no cpi context ----------------------------------------------
    perform_with_input_accounts(
        &mut test_indexer,
        &mut rpc,
        &payer,
        &compressed_account,
        None,
        light_system_program::errors::CompressedPdaError::CpiContextAccountUndefined.into(),
        WithInputAccountsMode::CpiContextAccountMissing,
    )
    .await
    .unwrap();
    // Failing 7 provide cpi context account but cpi context is empty ----------------------------------------------
    perform_with_input_accounts(
        &mut test_indexer,
        &mut rpc,
        &payer,
        &compressed_account,
        None,
        light_system_program::errors::CompressedPdaError::CpiContextEmpty.into(),
        WithInputAccountsMode::CpiContextEmpty,
    )
    .await
    .unwrap();
    // Failing 8 test signer checks trying to insert into cpi context account (invalid invoking program) ----------------------------------------------
    perform_with_input_accounts(
        &mut test_indexer,
        &mut rpc,
        &payer,
        &compressed_account,
        None,
        light_system_program::errors::CompressedPdaError::CpiSignerCheckFailed.into(),
        WithInputAccountsMode::CpiContextInvalidInvokingProgram,
    )
    .await
    .unwrap();
    // Failing 9 test signer checks trying to insert into cpi context account (invalid signer seeds) ----------------------------------------------
    perform_with_input_accounts(
        &mut test_indexer,
        &mut rpc,
        &payer,
        &compressed_account,
        None,
        light_system_program::errors::CompressedPdaError::CpiSignerCheckFailed.into(),
        WithInputAccountsMode::CpiContextInvalidSignerSeeds,
    )
    .await
    .unwrap();
    let compressed_token_account_data =
        test_indexer.get_compressed_token_accounts_by_owner(&payer.pubkey())[0].clone();
    // Failing 10 provide cpi context account but cpi context has a different proof ----------------------------------------------
    perform_with_input_accounts(
        &mut test_indexer,
        &mut rpc,
        &payer,
        &compressed_account,
        Some(compressed_token_account_data),
        light_system_program::errors::CompressedPdaError::CpiContextProofMismatch.into(),
        WithInputAccountsMode::CpiContextProofMismatch,
    )
    .await
    .unwrap();
    // Failing 11 write to account not owned ----------------------------------------------
    perform_with_input_accounts(
        &mut test_indexer,
        &mut rpc,
        &payer,
        &compressed_account,
        None,
        light_system_program::errors::CompressedPdaError::WriteAccessCheckFailed.into(),
        WithInputAccountsMode::CpiContextWriteToNotOwnedAccount,
    )
    .await
    .unwrap();
}

/// Test:
/// 1. Cannot create an address in a program owned address Merkle tree owned by a different program (InvalidMerkleTreeOwner)
/// 2. Cannot create a compressed account in a program owned state Merkle tree owned by a different program (InvalidMerkleTreeOwner)
/// 3. Create a compressed account and address in program owned state and address Merkle trees
#[tokio::test]
async fn test_create_pda_in_program_owned_merkle_trees() {
    let (mut rpc, env) =
        setup_test_programs_with_accounts(Some(vec![(String::from("system_cpi_test"), ID)])).await;

    let payer = rpc.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::init_from_env(
        &payer,
        &env,
        true,
        true,
        "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    )
    .await;
    // Failing test 1 invalid address Merkle tree ----------------------------------------------
    let program_owned_address_merkle_tree_keypair = Keypair::new();
    let program_owned_address_queue_keypair = Keypair::new();

    test_indexer
        .add_address_merkle_tree(
            &mut rpc,
            &program_owned_address_merkle_tree_keypair,
            &program_owned_address_queue_keypair,
            Some(light_compressed_token::ID),
        )
        .await;
    let env_with_program_owned_address_merkle_tree = EnvAccounts {
        address_merkle_tree_pubkey: program_owned_address_merkle_tree_keypair.pubkey(),
        address_merkle_tree_queue_pubkey: program_owned_address_queue_keypair.pubkey(),
        merkle_tree_pubkey: env.merkle_tree_pubkey,
        nullifier_queue_pubkey: env.nullifier_queue_pubkey,
        cpi_context_account_pubkey: env.cpi_context_account_pubkey,
        governance_authority: env.governance_authority.insecure_clone(),
        governance_authority_pda: env.governance_authority_pda,
        group_pda: env.group_pda,
        registered_program_pda: env.registered_program_pda,
    };

    perform_create_pda_failing(
        &mut test_indexer,
        &mut rpc,
        &env_with_program_owned_address_merkle_tree,
        &payer,
        vec![3u8; 32],
        &[4u8; 31],
        &ID,
        CreatePdaMode::ProgramIsSigner,
        light_system_program::errors::CompressedPdaError::InvalidMerkleTreeOwner.into(),
    )
    .await
    .unwrap();

    // Failing test 2 invalid state Merkle tree ----------------------------------------------
    let program_owned_state_merkle_tree_keypair = Keypair::new();
    let program_owned_state_queue_keypair = Keypair::new();
    let program_owned_cpi_context_keypair = Keypair::new();

    test_indexer
        .add_state_merkle_tree(
            &mut rpc,
            &program_owned_state_merkle_tree_keypair,
            &program_owned_state_queue_keypair,
            &program_owned_cpi_context_keypair,
            Some(light_compressed_token::ID),
        )
        .await;
    let env_with_program_owned_state_merkle_tree = EnvAccounts {
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_merkle_tree_queue_pubkey: env.address_merkle_tree_queue_pubkey,
        merkle_tree_pubkey: program_owned_state_merkle_tree_keypair.pubkey(),
        nullifier_queue_pubkey: program_owned_state_queue_keypair.pubkey(),
        cpi_context_account_pubkey: program_owned_cpi_context_keypair.pubkey(),
        governance_authority: env.governance_authority.insecure_clone(),
        governance_authority_pda: env.governance_authority_pda,
        group_pda: env.group_pda,
        registered_program_pda: env.registered_program_pda,
    };
    perform_create_pda_failing(
        &mut test_indexer,
        &mut rpc,
        &env_with_program_owned_state_merkle_tree,
        &payer,
        vec![3u8; 32],
        &[4u8; 31],
        &ID,
        CreatePdaMode::ProgramIsSigner,
        light_system_program::errors::CompressedPdaError::InvalidMerkleTreeOwner.into(),
    )
    .await
    .unwrap();

    // Functional test ----------------------------------------------
    let program_owned_state_merkle_tree_keypair = Keypair::new();
    let program_owned_state_queue_keypair = Keypair::new();
    let program_owned_cpi_context_keypair = Keypair::new();

    test_indexer
        .add_state_merkle_tree(
            &mut rpc,
            &program_owned_state_merkle_tree_keypair,
            &program_owned_state_queue_keypair,
            &program_owned_cpi_context_keypair,
            Some(ID),
        )
        .await;
    let program_owned_address_merkle_tree_keypair = Keypair::new();
    let program_owned_address_queue_keypair = Keypair::new();

    test_indexer
        .add_address_merkle_tree(
            &mut rpc,
            &program_owned_address_merkle_tree_keypair,
            &program_owned_address_queue_keypair,
            Some(ID),
        )
        .await;
    let env_with_program_owned_state_merkle_tree = EnvAccounts {
        address_merkle_tree_pubkey: program_owned_address_merkle_tree_keypair.pubkey(),
        address_merkle_tree_queue_pubkey: program_owned_address_queue_keypair.pubkey(),
        merkle_tree_pubkey: program_owned_state_merkle_tree_keypair.pubkey(),
        nullifier_queue_pubkey: program_owned_state_queue_keypair.pubkey(),
        cpi_context_account_pubkey: program_owned_cpi_context_keypair.pubkey(),
        governance_authority: env.governance_authority.insecure_clone(),
        governance_authority_pda: env.governance_authority_pda,
        group_pda: env.group_pda,
        registered_program_pda: env.registered_program_pda,
    };
    let seed = vec![4u8; 32];
    let data = [5u8; 31];
    perform_create_pda_with_event(
        &mut test_indexer,
        &mut rpc,
        &env_with_program_owned_state_merkle_tree,
        &payer,
        seed.to_vec(),
        &data,
        &ID,
        CreatePdaMode::ProgramIsSigner,
    )
    .await
    .unwrap();

    assert_created_pda(
        &mut test_indexer,
        &env_with_program_owned_state_merkle_tree,
        &payer,
        seed.to_vec(),
        &data,
    )
    .await;

    // let payer_pubkey = payer.pubkey();
    // let instruction = perform_create_pda(
    //     &env_with_program_owned_address_merkle_tree,
    //     seed,
    //     &mut test_indexer,
    //     &mut rpc,
    //     &data,
    //     payer_pubkey,
    //     &ID,
    //     CreatePdaMode::ProgramIsSigner,
    // )
    // .await;
    // let tx = Transaction::new_signed_with_payer(
    //     &[instruction],
    //     Some(&payer_pubkey),
    //     &[&payer],
    //     rpc.get_latest_blockhash().await.unwrap(),
    // );
    // let res = rpc.process_transaction(tx).await;
    // assert_custom_error_or_program_error(
    //     res,
    //     light_system_program::errors::CompressedPdaError::InvalidMerkleTreeOwner.into(),
    // )
    // .unwrap();
}

#[allow(clippy::too_many_arguments)]
pub async fn perform_create_pda_failing<R: RpcConnection>(
    test_indexer: &mut TestIndexer<200, R>,
    rpc: &mut R,
    env: &EnvAccounts,
    payer: &Keypair,
    seed: Vec<u8>,
    data: &[u8; 31],
    owner_program: &Pubkey,
    signer_is_program: CreatePdaMode,
    expected_error_code: u32,
) -> Result<(), RpcError> {
    let payer_pubkey = payer.pubkey();
    let instruction = perform_create_pda(
        env,
        seed,
        test_indexer,
        rpc,
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
        rpc.get_latest_blockhash().await.unwrap(),
    );
    let result = rpc.process_transaction(transaction).await;
    assert_rpc_error(result, 0, expected_error_code)
}

#[allow(clippy::too_many_arguments)]
pub async fn perform_create_pda_with_event<R: RpcConnection>(
    test_indexer: &mut TestIndexer<200, R>,
    rpc: &mut R,
    env: &EnvAccounts,
    payer: &Keypair,
    seed: Vec<u8>,
    data: &[u8; 31],
    owner_program: &Pubkey,
    signer_is_program: CreatePdaMode,
) -> Result<(), RpcError> {
    let payer_pubkey = payer.pubkey();
    let instruction = perform_create_pda(
        env,
        seed,
        test_indexer,
        rpc,
        data,
        payer_pubkey,
        owner_program,
        signer_is_program,
    )
    .await;
    let event = rpc
        .create_and_send_transaction_with_event(&[instruction], &payer_pubkey, &[payer], None)
        .await?;
    test_indexer.add_compressed_accounts_with_token_data(&event.unwrap());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn perform_create_pda<R: RpcConnection>(
    env: &EnvAccounts,
    seed: Vec<u8>,
    test_indexer: &mut TestIndexer<200, R>,
    rpc: &mut R,
    data: &[u8; 31],
    payer_pubkey: Pubkey,
    owner_program: &Pubkey,
    signer_is_program: CreatePdaMode,
) -> solana_sdk::instruction::Instruction {
    let address = derive_address(&env.address_merkle_tree_pubkey, &seed).unwrap();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            None,
            None,
            Some(&[address]),
            Some(vec![env.address_merkle_tree_pubkey]),
            rpc,
        )
        .await;

    let new_address_params = NewAddressParams {
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
        cpi_context_account: &env.cpi_context_account_pubkey,
        owner_program,
        signer_is_program,
        registered_program_pda: &env.registered_program_pda,
    };
    create_pda_instruction(create_ix_inputs.clone())
}

pub async fn assert_created_pda<R: RpcConnection>(
    test_indexer: &mut TestIndexer<200, R>,
    env: &EnvAccounts,
    payer: &Keypair,
    seed: Vec<u8>,
    data: &[u8; 31],
) {
    let compressed_escrow_pda = test_indexer
        .compressed_accounts
        .iter()
        .find(|x| x.compressed_account.owner == ID)
        .unwrap()
        .clone();
    let address = derive_address(&env.address_merkle_tree_pubkey, &seed).unwrap();
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
        hash_to_bn254_field_size_be(&compressed_escrow_pda_data.user_pubkey.to_bytes())
            .unwrap()
            .0;
    assert_eq!(
        compressed_escrow_pda_deserialized.data_hash,
        Poseidon::hashv(&[truncated_user_pubkey.as_slice(), data.as_slice()]).unwrap(),
    );
}

pub async fn perform_with_input_accounts<R: RpcConnection>(
    test_indexer: &mut TestIndexer<200, R>,
    rpc: &mut R,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
    token_account: Option<TokenDataWithContext>,
    expected_error_code: u32,
    mode: WithInputAccountsMode,
) -> Result<(), RpcError> {
    let payer_pubkey = payer.pubkey();
    let hash = compressed_account.hash().unwrap();
    let merkle_tree_pubkey = compressed_account.merkle_context.merkle_tree_pubkey;
    let nullifier_pubkey = compressed_account.merkle_context.nullifier_queue_pubkey;
    let cpi_context = match mode {
        WithInputAccountsMode::CpiContextMissing
        | WithInputAccountsMode::CpiContextAccountMissing
        | WithInputAccountsMode::CpiContextEmpty
        | WithInputAccountsMode::CpiContextInvalidInvokingProgram
        | WithInputAccountsMode::CpiContextInvalidSignerSeeds
        | WithInputAccountsMode::CpiContextProofMismatch
        | WithInputAccountsMode::CpiContextWriteToNotOwnedAccount => Some(CompressedCpiContext {
            cpi_context_account_index: 2,
            set_context: true,
        }),
        _ => None,
    };
    let cpi_context_account_pubkey = test_indexer
        .state_merkle_trees
        .iter()
        .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
        .unwrap()
        .accounts
        .cpi_context;
    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[hash]),
            Some(&[merkle_tree_pubkey]),
            None,
            None,
            rpc,
        )
        .await;

    let token_transfer_data = match token_account {
        Some(token_account) => Some(TokenTransferData {
            mint: token_account.token_data.mint,
            input_token_data_with_context: InputTokenDataWithContext {
                amount: token_account.token_data.amount,
                delegate_index: None,
                delegated_amount: None,
                is_native: None,
                root_index: rpc_result.root_indices[0],
                merkle_context: PackedMerkleContext {
                    leaf_index: token_account.compressed_account.merkle_context.leaf_index,
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 1,
                },
            },
        }),
        _ => None,
    };
    let create_ix_inputs = InvalidateNotOwnedCompressedAccountInstructionInputs {
        signer: &payer_pubkey,
        input_merkle_tree_pubkey: &merkle_tree_pubkey,
        input_nullifier_pubkey: &nullifier_pubkey,
        cpi_context_account: &cpi_context_account_pubkey,
        cpi_context,
        proof: &rpc_result.proof,
        compressed_account: &PackedCompressedAccountWithMerkleContext {
            compressed_account: compressed_account.compressed_account.clone(),
            merkle_context: PackedMerkleContext {
                leaf_index: compressed_account.merkle_context.leaf_index,
                merkle_tree_pubkey_index: 0,
                nullifier_queue_pubkey_index: 1,
            },
            root_index: rpc_result.root_indices[0],
        },
        token_transfer_data,
    };
    let instruction =
        create_invalidate_not_owned_account_instruction(create_ix_inputs.clone(), mode);
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        rpc.get_latest_blockhash().await.unwrap(),
    );
    let result = rpc.process_transaction(transaction).await;
    assert_rpc_error(result, 0, expected_error_code)
}
