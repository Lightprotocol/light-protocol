#![cfg(feature = "test-sbf")]

use account_compression::batched_merkle_tree::{
    create_hash_chain, get_merkle_tree_account_size, AppendBatchProofInputsIx, BatchProofInputsIx,
    InstructionDataBatchAppendProofInputs, InstructionDataBatchUpdateProofInputs,
    ZeroCopyBatchedMerkleTreeAccount,
};
use account_compression::batched_queue::{
    assert_queue_zero_copy_inited, get_output_queue_account_size, BatchedQueueAccount,
    ZeroCopyBatchedQueueAccount,
};
use account_compression::{assert_mt_zero_copy_inited, get_output_queue_account_default};
use account_compression::{
    batched_merkle_tree::BatchedMerkleTreeAccount, InitStateTreeAccountsInstructionData, ID,
};
use anchor_lang::prelude::AccountMeta;
use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};
use light_prover_client::gnark::helpers::{spawn_prover, ProofType, ProverConfig};
use light_prover_client::mock_batched_forester::MockBatchedForester;
use light_test_utils::test_env::NOOP_PROGRAM_ID;
use light_test_utils::{create_account_instruction, RpcConnection};
use light_test_utils::{rpc::ProgramTestRpcConnection, AccountZeroCopy};
use light_verifier::CompressedProof;
use solana_program_test::ProgramTest;
use solana_sdk::account::WritableAccount;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signer},
};

#[ignore]
#[tokio::test]
async fn test_init_state_merkle_tree() {
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

    {
        let params = InitStateTreeAccountsInstructionData::default();
        println!("params {:?}", params);
        let queue_account_size = get_output_queue_account_size(
            params.output_queue_batch_size,
            params.output_queue_zkp_batch_size,
        );
        let mt_account_size = get_merkle_tree_account_size(
            params.input_queue_batch_size,
            params.bloom_filter_capacity,
            params.input_queue_zkp_batch_size,
            params.root_history_capacity,
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
        );
        println!("pre assert_mt_zero_copy_inited");
        assert_mt_zero_copy_inited(
            &mut merkle_tree.account.data.as_mut_slice(),
            ref_mt_account,
            params.bloom_filter_num_iters,
        );
        println!("post assert_mt_zero_copy_inited");

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
        );
        println!("pre assert_queue_zero_copy_inited");
        assert_queue_zero_copy_inited(
            &mut queue.account.data.as_mut_slice(),
            ref_output_queue_account,
            0,
        );
        println!("post assert_queue_zero_copy_inited");
    }
    let mut mock_indexer = MockBatchedForester::<26>::default();

    // insert 10 leaves into output queue
    let num_of_leaves = 10;
    let num_tx = 5;
    let mut counter = 0;
    for _ in 0..num_tx {
        let mut leaves = vec![];
        for _ in 0..num_of_leaves {
            let mut leaf = [0u8; 32];
            leaf[31] = counter as u8;
            leaves.push((0, leaf));
            mock_indexer.output_queue_leaves.push(leaf);
            counter += 1;
        }

        let instruction = account_compression::instruction::AppendLeavesToMerkleTrees { leaves };
        let accounts = account_compression::accounts::InsertIntoQueues {
            authority: context.get_payer().pubkey(),
            fee_payer: context.get_payer().pubkey(),
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
            .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
            .await
            .unwrap();
    }
    spawn_prover(
        false,
        ProverConfig {
            run_mode: None,
            circuits: vec![ProofType::BatchAppend, ProofType::BatchUpdate],
        },
    )
    .await;

    // append 10 leaves
    for _ in 0..num_tx {
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
            &mut mock_indexer,
            &mut mt_account_data,
            &mut output_queue_account_data,
        )
        .await;
        let mut data = Vec::new();
        instruction_data.serialize(&mut data).unwrap();

        let instruction = account_compression::instruction::BatchAppend { data };
        let accounts = account_compression::accounts::BatchAppend {
            authority: context.get_payer().pubkey(),
            registered_program_pda: None,
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
            .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
            .await
            .unwrap();
    }
    // insert 10 leaves into input queue
    let num_of_leaves = 10;
    let num_tx = 5;
    let mut counter = 0;
    for i in 0..num_tx {
        println!("insert into nullifier queue tx: {:?}", i);
        // TODO: take leaves from active leaves (and fill active leaves before)
        let mut nullifiers = vec![];
        for _ in 0..num_of_leaves {
            let mut leaf = [0u8; 32];
            leaf[31] = counter as u8;
            nullifiers.push(leaf);
            mock_indexer.input_queue_leaves.push(leaf);
            counter += 1;
        }

        let instruction =
            account_compression::instruction::InsertIntoNullifierQueues { nullifiers };
        let accounts = account_compression::accounts::InsertIntoQueues {
            authority: context.get_payer().pubkey(),
            fee_payer: context.get_payer().pubkey(),
            registered_program_pda: None,
            system_program: Pubkey::default(),
        };
        let accounts = vec![
            accounts.to_account_metas(Some(true)),
            vec![
                AccountMeta {
                    pubkey: merkle_tree_pubkey,
                    is_signer: false,
                    is_writable: true,
                };
                num_of_leaves
            ],
        ]
        .concat();

        let instruction = Instruction {
            program_id: ID,
            accounts,
            data: instruction.data(),
        };
        context
            .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
            .await
            .unwrap();
    }
    // nullify 10 leaves
    for i in 0..num_tx {
        println!("nullify leaves tx: {:?}", i);
        let merkle_tree_account = &mut context
            .get_account(merkle_tree_pubkey)
            .await
            .unwrap()
            .unwrap();
        let mut mt_account_data = merkle_tree_account.data_as_mut_slice();
        let instruction_data =
            create_nullify_batch_ix_data(&mut mock_indexer, &mut mt_account_data).await;
        let mut data = Vec::new();
        instruction_data.serialize(&mut data).unwrap();

        let instruction = account_compression::instruction::BatchNullifyLeaves { data };
        let accounts = account_compression::accounts::BatchNullifyLeaves {
            authority: context.get_payer().pubkey(),
            registered_program_pda: None,
            log_wrapper: NOOP_PROGRAM_ID,
            merkle_tree: merkle_tree_pubkey,
        };

        let instruction = Instruction {
            program_id: ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction.data(),
        };
        context
            .create_and_send_transaction(&[instruction], &payer_pubkey, &[&payer])
            .await
            .unwrap();
    }
}
use std::ops::Deref;

pub async fn create_append_batch_ix_data(
    mock_indexer: &mut MockBatchedForester<26>,
    mt_account_data: &mut [u8],
    output_queue_account_data: &mut [u8],
) -> InstructionDataBatchAppendProofInputs {
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
    let leaves = output_zero_copy_account
        .value_vecs
        .get(next_full_batch as usize)
        .unwrap()
        .deref()
        .clone()
        .to_vec();
    println!("batch append leaves {:?}", leaves);
    let (proof, new_root, new_subtree_hash) = mock_indexer
        .get_batched_append_proof(
            next_index as usize,
            leaves.clone(),
            batch.get_num_inserted_zkps() as u32,
            batch.zkp_batch_size as u32,
        )
        .await
        .unwrap();

    InstructionDataBatchAppendProofInputs {
        public_inputs: AppendBatchProofInputsIx {
            new_root,
            new_subtrees_hash: new_subtree_hash,
        },
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
) -> InstructionDataBatchUpdateProofInputs {
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
    let new_subtrees = mock_indexer.merkle_tree.get_subtrees();
    let new_subtrees_hash = create_hash_chain::<26>(new_subtrees.try_into().unwrap()).unwrap();
    let instruction_data = InstructionDataBatchUpdateProofInputs {
        public_inputs: BatchProofInputsIx {
            new_root,
            old_root_index: old_root_index as u16,
            new_subtrees_hash,
        },
        compressed_proof: CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        },
    };
    instruction_data
}
