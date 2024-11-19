use account_compression::{
    assert_mt_zero_copy_inited,
    batched_merkle_tree::{
        get_merkle_tree_account_size, AppendBatchProofInputsIx, BatchAppendEvent,
        BatchNullifyEvent, BatchProofInputsIx, BatchedMerkleTreeAccount,
        InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
        ZeroCopyBatchedMerkleTreeAccount,
    },
    batched_queue::{
        assert_queue_zero_copy_inited, get_output_queue_account_size, BatchedQueueAccount,
        ZeroCopyBatchedQueueAccount,
    },
    get_output_queue_account_default, InitStateTreeAccountsInstructionData,
};
use anchor_lang::AnchorSerialize;
use forester_utils::{create_account_instruction, indexer::StateMerkleTreeBundle, AccountZeroCopy};
use light_client::rpc::{RpcConnection, RpcError};
use light_hasher::Poseidon;
use light_prover_client::{
    batch_append_with_proofs::get_batch_append_with_proofs_inputs,
    batch_append_with_subtrees::calculate_hash_chain,
    batch_update::get_batch_update_inputs,
    gnark::{
        batch_append_with_proofs_json_formatter::BatchAppendWithProofsInputsJson,
        batch_update_json_formatter::update_inputs_string,
        constants::{PROVE_PATH, SERVER_ADDRESS},
        proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    },
};
use light_registry::{
    account_compression_cpi::sdk::{
        create_batch_append_instruction, create_batch_nullify_instruction,
        create_initialize_batched_merkle_tree_instruction,
    },
    protocol_config::state::ProtocolConfig,
};
use light_utils::bigint::bigint_to_be_bytes_array;
use light_verifier::CompressedProof;
use reqwest::Client;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
pub async fn perform_batch_append<Rpc: RpcConnection>(
    rpc: &mut Rpc,
    bundle: &mut StateMerkleTreeBundle,
    forester: &Keypair,
    epoch: u64,
    _is_metadata_forester: bool,
    instruction_data: Option<InstructionDataBatchAppendInputs>,
) -> Result<Signature, RpcError> {
    // let forester_epoch_pda = get_forester_epoch_pda_from_authority(&forester.pubkey(), epoch).0;
    // let pre_forester_counter = if is_metadata_forester {
    //     0
    // } else {
    //     rpc.get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda)
    //         .await
    //         .unwrap()
    //         .unwrap()
    //         .work_counter
    // };
    let merkle_tree_pubkey = bundle.accounts.merkle_tree;
    let output_queue_pubkey = bundle.accounts.nullifier_queue;

    let data = if let Some(instruction_data) = instruction_data {
        instruction_data
    } else {
        create_append_batch_ix_data(rpc, bundle, merkle_tree_pubkey, output_queue_pubkey).await
    };
    let instruction = create_batch_append_instruction(
        forester.pubkey(),
        forester.pubkey(),
        merkle_tree_pubkey,
        output_queue_pubkey,
        epoch,
        data.try_to_vec().unwrap(),
    );
    let res = rpc
        .create_and_send_transaction_with_event::<BatchAppendEvent>(
            &[instruction],
            &forester.pubkey(),
            &[forester],
            None,
        )
        .await?
        .unwrap();
    println!("event {:?}", res.0);
    Ok(res.1)
}

pub async fn create_append_batch_ix_data<Rpc: RpcConnection>(
    rpc: &mut Rpc,
    bundle: &mut StateMerkleTreeBundle,
    merkle_tree_pubkey: Pubkey,
    output_queue_pubkey: Pubkey,
) -> InstructionDataBatchAppendInputs {
    let mut merkle_tree_account = rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree =
        ZeroCopyBatchedMerkleTreeAccount::from_bytes_mut(merkle_tree_account.data.as_mut_slice())
            .unwrap();
    let merkle_tree_next_index = merkle_tree.get_account().next_index as usize;

    let mut output_queue_account = rpc.get_account(output_queue_pubkey).await.unwrap().unwrap();
    let output_queue =
        ZeroCopyBatchedQueueAccount::from_bytes_mut(output_queue_account.data.as_mut_slice())
            .unwrap();
    let output_queue_account = output_queue.get_account();
    let full_batch_index = output_queue_account.queue.next_full_batch_index;
    let zkp_batch_size = output_queue_account.queue.zkp_batch_size;
    let max_num_zkp_updates = output_queue_account.queue.get_num_zkp_batches();

    let leaves = bundle.output_queue_elements.to_vec();

    let num_inserted_zkps = output_queue.batches[full_batch_index as usize].get_num_inserted_zkps();
    let leaves_hashchain =
        output_queue.hashchain_store[full_batch_index as usize][num_inserted_zkps as usize];
    let (proof, new_root) = {
        let start = num_inserted_zkps as usize * zkp_batch_size as usize;
        let end = start + zkp_batch_size as usize;
        let batch_update_leaves = leaves[start..end].to_vec();
        // if batch is complete, remove leaves from mock output queue
        if num_inserted_zkps == max_num_zkp_updates - 1 {
            for _ in 0..max_num_zkp_updates * zkp_batch_size {
                bundle.output_queue_elements.remove(0);
            }
        }

        let local_leaves_hashchain = calculate_hash_chain(&batch_update_leaves);
        assert_eq!(leaves_hashchain, local_leaves_hashchain);

        let old_root = bundle.merkle_tree.root();
        let mut old_leaves = vec![];
        let mut merkle_proofs = vec![];
        for i in merkle_tree_next_index..merkle_tree_next_index + zkp_batch_size as usize {
            match bundle.merkle_tree.get_leaf(i) {
                Ok(leaf) => {
                    old_leaves.push(leaf);
                }
                Err(_) => {
                    old_leaves.push([0u8; 32]);
                    if i <= bundle.merkle_tree.get_next_index() {
                        bundle.merkle_tree.append(&[0u8; 32]).unwrap();
                    }
                }
            }
            let proof = bundle.merkle_tree.get_proof_of_leaf(i, true).unwrap();
            merkle_proofs.push(proof.to_vec());
        }
        // Insert new leaves into the merkle tree. Every leaf which is not [0u8;
        // 32] has already been nullified hence shouldn't be updated.
        for (i, leaf) in batch_update_leaves.iter().enumerate() {
            if old_leaves[i] == [0u8; 32] {
                let index = merkle_tree_next_index + i;
                bundle.merkle_tree.update(leaf, index).unwrap();
            }
        }
        let circuit_inputs = get_batch_append_with_proofs_inputs::<26>(
            old_root,
            merkle_tree_next_index as u32,
            batch_update_leaves,
            local_leaves_hashchain,
            old_leaves,
            merkle_proofs,
            zkp_batch_size as u32,
        );
        assert_eq!(
            bigint_to_be_bytes_array::<32>(&circuit_inputs.new_root.to_biguint().unwrap()).unwrap(),
            bundle.merkle_tree.root()
        );
        let client = Client::new();
        let inputs_json = BatchAppendWithProofsInputsJson::from_inputs(&circuit_inputs).to_string();

        let response_result = client
            .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(inputs_json)
            .send()
            .await
            .expect("Failed to execute request.");
        if response_result.status().is_success() {
            let body = response_result.text().await.unwrap();
            let proof_json = deserialize_gnark_proof_json(&body).unwrap();
            let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
            let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
            (
                CompressedProof {
                    a: proof_a,
                    b: proof_b,
                    c: proof_c,
                },
                bigint_to_be_bytes_array::<32>(&circuit_inputs.new_root.to_biguint().unwrap())
                    .unwrap(),
            )
        } else {
            panic!("Failed to get proof from server.");
        }
    };

    InstructionDataBatchAppendInputs {
        public_inputs: AppendBatchProofInputsIx { new_root },
        compressed_proof: CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        },
    }
}

pub async fn perform_batch_nullify<Rpc: RpcConnection>(
    rpc: &mut Rpc,
    bundle: &mut StateMerkleTreeBundle,
    forester: &Keypair,
    epoch: u64,
    _is_metadata_forester: bool,
    instruction_data: Option<InstructionDataBatchNullifyInputs>,
) -> Result<Signature, RpcError> {
    // let forester_epoch_pda = get_forester_epoch_pda_from_authority(&forester.pubkey(), epoch).0;
    // let pre_forester_counter = if is_metadata_forester {
    //     0
    // } else {
    //     rpc.get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda)
    //         .await
    //         .unwrap()
    //         .unwrap()
    //         .work_counter
    // };
    let merkle_tree_pubkey = bundle.accounts.merkle_tree;

    let data = if let Some(instruction_data) = instruction_data {
        instruction_data
    } else {
        get_batched_nullify_ix_data(rpc, bundle, merkle_tree_pubkey).await?
    };
    let instruction = create_batch_nullify_instruction(
        forester.pubkey(),
        forester.pubkey(),
        merkle_tree_pubkey,
        epoch,
        data.try_to_vec().unwrap(),
    );
    let res = rpc
        .create_and_send_transaction_with_event::<BatchNullifyEvent>(
            &[instruction],
            &forester.pubkey(),
            &[forester],
            None,
        )
        .await?
        .unwrap();
    Ok(res.1)
}

pub async fn get_batched_nullify_ix_data<Rpc: RpcConnection>(
    rpc: &mut Rpc,
    bundle: &mut StateMerkleTreeBundle,
    merkle_tree_pubkey: Pubkey,
) -> Result<InstructionDataBatchNullifyInputs, RpcError> {
    let mut merkle_tree_account = rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree =
        ZeroCopyBatchedMerkleTreeAccount::from_bytes_mut(merkle_tree_account.data.as_mut_slice())
            .unwrap();
    let zkp_batch_size = merkle_tree.get_account().queue.zkp_batch_size;
    let full_batch_index = merkle_tree.get_account().queue.next_full_batch_index;
    let full_batch = &merkle_tree.batches[full_batch_index as usize];
    let zkp_batch_index = full_batch.get_num_inserted_zkps();
    let leaves_hashchain =
        merkle_tree.hashchain_store[full_batch_index as usize][zkp_batch_index as usize];
    let mut merkle_proofs = vec![];
    let leaf_indices_tx_hashes = bundle.input_leaf_indices[..zkp_batch_size as usize].to_vec();
    let mut leaves = Vec::new();
    let old_root_index = merkle_tree.root_history.last_index();
    let old_root: [u8; 32] = bundle.merkle_tree.root();
    assert_eq!(
        old_root,
        *merkle_tree.root_history.get(old_root_index).unwrap()
    );

    let mut nullifiers = Vec::new();
    let mut tx_hashes = Vec::new();
    let mut old_leaves = Vec::new();
    let mut path_indices = Vec::new();
    for (index, leaf, tx_hash) in leaf_indices_tx_hashes.iter() {
        path_indices.push(*index);
        let index = *index as usize;
        let leaf = *leaf;

        leaves.push(leaf);
        // + 2 because next index is + 1 and we need to init the leaf in
        //   pos[index]
        if bundle.merkle_tree.get_next_index() < index + 2 {
            old_leaves.push([0u8; 32]);
        } else {
            // We can get into a state where we have pushed 0 leaves into the
            // tree hence hit this else but the leaf hasn't been inserted yet.
            let leaf = bundle.merkle_tree.get_leaf(index).unwrap();
            old_leaves.push(leaf);
        }
        // Handle case that we nullify a leaf which has not been inserted yet.
        while bundle.merkle_tree.get_next_index() < index + 2 {
            bundle.merkle_tree.append(&[0u8; 32]).unwrap();
        }
        let proof = bundle.merkle_tree.get_proof_of_leaf(index, true).unwrap();
        merkle_proofs.push(proof.to_vec());
        bundle.input_leaf_indices.remove(0);
        let index_bytes = index.to_be_bytes();
        use light_hasher::Hasher;
        let nullifier = Poseidon::hashv(&[&leaf, &index_bytes, tx_hash]).unwrap();
        tx_hashes.push(*tx_hash);
        nullifiers.push(nullifier);
        bundle.merkle_tree.update(&nullifier, index).unwrap();
    }
    // local_leaves_hashchain is only used for a test assertion.
    let local_nullifier_hashchain = calculate_hash_chain(&nullifiers);
    assert_eq!(leaves_hashchain, local_nullifier_hashchain);
    let inputs = get_batch_update_inputs::<26>(
        old_root,
        tx_hashes,
        leaves.to_vec(),
        leaves_hashchain,
        old_leaves,
        merkle_proofs,
        path_indices,
        zkp_batch_size as u32,
    );
    let client = Client::new();
    let circuit_inputs_new_root =
        bigint_to_be_bytes_array::<32>(&inputs.new_root.to_biguint().unwrap()).unwrap();
    let inputs = update_inputs_string(&inputs);
    let new_root = bundle.merkle_tree.root();

    let response_result = client
        .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(inputs)
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(circuit_inputs_new_root, new_root);
    let (proof, new_root) = if response_result.status().is_success() {
        let body = response_result.text().await.unwrap();
        let proof_json = deserialize_gnark_proof_json(&body).unwrap();
        let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
        let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
        (
            CompressedProof {
                a: proof_a,
                b: proof_b,
                c: proof_c,
            },
            new_root,
        )
    } else {
        println!("response_result: {:?}", response_result);
        panic!("Failed to get proof from server.");
    };

    Ok(InstructionDataBatchNullifyInputs {
        public_inputs: BatchProofInputsIx {
            new_root,
            old_root_index: old_root_index as u16,
        },
        compressed_proof: CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        },
    })
}

use anchor_lang::{InstructionData, ToAccountMetas};

pub async fn create_batched_state_merkle_tree<R: RpcConnection>(
    payer: &Keypair,
    registry: bool,
    rpc: &mut R,
    merkle_tree_keypair: &Keypair,
    queue_keypair: &Keypair,
    cpi_context_keypair: &Keypair,
    params: InitStateTreeAccountsInstructionData,
) -> Result<Signature, RpcError> {
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
    let queue_rent = rpc
        .get_minimum_balance_for_rent_exemption(queue_account_size)
        .await
        .unwrap();
    let create_queue_account_ix = create_account_instruction(
        &payer.pubkey(),
        queue_account_size,
        queue_rent,
        &account_compression::ID,
        Some(queue_keypair),
    );
    let mt_rent = rpc
        .get_minimum_balance_for_rent_exemption(mt_account_size)
        .await
        .unwrap();
    let create_mt_account_ix = create_account_instruction(
        &payer.pubkey(),
        mt_account_size,
        mt_rent,
        &account_compression::ID,
        Some(merkle_tree_keypair),
    );
    let rent_cpi_config = rpc
        .get_minimum_balance_for_rent_exemption(ProtocolConfig::default().cpi_context_size as usize)
        .await
        .unwrap();
    let create_cpi_context_instruction = create_account_instruction(
        &payer.pubkey(),
        ProtocolConfig::default().cpi_context_size as usize,
        rent_cpi_config,
        &light_system_program::ID,
        Some(cpi_context_keypair),
    );
    let instruction = if registry {
        create_initialize_batched_merkle_tree_instruction(
            payer.pubkey(),
            merkle_tree_keypair.pubkey(),
            queue_keypair.pubkey(),
            cpi_context_keypair.pubkey(),
            params,
        )
    } else {
        let instruction =
            account_compression::instruction::InitializeBatchedStateMerkleTree { params };
        let accounts = account_compression::accounts::InitializeBatchedStateMerkleTreeAndQueue {
            authority: payer.pubkey(),
            merkle_tree: merkle_tree_keypair.pubkey(),
            queue: queue_keypair.pubkey(),
            registered_program_pda: None,
        };

        Instruction {
            program_id: account_compression::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction.data(),
        }
    };

    let transaction = Transaction::new_signed_with_payer(
        &[
            create_mt_account_ix,
            create_queue_account_ix,
            create_cpi_context_instruction,
            instruction,
        ],
        Some(&payer.pubkey()),
        &vec![
            payer,
            merkle_tree_keypair,
            queue_keypair,
            cpi_context_keypair,
        ],
        rpc.get_latest_blockhash().await.unwrap(),
    );
    rpc.process_transaction(transaction).await
}

pub async fn assert_registry_created_batched_state_merkle_tree<R: RpcConnection>(
    rpc: &mut R,
    payer_pubkey: Pubkey,
    merkle_tree_pubkey: Pubkey,
    output_queue_pubkey: Pubkey,
    // TODO: assert cpi_context_account creation
    _cpi_context_pubkey: Pubkey,
    params: InitStateTreeAccountsInstructionData,
) -> Result<(), RpcError> {
    let mut merkle_tree =
        AccountZeroCopy::<BatchedMerkleTreeAccount>::new(rpc, merkle_tree_pubkey).await;

    let mut queue = AccountZeroCopy::<BatchedQueueAccount>::new(rpc, output_queue_pubkey).await;
    let ref_mt_account = BatchedMerkleTreeAccount::get_state_tree_default(
        payer_pubkey,
        params.program_owner,
        params.forester,
        params.rollover_threshold,
        params.index,
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
        merkle_tree.account.data.as_mut_slice(),
        ref_mt_account,
        params.bloom_filter_num_iters,
    );

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
    let queue_rent = rpc
        .get_minimum_balance_for_rent_exemption(queue_account_size)
        .await
        .unwrap();
    let mt_rent = rpc
        .get_minimum_balance_for_rent_exemption(mt_account_size)
        .await
        .unwrap();
    let additional_bytes_rent = rpc
        .get_minimum_balance_for_rent_exemption(params.additional_bytes as usize)
        .await
        .unwrap();
    let total_rent = queue_rent + mt_rent + additional_bytes_rent;
    let ref_output_queue_account = get_output_queue_account_default(
        payer_pubkey,
        params.program_owner,
        params.forester,
        params.rollover_threshold,
        params.index,
        params.output_queue_batch_size,
        params.output_queue_zkp_batch_size,
        params.additional_bytes,
        total_rent,
        merkle_tree_pubkey,
        params.height,
        params.output_queue_num_batches,
    );

    assert_queue_zero_copy_inited(
        queue.account.data.as_mut_slice(),
        ref_output_queue_account,
        0, // output queue doesn't have a bloom filter hence no iterations
    );
    Ok(())
}
