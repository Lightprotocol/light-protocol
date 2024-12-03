use crate::errors::ForesterError;
use crate::Result;
use account_compression::batched_merkle_tree::{
    AppendBatchProofInputsIx, BatchAppendEvent, BatchNullifyEvent, BatchProofInputsIx,
    InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
    ZeroCopyBatchedMerkleTreeAccount,
};
use account_compression::batched_queue::ZeroCopyBatchedQueueAccount;
use borsh::BorshSerialize;
use forester_utils::indexer::Indexer;
use light_client::rpc::RpcConnection;
use light_client::rpc_pool::SolanaRpcPool;
use light_hasher::{Hasher, Poseidon};
use light_prover_client::batch_append_with_proofs::get_batch_append_with_proofs_inputs;
use light_prover_client::batch_append_with_subtrees::calculate_hash_chain;
use light_prover_client::batch_update::get_batch_update_inputs;
use light_prover_client::gnark::batch_append_with_proofs_json_formatter::BatchAppendWithProofsInputsJson;
use light_prover_client::gnark::batch_update_json_formatter::update_inputs_string;
use light_prover_client::gnark::constants::{PROVE_PATH, SERVER_ADDRESS};
use light_prover_client::gnark::proof_helpers::{
    compress_proof, deserialize_gnark_proof_json, proof_from_json_struct,
};
use light_registry::account_compression_cpi::sdk::{
    create_batch_append_instruction, create_batch_nullify_instruction,
};
use light_utils::bigint::bigint_to_be_bytes_array;
use light_verifier::CompressedProof;
use reqwest::Client;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

pub struct BatchedOperations<R: RpcConnection, I: Indexer<R>> {
    pub rpc_pool: Arc<SolanaRpcPool<R>>,
    pub indexer: Arc<Mutex<I>>,
    pub authority: Keypair,
    pub derivation: Pubkey,
    pub epoch: u64,
    pub merkle_tree: Pubkey,
    pub output_queue: Pubkey,
}
impl<R: RpcConnection, I: Indexer<R>> BatchedOperations<R, I> {
    async fn is_batch_ready(&self) -> bool {
        let mut rpc = self.rpc_pool.get_connection().await.unwrap();
        // let is_batch_ready = {
        //     let mut output_queue_account =
        //         rpc.get_account(self.output_queue).await.unwrap().unwrap();
        //     let output_queue = ZeroCopyBatchedQueueAccount::from_bytes_mut(
        //         output_queue_account.data.as_mut_slice(),
        //     )
        //     .unwrap();
        //     let idx = output_queue.get_account().queue.next_full_batch_index;
        //     let batch = &output_queue.batches[idx as usize];
        //     println!("is_batch_ready: batch: {:?}", batch);
        //     batch.get_state() == BatchState::ReadyToUpdateTree
        // };
        // is_batch_ready

        let is_batch_ready = {
            let mut output_queue_account =
                rpc.get_account(self.output_queue).await.unwrap().unwrap();
            let output_queue = ZeroCopyBatchedQueueAccount::from_bytes_mut(
                output_queue_account.data.as_mut_slice(),
            )
            .unwrap();
            output_queue.get_batch_num_inserted_in_current_batch() > 0
        };
        is_batch_ready
    }

    pub async fn perform_batch_append(&self) -> Result<usize> {
        info!("=== perform_batch_append begin ===");
        let mut rpc = self.rpc_pool.get_connection().await?;

        let (num_inserted_zkps, batch_size) = {
            let mut output_queue_account =
                rpc.get_account(self.output_queue).await.unwrap().unwrap();
            let mut output_queue = ZeroCopyBatchedQueueAccount::from_bytes_mut(
                output_queue_account.data.as_mut_slice(),
            )
            .unwrap();
            let queue_account = output_queue.get_account();
            let batch_index = queue_account.queue.next_full_batch_index;

            println!("queue: {:?}", queue_account.queue);

            let num_inserted_zkps =
                output_queue.batches[batch_index as usize].get_num_inserted_zkps();
            let zkp_batch_size = queue_account.queue.zkp_batch_size;

            let batches = &mut output_queue.batches;
            let full_batch = batches.get_mut(batch_index as usize).unwrap();
            println!("full batch: {:?}", full_batch);

            (num_inserted_zkps, zkp_batch_size)
        };

        println!(
            "num_inserted_zkps: {}, batch_size: {}",
            num_inserted_zkps, batch_size
        );

        let instruction_data = self.create_append_batch_ix_data().await;

        let instruction = create_batch_append_instruction(
            self.authority.pubkey(),
            self.derivation,
            self.merkle_tree,
            self.output_queue,
            self.epoch,
            instruction_data?.try_to_vec()?,
        );

        let result = rpc
            .create_and_send_transaction_with_event::<BatchAppendEvent>(
                &[instruction],
                &self.authority.pubkey(),
                &[&self.authority],
                None,
            )
            .await?;
        println!("batch append result: {:?}", result);

        self.indexer
            .lock()
            .await
            .update_test_indexer_after_append(
                &mut rpc,
                self.merkle_tree,
                self.output_queue,
                num_inserted_zkps,
            )
            .await;

        info!("=== perform_batch_append end ===");
        Ok(batch_size as usize)
    }

    pub async fn perform_batch_nullify(&self) -> Result<usize> {
        info!("=== perform_batch_nullify begin ===");
        let mut rpc = self.rpc_pool.get_connection().await?;

        let instruction_data = self.get_batched_nullify_ix_data().await?;

        let instruction = create_batch_nullify_instruction(
            self.authority.pubkey(),
            self.derivation,
            self.merkle_tree,
            self.epoch,
            instruction_data.try_to_vec()?,
        );

        let result = rpc
            .create_and_send_transaction_with_event::<BatchNullifyEvent>(
                &[instruction],
                &self.authority.pubkey(),
                &[&self.authority],
                None,
            )
            .await?;

        println!("batch nullify result: {:?}", result);

        let (batch_index, batch_size) = {
            let mut account = rpc.get_account(self.merkle_tree).await.unwrap().unwrap();
            let merkle_tree =
                ZeroCopyBatchedMerkleTreeAccount::from_bytes_mut(account.data.as_mut_slice())
                    .unwrap();
            (
                merkle_tree.get_account().queue.next_full_batch_index,
                merkle_tree.get_account().queue.zkp_batch_size,
            )
        };

        self.indexer
            .lock()
            .await
            .update_test_indexer_after_nullification(
                &mut rpc,
                self.merkle_tree,
                batch_index as usize,
            )
            .await;

        info!("=== perform_batch_nullify end ===");
        Ok(batch_size as usize)
    }

    async fn create_append_batch_ix_data(&self) -> Result<InstructionDataBatchAppendInputs> {
        info!("=== create_append_batch_ix_data begin ===");
        let mut rpc = self.rpc_pool.get_connection().await.unwrap();

        let (merkle_tree_next_index, current_root) = {
            let mut merkle_tree_account = rpc.get_account(self.merkle_tree).await.unwrap().unwrap();
            let merkle_tree = ZeroCopyBatchedMerkleTreeAccount::from_bytes_mut(
                merkle_tree_account.data.as_mut_slice(),
            )
            .unwrap();
            (
                merkle_tree.get_account().next_index,
                *merkle_tree.root_history.last().unwrap(),
            )
        };

        info!("Merkle tree next index: {}", merkle_tree_next_index);
        info!("Current root: {:?}", current_root);

        let (zkp_batch_size, _full_batch_index, num_inserted_zkps, leaves_hashchain) = {
            let mut output_queue_account =
                rpc.get_account(self.output_queue).await.unwrap().unwrap();
            let output_queue = ZeroCopyBatchedQueueAccount::from_bytes_mut(
                output_queue_account.data.as_mut_slice(),
            )
            .unwrap();

            let queue_account = output_queue.get_account();
            let full_batch_index = queue_account.queue.next_full_batch_index;
            let zkp_batch_size = queue_account.queue.zkp_batch_size;

            let num_inserted_zkps =
                output_queue.batches[full_batch_index as usize].get_num_inserted_zkps();

            let leaves_hashchain =
                output_queue.hashchain_store[full_batch_index as usize][num_inserted_zkps as usize];

            (
                zkp_batch_size,
                full_batch_index,
                num_inserted_zkps,
                leaves_hashchain,
            )
        };
        info!("ZKP batch size: {}", zkp_batch_size);
        info!("Number of inserted zkps: {}", num_inserted_zkps);

        let start = num_inserted_zkps as usize * zkp_batch_size as usize;
        let end = start + zkp_batch_size as usize;

        let leaves = self
            .indexer
            .lock()
            .await
            .get_queue_elements(self.merkle_tree, start as u64, end as u64)
            .await
            .unwrap();

        let local_leaves_hashchain = calculate_hash_chain(&leaves);
        info!("start index: {}", start);
        info!("end index: {}", end);
        info!("num inserted zkps: {}", num_inserted_zkps);
        info!("zkp batch size: {}", zkp_batch_size);
        assert_eq!(local_leaves_hashchain, leaves_hashchain);
        // info!("In hash chain Batch update leaves: {:?}", leaves);

        let (old_leaves, merkle_proofs) = {
            let mut old_leaves = vec![];
            let mut merkle_proofs = vec![];
            let indices = (merkle_tree_next_index..merkle_tree_next_index + zkp_batch_size)
                .collect::<Vec<_>>();
            let proofs = self
                .indexer
                .lock()
                .await
                .get_proofs_by_indices(self.merkle_tree, &indices);
            proofs.iter().for_each(|proof| {
                old_leaves.push(proof.leaf);
                merkle_proofs.push(proof.proof.clone());
            });

            (old_leaves, merkle_proofs)
        };

        let leaf_strings = leaves
            .iter()
            .map(|l| Pubkey::from(*l).to_string())
            .collect::<Vec<_>>();
        println!("leaves: {:?}", leaf_strings);

        let old_leaf_strings = old_leaves
            .iter()
            .map(|l| Pubkey::from(*l).to_string())
            .collect::<Vec<_>>();
        println!("old_leaves: {:?}", old_leaf_strings);

        let (proof, new_root) = {
            let circuit_inputs = get_batch_append_with_proofs_inputs::<26>(
                current_root,
                merkle_tree_next_index as u32,
                leaves,
                leaves_hashchain,
                old_leaves,
                merkle_proofs,
                zkp_batch_size as u32,
            );

            let client = Client::new();
            let inputs_json =
                BatchAppendWithProofsInputsJson::from_inputs(&circuit_inputs).to_string();

            let response = client
                .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(inputs_json)
                .send()
                .await
                .expect("Failed to execute request.");

            if response.status().is_success() {
                let body = response.text().await.unwrap();
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
                error!(
                    "create_append_batch_ix_data: failed to get proof from server: {:?}",
                    response.text().await
                );
                return Err(ForesterError::Custom(
                    "Failed to get proof from server".into(),
                ));
            }
        };

        info!("=== create_append_batch_ix_data end ===");

        Ok(InstructionDataBatchAppendInputs {
            public_inputs: AppendBatchProofInputsIx { new_root },
            compressed_proof: proof,
        })
    }

    async fn get_batched_nullify_ix_data(&self) -> Result<InstructionDataBatchNullifyInputs> {
        info!("=== get_batched_nullify_ix_data begin ===");
        let mut rpc = self.rpc_pool.get_connection().await.unwrap();

        let (zkp_batch_size, old_root, old_root_index, leaves_hashchain) = {
            let mut account = rpc.get_account(self.merkle_tree).await.unwrap().unwrap();
            let merkle_tree =
                ZeroCopyBatchedMerkleTreeAccount::from_bytes_mut(account.data.as_mut_slice())
                    .unwrap();
            let account = merkle_tree.get_account();
            let batch_idx = account.queue.next_full_batch_index as usize;
            let zkp_size = account.queue.zkp_batch_size;
            let batch = &merkle_tree.batches[batch_idx];
            let zkp_idx = batch.get_num_inserted_zkps();

            let hashchains = merkle_tree
                .hashchain_store
                .clone()
                .iter()
                .map(|x| {
                    let x = x.clone();
                    x.as_slice().to_vec()
                })
                .collect::<Vec<_>>();
            for (i, x) in hashchains.iter().enumerate() {
                println!("hashchain {}: {:?}", i, x);
            }

            let hashchain = merkle_tree.hashchain_store[batch_idx][zkp_idx as usize];
            let root_idx = merkle_tree.root_history.last_index();
            let root = *merkle_tree.root_history.last().unwrap();

            (zkp_size, root, root_idx, hashchain)
        };

        let leaf_indices_tx_hashes = self
            .indexer
            .lock()
            .await
            .get_leaf_indices_tx_hashes(self.merkle_tree, zkp_batch_size as usize);

        let mut leaves = Vec::new();
        let mut tx_hashes = Vec::new();
        let mut old_leaves = Vec::new();
        let mut path_indices = Vec::new();
        let mut merkle_proofs = Vec::new();
        let mut nullifiers = Vec::new();

        let proofs = self.indexer.lock().await.get_proofs_by_indices(
            self.merkle_tree,
            &leaf_indices_tx_hashes
                .iter()
                .map(|(index, _, _)| *index as u64)
                .collect::<Vec<_>>(),
        );

        for ((index, leaf, tx_hash), proof) in leaf_indices_tx_hashes.iter().zip(proofs.iter()) {
            path_indices.push(*index);
            leaves.push(*leaf);
            old_leaves.push(proof.leaf);
            merkle_proofs.push(proof.proof.clone());
            tx_hashes.push(*tx_hash);
            let index_bytes = index.to_be_bytes();
            let nullifier = Poseidon::hashv(&[leaf, &index_bytes, tx_hash]).unwrap();
            nullifiers.push(nullifier);
        }

        let leaf_strings = leaves
            .iter()
            .map(|l| Pubkey::from(*l).to_string())
            .collect::<Vec<_>>();

        let old_leaf_strings = old_leaves
            .iter()
            .map(|l| Pubkey::from(*l).to_string())
            .collect::<Vec<_>>();

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

        let new_root =
            bigint_to_be_bytes_array::<32>(&inputs.new_root.to_biguint().unwrap()).unwrap();

        let client = Client::new();
        let response = client
            .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(update_inputs_string(&inputs))
            .send()
            .await?;

        let proof = if response.status().is_success() {
            let body = response.text().await.unwrap();
            let proof_json = deserialize_gnark_proof_json(&body).unwrap();
            let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
            let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
            CompressedProof {
                a: proof_a,
                b: proof_b,
                c: proof_c,
            }
        } else {
            error!(
                "get_batched_nullify_ix_data: failed to get proof from server: {:?}",
                response.text().await
            );
            return Err(ForesterError::Custom(
                "Failed to get proof from server".into(),
            ));
        };

        info!("=== get_batched_nullify_ix_data end ===");

        Ok(InstructionDataBatchNullifyInputs {
            public_inputs: BatchProofInputsIx {
                new_root,
                old_root_index: old_root_index as u16,
            },
            compressed_proof: proof,
        })
    }
}

pub async fn process_batched_operations<R: RpcConnection, I: Indexer<R>>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    indexer: Arc<Mutex<I>>,
    authority: Keypair,
    derivation: Pubkey,
    epoch: u64,
    merkle_tree: Pubkey,
    output_queue: Pubkey,
) -> Result<usize> {
    let ops = BatchedOperations {
        rpc_pool,
        indexer,
        authority,
        derivation,
        epoch,
        merkle_tree,
        output_queue,
    };

    if ops.is_batch_ready().await {
        let processed_appends_count = ops.perform_batch_append().await?;
        let processed_nullifications_count = ops.perform_batch_nullify().await?;
        Ok(processed_appends_count + processed_nullifications_count)
    } else {
        Ok(0)
    }
}
