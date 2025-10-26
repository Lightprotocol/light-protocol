use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use light_account_checks::error::AccountError;
use light_batched_merkle_tree::initialize_state_tree::InitStateTreeAccountsInstructionData;
use light_client::indexer::{
    AddressMerkleTreeAccounts, AddressWithTree, Indexer, StateMerkleTreeAccounts,
};
use light_compressed_account::{
    address::{derive_address, derive_address_legacy},
    compressed_account::{
        CompressedAccountWithMerkleContext, PackedCompressedAccountWithMerkleContext,
        PackedMerkleContext,
    },
    hash_to_bn254_field_size_be,
    instruction_data::{
        cpi_context::CompressedCpiContext,
        data::{NewAddressParams, ReadOnlyAddress},
    },
    TreeType,
};
use light_compressed_token::process_transfer::InputTokenDataWithContext;
use light_hasher::{Hasher, Poseidon};
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;
use light_program_test::{
    accounts::test_accounts::TestAccounts,
    indexer::{TestIndexer, TestIndexerExtensions},
    program_test::LightProgramTest,
    utils::assert::assert_rpc_error,
    ProgramTestConfig,
};
use light_registry::account_compression_cpi::sdk::create_batch_update_address_tree_instruction;
use light_sdk::token::{AccountState, TokenDataWithMerkleContext};
use light_system_program::errors::SystemProgramError;
use light_test_utils::{
    e2e_test_env::init_program_test_env,
    spl::{create_mint_helper, mint_tokens_helper},
    system_program::transfer_compressed_sol_test,
    test_batch_forester::{
        create_batch_update_address_tree_instruction_data_with_proof, perform_batch_append,
    },
    Rpc, RpcError,
};
use light_verifier::VerifierError;
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};
use system_cpi_test::{
    self,
    sdk::{
        create_invalidate_not_owned_account_instruction, create_pda_instruction,
        CreateCompressedPdaInstructionInputs, InvalidateNotOwnedCompressedAccountInstructionInputs,
    },
    CreatePdaMode, RegisteredUser, TokenTransferData, WithInputAccountsMode, ID,
};

/// Tests:
/// 1. functional - 1 read only account proof by index
/// 2. functional - 1 read only account proof by index, 1 read only account by zkp
/// 3. functional - 10 read only account proof by index
/// 4. failing - read only account in v1 state mt
/// 5. failing - invalid read only account proof by index
/// 6. failing - invalid output queue
/// 7. failing - proof by index for invalidated account
/// 8. failing - proof is none
/// 9. failing - invalid proof
/// 10. failing - invalid root index
/// 11. failing - invalid read only account with zkp
/// 12. failing - zkp for invalidated account
/// 13. failing - invalid state mt
/// 14. failing - account marked as proof by index but index cannot be in value vec
/// 15. failing - invalid leaf index, proof by index
/// 16. functional - 4 read only accounts by zkp
/// 17. functional - 3 read only accounts by zkp 1 regular input
/// 18. functional - 1 read only account by zkp 3 regular inputs
///
/// Read only account specific inputs:
/// struct PackedReadOnlyCompressedAccount {
///     account_hash: [u8; 32], // tested in 5 & 11
///     merkle_context: PackedMerkleContext,
///     root_index: u16, // tested in 10
/// }
///
/// struct PackedMerkleContext {
///     merkle_tree_pubkey_index: u8, // tested in 13
///     nullifier_queue_pubkey_index: u8, // tested in 6
///     leaf_index: u32, // tested in 15 (not used with zkp)
///     prove_by_index: bool, // tested in 14
///}
///
#[serial]
#[tokio::test]
#[ignore = "Currently failes with Prover failed to generate proof."]
async fn test_read_only_accounts() {
    let _rpc = LightProgramTest::new(ProgramTestConfig::new(
        false,
        Some(vec![("system_cpi_test", ID)]),
    ))
    .await
    .expect("Failed to setup test programs with accounts");
    let env = _rpc.test_accounts.clone();
    let payer = _rpc.get_payer().insecure_clone();

    let mut e2e_env = init_program_test_env(_rpc, &env, 0).await;
    e2e_env.keypair_action_config.fee_assert = false;

    // Create system state with accounts:
    // - inserted a batched Merkle tree
    // - inserted a batched output queue
    // - inserted a batched output queue and batched Merkle tree
    {
        let params = InitStateTreeAccountsInstructionData::test_default();
        let max_index = params.output_queue_batch_size * 2;

        // fill two batches
        for i in 0..max_index {
            let seed = [i as u8; 32];
            let data = [i as u8; 31];
            perform_create_pda_with_event(
                &mut e2e_env.indexer,
                &mut e2e_env.rpc,
                &env,
                &payer,
                seed,
                &data,
                &ID,
                None,
                None,
                CreatePdaMode::BatchFunctional,
            )
            .await
            .unwrap();
        }
        println!("max_index: {}", max_index);
        println!(
            "params.output_queue_zkp_batch_size : {}",
            params.output_queue_zkp_batch_size
        );
        println!("inserted two batches");
        // insert one batch and one proof for batch 2 to zero out the bloom filter of batch 1
        for i in 0..6 {
            println!("inserting batch {}", i);

            let mut bundle = e2e_env.indexer.state_merkle_trees[1].clone();
            perform_batch_append(
                &mut e2e_env.rpc,
                &mut bundle,
                &env.protocol.forester,
                0,
                false,
                None,
            )
            .await
            .unwrap();
            e2e_env.indexer.state_merkle_trees[1] = bundle;

            // fails because of invalid leaves hash_chain in some iteration
            let instruction_data = create_batch_update_address_tree_instruction_data_with_proof(
                &mut e2e_env.rpc,
                &mut e2e_env.indexer,
                env.v2_address_trees[0],
            )
            .await
            .unwrap();

            let instruction = create_batch_update_address_tree_instruction(
                env.protocol.forester.pubkey(),
                env.protocol.forester.pubkey(),
                env.v2_address_trees[0],
                0,
                instruction_data.try_to_vec().unwrap(),
            );
            e2e_env
                .rpc
                .create_and_send_transaction(
                    &[instruction],
                    &env.protocol.forester.pubkey(),
                    &[&env.protocol.forester],
                )
                .await
                .unwrap();
            let mut account = e2e_env
                .rpc
                .get_account(env.v2_address_trees[0])
                .await
                .unwrap()
                .unwrap();
            e2e_env
                .indexer
                .finalize_batched_address_tree_update(
                    env.v2_address_trees[0],
                    account.data.as_mut_slice(),
                )
                .await;
        }

        for i in 0..params.output_queue_zkp_batch_size {
            let seed = [i as u8 + 100; 32];
            let data = [i as u8 + 100; 31];
            perform_create_pda_with_event(
                &mut e2e_env.indexer,
                &mut e2e_env.rpc,
                &env,
                &payer,
                seed,
                &data,
                &ID,
                None,
                None,
                CreatePdaMode::BatchFunctional,
            )
            .await
            .unwrap();
        }
    }

    // account in batched state mt and value vec
    let account_in_value_array = e2e_env
        .indexer
        .get_compressed_accounts_with_merkle_context_by_owner(&ID)
        .iter()
        .find(|x| {
            x.merkle_context.leaf_index == 101
                && x.merkle_context.merkle_tree_pubkey.to_bytes()
                    == env.v2_state_trees[0].merkle_tree.to_bytes()
        })
        .unwrap()
        .clone();

    let account_not_in_value_array_and_in_mt = e2e_env
        .indexer
        .get_compressed_accounts_with_merkle_context_by_owner(&ID)
        .iter()
        .find(|x| {
            x.merkle_context.leaf_index == 1
                && x.merkle_context.merkle_tree_pubkey.to_bytes()
                    == env.v2_state_trees[0].merkle_tree.to_bytes()
        })
        .unwrap()
        .clone();

    // 1. functional - 1 read only account proof by index, an create 1 new account
    {
        let seed = [202u8; 32];
        let data = [2u8; 31];

        perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            Some(vec![account_in_value_array.clone()]),
            CreatePdaMode::BatchFunctional,
        )
        .await
        .unwrap();
    }
    println!("post 1");
    // 2. functional - 1 read only account proof by index, 1 read only account by zkp
    {
        let seed = [203u8; 32];
        let data = [3u8; 31];

        perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            Some(vec![
                account_in_value_array.clone(),
                account_not_in_value_array_and_in_mt.clone(),
            ]),
            CreatePdaMode::BatchFunctional,
        )
        .await
        .unwrap();
    }
    println!("post 2");
    // 3. functional - 10 read only account proof by index
    {
        let seed = [200u8; 32];
        let data = [3u8; 31];

        perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            Some(vec![account_in_value_array.clone(); 10]),
            CreatePdaMode::BatchFunctional,
        )
        .await
        .unwrap();
    }
    println!("post 3");

    // 4. Failing - read only account in v1 state mt
    {
        let seed = [204u8; 32];
        let data = [4u8; 31];
        perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            None,
            CreatePdaMode::Functional,
        )
        .await
        .unwrap();
        let seed = [205u8; 32];
        let data = [4u8; 31];
        let account_in_v1_tree = e2e_env
            .indexer
            .get_compressed_accounts_with_merkle_context_by_owner(&ID)
            .iter()
            .find(|x| {
                x.merkle_context.merkle_tree_pubkey.to_bytes()
                    == env.v1_state_trees[0].merkle_tree.to_bytes()
            })
            .unwrap()
            .clone();
        let result = perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            Some(vec![
                // account_in_value_array.clone(),
                account_in_v1_tree.clone(),
            ]),
            CreatePdaMode::Functional,
        )
        .await;
        assert_rpc_error(result, 0, SystemProgramError::InvalidAccount.into()).unwrap();
    }

    let seed = [206u8; 32];
    let data = [5u8; 31];
    println!("post 4");

    // 5. Failing - invalid read only account proof by index
    {
        let result = perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            Some(vec![account_in_value_array.clone()]),
            CreatePdaMode::InvalidReadOnlyAccount,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            SystemProgramError::ReadOnlyAccountDoesNotExist.into(),
        )
        .unwrap();
    }
    println!("post 5");

    // 6. failing - invalid output queue
    {
        let result = perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            Some(vec![account_in_value_array.clone()]),
            CreatePdaMode::InvalidReadOnlyAccountOutputQueue,
        )
        .await;

        assert_rpc_error(result, 0, AccountError::InvalidDiscriminator.into()).unwrap();
    }
    println!("post 6");

    // 8. failing - proof is none
    {
        let result = perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            Some(vec![account_not_in_value_array_and_in_mt.clone()]),
            CreatePdaMode::ProofIsNoneReadOnlyAccount,
        )
        .await;
        assert_rpc_error(result, 0, SystemProgramError::ProofIsNone.into()).unwrap();
    }
    println!("post 8");
    // 9. failing - invalid proof
    {
        let result = perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            Some(vec![account_not_in_value_array_and_in_mt.clone()]),
            CreatePdaMode::InvalidProofReadOnlyAccount,
        )
        .await;
        assert_rpc_error(result, 0, VerifierError::ProofVerificationFailed.into()).unwrap();
    }
    println!("post 9");
    // 10. failing - invalid root index
    {
        let result = perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            Some(vec![account_not_in_value_array_and_in_mt.clone()]),
            CreatePdaMode::InvalidReadOnlyAccountRootIndex,
        )
        .await;
        assert_rpc_error(result, 0, VerifierError::ProofVerificationFailed.into()).unwrap();
    }
    println!("post 10");
    // 11. failing - invalid read only account with zkp
    {
        let result = perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            Some(vec![account_not_in_value_array_and_in_mt.clone()]),
            CreatePdaMode::InvalidReadOnlyAccount,
        )
        .await;
        assert_rpc_error(result, 0, VerifierError::ProofVerificationFailed.into()).unwrap();
    }
    println!("post 11");
    // 13. failing - invalid state mt
    {
        let result = perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            Some(vec![account_not_in_value_array_and_in_mt.clone()]),
            CreatePdaMode::InvalidReadOnlyAccountMerkleTree,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated.into(),
        )
        .unwrap();
    }
    println!("post 13");
    // 14. failing - account marked as proof by index but index cannot be in value vec
    {
        let result = perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            Some(vec![account_not_in_value_array_and_in_mt.clone()]),
            CreatePdaMode::AccountNotInValueVecMarkedProofByIndex,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            SystemProgramError::ReadOnlyAccountDoesNotExist.into(),
        )
        .unwrap();
    }
    println!("post 14");
    // 15. failing - invalid leaf index, proof by index
    {
        let result = perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            Some(vec![account_in_value_array.clone()]),
            CreatePdaMode::InvalidLeafIndex,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            SystemProgramError::ReadOnlyAccountDoesNotExist.into(),
        )
        .unwrap();
    }
    println!("post 14 A");

    // // 15. functional - proof by index for account which is invalidated in the same tx
    // {
    //     perform_create_pda_with_event(
    //         &mut e2e_env.indexer,
    //         &mut e2e_env.rpc,
    //         &env,
    //         &payer,
    //         seed,
    //         &data,
    //         &ID,
    //         None,
    //         Some(vec![account_in_value_array.clone()]),
    //         CreatePdaMode::ReadOnlyProofOfInsertedAccount,
    //     )
    //     .await
    //     .unwrap();
    // }
    println!("post 15");

    // 16. failing - proof by index for invalidated account & functional - proof by index for account which is invalidated in the same tx
    {
        let result = perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            Some(vec![account_in_value_array.clone()]),
            Some(vec![account_in_value_array.clone()]),
            CreatePdaMode::ReadOnlyProofOfInsertedAccount,
        )
        .await;
        assert_rpc_error(
            result,
            1,
            SystemProgramError::ReadOnlyAccountDoesNotExist.into(),
        )
        .unwrap();
    }
    println!("post 7");
    println!("post 15");
    // 16. functional - 4 read only accounts by zkp
    {
        let seed = [207u8; 32];
        let data = [5u8; 31];
        perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            Some(vec![account_not_in_value_array_and_in_mt.clone(); 4]),
            CreatePdaMode::BatchFunctional,
        )
        .await
        .unwrap();
    }
    println!("post 16");

    // 17. functional - 3 read only accounts by zkp 1 regular input
    {
        let seed = [208u8; 32];
        let data = [5u8; 31];
        let input_account_in_mt = e2e_env
            .indexer
            .get_compressed_accounts_with_merkle_context_by_owner(&ID)
            .iter()
            .find(|x| {
                x.merkle_context.leaf_index == 2
                    && x.merkle_context.merkle_tree_pubkey.to_bytes()
                        == env.v2_state_trees[0].merkle_tree.to_bytes()
                    && x.merkle_context.leaf_index
                        != account_not_in_value_array_and_in_mt
                            .merkle_context
                            .leaf_index
            })
            .unwrap()
            .clone();
        perform_create_pda_with_event(
            &mut e2e_env.indexer,
            &mut e2e_env.rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            Some(vec![input_account_in_mt.clone()]),
            Some(vec![account_not_in_value_array_and_in_mt.clone(); 3]),
            CreatePdaMode::BatchFunctional,
        )
        .await
        .unwrap();
    }

    println!("post 17");
    // Doesn't yield the expected result due to unclean test setup
    // // 18. functional - 1 read only account by zkp 3 regular inputs && failing - zkp for invalidated account
    // {
    //     let seed = [254u8; 32];
    //     let data = [5u8; 31];
    //     let mut input_accounts = Vec::new();
    //     let compressed_accounts = e2e_env.indexer.get_compressed_accounts_by_owner(&ID);
    //     for i in 0..100 {
    //         let input_account_in_mt = compressed_accounts.iter().find(|x| {
    //             x.merkle_context.leaf_index == i
    //                 && x.merkle_context.merkle_tree_pubkey == env.v2_state_trees[0].merkle_tree
    //                 && x.merkle_context.leaf_index
    //                     != account_not_in_value_array_and_in_mt
    //                         .merkle_context
    //                         .leaf_index
    //         });
    //         if let Some(input_account_in_mt) = input_account_in_mt.clone() {
    //             input_accounts.push(input_account_in_mt.clone());
    //         }
    //         if input_accounts.len() == 3 {
    //             break;
    //         }
    //     }
    //     let result = perform_create_pda_with_event(
    //         &mut e2e_env.indexer,
    //         &mut e2e_env.rpc,
    //         &env,
    //         &payer,
    //         seed,
    //         &data,
    //         &ID,
    //         Some(input_accounts),
    //         Some(vec![account_not_in_value_array_and_in_mt.clone()]),
    //         CreatePdaMode::ReadOnlyZkpOfInsertedAccount,
    //     )
    //     .await;
    //     assert_rpc_error(
    //         result,
    //         1,
    //         SystemProgramError::ReadOnlyAccountDoesNotExist.into(),
    //     )
    //     .unwrap();
    // }
    // // 12. failing - zkp for invalidated account
    // {
    //     let result = perform_create_pda_with_event(
    //         &mut e2e_env.indexer,
    //         &mut e2e_env.rpc,
    //         &env,
    //         &payer,
    //         seed,
    //         &data,
    //         &ID,
    //         Some(vec![account_not_in_value_array_and_in_mt.clone()]),
    //         Some(vec![account_not_in_value_array_and_in_mt.clone()]),
    //         CreatePdaMode::ReadOnlyZkpOfInsertedAccount,
    //     )
    //     .await;
    //     assert_rpc_error(
    //         result,
    //         1,
    //         SystemProgramError::ReadOnlyAccountDoesNotExist.into(),
    //     )
    //     .unwrap();
    // }
    // println!("post 12");
}

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
/// 10. provide cpi context account but cpi context has a different fee payer (CpiContextFeePayerMismatch)
/// 11. write data to an account that it doesn't own (WriteAccessCheckFailed)
/// 12. Spend Program owned account with program keypair (SignerCheckFailed)
/// 13. Create program owned account without data (DataFieldUndefined)
#[serial]
#[tokio::test]
async fn only_test_create_pda() {
    let mut rpc = LightProgramTest::new({
        let mut config = ProgramTestConfig::default_with_batched_trees(false);
        config.additional_programs = Some(vec![("system_cpi_test", ID)]);
        config.log_light_protocol_events = true;
        config
    })
    .await
    .expect("Failed to setup test programs with accounts");
    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 0).await;
    {
        let seed = [5u8; 32];
        let data = [2u8; 31];

        let result = perform_create_pda_with_event(
            &mut test_indexer,
            &mut rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            None,
            CreatePdaMode::InvalidReadOnlyAddress,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            SystemProgramError::ProofVerificationFailed.into(),
        )
        .unwrap();

        let result = perform_create_pda_with_event(
            &mut test_indexer,
            &mut rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            None,
            CreatePdaMode::InvalidReadOnlyMerkleTree,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            SystemProgramError::AddressMerkleTreeAccountDiscriminatorMismatch.into(),
        )
        .unwrap();

        let result = perform_create_pda_with_event(
            &mut test_indexer,
            &mut rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            None,
            CreatePdaMode::InvalidReadOnlyRootIndex,
        )
        .await;

        assert_rpc_error(
            result,
            0,
            SystemProgramError::ProofVerificationFailed.into(),
        )
        .unwrap();

        let result = perform_create_pda_with_event(
            &mut test_indexer,
            &mut rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            None,
            CreatePdaMode::UseReadOnlyAddressInAccount,
        )
        .await;
        assert_rpc_error(result, 0, SystemProgramError::AddressDoesNotMatch.into()).unwrap();

        // Functional readonly address ----------------------------------------------
        perform_create_pda_with_event(
            &mut test_indexer,
            &mut rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            None,
            CreatePdaMode::OneReadOnlyAddress,
        )
        .await
        .unwrap();

        let seed = [6u8; 32];
        let data = [2u8; 31];
        perform_create_pda_with_event(
            &mut test_indexer,
            &mut rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            None,
            CreatePdaMode::TwoReadOnlyAddresses,
        )
        .await
        .unwrap();
    }
    {
        let seed = [3u8; 32];
        let data = [2u8; 31];

        // Functional batch address ----------------------------------------------
        perform_create_pda_with_event(
            &mut test_indexer,
            &mut rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            None,
            CreatePdaMode::BatchAddressFunctional,
        )
        .await
        .unwrap();

        // Failing batch address double spend ----------------------------------------------
        let result = perform_create_pda_with_event(
            &mut test_indexer,
            &mut rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            None,
            CreatePdaMode::BatchAddressFunctional,
        )
        .await;
        // bloom filter full
        assert_rpc_error(result, 0, 14201).unwrap();
        let seed = [4u8; 32];

        let result = perform_create_pda_with_event(
            &mut test_indexer,
            &mut rpc,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            None,
            None,
            CreatePdaMode::InvalidBatchTreeAccount,
        )
        .await;
        assert_rpc_error(
            result, 0,
            21, // Panic AccountCompressionErrorCode::AddressMerkleTreeAccountDiscriminatorMismatch.into(),
        )
        .unwrap();
    }
    let seed = [1u8; 32];
    let data = [2u8; 31];

    // Functional test 1 ----------------------------------------------
    perform_create_pda_with_event(
        &mut test_indexer,
        &mut rpc,
        &env,
        &payer,
        seed,
        &data,
        &ID,
        None,
        None,
        CreatePdaMode::ProgramIsSigner,
    )
    .await
    .unwrap();

    assert_created_pda::<LightProgramTest, TestIndexer>(
        &mut test_indexer,
        &env,
        &payer,
        &seed,
        &data,
    )
    .await;

    let seed = [2u8; 32];
    let data = [3u8; 31];

    // Failing 2 invoking program ----------------------------------------------
    perform_create_pda_failing(
        &mut rpc,
        &mut test_indexer,
        &env,
        &payer,
        seed,
        &data,
        &ID,
        CreatePdaMode::InvalidInvokingProgram,
        SystemProgramError::CpiSignerCheckFailed.into(),
    )
    .await
    .unwrap();

    // Failing 3 write to account not owned ----------------------------------------------
    perform_create_pda_failing(
        &mut rpc,
        &mut test_indexer,
        &env,
        &payer,
        seed,
        &data,
        &ID,
        CreatePdaMode::WriteToAccountNotOwned,
        SystemProgramError::WriteAccessCheckFailed.into(),
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
            None, // Program owned trees are deprecated
            None,
            TreeType::StateV1,
        )
        .await;

    rpc.indexer.as_mut().unwrap().state_merkle_trees = test_indexer.state_merkle_trees.clone();

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

    let compressed_account = test_indexer
        .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items[0]
        .account
        .clone()
        .into();
    println!("only_test_create_pda 8");

    // Failing 4 input account that is not owned by signer ----------------------------------------------
    perform_with_input_accounts(
        &mut test_indexer,
        &mut rpc,
        &payer,
        None,
        &compressed_account,
        None,
        SystemProgramError::SignerCheckFailed.into(),
        WithInputAccountsMode::NotOwnedCompressedAccount,
    )
    .await
    .unwrap();
    {
        let compressed_account =
            test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&ID)[0].clone();
        // Failing 5 provide cpi context but no cpi context account ----------------------------------------------
        perform_with_input_accounts(
            &mut test_indexer,
            &mut rpc,
            &payer,
            None,
            &compressed_account,
            None,
            SystemProgramError::CpiContextMissing.into(),
            WithInputAccountsMode::CpiContextMissing,
        )
        .await
        .unwrap();
        println!("only_test_create_pda 9");

        // Failing 6 provide cpi context account but no cpi context ----------------------------------------------
        perform_with_input_accounts(
            &mut test_indexer,
            &mut rpc,
            &payer,
            None,
            &compressed_account,
            None,
            SystemProgramError::CpiContextAccountUndefined.into(),
            WithInputAccountsMode::CpiContextAccountMissing,
        )
        .await
        .unwrap();
        println!("only_test_create_pda 10");

        // Failing 7 provide cpi context account but cpi context is empty ----------------------------------------------
        perform_with_input_accounts(
            &mut test_indexer,
            &mut rpc,
            &payer,
            None,
            &compressed_account,
            None,
            SystemProgramError::CpiContextEmpty.into(),
            WithInputAccountsMode::CpiContextEmpty,
        )
        .await
        .unwrap();
        println!("only_test_create_pda 11");

        // Failing 8 test signer checks trying to insert into cpi context account (invalid invoking program) ----------------------------------------------
        perform_with_input_accounts(
            &mut test_indexer,
            &mut rpc,
            &payer,
            None,
            &compressed_account,
            None,
            SystemProgramError::CpiSignerCheckFailed.into(),
            WithInputAccountsMode::CpiContextInvalidInvokingProgram,
        )
        .await
        .unwrap();
        let compressed_token_account_data = test_indexer
            .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items[0]
            .clone();
        println!("only_test_create_pda 12");

        // Failing 10 provide cpi context account but cpi context has a different proof ----------------------------------------------
        perform_with_input_accounts(
            &mut test_indexer,
            &mut rpc,
            &payer,
            None,
            &compressed_account,
            Some(compressed_token_account_data.into()),
            SystemProgramError::CpiContextFeePayerMismatch.into(),
            WithInputAccountsMode::CpiContextFeePayerMismatch,
        )
        .await
        .unwrap();
        println!("only_test_create_pda 13");

        // Failing 11 write to account not owned ----------------------------------------------
        perform_with_input_accounts(
            &mut test_indexer,
            &mut rpc,
            &payer,
            None,
            &compressed_account,
            None,
            SystemProgramError::WriteAccessCheckFailed.into(),
            WithInputAccountsMode::CpiContextWriteToNotOwnedAccount,
        )
        .await
        .unwrap();
        println!("only_test_create_pda 14");

        // Failing 12 Spend with program keypair
        {
            const CPI_SYSTEM_TEST_PROGRAM_ID_KEYPAIR: [u8; 64] = [
                57, 80, 188, 3, 162, 80, 232, 181, 222, 192, 247, 98, 140, 227, 70, 15, 169, 202,
                73, 184, 23, 90, 69, 95, 211, 74, 128, 232, 155, 216, 5, 230, 213, 158, 155, 203,
                26, 211, 193, 195, 11, 219, 9, 155, 58, 172, 58, 200, 254, 75, 231, 106, 31, 168,
                183, 76, 179, 113, 234, 101, 191, 99, 156, 98,
            ];
            let compressed_account =
                test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&ID)[0].clone();

            let keypair = Keypair::try_from(CPI_SYSTEM_TEST_PROGRAM_ID_KEYPAIR.as_slice()).unwrap();
            let result = transfer_compressed_sol_test(
                &mut rpc,
                &mut test_indexer,
                &keypair,
                &[compressed_account],
                &[Pubkey::new_unique()],
                &[env.v1_state_trees[0].merkle_tree],
                None,
            )
            .await;
            assert_rpc_error(result, 0, SystemProgramError::SignerCheckFailed.into()).unwrap();
        }
        println!("only_test_create_pda 15");

        // Failing 13 DataFieldUndefined ----------------------------------------------
        perform_create_pda_failing(
            &mut rpc,
            &mut test_indexer,
            &env,
            &payer,
            seed,
            &data,
            &ID,
            CreatePdaMode::NoData,
            SystemProgramError::DataFieldUndefined.into(),
        )
        .await
        .unwrap();
    }
}

// TODO: add transfer and burn with delegate
// TODO: create a cleaner function than perform_with_input_accounts which was
// build for failing tests to execute the instructions
/// Functional Tests:
/// - tests the following methods with cpi context:
/// 1. Approve
/// 2. Revoke
/// 3. Freeze
/// 4. Thaw
/// 5. Burn
#[serial]
#[tokio::test]
async fn test_approve_revoke_burn_freeze_thaw_with_cpi_context() {
    let config = ProgramTestConfig::new(false, Some(vec![("system_cpi_test", ID)]));
    let mut rpc = LightProgramTest::new(config)
        .await
        .expect("Failed to setup test programs with accounts");
    rpc.indexer = None;
    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 0).await;
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    mint_tokens_helper(
        &mut rpc,
        &mut test_indexer,
        &env.v1_state_trees[0].merkle_tree,
        &payer,
        &mint,
        vec![amount],
        vec![payer.pubkey()],
    )
    .await;

    let seed = [1u8; 32];
    let data = [2u8; 31];
    perform_create_pda_with_event(
        &mut test_indexer,
        &mut rpc,
        &env,
        &payer,
        seed,
        &data,
        &ID,
        None,
        None,
        CreatePdaMode::ProgramIsSigner,
    )
    .await
    .unwrap();
    let delegate = Keypair::new();

    let ref_compressed_token_data = test_indexer
        .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items[0]
        .clone();
    // 1. Approve functional with cpi context
    {
        let compressed_account =
            test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&ID)[0].clone();
        let compressed_token_data = test_indexer
            .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items[0]
            .clone();
        perform_with_input_accounts(
            &mut test_indexer,
            &mut rpc,
            &payer,
            Some(&delegate),
            &compressed_account,
            Some(compressed_token_data.into()),
            u32::MAX,
            WithInputAccountsMode::Approve,
        )
        .await
        .unwrap();

        let compressed_token_data = test_indexer
            .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items[0]
            .clone();
        let mut ref_data = ref_compressed_token_data.token.clone();
        ref_data.delegate = Some(delegate.pubkey());
        assert_eq!(compressed_token_data.token, ref_data);
        assert_ne!(
            ref_compressed_token_data.account.leaf_index,
            compressed_token_data.account.leaf_index
        );
    }
    // 2. Revoke functional with cpi context
    {
        let compressed_account =
            test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&ID)[0].clone();
        let compressed_token_data = test_indexer
            .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items
            .iter()
            .filter(|x| x.token.delegate.is_some())
            .collect::<Vec<_>>()[0]
            .clone();

        perform_with_input_accounts(
            &mut test_indexer,
            &mut rpc,
            &payer,
            Some(&delegate),
            &compressed_account,
            Some(compressed_token_data.into()),
            u32::MAX,
            WithInputAccountsMode::Revoke,
        )
        .await
        .unwrap();

        let compressed_token_data = test_indexer
            .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items[0]
            .clone();
        let ref_data = ref_compressed_token_data.token.clone();
        assert_eq!(compressed_token_data.token, ref_data);
    }
    // 3. Freeze functional with cpi context
    {
        let compressed_account =
            test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&ID)[0].clone();
        let compressed_token_data = test_indexer
            .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items[0]
            .clone();
        perform_with_input_accounts(
            &mut test_indexer,
            &mut rpc,
            &payer,
            None,
            &compressed_account,
            Some(compressed_token_data.into()),
            u32::MAX,
            WithInputAccountsMode::Freeze,
        )
        .await
        .unwrap();
        let compressed_token_data = test_indexer
            .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items[0]
            .clone();
        let mut ref_data = ref_compressed_token_data.token.clone();
        ref_data.state = AccountState::Frozen;
        assert_eq!(compressed_token_data.token, ref_data);
    }
    // 4. Thaw functional with cpi context
    {
        let compressed_account =
            test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&ID)[0].clone();
        let compressed_token_data = test_indexer
            .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items[0]
            .clone();
        perform_with_input_accounts(
            &mut test_indexer,
            &mut rpc,
            &payer,
            None,
            &compressed_account,
            Some(compressed_token_data.into()),
            u32::MAX,
            WithInputAccountsMode::Thaw,
        )
        .await
        .unwrap();
        let compressed_token_data = test_indexer
            .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items[0]
            .clone();
        let ref_data = ref_compressed_token_data.token.clone();
        assert_eq!(compressed_token_data.token, ref_data);
    }
    // 5. Burn functional with cpi context
    {
        let compressed_account =
            test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&ID)[0].clone();
        let compressed_token_data = test_indexer
            .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
            .await
            .unwrap()
            .value
            .items[0]
            .clone();
        let res = perform_with_input_accounts(
            &mut test_indexer,
            &mut rpc,
            &payer,
            None,
            &compressed_account,
            Some(compressed_token_data.into()),
            u32::MAX,
            WithInputAccountsMode::Burn,
        )
        .await;
        assert_rpc_error(
            res,
            0,
            light_compressed_token::ErrorCode::CpiContextSetNotUsable.into(),
        )
        .unwrap();
    }
}

/// Test:
/// 1. Cannot create an address in a program owned address Merkle tree owned by a different program (InvalidMerkleTreeOwner)
/// 2. Cannot create a compressed account in a program owned state Merkle tree owned by a different program (InvalidMerkleTreeOwner)
/// 3. Create a compressed account and address in program owned state and address Merkle trees
#[serial]
#[tokio::test]
async fn test_create_pda_in_program_owned_merkle_trees() {
    let config = ProgramTestConfig::new(false, Some(vec![("system_cpi_test", ID)]));
    let mut rpc = LightProgramTest::new(config)
        .await
        .expect("Failed to setup test programs with accounts");
    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();
    let mut test_indexer = TestIndexer::init_from_acounts(&payer, &env, 0).await;
    // Failing test 1 invalid address Merkle tree ----------------------------------------------
    let program_owned_address_merkle_tree_keypair = Keypair::new();
    let program_owned_address_queue_keypair = Keypair::new();

    test_indexer
        .add_address_merkle_tree(
            &mut rpc,
            &program_owned_address_merkle_tree_keypair,
            &program_owned_address_queue_keypair,
            Some(light_compressed_token::ID),
            TreeType::AddressV1,
        )
        .await
        .unwrap();
    rpc.indexer.as_mut().unwrap().address_merkle_trees = test_indexer.address_merkle_trees.clone();
    let mut env_with_program_owned_address_merkle_tree = env.clone();
    env_with_program_owned_address_merkle_tree.v1_address_trees = vec![AddressMerkleTreeAccounts {
        merkle_tree: program_owned_address_merkle_tree_keypair.pubkey(),
        queue: program_owned_address_queue_keypair.pubkey(),
    }];

    perform_create_pda_failing(
        &mut rpc,
        &mut test_indexer,
        &env_with_program_owned_address_merkle_tree,
        &payer,
        [3u8; 32],
        &[4u8; 31],
        &ID,
        CreatePdaMode::ProgramIsSigner,
        SystemProgramError::InvalidMerkleTreeOwner.into(),
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
            None,
            None,
            TreeType::StateV1,
        )
        .await;
    rpc.indexer.as_mut().unwrap().state_merkle_trees = test_indexer.state_merkle_trees.clone();

    let mut env_with_program_owned_state_merkle_tree = env.clone();

    env_with_program_owned_state_merkle_tree.v1_state_trees = vec![StateMerkleTreeAccounts {
        merkle_tree: program_owned_state_merkle_tree_keypair.pubkey(),
        nullifier_queue: program_owned_state_queue_keypair.pubkey(),
        cpi_context: program_owned_cpi_context_keypair.pubkey(),
        tree_type: TreeType::StateV1,
    }];
    // Program owned state trees are deprecated.
    // perform_create_pda_failing(
    //     &mut rpc,
    //     &mut test_indexer,
    //     &env_with_program_owned_state_merkle_tree,
    //     &payer,
    //     [3u8; 32],
    //     &[4u8; 31],
    //     &ID,
    //     CreatePdaMode::ProgramIsSigner,
    //     SystemProgramError::InvalidMerkleTreeOwner.into(),
    // )
    // .await
    // .unwrap();

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
            None,
            None,
            TreeType::StateV1,
        )
        .await;
    rpc.indexer.as_mut().unwrap().state_merkle_trees = test_indexer.state_merkle_trees.clone();
    let program_owned_address_merkle_tree_keypair = Keypair::new();
    let program_owned_address_queue_keypair = Keypair::new();

    test_indexer
        .add_address_merkle_tree(
            &mut rpc,
            &program_owned_address_merkle_tree_keypair,
            &program_owned_address_queue_keypair,
            Some(ID),
            TreeType::AddressV1,
        )
        .await
        .unwrap();
    rpc.indexer.as_mut().unwrap().address_merkle_trees = test_indexer.address_merkle_trees.clone();

    let mut env_with_program_owned_state_merkle_tree = env.clone();
    env_with_program_owned_state_merkle_tree.v1_address_trees = vec![AddressMerkleTreeAccounts {
        merkle_tree: program_owned_address_merkle_tree_keypair.pubkey(),
        queue: program_owned_address_queue_keypair.pubkey(),
    }];
    env_with_program_owned_state_merkle_tree.v1_state_trees = vec![StateMerkleTreeAccounts {
        merkle_tree: program_owned_state_merkle_tree_keypair.pubkey(),
        nullifier_queue: program_owned_state_queue_keypair.pubkey(),
        cpi_context: program_owned_cpi_context_keypair.pubkey(),
        tree_type: TreeType::StateV1,
    }];

    // TestAccounts {
    //     address_merkle_tree_pubkey: program_owned_address_merkle_tree_keypair.pubkey(),
    //     address_merkle_tree_queue_pubkey: program_owned_address_queue_keypair.pubkey(),
    //     merkle_tree_pubkey: program_owned_state_merkle_tree_keypair.pubkey(),
    //     nullifier_queue_pubkey: program_owned_state_queue_keypair.pubkey(),
    //     cpi_context_account_pubkey: program_owned_cpi_context_keypair.pubkey(),
    //     governance_authority: env.protocol.governance_authority.insecure_clone(),
    //     governance_authority_pda: env.protocol.governance_authority_pda,
    //     group_pda: env.protocol.group_pda,
    //     registered_program_pda: env.protocol.registered_program_pda,
    //     registered_registry_program_pda: env.protocol.registered_registry_program_pda,
    //     forester: env.protocol.forester.insecure_clone(),
    //     registered_forester_pda: env.protocol.registered_forester_pda,
    //     forester_epoch: env.protocol.forester_epoch.clone(),
    //     batched_cpi_context: env.batched_cpi_context,
    //     batched_output_queue: env.v2_state_trees[0].output_queue,
    //     batched_state_merkle_tree: env.v2_state_trees[0].merkle_tree,
    //     batch_address_merkle_tree: env.v2_address_trees[0],
    // };
    let seed = [4u8; 32];
    let data = [5u8; 31];
    perform_create_pda_with_event(
        &mut test_indexer,
        &mut rpc,
        &env_with_program_owned_state_merkle_tree,
        &payer,
        seed,
        &data,
        &ID,
        None,
        None,
        CreatePdaMode::ProgramIsSigner,
    )
    .await
    .unwrap();

    assert_created_pda::<LightProgramTest, TestIndexer>(
        &mut test_indexer,
        &env_with_program_owned_state_merkle_tree,
        &payer,
        &seed,
        &data,
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
pub async fn perform_create_pda_failing<R: Rpc, I: Indexer + TestIndexerExtensions>(
    rpc: &mut R,
    test_indexer: &mut I,
    env: &TestAccounts,
    payer: &Keypair,
    seed: [u8; 32],
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
        data,
        payer_pubkey,
        owner_program,
        None,
        None,
        signer_is_program,
    )
    .await;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        rpc.get_latest_blockhash().await.unwrap().0,
    );
    let result = rpc.process_transaction(transaction).await;
    assert_rpc_error(result, 0, expected_error_code)
}

#[allow(clippy::too_many_arguments)]
pub async fn perform_create_pda_with_event<R: Rpc, I: Indexer + TestIndexerExtensions>(
    test_indexer: &mut I,
    rpc: &mut R,
    env: &TestAccounts,
    payer: &Keypair,
    seed: [u8; 32],
    data: &[u8; 31],
    owner_program: &Pubkey,
    input_accounts: Option<Vec<CompressedAccountWithMerkleContext>>,
    read_only_accounts: Option<Vec<CompressedAccountWithMerkleContext>>,
    mode: CreatePdaMode,
) -> Result<(), RpcError> {
    let payer_pubkey = payer.pubkey();
    let mut instructions = vec![
        perform_create_pda(
            env,
            seed,
            test_indexer,
            data,
            payer_pubkey,
            owner_program,
            input_accounts,
            read_only_accounts.clone(),
            mode.clone(),
        )
        .await,
    ];
    // create instruction which invalidates account
    if mode == CreatePdaMode::ReadOnlyZkpOfInsertedAccount
        || mode == CreatePdaMode::ReadOnlyProofOfInsertedAccount
    {
        instructions.push(instructions[0].clone());
    }

    let event = rpc
        .create_and_send_transaction_with_public_event(&instructions, &payer_pubkey, &[payer])
        .await?;
    if let Some(event) = event {
        let slot: u64 = rpc.get_slot().await.unwrap();
        test_indexer.add_compressed_accounts_with_token_data(slot, &event.0);
    } else if mode != CreatePdaMode::TwoReadOnlyAddresses {
        println!("mode {:?}", mode);
        return Err(RpcError::CustomError("NoEvent".to_string()));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn perform_create_pda<I: Indexer + TestIndexerExtensions>(
    env: &TestAccounts,
    seed: [u8; 32],
    test_indexer: &mut I,
    data: &[u8; 31],
    payer_pubkey: Pubkey,
    owner_program: &Pubkey,
    input_accounts: Option<Vec<CompressedAccountWithMerkleContext>>,
    read_only_accounts: Option<Vec<CompressedAccountWithMerkleContext>>,
    mode: CreatePdaMode,
) -> solana_sdk::instruction::Instruction {
    let output_compressed_account_merkle_tree_pubkey = if mode == CreatePdaMode::BatchFunctional {
        &env.v2_state_trees[0].output_queue
    } else {
        &env.v1_state_trees[0].merkle_tree
    };
    let (address, mut address_merkle_tree_pubkey, address_queue_pubkey) = if mode
        == CreatePdaMode::BatchAddressFunctional
        || mode == CreatePdaMode::InvalidReadOnlyAddress
        || mode == CreatePdaMode::InvalidReadOnlyMerkleTree
        || mode == CreatePdaMode::InvalidReadOnlyRootIndex
        || mode == CreatePdaMode::TwoReadOnlyAddresses
        || mode == CreatePdaMode::OneReadOnlyAddress
        || mode == CreatePdaMode::ReadOnlyProofOfInsertedAddress
        || mode == CreatePdaMode::UseReadOnlyAddressInAccount
        || mode == CreatePdaMode::BatchFunctional
        || mode == CreatePdaMode::InvalidReadOnlyAccountOutputQueue
        || mode == CreatePdaMode::ProofIsNoneReadOnlyAccount
        || mode == CreatePdaMode::InvalidProofReadOnlyAccount
        || mode == CreatePdaMode::InvalidReadOnlyAccountRootIndex
        || mode == CreatePdaMode::InvalidReadOnlyAccount
        || mode == CreatePdaMode::InvalidReadOnlyAccountMerkleTree
        || mode == CreatePdaMode::AccountNotInValueVecMarkedProofByIndex
        || mode == CreatePdaMode::ReadOnlyZkpOfInsertedAccount
    {
        let address = derive_address(
            &seed,
            &env.v2_address_trees[0].to_bytes(),
            &system_cpi_test::ID.to_bytes(),
        );

        (address, env.v2_address_trees[0], env.v2_address_trees[0])
    } else {
        let address =
            derive_address_legacy(&env.v1_address_trees[0].merkle_tree.into(), &seed).unwrap();
        (
            address,
            env.v1_address_trees[0].merkle_tree,
            env.v1_address_trees[0].queue,
        )
    };
    let mut addresses = vec![address];
    let mut address_merkle_tree_pubkeys = vec![address_merkle_tree_pubkey];
    // InvalidReadOnlyAddress add address to proof but don't send in the instruction
    if mode == CreatePdaMode::OneReadOnlyAddress
        || mode == CreatePdaMode::InvalidReadOnlyAddress
        || mode == CreatePdaMode::InvalidReadOnlyMerkleTree
        || mode == CreatePdaMode::InvalidReadOnlyRootIndex
        || mode == CreatePdaMode::ReadOnlyProofOfInsertedAddress
        || mode == CreatePdaMode::UseReadOnlyAddressInAccount
    {
        let mut read_only_address = hash_to_bn254_field_size_be(&Pubkey::new_unique().to_bytes());
        read_only_address[30] = 0;
        read_only_address[29] = 0;
        addresses.push(read_only_address);
        address_merkle_tree_pubkeys.push(address_merkle_tree_pubkey);
    }
    if mode == CreatePdaMode::TwoReadOnlyAddresses {
        let mut read_only_address = hash_to_bn254_field_size_be(&Pubkey::new_unique().to_bytes());
        read_only_address[30] = 0;
        read_only_address[29] = 0;
        addresses.insert(0, read_only_address);
        address_merkle_tree_pubkeys.push(address_merkle_tree_pubkey);
    }
    let mut compressed_account_hashes = Vec::new();
    let mut compressed_account_merkle_tree_pubkeys = Vec::new();
    if let Some(input_accounts) = input_accounts.as_ref() {
        input_accounts.iter().for_each(|x| {
            compressed_account_hashes.push(x.hash().unwrap());
            compressed_account_merkle_tree_pubkeys.push(x.merkle_context.merkle_tree_pubkey);
        });
    }
    if let Some(read_only_accounts) = read_only_accounts.as_ref() {
        read_only_accounts.iter().for_each(|x| {
            compressed_account_hashes.push(x.hash().unwrap());
            compressed_account_merkle_tree_pubkeys.push(x.merkle_context.merkle_tree_pubkey);
        });
    }
    let hashes = if compressed_account_hashes.is_empty() {
        Vec::new()
    } else {
        compressed_account_hashes
    };

    let addresses_with_tree = addresses
        .iter()
        .zip(address_merkle_tree_pubkeys.iter())
        .map(|(address, tree)| AddressWithTree {
            address: *address,
            tree: *tree,
        })
        .collect::<Vec<_>>();
    println!("get_validity_proof_v2 inputs: hashes {:?}", hashes);
    println!(
        "get_validity_proof_v2 inputs: addresses_with_tree {:?}",
        addresses_with_tree
    );

    let rpc_result = test_indexer
        .get_validity_proof(hashes, addresses_with_tree, None)
        .await
        .unwrap();
    if mode == CreatePdaMode::InvalidBatchTreeAccount {
        address_merkle_tree_pubkey = env.v1_state_trees[0].merkle_tree;
    }
    let address_root_indices: Vec<_> = rpc_result
        .value
        .addresses
        .iter()
        .map(|x| x.root_index)
        .collect();
    let account_root_indices: Vec<_> = rpc_result
        .value
        .accounts
        .iter()
        .map(|x| x.root_index)
        .collect();
    let new_address_params = NewAddressParams {
        seed,
        address_merkle_tree_pubkey: address_merkle_tree_pubkey.into(),
        address_queue_pubkey: address_queue_pubkey.into(),
        address_merkle_tree_root_index: address_root_indices[0],
    };
    let readonly_adresses = if addresses.len() == 2 && mode != CreatePdaMode::TwoReadOnlyAddresses {
        let read_only_address = vec![ReadOnlyAddress {
            address: addresses[1],
            address_merkle_tree_pubkey: address_merkle_tree_pubkey.into(),
            address_merkle_tree_root_index: address_root_indices[1],
        }];
        Some(read_only_address)
    } else if mode == CreatePdaMode::TwoReadOnlyAddresses {
        let read_only_address = vec![
            ReadOnlyAddress {
                address: addresses[0],
                address_merkle_tree_pubkey: address_merkle_tree_pubkey.into(),
                address_merkle_tree_root_index: address_root_indices[0],
            },
            ReadOnlyAddress {
                address: addresses[1],
                address_merkle_tree_pubkey: address_merkle_tree_pubkey.into(),
                address_merkle_tree_root_index: address_root_indices[1],
            },
        ];
        Some(read_only_address)
    } else {
        None
    };
    let mut index = 0;
    let state_roots = if input_accounts.as_ref().is_none() {
        None
    } else {
        let input_account_len = input_accounts.as_ref().unwrap().len();
        index += input_account_len;
        Some(
            account_root_indices[..index]
                .iter()
                .map(|x| x.root_index())
                .collect::<Vec<_>>(),
        )
    };

    let read_only_accounts = read_only_accounts.as_ref().map(|read_only_accounts| {
        read_only_accounts
            .iter()
            .map(|x| {
                index += 1;
                x.into_read_only(account_root_indices[index - 1].root_index())
                    .unwrap()
            })
            .collect::<Vec<_>>()
    });

    let create_ix_inputs = CreateCompressedPdaInstructionInputs {
        data: *data,
        signer: &payer_pubkey,
        output_compressed_account_merkle_tree_pubkey,
        proof: &rpc_result.value.proof.0.unwrap(),
        new_address_params,
        cpi_context_account: &env.v1_state_trees[0].cpi_context,
        owner_program,
        signer_is_program: mode.clone(),
        registered_program_pda: &env.protocol.registered_program_pda,
        readonly_adresses,
        read_only_accounts,
        input_compressed_accounts_with_merkle_context: input_accounts,
        state_roots,
    };
    create_pda_instruction(create_ix_inputs)
}

pub async fn assert_created_pda<R: Rpc, I: Indexer + TestIndexerExtensions>(
    test_indexer: &mut I,
    env: &TestAccounts,
    payer: &Keypair,
    seed: &[u8; 32],
    data: &[u8; 31],
) {
    let compressed_escrow_pda =
        test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&ID)[0].clone();
    let address = derive_address_legacy(&env.v1_address_trees[0].merkle_tree.into(), seed).unwrap();
    assert_eq!(
        compressed_escrow_pda.compressed_account.address.unwrap(),
        address
    );
    assert_eq!(
        compressed_escrow_pda.compressed_account.owner.to_bytes(),
        ID.to_bytes()
    );
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
        hash_to_bn254_field_size_be(&compressed_escrow_pda_data.user_pubkey.to_bytes());

    assert_eq!(
        compressed_escrow_pda_deserialized.data_hash,
        Poseidon::hashv(&[truncated_user_pubkey.as_slice(), data.as_slice()]).unwrap(),
    );
}

#[allow(clippy::too_many_arguments)]
pub async fn perform_with_input_accounts<R: Rpc, I: Indexer + TestIndexerExtensions>(
    test_indexer: &mut I,
    rpc: &mut R,
    payer: &Keypair,
    fee_payer: Option<&Keypair>,
    compressed_account: &CompressedAccountWithMerkleContext,
    token_account: Option<TokenDataWithMerkleContext>,
    expected_error_code: u32,
    mode: WithInputAccountsMode,
) -> Result<(), RpcError> {
    let payer_pubkey = payer.pubkey();
    let hash = compressed_account.hash().unwrap();
    let mut hashes = vec![hash];
    let mut merkle_tree_pubkeys = vec![compressed_account.merkle_context.merkle_tree_pubkey];
    if let Some(token_account) = token_account.as_ref() {
        hashes.push(token_account.compressed_account.hash().unwrap());
        merkle_tree_pubkeys.push(
            token_account
                .compressed_account
                .merkle_context
                .merkle_tree_pubkey,
        );
    }
    // CPi context order changed, since it is always 2 accounts in this case it is reversed.
    hashes.reverse();
    let merkle_tree_pubkey = compressed_account.merkle_context.merkle_tree_pubkey;
    let nullifier_pubkey = compressed_account.merkle_context.queue_pubkey;
    let cpi_context = match mode {
        WithInputAccountsMode::Freeze
        | WithInputAccountsMode::Thaw
        | WithInputAccountsMode::Burn
        | WithInputAccountsMode::Approve
        | WithInputAccountsMode::Revoke
        | WithInputAccountsMode::CpiContextMissing
        | WithInputAccountsMode::CpiContextAccountMissing
        | WithInputAccountsMode::CpiContextInvalidInvokingProgram
        | WithInputAccountsMode::CpiContextFeePayerMismatch
        | WithInputAccountsMode::CpiContextWriteToNotOwnedAccount => Some(CompressedCpiContext {
            cpi_context_account_index: 2,
            set_context: true,
            first_set_context: true,
        }),
        WithInputAccountsMode::CpiContextEmpty => Some(CompressedCpiContext {
            cpi_context_account_index: 2,
            set_context: false,
            first_set_context: false,
        }),
        _ => None,
    };
    let cpi_context_account_pubkey = test_indexer
        .get_state_merkle_trees()
        .iter()
        .find(|x| x.accounts.merkle_tree.to_bytes() == merkle_tree_pubkey.to_bytes())
        .unwrap()
        .accounts
        .cpi_context;
    let rpc_result = test_indexer
        .get_validity_proof(
            hashes,
            Vec::new(), // No addresses needed
            None,
        )
        .await
        .unwrap();

    let token_transfer_data = match token_account {
        Some(token_account) => Some(TokenTransferData {
            mint: token_account.token_data.mint,
            input_token_data_with_context: InputTokenDataWithContext {
                amount: token_account.token_data.amount,
                delegate_index: if token_account.token_data.delegate.is_some() {
                    Some(3)
                } else {
                    None
                },
                root_index: rpc_result.value.accounts[0]
                    .root_index
                    .root_index()
                    .unwrap(),
                merkle_context: PackedMerkleContext {
                    leaf_index: token_account.compressed_account.merkle_context.leaf_index,
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: 1,
                    prove_by_index: false,
                },
                lamports: if token_account.compressed_account.compressed_account.lamports != 0 {
                    Some(token_account.compressed_account.compressed_account.lamports)
                } else {
                    None
                },
                tlv: None,
            },
        }),
        _ => None,
    };
    let invalid_fee_payer = if let Some(fee_payer) = fee_payer {
        fee_payer
    } else {
        &Keypair::new()
    };
    let create_ix_inputs = InvalidateNotOwnedCompressedAccountInstructionInputs {
        signer: &payer_pubkey,
        input_merkle_tree_pubkey: &merkle_tree_pubkey.into(),
        input_nullifier_pubkey: &nullifier_pubkey.into(),
        cpi_context_account: &cpi_context_account_pubkey,
        cpi_context,
        proof: &rpc_result.value.proof.0.unwrap(),
        compressed_account: &PackedCompressedAccountWithMerkleContext {
            compressed_account: compressed_account.compressed_account.clone(),
            merkle_context: PackedMerkleContext {
                leaf_index: compressed_account.merkle_context.leaf_index,
                merkle_tree_pubkey_index: 0,
                queue_pubkey_index: 1,
                prove_by_index: false,
            },
            root_index: rpc_result.value.accounts[0]
                .root_index
                .root_index()
                .unwrap(),
            read_only: false,
        },
        token_transfer_data,
        invalid_fee_payer: &invalid_fee_payer.pubkey(),
    };
    let instruction =
        create_invalidate_not_owned_account_instruction(create_ix_inputs.clone(), mode);
    let result = rpc
        .create_and_send_transaction_with_public_event(
            &[instruction],
            &payer_pubkey,
            &[payer, invalid_fee_payer],
        )
        .await;
    if expected_error_code == u32::MAX {
        let result = result?.unwrap();
        let slot: u64 = rpc.get_slot().await.unwrap();
        test_indexer.add_compressed_accounts_with_token_data(slot, &result.0);
        Ok(())
    } else {
        assert_rpc_error(result, 0, expected_error_code)
    }
}
