use std::sync::Arc;

use borsh::BorshSerialize;
use forester_utils::indexer::Indexer;
use light_batched_merkle_tree::{
    batch::BatchState,
    constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
    merkle_tree::{
        BatchProofInputsIx, BatchedMerkleTreeAccount, InstructionDataBatchNullifyInputs,
    },
};
use light_client::{
    rpc::{RpcConnection, RpcError},
    rpc_pool::SolanaRpcPool,
};
use light_prover_client::{
    batch_address_append::get_batch_address_append_circuit_inputs,
    gnark::{
        batch_address_append_json_formatter::to_json,
        constants::{PROVE_PATH, SERVER_ADDRESS},
        proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    },
};
use light_registry::account_compression_cpi::sdk::create_batch_update_address_tree_instruction;
use light_utils::bigint::bigint_to_be_bytes_array;
use light_verifier::CompressedProof;
use reqwest::Client;
use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::Keypair, signer::Signer};
use tokio::sync::Mutex;
use tracing::info;

use crate::{errors::ForesterError, Result};

pub struct BatchedAddressOperations<R: RpcConnection, I: Indexer<R>> {
    pub rpc_pool: Arc<SolanaRpcPool<R>>,
    pub indexer: Arc<Mutex<I>>,
    pub authority: Keypair,
    pub derivation: Pubkey,
    pub epoch: u64,
    pub merkle_tree: Pubkey,
    pub output_queue: Pubkey,
}
impl<R: RpcConnection, I: Indexer<R>> BatchedAddressOperations<R, I> {
    async fn is_batch_ready(&self) -> bool {
        let mut rpc = self.rpc_pool.get_connection().await.unwrap();
        let is_batch_ready = {
            let mut account = rpc.get_account(self.merkle_tree).await.unwrap().unwrap();
            let merkle_tree =
                BatchedMerkleTreeAccount::address_tree_from_bytes_mut(account.data.as_mut_slice())
                    .unwrap();
            let batch_index = merkle_tree
                .get_metadata()
                .queue_metadata
                .next_full_batch_index;
            let full_batch = merkle_tree.batches.get(batch_index as usize).unwrap();

            info!("Batch state: {:?}", full_batch.get_state());
            info!(
                "Current zkp batch index: {:?}",
                full_batch.get_current_zkp_batch_index()
            );
            info!(
                "Num inserted zkps: {:?}",
                full_batch.get_num_inserted_zkps()
            );

            full_batch.get_state() != BatchState::Inserted
                && full_batch.get_current_zkp_batch_index() > full_batch.get_num_inserted_zkps()
        };
        is_batch_ready
    }

    pub async fn perform_batch_address_merkle_tree_update(&self) -> Result<usize> {
        info!("Performing batch address merkle tree update");
        let mut rpc = self.rpc_pool.get_connection().await?;
        let (instruction_data, batch_size) = self
            .create_batch_update_address_tree_instruction_data_with_proof()
            .await?;

        let instruction = create_batch_update_address_tree_instruction(
            self.authority.pubkey(),
            self.derivation,
            self.merkle_tree,
            self.epoch,
            instruction_data.try_to_vec()?,
        );
        let result = rpc
            .create_and_send_transaction(
                &[instruction],
                &self.authority.pubkey(),
                &[&self.authority],
            )
            .await;
        match result {
            Ok(sig) => {
                info!("Batch address update sent with signature: {:?}", sig);
                self.finalize_batch_address_merkle_tree_update().await?;
                Ok(batch_size)
            }
            Err(e) => {
                info!("Failed to send batch address update: {:?}", e);
                Err(ForesterError::from(e))
            }
        }
    }

    async fn finalize_batch_address_merkle_tree_update(&self) -> Result<()> {
        info!("Finalizing batch address merkle tree update");
        let mut rpc = self.rpc_pool.get_connection().await?;
        self.indexer
            .lock()
            .await
            .finalize_batched_address_tree_update(&mut *rpc, self.merkle_tree)
            .await;

        Ok(())
    }

    async fn create_batch_update_address_tree_instruction_data_with_proof(
        &self,
    ) -> Result<(InstructionDataBatchNullifyInputs, usize)> {
        let mut rpc = self.rpc_pool.get_connection().await?;

        let mut merkle_tree_account = rpc.get_account(self.merkle_tree).await?.unwrap();

        let (
            old_root_index,
            leaves_hashchain,
            start_index,
            current_root,
            batch_size,
            full_batch_index,
        ) = {
            let merkle_tree = BatchedMerkleTreeAccount::address_tree_from_bytes_mut(
                merkle_tree_account.data.as_mut_slice(),
            )
            .unwrap();

            let old_root_index = merkle_tree.root_history.last_index();
            let full_batch_index = merkle_tree
                .get_metadata()
                .queue_metadata
                .next_full_batch_index;
            let batch = &merkle_tree.batches[full_batch_index as usize];
            let zkp_batch_index = batch.get_num_inserted_zkps();
            let leaves_hashchain =
                merkle_tree.hashchain_store[full_batch_index as usize][zkp_batch_index as usize];
            let start_index = merkle_tree.get_metadata().next_index;
            let current_root = *merkle_tree.root_history.last().unwrap();
            let batch_size = batch.zkp_batch_size as usize;

            (
                old_root_index,
                leaves_hashchain,
                start_index,
                current_root,
                batch_size,
                full_batch_index,
            )
        };

        let batch_start_index = self
            .indexer
            .lock()
            .await
            .get_address_merkle_trees()
            .iter()
            .find(|x| x.accounts.merkle_tree == self.merkle_tree)
            .unwrap()
            .merkle_tree
            .merkle_tree
            .rightmost_index;

        let addresses = self
            .indexer
            .lock()
            .await
            .get_queue_elements(
                self.merkle_tree.to_bytes(),
                full_batch_index,
                0,
                batch_size as u64,
            )
            .await?;

        let batch_size = addresses.len();

        // // local_leaves_hashchain is only used for a test assertion.
        // let local_nullifier_hashchain = create_hash_chain_from_array(&addresses);
        // assert_eq!(leaves_hashchain, local_nullifier_hashchain);

        // Get proof info after addresses are retrieved
        let non_inclusion_proofs = self
            .indexer
            .lock()
            .await
            .get_multiple_new_address_proofs_full(self.merkle_tree.to_bytes(), addresses.clone())
            .await?;

        let mut low_element_values = Vec::new();
        let mut low_element_indices = Vec::new();
        let mut low_element_next_indices = Vec::new();
        let mut low_element_next_values = Vec::new();
        let mut low_element_proofs: Vec<Vec<[u8; 32]>> = Vec::new();

        for non_inclusion_proof in &non_inclusion_proofs {
            low_element_values.push(non_inclusion_proof.low_address_value);
            low_element_indices.push(non_inclusion_proof.low_address_index as usize);
            low_element_next_indices.push(non_inclusion_proof.low_address_next_index as usize);
            low_element_next_values.push(non_inclusion_proof.low_address_next_value);
            low_element_proofs.push(non_inclusion_proof.low_address_proof.to_vec());
        }

        let subtrees = self
            .indexer
            .lock()
            .await
            .get_subtrees(self.merkle_tree.to_bytes())
            .await?
            .try_into()
            .unwrap();

        let inputs = get_batch_address_append_circuit_inputs::<
            { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize },
        >(
            start_index as usize,
            current_root,
            low_element_values,
            low_element_next_values,
            low_element_indices,
            low_element_next_indices,
            low_element_proofs,
            addresses,
            subtrees,
            leaves_hashchain,
            batch_start_index,
            batch_size,
        )
        .map_err(|e| {
            ForesterError::Custom(format!(
                "Can't create batch address append circuit inputs: {:?}",
                e.to_string()
            ))
        })?;

        let client = Client::new();
        let circuit_inputs_new_root = bigint_to_be_bytes_array::<32>(&inputs.new_root).unwrap();
        let inputs = to_json(&inputs);

        let response_result = client
            .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(inputs)
            .send()
            .await
            .expect("Failed to execute request.");

        if response_result.status().is_success() {
            let body = response_result.text().await.unwrap();
            let proof_json = deserialize_gnark_proof_json(&body).unwrap();
            let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
            let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
            let instruction_data = InstructionDataBatchNullifyInputs {
                public_inputs: BatchProofInputsIx {
                    new_root: circuit_inputs_new_root,
                    old_root_index: old_root_index as u16,
                },
                compressed_proof: CompressedProof {
                    a: proof_a,
                    b: proof_b,
                    c: proof_c,
                },
            };
            Ok((instruction_data, batch_size))
        } else {
            Err(ForesterError::from(RpcError::CustomError(
                "Prover failed to generate proof".to_string(),
            )))
        }
    }
}

pub async fn process_batched_address_operations<R: RpcConnection, I: Indexer<R>>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    indexer: Arc<Mutex<I>>,
    authority: Keypair,
    derivation: Pubkey,
    epoch: u64,
    merkle_tree: Pubkey,
    output_queue: Pubkey,
) -> Result<usize> {
    let ops = BatchedAddressOperations {
        rpc_pool,
        indexer,
        authority,
        derivation,
        epoch,
        merkle_tree,
        output_queue,
    };

    info!("Processing batched address operations");

    if ops.is_batch_ready().await {
        info!("Batch is ready");
        let processed_count = ops.perform_batch_address_merkle_tree_update().await?;
        Ok(processed_count)
    } else {
        info!("Batch is not ready");
        Ok(0)
    }
}
