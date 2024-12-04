#![cfg(feature = "test-sbf")]

use account_compression::batched_merkle_tree::{
    get_merkle_tree_account_size, AppendBatchProofInputsIx, BatchProofInputsIx,
    InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
    ZeroCopyBatchedMerkleTreeAccount,
};
use account_compression::batched_queue::{
    assert_queue_zero_copy_inited, get_output_queue_account_size, BatchedQueueAccount,
    ZeroCopyBatchedQueueAccount,
};
use account_compression::errors::AccountCompressionErrorCode;
use account_compression::{assert_mt_zero_copy_inited, get_output_queue_account_default};
use account_compression::{
    batched_merkle_tree::BatchedMerkleTreeAccount, InitStateTreeAccountsInstructionData, ID,
};
use anchor_lang::error::ErrorCode;
use anchor_lang::prelude::AccountMeta;
use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};
use light_prover_client::gnark::helpers::{spawn_prover, ProofType, ProverConfig};
use light_prover_client::mock_batched_forester::{MockBatchedForester, MockTxEvent};
use light_system_program::invoke::verify_state_proof::create_tx_hash_offchain;
use light_test_utils::test_batch_forester::assert_perform_state_mt_roll_over;
use light_test_utils::test_env::NOOP_PROGRAM_ID;
use light_test_utils::{
    airdrop_lamports, assert_rpc_error, create_account_instruction, RpcConnection, RpcError,
};
use light_test_utils::{rpc::ProgramTestRpcConnection, AccountZeroCopy};
use light_verifier::CompressedProof;
use serial_test::serial;
use solana_program_test::ProgramTest;
use solana_sdk::account::WritableAccount;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signer},
};

pub enum TestMode {
    InvalidMerkleTree,
    InvalidOutputQueue,
    Functional,
    InvalidRegisteredProgram,
}

/// 1.  init accounts       - Functional: initialize a batched Merkle tree and output queue
/// 2.  append leaves       - Failing: Invalid signe
/// 3.  append leaves       - Functional insert 10 leaves into output queue
/// 4.  batch append        - Failing: Invalid Signer
/// 5.  batch append        - Failing: Invalid Output queue - association
/// 6.  batch append        - Failing: append Invalid Merkle tree
/// 7.  batch append        - Failing: Invalid Registered Program
/// 8.  batch append        - Functional: batch append 10 leaves
/// 9.  insert_into_queue   - Failing Invalid authority (input queue)
/// 10. insert_into_queue   - Failing Invalid Merkle tree - association
/// 11. insert_into_queue   - Functional insert 10 leaves into input queue
/// 12. batch nullify       - Failing Invalid authority
/// 13. batch nullify       - Failing Invalid merkle tree
/// 14. batch nullify       - Failing Invalid registered program
/// 15. batch nullify       - Functional batch nullify 10 leaves
#[serial]
#[tokio::test]
async fn test_batch_state_merkle_tree() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.add_program(
        "spl_noop",
        Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        None,
    );
    let merkle_tree_keypair = Keypair::new();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let nullifier_queue_keypair = Keypair::new();
    let output_queue_pubkey = nullifier_queue_keypair.pubkey();
    program_test.set_compute_max_units(1_400_000u64);
    let context = program_test.start_with_context().await;
    let mut context = ProgramTestRpcConnection { context };
    let payer_pubkey = context.get_payer().pubkey();
    let payer = context.get_payer().insecure_clone();
    // 1. Functional initialize a batched Merkle tree and output queue
    {
        let params = InitStateTreeAccountsInstructionData::test_default();
        let queue_account_size = get_output_queue_account_size(
            params.output_queue_batch_size,
            params.output_queue_zkp_batch_size,
            params.output_queue_num_batches,
        );
        let mt_account_size = get_merkle_tree_account_size(
            params.input_queue_batch_size,
            params.bloom_filter_capacity,
            params.input_queue_zkp_batch_size,
            params.root_history_capacity,
            params.height,
            params.input_queue_num_batches,
        );
        let queue_rent = context
            .get_minimum_balance_for_rent_exemption(queue_account_size)
            .await
            .unwrap();
        let create_queue_account_ix = create_account_instruction(
            &payer_pubkey,
            queue_account_size,
            queue_rent,
            &ID,
            Some(&nullifier_queue_keypair),
        );
        let mt_rent = context
            .get_minimum_balance_for_rent_exemption(mt_account_size)
            .await
            .unwrap();
        let additional_bytes_rent = context
            .get_minimum_balance_for_rent_exemption(params.additional_bytes as usize)
            .await
            .unwrap();
        let total_rent = queue_rent + mt_rent + additional_bytes_rent;
        let create_mt_account_ix = create_account_instruction(
            &payer_pubkey,
            mt_account_size,
            mt_rent,
            &ID,
            Some(&merkle_tree_keypair),
        );

        let instruction =
            account_compression::instruction::InitializeBatchedStateMerkleTree { params };
        let accounts = account_compression::accounts::InitializeBatchedStateMerkleTreeAndQueue {
            authority: context.get_payer().pubkey(),
            merkle_tree: merkle_tree_pubkey,
            queue: output_queue_pubkey,
            registered_program_pda: None,
        };

        let instruction = Instruction {
            program_id: ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction.data(),
        };
        context
            .create_and_send_transaction(
                &[create_queue_account_ix, create_mt_account_ix, instruction],
                &payer_pubkey,
                &[&payer, &nullifier_queue_keypair, &merkle_tree_keypair],
            )
            .await
            .unwrap();
        let mut merkle_tree =
            AccountZeroCopy::<BatchedMerkleTreeAccount>::new(&mut context, merkle_tree_pubkey)
                .await;

        let mut queue =
            AccountZeroCopy::<BatchedQueueAccount>::new(&mut context, output_queue_pubkey).await;
        let owner = context.get_payer().pubkey();

        let ref_mt_account = BatchedMerkleTreeAccount::get_state_tree_default(
            owner,
            None,
            None,
            params.rollover_threshold,
            0,
            params.network_fee.unwrap_or_default(),
            params.input_queue_batch_size,
            params.input_queue_zkp_batch_size,
            params.bloom_filter_capacity,
            params.root_history_capacity,
            output_queue_pubkey,
            params.height,
            params.input_queue_num_batches,
        );

        assert_mt_zero_copy_inited(
            &mut merkle_tree.account.data.as_mut_slice(),
            ref_mt_account,
            params.bloom_filter_num_iters,
        );

        let ref_output_queue_account = get_output_queue_account_default(
            owner,
            None,
            None,
            params.rollover_threshold,
            0,
            params.output_queue_batch_size,
            params.output_queue_zkp_batch_size,
            params.additional_bytes,
            total_rent,
            merkle_tree_pubkey,
            params.height,
            params.output_queue_num_batches,
            params.network_fee.unwrap_or_default(),
        );
        assert_queue_zero_copy_inited(
            &mut queue.account.data.as_mut_slice(),
            ref_output_queue_account,
            0,
        );
    }
    let mut mock_indexer = MockBatchedForester::<26>::default();
    let invalid_payer = Keypair::new();
    context
        .airdrop_lamports(&invalid_payer.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    // 2. Failing: Invalid signer (insert into output queue)
    {
        let mut mock_indexer = mock_indexer.clone();
        let result = perform_insert_into_output_queue(
            &mut context,
            &mut mock_indexer,
            output_queue_pubkey,
            &invalid_payer,
            &mut 0,
            5,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            AccountCompressionErrorCode::InvalidAuthority.into(),
        )
        .unwrap();
    }
    // 3. Functional: insert 10 leaves into output queue
    let num_of_leaves = 10;
    let num_tx = 5;
    let mut counter = 0;
    for _ in 0..num_tx {
        perform_insert_into_output_queue(
            &mut context,
            &mut mock_indexer,
            output_queue_pubkey,
            &payer,
            &mut counter,
            num_of_leaves,
        )
        .await
        .unwrap();
    }
    spawn_prover(
        true,
        ProverConfig {
            run_mode: None,
            circuits: vec![
                ProofType::BatchAppendWithProofsTest,
                ProofType::BatchUpdateTest,
            ],
        },
    )
    .await;

    // 4. Failing Invalid Signer (batch append)
    {
        let mut mock_indexer = mock_indexer.clone();
        let result = perform_batch_append(
            &mut context,
            &mut mock_indexer,
            merkle_tree_pubkey,
            output_queue_pubkey,
            &invalid_payer,
            TestMode::Functional,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            AccountCompressionErrorCode::InvalidAuthority.into(),
        )
        .unwrap();
    }
    // 5. Failing Invalid Output queue - association (batch append)
    {
        let mut mock_indexer = mock_indexer.clone();
        let result = perform_batch_append(
            &mut context,
            &mut mock_indexer,
            merkle_tree_pubkey,
            output_queue_pubkey,
            &payer,
            TestMode::InvalidOutputQueue,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
        )
        .unwrap();
    }
    // 6. Failing append Invalid Merkle tree (batch append)
    {
        let mut mock_indexer = mock_indexer.clone();
        let result = perform_batch_append(
            &mut context,
            &mut mock_indexer,
            merkle_tree_pubkey,
            output_queue_pubkey,
            &payer,
            TestMode::InvalidMerkleTree,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            AccountCompressionErrorCode::InvalidDiscriminator.into(),
        )
        .unwrap();
    }
    // 7. Failing Invalid Registered Program (batch append)
    {
        let mut mock_indexer = mock_indexer.clone();
        let result = perform_batch_append(
            &mut context,
            &mut mock_indexer,
            merkle_tree_pubkey,
            output_queue_pubkey,
            &payer,
            TestMode::InvalidRegisteredProgram,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into(),
        )
        .unwrap();
    }

    // 8. Functional batch append 10 leaves
    for _ in 0..num_tx {
        perform_batch_append(
            &mut context,
            &mut mock_indexer,
            merkle_tree_pubkey,
            output_queue_pubkey,
            &payer,
            TestMode::Functional,
        )
        .await
        .unwrap();
    }

    // 9. Failing Invalid authority (insert into nullifier queue)
    {
        let mut mock_indexer = mock_indexer.clone();
        let result = perform_insert_into_input_queue(
            &mut context,
            &mut mock_indexer,
            &mut 0,
            10,
            output_queue_pubkey,
            merkle_tree_pubkey,
            &invalid_payer,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            AccountCompressionErrorCode::InvalidAuthority.into(),
        )
        .unwrap();
    }

    // 10. Failing Invalid Merkle tree - association (insert into nullifier queue)
    {
        let mut mock_indexer = mock_indexer.clone();
        let result = perform_insert_into_input_queue(
            &mut context,
            &mut mock_indexer,
            &mut 0,
            10,
            output_queue_pubkey,
            output_queue_pubkey,
            &invalid_payer,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
        )
        .unwrap();
    }
    // 11. Functional insert 10 leaves into input queue
    let num_of_leaves = 10;
    let num_tx = 5;
    let mut counter = 0;
    for _ in 0..num_tx {
        perform_insert_into_input_queue(
            &mut context,
            &mut mock_indexer,
            &mut counter,
            num_of_leaves,
            output_queue_pubkey,
            merkle_tree_pubkey,
            &payer,
        )
        .await
        .unwrap();
    }
    // 12. Failing Invalid authority (batch nullify)
    {
        let mut mock_indexer = mock_indexer.clone();
        let result = perform_batch_nullify(
            &mut context,
            &mut mock_indexer,
            merkle_tree_pubkey,
            output_queue_pubkey,
            &invalid_payer,
            TestMode::Functional,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            AccountCompressionErrorCode::InvalidAuthority.into(),
        )
        .unwrap();
    }
    // 13. Failing Invalid merkle tree (batch nullify)
    {
        let mut mock_indexer = mock_indexer.clone();
        let result = perform_batch_nullify(
            &mut context,
            &mut mock_indexer,
            merkle_tree_pubkey,
            output_queue_pubkey,
            &payer,
            TestMode::InvalidMerkleTree,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            AccountCompressionErrorCode::InvalidDiscriminator.into(),
        )
        .unwrap();
    }
    // 14. Failing Invalid registered program (batch nullify)
    {
        let mut mock_indexer = mock_indexer.clone();
        let result = perform_batch_nullify(
            &mut context,
            &mut mock_indexer,
            merkle_tree_pubkey,
            output_queue_pubkey,
            &invalid_payer,
            TestMode::InvalidRegisteredProgram,
        )
        .await;
        assert_rpc_error(
            result,
            0,
            anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into(),
        )
        .unwrap();
    }
    // 15. Functional batch nullify 10 leaves
    for i in 0..num_tx {
        println!("nullify leaves tx: {:?}", i);
        perform_batch_nullify(
            &mut context,
            &mut mock_indexer,
            merkle_tree_pubkey,
            output_queue_pubkey,
            &payer,
            TestMode::Functional,
        )
        .await
        .unwrap();
    }
}

pub async fn perform_insert_into_output_queue(
    context: &mut ProgramTestRpcConnection,
    mock_indexer: &mut MockBatchedForester<26>,
    output_queue_pubkey: Pubkey,
    payer: &Keypair,
    counter: &mut u32,
    num_of_leaves: u32,
) -> Result<Signature, RpcError> {
    let mut leaves = vec![];
    for _ in 0..num_of_leaves {
        let mut leaf = [0u8; 32];
        leaf[31] = *counter as u8;
        leaves.push((0, leaf));
        mock_indexer.output_queue_leaves.push(leaf);
        mock_indexer.tx_events.push(MockTxEvent {
            tx_hash: [0u8; 32],
            inputs: vec![],
            outputs: vec![leaf],
        });
        *counter += 1;
    }

    let instruction = account_compression::instruction::AppendLeavesToMerkleTrees { leaves };
    let accounts = account_compression::accounts::InsertIntoQueues {
        authority: payer.pubkey(),
        fee_payer: payer.pubkey(),
        registered_program_pda: None,
        system_program: Pubkey::default(),
    };
    let accounts = vec![
        accounts.to_account_metas(Some(true)),
        vec![AccountMeta {
            pubkey: output_queue_pubkey,
            is_signer: false,
            is_writable: true,
        }],
    ]
    .concat();

    let instruction = Instruction {
        program_id: ID,
        accounts,
        data: instruction.data(),
    };
    context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
}
pub async fn perform_batch_append(
    context: &mut ProgramTestRpcConnection,
    mock_indexer: &mut MockBatchedForester<26>,
    merkle_tree_pubkey: Pubkey,
    output_queue_pubkey: Pubkey,
    payer: &Keypair,
    mode: TestMode,
) -> Result<Signature, RpcError> {
    let merkle_tree_account = &mut context
        .get_account(merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();
    let output_queue_account = &mut context
        .get_account(output_queue_pubkey)
        .await
        .unwrap()
        .unwrap();
    let mut mt_account_data = merkle_tree_account.data_as_mut_slice();
    let mut output_queue_account_data = output_queue_account.data_as_mut_slice();
    let instruction_data = create_append_batch_ix_data(
        mock_indexer,
        &mut mt_account_data,
        &mut output_queue_account_data,
    )
    .await;
    let mut data = Vec::new();
    instruction_data.serialize(&mut data).unwrap();
    let (merkle_tree_pubkey, output_queue_pubkey, registered_program_pda) = match mode {
        TestMode::Functional => (merkle_tree_pubkey, output_queue_pubkey, None),
        TestMode::InvalidOutputQueue => (merkle_tree_pubkey, Pubkey::new_unique(), None),
        TestMode::InvalidMerkleTree => (output_queue_pubkey, output_queue_pubkey, None),
        TestMode::InvalidRegisteredProgram => (
            merkle_tree_pubkey,
            output_queue_pubkey,
            Some(output_queue_pubkey),
        ),
    };

    let instruction = account_compression::instruction::BatchAppend { data };
    let accounts = account_compression::accounts::BatchAppend {
        authority: payer.pubkey(),
        registered_program_pda,
        log_wrapper: NOOP_PROGRAM_ID,
        merkle_tree: merkle_tree_pubkey,
        output_queue: output_queue_pubkey,
    };

    let instruction = Instruction {
        program_id: ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction.data(),
    };
    context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
}
pub async fn perform_batch_nullify(
    context: &mut ProgramTestRpcConnection,
    mock_indexer: &mut MockBatchedForester<26>,
    merkle_tree_pubkey: Pubkey,
    output_queue_pubkey: Pubkey,
    payer: &Keypair,
    mode: TestMode,
) -> Result<Signature, RpcError> {
    let merkle_tree_account = &mut context
        .get_account(merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();
    let mut mt_account_data = merkle_tree_account.data_as_mut_slice();
    let instruction_data = create_nullify_batch_ix_data(mock_indexer, &mut mt_account_data).await;
    let mut data = Vec::new();
    instruction_data.serialize(&mut data).unwrap();
    let (merkle_tree_pubkey, registered_program_pda) = match mode {
        TestMode::Functional => (merkle_tree_pubkey, None),
        TestMode::InvalidMerkleTree => (output_queue_pubkey, None),
        TestMode::InvalidRegisteredProgram => (merkle_tree_pubkey, Some(output_queue_pubkey)),
        _ => panic!("Invalid mode"),
    };
    let instruction = account_compression::instruction::BatchNullify { data };
    let accounts = account_compression::accounts::BatchNullify {
        authority: payer.pubkey(),
        registered_program_pda,
        log_wrapper: NOOP_PROGRAM_ID,
        merkle_tree: merkle_tree_pubkey,
    };

    let instruction = Instruction {
        program_id: ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction.data(),
    };
    context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
}

pub async fn perform_insert_into_input_queue(
    context: &mut ProgramTestRpcConnection,
    mock_indexer: &mut MockBatchedForester<26>,
    counter: &mut u32,
    num_of_leaves: u32,
    output_queue_pubkey: Pubkey,
    merkle_tree_pubkey: Pubkey,
    payer: &Keypair,
) -> Result<Signature, RpcError> {
    let mut leaves = vec![];
    let leaf_indices = (counter.clone()..counter.clone() + num_of_leaves).collect::<Vec<u32>>();
    for _ in 0..num_of_leaves {
        let mut leaf = [0u8; 32];
        leaf[31] = *counter as u8;
        leaves.push(leaf);
        mock_indexer
            .input_queue_leaves
            .push((leaf, *counter as usize));

        *counter += 1;
    }
    let slot = context.get_slot().await.unwrap();
    let tx_hash = create_tx_hash_offchain(&leaves, &vec![], slot);
    mock_indexer.tx_events.push(MockTxEvent {
        tx_hash,
        inputs: leaves.clone(),
        outputs: vec![],
    });

    let instruction = account_compression::instruction::InsertIntoNullifierQueues {
        nullifiers: leaves,
        leaf_indices,
        tx_hash: Some(tx_hash),
    };
    let accounts = account_compression::accounts::InsertIntoQueues {
        authority: payer.pubkey(),
        fee_payer: payer.pubkey(),
        registered_program_pda: None,
        system_program: Pubkey::default(),
    };
    let mut account_metas = Vec::new();
    for _ in 0..num_of_leaves {
        account_metas.push(AccountMeta {
            pubkey: output_queue_pubkey,
            is_signer: false,
            is_writable: true,
        });
        account_metas.push(AccountMeta {
            pubkey: merkle_tree_pubkey,
            is_signer: false,
            is_writable: true,
        });
    }
    let accounts = vec![accounts.to_account_metas(Some(true)), account_metas].concat();

    let instruction = Instruction {
        program_id: ID,
        accounts,
        data: instruction.data(),
    };
    context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
}

pub async fn create_append_batch_ix_data(
    mock_indexer: &mut MockBatchedForester<26>,
    mt_account_data: &mut [u8],
    output_queue_account_data: &mut [u8],
) -> InstructionDataBatchAppendInputs {
    let zero_copy_account =
        ZeroCopyBatchedMerkleTreeAccount::from_bytes_mut(mt_account_data).unwrap();
    let output_zero_copy_account =
        ZeroCopyBatchedQueueAccount::from_bytes_mut(output_queue_account_data).unwrap();

    let next_index = zero_copy_account.get_account().next_index;
    let next_full_batch = output_zero_copy_account
        .get_account()
        .queue
        .next_full_batch_index;
    let batch = output_zero_copy_account
        .batches
        .get(next_full_batch as usize)
        .unwrap();
    let leaves_hashchain = output_zero_copy_account
        .hashchain_store
        .get(next_full_batch as usize)
        .unwrap()
        .get(batch.get_num_inserted_zkps() as usize)
        .unwrap();
    let (proof, new_root) = mock_indexer
        .get_batched_append_proof(
            next_index as usize,
            batch.get_num_inserted_zkps() as u32,
            batch.zkp_batch_size as u32,
            *leaves_hashchain,
            batch.get_num_zkp_batches() as u32,
        )
        .await
        .unwrap();

    InstructionDataBatchAppendInputs {
        public_inputs: AppendBatchProofInputsIx { new_root },
        compressed_proof: CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        },
    }
}

pub async fn create_nullify_batch_ix_data(
    mock_indexer: &mut MockBatchedForester<26>,
    account_data: &mut [u8],
) -> InstructionDataBatchNullifyInputs {
    let zero_copy_account: ZeroCopyBatchedMerkleTreeAccount =
        ZeroCopyBatchedMerkleTreeAccount::from_bytes_mut(account_data).unwrap();
    println!("batches {:?}", zero_copy_account.batches);

    let old_root_index = zero_copy_account.root_history.last_index();
    let next_full_batch = zero_copy_account.get_account().queue.next_full_batch_index;
    let batch = zero_copy_account
        .batches
        .get(next_full_batch as usize)
        .unwrap();
    println!(
        "zero_copy_account
                        .hashchain_store {:?}",
        zero_copy_account.hashchain_store
    );
    println!(
        "hashchain store len {:?}",
        zero_copy_account.hashchain_store.len()
    );
    println!(
        "batch.get_num_inserted_zkps() as usize {:?}",
        batch.get_num_inserted_zkps() as usize
    );
    let leaves_hashchain = zero_copy_account
        .hashchain_store
        .get(next_full_batch as usize)
        .unwrap()
        .get(batch.get_num_inserted_zkps() as usize)
        .unwrap();
    let (proof, new_root) = mock_indexer
        .get_batched_update_proof(
            zero_copy_account.get_account().queue.zkp_batch_size as u32,
            *leaves_hashchain,
        )
        .await
        .unwrap();
    let instruction_data = InstructionDataBatchNullifyInputs {
        public_inputs: BatchProofInputsIx {
            new_root,
            old_root_index: old_root_index as u16,
        },
        compressed_proof: CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        },
    };
    instruction_data
}

#[serial]
#[tokio::test]
async fn test_init_batch_state_merkle_trees() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.add_program(
        "spl_noop",
        Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        None,
    );
    program_test.set_compute_max_units(1_400_000u64);
    let context = program_test.start_with_context().await;
    let mut context = ProgramTestRpcConnection { context };

    let payer = context.get_payer().insecure_clone();
    let params = InitStateTreeAccountsInstructionData::test_default();
    let e2e_test_params = InitStateTreeAccountsInstructionData::e2e_test_default();
    let default_params = InitStateTreeAccountsInstructionData::default();
    let param_vec = vec![params, e2e_test_params, default_params];
    for params in param_vec.iter() {
        println!("Init new mt with params {:?}", params);
        let merkle_tree_keypair = Keypair::new();
        let nullifier_queue_keypair = Keypair::new();
        let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
        let output_queue_pubkey = nullifier_queue_keypair.pubkey();
        let (_, total_rent) = perform_init_batch_state_merkle_tree(
            &mut context,
            &payer,
            &merkle_tree_keypair,
            &nullifier_queue_keypair,
            params.clone(),
        )
        .await
        .unwrap();
        let merkle_tree =
            AccountZeroCopy::<BatchedMerkleTreeAccount>::new(&mut context, merkle_tree_pubkey)
                .await;

        let mut queue =
            AccountZeroCopy::<BatchedQueueAccount>::new(&mut context, output_queue_pubkey).await;
        let owner = context.get_payer().pubkey();

        let ref_mt_account = BatchedMerkleTreeAccount::get_state_tree_default(
            owner,
            None,
            None,
            params.rollover_threshold,
            0,
            params.network_fee.unwrap_or_default(),
            params.input_queue_batch_size,
            params.input_queue_zkp_batch_size,
            params.bloom_filter_capacity,
            params.root_history_capacity,
            output_queue_pubkey,
            params.height,
            params.input_queue_num_batches,
        );

        let mut tree_data = merkle_tree.account.data.clone();
        assert_mt_zero_copy_inited(
            &mut tree_data.as_mut_slice(),
            ref_mt_account,
            params.bloom_filter_num_iters,
        );

        let ref_output_queue_account = get_output_queue_account_default(
            owner,
            None,
            None,
            params.rollover_threshold,
            0,
            params.output_queue_batch_size,
            params.output_queue_zkp_batch_size,
            params.additional_bytes,
            total_rent,
            merkle_tree_pubkey,
            params.height,
            params.output_queue_num_batches,
            params.network_fee.unwrap_or_default(),
        );
        assert_queue_zero_copy_inited(
            &mut queue.account.data.as_mut_slice(),
            ref_output_queue_account,
            0,
        );
    }
}

pub async fn perform_init_batch_state_merkle_tree(
    context: &mut ProgramTestRpcConnection,
    payer: &Keypair,
    merkle_tree_keypair: &Keypair,
    nullifier_queue_keypair: &Keypair,
    params: InitStateTreeAccountsInstructionData,
) -> Result<(Signature, u64), RpcError> {
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();

    let output_queue_pubkey = nullifier_queue_keypair.pubkey();
    let queue_account_size = get_output_queue_account_size(
        params.output_queue_batch_size,
        params.output_queue_zkp_batch_size,
        params.output_queue_num_batches,
    );
    let mt_account_size = get_merkle_tree_account_size(
        params.input_queue_batch_size,
        params.bloom_filter_capacity,
        params.input_queue_zkp_batch_size,
        params.root_history_capacity,
        params.height,
        params.input_queue_num_batches,
    );
    let queue_rent = context
        .get_minimum_balance_for_rent_exemption(queue_account_size)
        .await
        .unwrap();
    let create_queue_account_ix = create_account_instruction(
        &payer_pubkey,
        queue_account_size,
        queue_rent,
        &ID,
        Some(&nullifier_queue_keypair),
    );
    let mt_rent = context
        .get_minimum_balance_for_rent_exemption(mt_account_size)
        .await
        .unwrap();
    let additional_bytes_rent = context
        .get_minimum_balance_for_rent_exemption(params.additional_bytes as usize)
        .await
        .unwrap();
    let total_rent = queue_rent + mt_rent + additional_bytes_rent;
    let create_mt_account_ix = create_account_instruction(
        &payer_pubkey,
        mt_account_size,
        mt_rent,
        &ID,
        Some(&merkle_tree_keypair),
    );

    let instruction = account_compression::instruction::InitializeBatchedStateMerkleTree { params };
    let accounts = account_compression::accounts::InitializeBatchedStateMerkleTreeAndQueue {
        authority: context.get_payer().pubkey(),
        merkle_tree: merkle_tree_pubkey,
        queue: output_queue_pubkey,
        registered_program_pda: None,
    };

    let instruction = Instruction {
        program_id: ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction.data(),
    };
    Ok((
        context
            .create_and_send_transaction(
                &[create_queue_account_ix, create_mt_account_ix, instruction],
                &payer_pubkey,
                &[&payer, &nullifier_queue_keypair, &merkle_tree_keypair],
            )
            .await?,
        total_rent,
    ))
}

/// Tests:
/// 1. Failing - Invalid authority
/// 2. Failing - State tree invalid program owner
/// 3. Failing - Queue invalid program owner
/// 4. Failing - State tree invalid discriminator
/// 5. Failing - Queue invalid discriminator
/// 6. Failing - Merkle tree and queue not associated
/// 7. functional
#[serial]
#[tokio::test]
async fn test_rollover_batch_state_merkle_trees() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.add_program(
        "spl_noop",
        Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        None,
    );
    program_test.set_compute_max_units(1_400_000u64);
    let context = program_test.start_with_context().await;
    let mut context = ProgramTestRpcConnection { context };
    let payer = context.get_payer().insecure_clone();
    let mut params = InitStateTreeAccountsInstructionData::test_default();
    params.rollover_threshold = Some(0);
    println!("Init new mt with params {:?}", params);
    let merkle_tree_keypair = Keypair::new();
    let nullifier_queue_keypair = Keypair::new();
    perform_init_batch_state_merkle_tree(
        &mut context,
        &payer,
        &merkle_tree_keypair,
        &nullifier_queue_keypair,
        params.clone(),
    )
    .await
    .unwrap();
    let mut mock_indexer = MockBatchedForester::<26>::default();
    let output_queue_pubkey = nullifier_queue_keypair.pubkey();

    perform_insert_into_output_queue(
        &mut context,
        &mut mock_indexer,
        output_queue_pubkey,
        &payer,
        &mut 0,
        1,
    )
    .await
    .unwrap();
    let new_state_merkle_tree_keypair = Keypair::new();
    let new_output_queue_keypair = Keypair::new();
    // 1. failing - invalid authority
    {
        let invalid_authority = Keypair::new();
        airdrop_lamports(&mut context, &invalid_authority.pubkey(), 10_000_000_000)
            .await
            .unwrap();
        let result = perform_rollover_batch_state_merkle_tree(
            &mut context,
            &invalid_authority,
            merkle_tree_keypair.pubkey(),
            nullifier_queue_keypair.pubkey(),
            &new_state_merkle_tree_keypair,
            &new_output_queue_keypair,
            params.additional_bytes,
            params.network_fee,
            BatchStateMerkleTreeRollOverTestMode::Functional,
        )
        .await;
        assert_rpc_error(
            result,
            2,
            AccountCompressionErrorCode::InvalidAuthority.into(),
        )
        .unwrap();
    }
    // 2. failing - state tree invalid program owner
    {
        let result = perform_rollover_batch_state_merkle_tree(
            &mut context,
            &payer,
            merkle_tree_keypair.pubkey(),
            nullifier_queue_keypair.pubkey(),
            &new_state_merkle_tree_keypair,
            &new_output_queue_keypair,
            params.additional_bytes,
            params.network_fee,
            BatchStateMerkleTreeRollOverTestMode::InvalidProgramOwnerMerkleTree,
        )
        .await;
        assert_rpc_error(result, 2, ErrorCode::AccountOwnedByWrongProgram.into()).unwrap();
    }
    // 3. failing - queue invalid program owner
    {
        let result = perform_rollover_batch_state_merkle_tree(
            &mut context,
            &payer,
            merkle_tree_keypair.pubkey(),
            nullifier_queue_keypair.pubkey(),
            &new_state_merkle_tree_keypair,
            &new_output_queue_keypair,
            params.additional_bytes,
            params.network_fee,
            BatchStateMerkleTreeRollOverTestMode::InvalidProgramOwnerQueue,
        )
        .await;
        assert_rpc_error(result, 2, ErrorCode::AccountOwnedByWrongProgram.into()).unwrap();
    }
    // 4. failing - state tree invalid discriminator
    {
        let result = perform_rollover_batch_state_merkle_tree(
            &mut context,
            &payer,
            merkle_tree_keypair.pubkey(),
            nullifier_queue_keypair.pubkey(),
            &new_state_merkle_tree_keypair,
            &new_output_queue_keypair,
            params.additional_bytes,
            params.network_fee,
            BatchStateMerkleTreeRollOverTestMode::InvalidDiscriminatorMerkleTree,
        )
        .await;
        assert_rpc_error(
            result,
            2,
            AccountCompressionErrorCode::InvalidDiscriminator.into(),
        )
        .unwrap();
    }
    // 5. failing - queue invalid discriminator
    {
        let result = perform_rollover_batch_state_merkle_tree(
            &mut context,
            &payer,
            merkle_tree_keypair.pubkey(),
            nullifier_queue_keypair.pubkey(),
            &new_state_merkle_tree_keypair,
            &new_output_queue_keypair,
            params.additional_bytes,
            params.network_fee,
            BatchStateMerkleTreeRollOverTestMode::InvalidDiscriminatorQueue,
        )
        .await;
        assert_rpc_error(
            result,
            2,
            AccountCompressionErrorCode::InvalidDiscriminator.into(),
        )
        .unwrap();
    }
    // 6. failing -  merkle tree and queue not associated
    {
        let merkle_tree_keypair_1 = Keypair::new();
        let nullifier_queue_keypair_1 = Keypair::new();
        perform_init_batch_state_merkle_tree(
            &mut context,
            &payer,
            &merkle_tree_keypair_1,
            &nullifier_queue_keypair_1,
            params.clone(),
        )
        .await
        .unwrap();
        let result = perform_rollover_batch_state_merkle_tree(
            &mut context,
            &payer,
            merkle_tree_keypair_1.pubkey(),
            nullifier_queue_keypair.pubkey(),
            &new_state_merkle_tree_keypair,
            &new_output_queue_keypair,
            params.additional_bytes,
            params.network_fee,
            BatchStateMerkleTreeRollOverTestMode::Functional,
        )
        .await;
        assert_rpc_error(
            result,
            2,
            AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
        )
        .unwrap();
    }
    // 7. functional
    {
        perform_rollover_batch_state_merkle_tree(
            &mut context,
            &payer,
            merkle_tree_keypair.pubkey(),
            nullifier_queue_keypair.pubkey(),
            &new_state_merkle_tree_keypair,
            &new_output_queue_keypair,
            params.additional_bytes,
            params.network_fee,
            BatchStateMerkleTreeRollOverTestMode::Functional,
        )
        .await
        .unwrap();
        let additional_bytes_rent = context
            .get_minimum_balance_for_rent_exemption(params.additional_bytes as usize)
            .await
            .unwrap();
        assert_perform_state_mt_roll_over(
            &mut context,
            payer.pubkey(),
            merkle_tree_keypair.pubkey(),
            new_state_merkle_tree_keypair.pubkey(),
            nullifier_queue_keypair.pubkey(),
            new_output_queue_keypair.pubkey(),
            params,
            additional_bytes_rent,
        )
        .await;
    }
}

#[derive(Debug, PartialEq)]
pub enum BatchStateMerkleTreeRollOverTestMode {
    Functional,
    InvalidProgramOwnerMerkleTree,
    InvalidProgramOwnerQueue,
    InvalidDiscriminatorMerkleTree,
    InvalidDiscriminatorQueue,
}

pub async fn perform_rollover_batch_state_merkle_tree<R: RpcConnection>(
    rpc: &mut R,
    payer: &Keypair,
    old_merkle_tree_pubkey: Pubkey,
    old_output_queue_pubkey: Pubkey,
    new_state_merkle_tree_keypair: &Keypair,
    new_output_queue_keypair: &Keypair,
    additional_bytes: u64,
    network_fee: Option<u64>,
    test_mode: BatchStateMerkleTreeRollOverTestMode,
) -> Result<Signature, RpcError> {
    let payer_pubkey = payer.pubkey();
    let mut account = rpc.get_account(old_merkle_tree_pubkey).await?.unwrap();
    let old_merkle_tree =
        ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(account.data.as_mut_slice())
            .unwrap();
    let batch_zero = &old_merkle_tree.batches[0];
    let num_batches = old_merkle_tree.batches.len();
    let old_merkle_tree = old_merkle_tree.get_account();
    let mt_account_size = get_merkle_tree_account_size(
        batch_zero.batch_size,
        batch_zero.bloom_filter_capacity,
        batch_zero.zkp_batch_size,
        old_merkle_tree.root_history_capacity,
        old_merkle_tree.height,
        num_batches as u64,
    );

    let mt_rent = rpc
        .get_minimum_balance_for_rent_exemption(mt_account_size)
        .await
        .unwrap();

    let mut account = rpc.get_account(old_output_queue_pubkey).await?.unwrap();
    let old_queue_account =
        ZeroCopyBatchedQueueAccount::from_bytes_mut(account.data.as_mut_slice()).unwrap();
    let batch_zero = &old_queue_account.batches[0];
    let queue_account_size = get_output_queue_account_size(
        batch_zero.batch_size,
        batch_zero.zkp_batch_size,
        num_batches as u64,
    );
    let queue_rent = rpc
        .get_minimum_balance_for_rent_exemption(queue_account_size)
        .await
        .unwrap();

    let create_mt_account_ix = create_account_instruction(
        &payer_pubkey,
        mt_account_size,
        mt_rent,
        &account_compression::ID,
        Some(&new_state_merkle_tree_keypair),
    );

    let create_queue_account_ix = create_account_instruction(
        &payer_pubkey,
        queue_account_size,
        queue_rent,
        &account_compression::ID,
        Some(new_output_queue_keypair),
    );
    let old_state_merkle_tree = if test_mode
        == BatchStateMerkleTreeRollOverTestMode::InvalidProgramOwnerMerkleTree
    {
        Pubkey::new_unique()
    } else if test_mode == BatchStateMerkleTreeRollOverTestMode::InvalidDiscriminatorMerkleTree {
        old_output_queue_pubkey
    } else {
        old_merkle_tree_pubkey
    };
    let old_output_queue =
        if test_mode == BatchStateMerkleTreeRollOverTestMode::InvalidProgramOwnerQueue {
            Pubkey::new_unique()
        } else if test_mode == BatchStateMerkleTreeRollOverTestMode::InvalidDiscriminatorQueue {
            old_merkle_tree_pubkey
        } else {
            old_output_queue_pubkey
        };
    let accounts = account_compression::accounts::RolloverBatchStateMerkleTree {
        fee_payer: payer_pubkey,
        authority: payer_pubkey,
        old_state_merkle_tree,
        new_state_merkle_tree: new_state_merkle_tree_keypair.pubkey(),
        old_output_queue,
        new_output_queue: new_output_queue_keypair.pubkey(),
        registered_program_pda: None,
    };
    let instruction_data = account_compression::instruction::RolloverBatchStateMerkleTree {
        additional_bytes,
        network_fee,
    };
    let instruction = Instruction {
        program_id: ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    };

    Ok(rpc
        .create_and_send_transaction(
            &[create_mt_account_ix, create_queue_account_ix, instruction],
            &payer_pubkey,
            &[
                &payer,
                &new_state_merkle_tree_keypair,
                &new_output_queue_keypair,
            ],
        )
        .await?)
}
