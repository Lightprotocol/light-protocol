use std::{marker::PhantomData, time::Duration};

use account_compression::{
    AddressMerkleTreeAccount, AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig,
    StateMerkleTreeAccount, StateMerkleTreeConfig,
};
use async_trait::async_trait;
use borsh::BorshDeserialize;
use forester_utils::{get_concurrent_merkle_tree, get_indexed_merkle_tree, AccountZeroCopy};
use light_batched_merkle_tree::{
    batch::BatchState,
    constants::{DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, DEFAULT_BATCH_STATE_TREE_HEIGHT},
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount,
    queue::{BatchedQueueAccount, BatchedQueueMetadata},
};
use light_client::{
    indexer::{
        AddressMerkleTreeAccounts, AddressMerkleTreeBundle, Indexer, IndexerError, MerkleProof,
        NewAddressProofWithContext, ProofOfLeaf, StateMerkleTreeAccounts, StateMerkleTreeBundle,
    },
    rpc::{merkle_tree::MerkleTreeExt, RpcConnection},
    transaction_params::FeeConfig,
};
use light_hasher::{Hasher, Poseidon};
use light_indexed_merkle_tree::{array::IndexedArray, reference::IndexedMerkleTree};
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::{
    gnark::{
        combined_json_formatter::CombinedJsonStruct,
        combined_json_formatter_legacy::CombinedJsonStruct as CombinedJsonStructLegacy,
        constants::{PROVE_PATH, SERVER_ADDRESS},
        helpers::{big_int_to_string, spawn_prover, string_to_big_int, ProofType, ProverConfig},
        inclusion_json_formatter::BatchInclusionJsonStruct,
        inclusion_json_formatter_legacy::BatchInclusionJsonStruct as BatchInclusionJsonStructLegacy,
        non_inclusion_json_formatter::BatchNonInclusionJsonStruct,
        non_inclusion_json_formatter_legacy::BatchNonInclusionJsonStruct as BatchNonInclusionJsonStructLegacy,
        proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    },
    helpers::bigint_to_u8_32,
    inclusion::merkle_inclusion_proof_inputs::{InclusionMerkleProofInputs, InclusionProofInputs},
    inclusion_legacy::merkle_inclusion_proof_inputs::InclusionProofInputs as InclusionProofInputsLegacy,
    non_inclusion::merkle_non_inclusion_proof_inputs::{
        get_non_inclusion_proof_inputs, NonInclusionProofInputs,
    },
    non_inclusion_legacy::merkle_non_inclusion_proof_inputs::NonInclusionProofInputs as NonInclusionProofInputsLegacy,
};
use light_sdk::{
    compressed_account::CompressedAccountWithMerkleContext,
    event::PublicTransactionEvent,
    merkle_context::MerkleContext,
    proof::{BatchedTreeProofRpcResult, CompressedProof, ProofRpcResult},
    token::{TokenData, TokenDataWithMerkleContext},
    STATE_MERKLE_TREE_CANOPY_DEPTH,
};
use light_utils::{
    bigint::bigint_to_be_bytes_array,
    hashchain::{create_hash_chain_from_slice, create_tx_hash},
};
use log::{info, warn};
use num_bigint::{BigInt, BigUint};
use num_traits::FromBytes;
use reqwest::Client;
use solana_sdk::{
    bs58,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use light_client::indexer::LeafIndexInfo;
use crate::{
    indexer::{
        utils::create_address_merkle_tree_and_queue_account_with_assert, TestIndexerExtensions,
    },
    test_batch_forester::{create_batch_address_merkle_tree, create_batched_state_merkle_tree},
    test_env::{
        create_state_merkle_tree_and_queue_account, EnvAccounts, BATCHED_OUTPUT_QUEUE_TEST_KEYPAIR,
    },
};

#[derive(Debug)]
pub struct TestIndexer<R>
where
    R: RpcConnection + MerkleTreeExt,
{
    pub state_merkle_trees: Vec<StateMerkleTreeBundle>,
    pub address_merkle_trees: Vec<AddressMerkleTreeBundle>,
    pub payer: Keypair,
    pub group_pda: Pubkey,
    pub compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub nullified_compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub token_compressed_accounts: Vec<TokenDataWithMerkleContext>,
    pub token_nullified_compressed_accounts: Vec<TokenDataWithMerkleContext>,
    pub events: Vec<PublicTransactionEvent>,
    pub prover_config: Option<ProverConfig>,
    phantom: PhantomData<R>,
}

#[async_trait]
impl<R> Indexer<R> for TestIndexer<R>
where
    R: RpcConnection + MerkleTreeExt,
{
    async fn get_queue_elements(
        &self,
        pubkey: [u8; 32],
        _batch: u64,
        start_offset: u64,
        end_offset: u64,
    ) -> Result<Vec<[u8; 32]>, IndexerError> {
        let pubkey = Pubkey::new_from_array(pubkey);
        let address_tree_bundle = self
            .address_merkle_trees
            .iter()
            .find(|x| x.accounts.merkle_tree == pubkey);
        if let Some(address_tree_bundle) = address_tree_bundle {
            return Ok(address_tree_bundle.queue_elements
                [start_offset as usize..end_offset as usize]
                .to_vec());
        }
        let state_tree_bundle = self
            .state_merkle_trees
            .iter()
            .find(|x| x.accounts.merkle_tree == pubkey);
        if let Some(state_tree_bundle) = state_tree_bundle {
            return Ok(state_tree_bundle.output_queue_elements
                [start_offset as usize..end_offset as usize]
                .to_vec());
        }
        Err(IndexerError::Custom("Merkle tree not found".to_string()))
    }

    fn get_subtrees(&self, merkle_tree_pubkey: [u8; 32]) -> Result<Vec<[u8; 32]>, IndexerError> {
        let merkle_tree_pubkey = Pubkey::new_from_array(merkle_tree_pubkey);
        let address_tree_bundle = self
            .address_merkle_trees
            .iter()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey);
        if let Some(address_tree_bundle) = address_tree_bundle {
            Ok(address_tree_bundle.merkle_tree.merkle_tree.get_subtrees())
        } else {
            let state_tree_bundle = self
                .state_merkle_trees
                .iter()
                .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey);
            if let Some(state_tree_bundle) = state_tree_bundle {
                Ok(state_tree_bundle.merkle_tree.get_subtrees())
            } else {
                Err(IndexerError::Custom("Merkle tree not found".to_string()))
            }
        }
    }

    // fn add_event_and_compressed_accounts(
    //     &mut self,
    //     event: &PublicTransactionEvent,
    // ) -> (
    //     Vec<CompressedAccountWithMerkleContext>,
    //     Vec<TokenDataWithMerkleContext>,
    // ) {
    //     for hash in event.input_compressed_account_hashes.iter() {
    //         let index = self.compressed_accounts.iter().position(|x| {
    //             x.compressed_account
    //                 .hash::<Poseidon>(
    //                     &x.merkle_context.merkle_tree_pubkey,
    //                     &x.merkle_context.leaf_index,
    //                 )
    //                 .unwrap()
    //                 == *hash
    //         });
    //         if let Some(index) = index {
    //             self.nullified_compressed_accounts
    //                 .push(self.compressed_accounts[index].clone());
    //             self.compressed_accounts.remove(index);
    //             continue;
    //         };
    //         if index.is_none() {
    //             let index = self
    //                 .token_compressed_accounts
    //                 .iter()
    //                 .position(|x| {
    //                     x.compressed_account
    //                         .compressed_account
    //                         .hash::<Poseidon>(
    //                             &x.compressed_account.merkle_context.merkle_tree_pubkey,
    //                             &x.compressed_account.merkle_context.leaf_index,
    //                         )
    //                         .unwrap()
    //                         == *hash
    //                 })
    //                 .expect("input compressed account not found");
    //             self.token_nullified_compressed_accounts
    //                 .push(self.token_compressed_accounts[index].clone());
    //             self.token_compressed_accounts.remove(index);
    //         }
    //     }
    //
    //     let mut compressed_accounts = Vec::new();
    //     let mut token_compressed_accounts = Vec::new();
    //     for (i, compressed_account) in event.output_compressed_accounts.iter().enumerate() {
    //         let nullifier_queue_pubkey = self
    //             .state_merkle_trees
    //             .iter()
    //             .find(|x| {
    //                 x.accounts.merkle_tree
    //                     == event.pubkey_array
    //                         [event.output_compressed_accounts[i].merkle_tree_index as usize]
    //             })
    //             .unwrap()
    //             .accounts
    //             .nullifier_queue;
    //         // if data is some, try to deserialize token data, if it fails, add to compressed_accounts
    //         // if data is none add to compressed_accounts
    //         // new accounts are inserted in front so that the newest accounts are found first
    //         match compressed_account.compressed_account.data.as_ref() {
    //             Some(data) => {
    //                 if compressed_account.compressed_account.owner == PROGRAM_ID_LIGHT_SYSTEM
    //                     && data.discriminator == TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR
    //                 {
    //                     if let Ok(token_data) = TokenData::deserialize(&mut data.data.as_slice()) {
    //                         let token_account = TokenDataWithMerkleContext {
    //                             token_data,
    //                             compressed_account: CompressedAccountWithMerkleContext {
    //                                 compressed_account: compressed_account
    //                                     .compressed_account
    //                                     .clone(),
    //                                 merkle_context: MerkleContext {
    //                                     leaf_index: event.output_leaf_indices[i],
    //                                     merkle_tree_pubkey: event.pubkey_array[event
    //                                         .output_compressed_accounts[i]
    //                                         .merkle_tree_index
    //                                         as usize],
    //                                     nullifier_queue_pubkey,
    //                                     queue_index: None,
    //                                 },
    //                             },
    //                         };
    //                         token_compressed_accounts.push(token_account.clone());
    //                         self.token_compressed_accounts.insert(0, token_account);
    //                     }
    //                 } else {
    //                     let compressed_account = CompressedAccountWithMerkleContext {
    //                         compressed_account: compressed_account.compressed_account.clone(),
    //                         merkle_context: MerkleContext {
    //                             leaf_index: event.output_leaf_indices[i],
    //                             merkle_tree_pubkey: event.pubkey_array[event
    //                                 .output_compressed_accounts[i]
    //                                 .merkle_tree_index
    //                                 as usize],
    //                             nullifier_queue_pubkey,
    //                             queue_index: None,
    //                         },
    //                     };
    //                     compressed_accounts.push(compressed_account.clone());
    //                     self.compressed_accounts.insert(0, compressed_account);
    //                 }
    //             }
    //             None => {
    //                 let compressed_account = CompressedAccountWithMerkleContext {
    //                     compressed_account: compressed_account.compressed_account.clone(),
    //                     merkle_context: MerkleContext {
    //                         leaf_index: event.output_leaf_indices[i],
    //                         merkle_tree_pubkey: event.pubkey_array
    //                             [event.output_compressed_accounts[i].merkle_tree_index as usize],
    //                         nullifier_queue_pubkey,
    //                         queue_index: None,
    //                     },
    //                 };
    //                 compressed_accounts.push(compressed_account.clone());
    //                 self.compressed_accounts.insert(0, compressed_account);
    //             }
    //         };
    //         let merkle_tree = &mut self
    //             .state_merkle_trees
    //             .iter_mut()
    //             .find(|x| {
    //                 x.accounts.merkle_tree
    //                     == event.pubkey_array
    //                         [event.output_compressed_accounts[i].merkle_tree_index as usize]
    //             })
    //             .unwrap()
    //             .merkle_tree;
    //         merkle_tree
    //             .append(
    //                 &compressed_account
    //                     .compressed_account
    //                     .hash::<Poseidon>(
    //                         &event.pubkey_array
    //                             [event.output_compressed_accounts[i].merkle_tree_index as usize],
    //                         &event.output_leaf_indices[i],
    //                     )
    //                     .unwrap(),
    //             )
    //             .expect("insert failed");
    //     }
    //
    //     self.events.push(event.clone());
    //     (compressed_accounts, token_compressed_accounts)
    // }

    async fn create_proof_for_compressed_accounts(
        &mut self,
        compressed_accounts: Option<Vec<[u8; 32]>>,
        state_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        new_addresses: Option<&[[u8; 32]]>,
        address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        rpc: &mut R,
    ) -> ProofRpcResult {
        if compressed_accounts.is_some()
            && ![1usize, 2usize, 3usize, 4usize, 8usize]
                .contains(&compressed_accounts.as_ref().unwrap().len())
        {
            panic!(
                "compressed_accounts must be of length 1, 2, 3, 4 or 8 != {}",
                compressed_accounts.unwrap().len()
            )
        }
        if new_addresses.is_some() && ![1usize, 2usize].contains(&new_addresses.unwrap().len()) {
            panic!("new_addresses must be of length 1, 2")
        }
        let client = Client::new();
        let (root_indices, address_root_indices, json_payload) =
            match (compressed_accounts, new_addresses) {
                (Some(accounts), None) => {
                    let (payload, payload_legacy, indices) = self
                        .process_inclusion_proofs(
                            &state_merkle_tree_pubkeys.unwrap(),
                            &accounts,
                            rpc,
                        )
                        .await;
                    if let Some(payload) = payload {
                        (indices, Vec::new(), payload.to_string())
                    } else {
                        (indices, Vec::new(), payload_legacy.unwrap().to_string())
                    }
                }
                (None, Some(addresses)) => {
                    let (payload, payload_legacy, indices) = self
                        .process_non_inclusion_proofs(
                            address_merkle_tree_pubkeys.unwrap().as_slice(),
                            addresses,
                            rpc,
                        )
                        .await;
                    let payload_string = if let Some(payload) = payload {
                        payload.to_string()
                    } else {
                        payload_legacy.unwrap().to_string()
                    };
                    (Vec::<u16>::new(), indices, payload_string)
                }
                (Some(accounts), Some(addresses)) => {
                    let (inclusion_payload, inclusion_payload_legacy, inclusion_indices) = self
                        .process_inclusion_proofs(
                            &state_merkle_tree_pubkeys.unwrap(),
                            &accounts,
                            rpc,
                        )
                        .await;

                    let (
                        non_inclusion_payload,
                        non_inclusion_payload_legacy,
                        non_inclusion_indices,
                    ) = self
                        .process_non_inclusion_proofs(
                            address_merkle_tree_pubkeys.unwrap().as_slice(),
                            addresses,
                            rpc,
                        )
                        .await;
                    let json_payload = if let Some(non_inclusion_payload) = non_inclusion_payload {
                        let public_input_hash = BigInt::from_bytes_be(
                            num_bigint::Sign::Plus,
                            &create_hash_chain_from_slice(&[
                                bigint_to_u8_32(
                                    &string_to_big_int(
                                        &inclusion_payload.as_ref().unwrap().public_input_hash,
                                    )
                                    .unwrap(),
                                )
                                .unwrap(),
                                bigint_to_u8_32(
                                    &string_to_big_int(&non_inclusion_payload.public_input_hash)
                                        .unwrap(),
                                )
                                .unwrap(),
                            ])
                            .unwrap(),
                        );
                        println!(
                            "inclusion public input hash offchain {:?}",
                            bigint_to_u8_32(
                                &string_to_big_int(
                                    &inclusion_payload.as_ref().unwrap().public_input_hash,
                                )
                                .unwrap(),
                            )
                            .unwrap()
                        );
                        println!(
                            "non inclusion public input hash offchain {:?}",
                            bigint_to_u8_32(
                                &string_to_big_int(&non_inclusion_payload.public_input_hash)
                                    .unwrap()
                            )
                            .unwrap()
                        );

                        println!(
                            "public input hash offchain {:?}",
                            public_input_hash.to_bytes_be()
                        );

                        CombinedJsonStruct {
                            circuit_type: ProofType::Combined.to_string(),
                            state_tree_height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
                            address_tree_height: DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
                            public_input_hash: big_int_to_string(&public_input_hash),
                            inclusion: inclusion_payload.unwrap().inputs,
                            non_inclusion: non_inclusion_payload.inputs,
                        }
                        .to_string()
                    } else if let Some(non_inclusion_payload) = non_inclusion_payload_legacy {
                        CombinedJsonStructLegacy {
                            circuit_type: ProofType::Combined.to_string(),
                            state_tree_height: 26,
                            address_tree_height: 26,
                            inclusion: inclusion_payload_legacy.unwrap().inputs,
                            non_inclusion: non_inclusion_payload.inputs,
                        }
                        .to_string()
                    } else {
                        panic!("Unsupported tree height")
                    };
                    (inclusion_indices, non_inclusion_indices, json_payload)
                }
                _ => {
                    panic!("At least one of compressed_accounts or new_addresses must be provided")
                }
            };

        println!("json_payload {:?}", json_payload);
        let mut retries = 3;
        while retries > 0 {
            let response_result = client
                .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(json_payload.clone())
                .send()
                .await
                .expect("Failed to execute request.");
            println!("response_result {:?}", response_result);
            if response_result.status().is_success() {
                let body = response_result.text().await.unwrap();
                println!("body {:?}", body);
                println!("root_indices {:?}", root_indices);
                println!("address_root_indices {:?}", address_root_indices);
                let proof_json = deserialize_gnark_proof_json(&body).unwrap();
                let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
                let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
                let root_indices = root_indices.iter().map(|x| Some(*x)).collect();
                return ProofRpcResult {
                    root_indices,
                    address_root_indices: address_root_indices.clone(),
                    proof: CompressedProof {
                        a: proof_a,
                        b: proof_b,
                        c: proof_c,
                    },
                };
            } else {
                warn!("Error: {}", response_result.text().await.unwrap());
                tokio::time::sleep(Duration::from_secs(1)).await;
                retries -= 1;
            }
        }
        panic!("Failed to get proof from server");
    }

    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<MerkleProof>, IndexerError> {
        info!("Getting proofs for {:?}", hashes);
        let mut proofs: Vec<MerkleProof> = Vec::new();
        hashes.iter().for_each(|hash| {
            let hash_array: [u8; 32] = bs58::decode(hash)
                .into_vec()
                .unwrap()
                .as_slice()
                .try_into()
                .unwrap();

            self.state_merkle_trees.iter().for_each(|tree| {
                if let Some(leaf_index) = tree.merkle_tree.get_leaf_index(&hash_array) {
                    let proof = tree
                        .merkle_tree
                        .get_proof_of_leaf(leaf_index, false)
                        .unwrap();
                    proofs.push(MerkleProof {
                        hash: hash.clone(),
                        leaf_index: leaf_index as u64,
                        merkle_tree: tree.accounts.merkle_tree.to_string(),
                        proof: proof.to_vec(),
                        root_seq: tree.merkle_tree.sequence_number as u64,
                    });
                }
            })
        });
        Ok(proofs)
    }

    /// Returns compressed accounts owned by the given `owner`.
    // fn get_compressed_accounts_by_owner(
    //     &self,
    //     owner: &Pubkey,
    // ) -> Vec<CompressedAccountWithMerkleContext> {
    //     self.compressed_accounts
    //         .iter()
    //         .filter(|x| x.compressed_account.owner == *owner)
    //         .cloned()
    //         .collect()
    // }
    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<String>, IndexerError> {
        let result = self.get_compressed_accounts_with_merkle_context_by_owner(owner);
        let mut hashes: Vec<String> = Vec::new();
        for account in result.iter() {
            let hash = account.hash().unwrap();
            let bs58_hash = bs58::encode(hash).into_string();
            hashes.push(bs58_hash);
        }
        Ok(hashes)
    }

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<16>>, IndexerError> {
        self._get_multiple_new_address_proofs(merkle_tree_pubkey, addresses, false)
            .await
    }

    async fn get_multiple_new_address_proofs_full(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<40>>, IndexerError> {
        self._get_multiple_new_address_proofs(merkle_tree_pubkey, addresses, true)
            .await
    }

    fn get_proofs_by_indices(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        indices: &[u64],
    ) -> Vec<ProofOfLeaf> {
        indices
            .iter()
            .map(|&index| self.get_proof_by_index(merkle_tree_pubkey, index))
            .collect()
    }

    fn get_leaf_indices_tx_hashes(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        zkp_batch_size: usize,
    ) -> Vec<LeafIndexInfo> {
        let state_merkle_tree_bundle = self
            .state_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
            .unwrap();

        state_merkle_tree_bundle.input_leaf_indices[..zkp_batch_size].to_vec()
    }

    fn get_address_merkle_trees(&self) -> &Vec<AddressMerkleTreeBundle> {
        &self.address_merkle_trees
    }
}

#[async_trait]
impl<R> TestIndexerExtensions<R> for TestIndexer<R>
where
    R: RpcConnection + MerkleTreeExt,
{
    fn get_address_merkle_tree(
        &self,
        merkle_tree_pubkey: Pubkey,
    ) -> Option<&AddressMerkleTreeBundle> {
        self.address_merkle_trees
            .iter()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
    }

    /// deserializes an event
    /// adds the output_compressed_accounts to the compressed_accounts
    /// removes the input_compressed_accounts from the compressed_accounts
    /// adds the input_compressed_accounts to the nullified_compressed_accounts
    /// deserialiazes token data from the output_compressed_accounts
    /// adds the token_compressed_accounts to the token_compressed_accounts
    fn add_compressed_accounts_with_token_data(
        &mut self,
        slot: u64,
        event: &PublicTransactionEvent,
    ) {
        self.add_event_and_compressed_accounts(slot, event);
    }

    fn account_nullified(&mut self, merkle_tree_pubkey: Pubkey, account_hash: &str) {
        let decoded_hash: [u8; 32] = bs58::decode(account_hash)
            .into_vec()
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap();

        if let Some(state_tree_bundle) = self
            .state_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
        {
            if let Some(leaf_index) = state_tree_bundle.merkle_tree.get_leaf_index(&decoded_hash) {
                state_tree_bundle
                    .merkle_tree
                    .update(&[0u8; 32], leaf_index)
                    .unwrap();
            }
        }
    }

    fn address_tree_updated(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        context: &NewAddressProofWithContext<16>,
    ) {
        info!("Updating address tree...");
        let address_tree_bundle: &mut AddressMerkleTreeBundle = self
            .address_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
            .unwrap();

        let new_low_element = context.new_low_element.clone().unwrap();
        let new_element = context.new_element.clone().unwrap();
        let new_element_next_value = context.new_element_next_value.clone().unwrap();
        address_tree_bundle
            .merkle_tree
            .update(&new_low_element, &new_element, &new_element_next_value)
            .unwrap();
        address_tree_bundle
            .indexed_array
            .append_with_low_element_index(new_low_element.index, &new_element.value)
            .unwrap();
        info!("Address tree updated");
    }

    fn get_state_merkle_tree_accounts(&self, pubkeys: &[Pubkey]) -> Vec<StateMerkleTreeAccounts> {
        pubkeys
            .iter()
            .map(|x| {
                self.state_merkle_trees
                    .iter()
                    .find(|y| y.accounts.merkle_tree == *x || y.accounts.nullifier_queue == *x)
                    .unwrap()
                    .accounts
            })
            .collect::<Vec<_>>()
    }

    fn get_state_merkle_trees(&self) -> &Vec<StateMerkleTreeBundle> {
        &self.state_merkle_trees
    }

    fn get_state_merkle_trees_mut(&mut self) -> &mut Vec<StateMerkleTreeBundle> {
        &mut self.state_merkle_trees
    }

    fn get_address_merkle_trees_mut(&mut self) -> &mut Vec<AddressMerkleTreeBundle> {
        &mut self.address_merkle_trees
    }

    fn get_token_compressed_accounts(&self) -> &Vec<TokenDataWithMerkleContext> {
        &self.token_compressed_accounts
    }

    fn get_payer(&self) -> &Keypair {
        &self.payer
    }

    fn get_group_pda(&self) -> &Pubkey {
        &self.group_pda
    }

    async fn create_proof_for_compressed_accounts2(
        &mut self,
        compressed_accounts: Option<Vec<[u8; 32]>>,
        state_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        new_addresses: Option<&[[u8; 32]]>,
        address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        rpc: &mut R,
    ) -> BatchedTreeProofRpcResult {
        let mut indices_to_remove = Vec::new();

        // for all accounts in batched trees, check whether values are in tree or queue
        let (compressed_accounts, state_merkle_tree_pubkeys) =
            if let Some((compressed_accounts, state_merkle_tree_pubkeys)) =
                compressed_accounts.zip(state_merkle_tree_pubkeys)
            {
                for (i, (compressed_account, state_merkle_tree_pubkey)) in compressed_accounts
                    .iter()
                    .zip(state_merkle_tree_pubkeys.iter())
                    .enumerate()
                {
                    let accounts = self.state_merkle_trees.iter().find(|x| {
                        x.accounts.merkle_tree == *state_merkle_tree_pubkey && x.version == 2
                    });
                    if let Some(accounts) = accounts {
                        let output_queue_pubkey = accounts.accounts.nullifier_queue;
                        let mut queue =
                            AccountZeroCopy::<BatchedQueueMetadata>::new(rpc, output_queue_pubkey)
                                .await;
                        let queue_zero_copy = BatchedQueueAccount::output_queue_from_bytes_mut(
                            queue.account.data.as_mut_slice(),
                        )
                        .unwrap();
                        for value_array in queue_zero_copy.value_vecs.iter() {
                            let index = value_array.iter().position(|x| *x == *compressed_account);
                            if index.is_some() {
                                indices_to_remove.push(i);
                            }
                        }
                    }
                }
                let compress_accounts = compressed_accounts
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !indices_to_remove.contains(i))
                    .map(|(_, x)| *x)
                    .collect::<Vec<_>>();
                let state_merkle_tree_pubkeys = state_merkle_tree_pubkeys
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !indices_to_remove.contains(i))
                    .map(|(_, x)| *x)
                    .collect::<Vec<_>>();
                if compress_accounts.is_empty() {
                    (None, None)
                } else {
                    (Some(compress_accounts), Some(state_merkle_tree_pubkeys))
                }
            } else {
                (None, None)
            };
        let rpc_result = if (compressed_accounts.is_some()
            && !compressed_accounts.as_ref().unwrap().is_empty())
            || address_merkle_tree_pubkeys.is_some()
        {
            Some(
                self.create_proof_for_compressed_accounts(
                    compressed_accounts,
                    state_merkle_tree_pubkeys,
                    new_addresses,
                    address_merkle_tree_pubkeys,
                    rpc,
                )
                .await,
            )
        } else {
            None
        };
        let address_root_indices = if let Some(rpc_result) = rpc_result.as_ref() {
            rpc_result.address_root_indices.clone()
        } else {
            Vec::new()
        };
        let root_indices = {
            let mut root_indices = if let Some(rpc_result) = rpc_result.as_ref() {
                rpc_result.root_indices.clone()
            } else {
                Vec::new()
            };
            for index in indices_to_remove {
                root_indices.insert(index, None);
            }
            root_indices
        };
        BatchedTreeProofRpcResult {
            proof: rpc_result.map(|x| x.proof),
            root_indices,
            address_root_indices,
        }
    }

    fn add_address_merkle_tree_accounts(
        &mut self,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        _owning_program_id: Option<Pubkey>,
    ) -> AddressMerkleTreeAccounts {
        info!("Adding address merkle tree accounts...");
        let address_merkle_tree_accounts = AddressMerkleTreeAccounts {
            merkle_tree: merkle_tree_keypair.pubkey(),
            queue: queue_keypair.pubkey(),
        };
        self.address_merkle_trees
            .push(Self::add_address_merkle_tree_bundle(
                address_merkle_tree_accounts,
            ));
        info!(
            "Address merkle tree accounts added. Total: {}",
            self.address_merkle_trees.len()
        );
        address_merkle_tree_accounts
    }

    fn get_compressed_accounts_with_merkle_context_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext> {
        self.compressed_accounts
            .iter()
            .filter(|x| x.compressed_account.owner == *owner)
            .cloned()
            .collect()
    }

    fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<TokenDataWithMerkleContext> {
        self.token_compressed_accounts
            .iter()
            .filter(|x| x.token_data.owner == *owner)
            .cloned()
            .collect()
    }

    fn add_state_bundle(&mut self, state_bundle: StateMerkleTreeBundle) {
        self.get_state_merkle_trees_mut().push(state_bundle);
    }

    fn add_event_and_compressed_accounts(
        &mut self,
        slot: u64,
        event: &PublicTransactionEvent,
    ) -> (
        Vec<CompressedAccountWithMerkleContext>,
        Vec<TokenDataWithMerkleContext>,
    ) {
        let mut compressed_accounts = Vec::new();
        let mut token_compressed_accounts = Vec::new();
        let event_inputs_len = event.input_compressed_account_hashes.len();
        let event_outputs_len = event.output_compressed_account_hashes.len();
        for i in 0..std::cmp::max(event_inputs_len, event_outputs_len) {
            self.process_v1_compressed_account(
                slot,
                event,
                i,
                &mut token_compressed_accounts,
                &mut compressed_accounts,
            );
        }

        self.events.push(event.clone());
        (compressed_accounts, token_compressed_accounts)
    }

    fn get_proof_by_index(&mut self, merkle_tree_pubkey: Pubkey, index: u64) -> ProofOfLeaf {
        let bundle = self
            .state_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
            .unwrap();

        while bundle.merkle_tree.leaves().len() <= index as usize {
            bundle.merkle_tree.append(&[0u8; 32]).unwrap();
        }

        let leaf = match bundle.merkle_tree.get_leaf(index as usize) {
            Ok(leaf) => leaf,
            Err(_) => {
                bundle.merkle_tree.append(&[0u8; 32]).unwrap();
                bundle.merkle_tree.get_leaf(index as usize).unwrap()
            }
        };

        let proof = bundle
            .merkle_tree
            .get_proof_of_leaf(index as usize, true)
            .unwrap()
            .to_vec();

        ProofOfLeaf { leaf, proof }
    }

    async fn update_test_indexer_after_append(
        &mut self,
        rpc: &mut R,
        merkle_tree_pubkey: Pubkey,
        output_queue_pubkey: Pubkey,
        num_inserted_zkps: u64,
    ) {
        let state_merkle_tree_bundle = self
            .state_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
            .unwrap();

        let (merkle_tree_next_index, root) = {
            let mut merkle_tree_account =
                rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
            let merkle_tree = BatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                merkle_tree_account.data.as_mut_slice(),
            )
            .unwrap();
            (
                merkle_tree.get_metadata().next_index as usize,
                *merkle_tree.root_history.last().unwrap(),
            )
        };

        let (max_num_zkp_updates, zkp_batch_size) = {
            let mut output_queue_account =
                rpc.get_account(output_queue_pubkey).await.unwrap().unwrap();
            let output_queue = BatchedQueueAccount::output_queue_from_bytes_mut(
                output_queue_account.data.as_mut_slice(),
            )
            .unwrap();

            let output_queue_account = output_queue.get_metadata();
            let max_num_zkp_updates = output_queue_account.batch_metadata.get_num_zkp_batches();
            let zkp_batch_size = output_queue_account.batch_metadata.zkp_batch_size;
            (max_num_zkp_updates, zkp_batch_size)
        };

        let leaves = state_merkle_tree_bundle.output_queue_elements.to_vec();

        let start = (num_inserted_zkps as usize) * zkp_batch_size as usize;
        let end = start + zkp_batch_size as usize;
        let batch_update_leaves = leaves[start..end].to_vec();

        for (i, _) in batch_update_leaves.iter().enumerate() {
            // if leaves[i] == [0u8; 32] {
            let index = merkle_tree_next_index + i - zkp_batch_size as usize;
            // This is dangerous it should call self.get_leaf_by_index() but it
            // can t for mutable borrow
            // TODO: call a get_leaf_by_index equivalent, we could move the method to the reference merkle tree
            let leaf = state_merkle_tree_bundle
                .merkle_tree
                .get_leaf(index)
                .unwrap();
            if leaf == [0u8; 32] {
                state_merkle_tree_bundle
                    .merkle_tree
                    .update(&batch_update_leaves[i], index)
                    .unwrap();
            }
        }
        assert_eq!(
            root,
            state_merkle_tree_bundle.merkle_tree.root(),
            "update indexer after append root invalid"
        );

        let num_inserted_zkps = num_inserted_zkps + 1;
        // check can we get rid of this and use the data from the merkle tree
        if num_inserted_zkps == max_num_zkp_updates {
            for _ in 0..zkp_batch_size * max_num_zkp_updates {
                state_merkle_tree_bundle.output_queue_elements.remove(0);
            }
        }
    }

    async fn update_test_indexer_after_nullification(
        &mut self,
        rpc: &mut R,
        merkle_tree_pubkey: Pubkey,
        batch_index: usize,
    ) {
        let state_merkle_tree_bundle = self
            .state_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
            .unwrap();

        let mut merkle_tree_account = rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
        let merkle_tree = BatchedMerkleTreeAccount::state_tree_from_bytes_mut(
            merkle_tree_account.data.as_mut_slice(),
        )
        .unwrap();

        let batch = &merkle_tree.batches[batch_index];
        if batch.get_state() == BatchState::Inserted || batch.get_state() == BatchState::Full {
            let batch_size = batch.zkp_batch_size;
            let leaf_indices_tx_hashes =
                state_merkle_tree_bundle.input_leaf_indices[..batch_size as usize].to_vec();
            for leaf_info in leaf_indices_tx_hashes.iter() {
                let index = leaf_info.leaf_index as usize;
                let leaf = leaf_info.leaf;
                let index_bytes = index.to_be_bytes();

                let nullifier = Poseidon::hashv(&[&leaf, &index_bytes, &leaf_info.tx_hash]).unwrap();

                state_merkle_tree_bundle.input_leaf_indices.remove(0);
                state_merkle_tree_bundle
                    .merkle_tree
                    .update(&nullifier, index)
                    .unwrap();
            }
        }
    }

    async fn finalize_batched_address_tree_update(
        &mut self,
        rpc: &mut R,
        merkle_tree_pubkey: Pubkey,
    ) {
        let mut account = rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
        let onchain_account =
            BatchedMerkleTreeAccount::address_tree_from_bytes_mut(account.data.as_mut_slice())
                .unwrap();
        let address_tree = self
            .address_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
            .unwrap();
        let address_tree_index = address_tree.merkle_tree.merkle_tree.rightmost_index;
        let onchain_next_index = onchain_account.get_metadata().next_index;
        let diff_onchain_indexer = onchain_next_index - address_tree_index as u64;
        let addresses = address_tree.queue_elements[0..diff_onchain_indexer as usize].to_vec();

        for _ in 0..diff_onchain_indexer {
            address_tree.queue_elements.remove(0);
        }
        for new_element_value in &addresses {
            address_tree
                .merkle_tree
                .append(
                    &BigUint::from_bytes_be(new_element_value),
                    &mut address_tree.indexed_array,
                )
                .unwrap();
        }

        let onchain_root = onchain_account.root_history.last().unwrap();
        let new_root = address_tree.merkle_tree.root();
        assert_eq!(*onchain_root, new_root);
        println!("finalized batched address tree update");
    }
}

impl<R> TestIndexer<R>
where
    R: RpcConnection + MerkleTreeExt,
{
    pub async fn init_from_env(
        payer: &Keypair,
        env: &EnvAccounts,
        prover_config: Option<ProverConfig>,
    ) -> Self {
        Self::new(
            vec![
                StateMerkleTreeAccounts {
                    merkle_tree: env.merkle_tree_pubkey,
                    nullifier_queue: env.nullifier_queue_pubkey,
                    cpi_context: env.cpi_context_account_pubkey,
                },
                StateMerkleTreeAccounts {
                    merkle_tree: env.batched_state_merkle_tree,
                    nullifier_queue: env.batched_output_queue,
                    cpi_context: env.batched_cpi_context,
                },
            ],
            vec![
                AddressMerkleTreeAccounts {
                    merkle_tree: env.address_merkle_tree_pubkey,
                    queue: env.address_merkle_tree_queue_pubkey,
                },
                AddressMerkleTreeAccounts {
                    merkle_tree: env.batch_address_merkle_tree,
                    queue: env.batch_address_merkle_tree,
                },
            ],
            payer.insecure_clone(),
            env.group_pda,
            prover_config,
        )
        .await
    }

    pub async fn new(
        state_merkle_tree_accounts: Vec<StateMerkleTreeAccounts>,
        address_merkle_tree_accounts: Vec<AddressMerkleTreeAccounts>,
        payer: Keypair,
        group_pda: Pubkey,
        prover_config: Option<ProverConfig>,
    ) -> Self {
        if let Some(ref prover_config) = prover_config {
            // TODO: remove restart input and check whether prover is already
            // running with correct config
            spawn_prover(true, prover_config.clone()).await;
        }
        let mut state_merkle_trees = Vec::new();
        for state_merkle_tree_account in state_merkle_tree_accounts.iter() {
            let test_batched_output_queue =
                Keypair::from_bytes(&BATCHED_OUTPUT_QUEUE_TEST_KEYPAIR).unwrap();
            let (version, merkle_tree) = if state_merkle_tree_account.nullifier_queue
                == test_batched_output_queue.pubkey()
            {
                let merkle_tree = Box::new(MerkleTree::<Poseidon>::new(
                    DEFAULT_BATCH_STATE_TREE_HEIGHT as usize,
                    0,
                ));
                (2, merkle_tree)
            } else {
                let merkle_tree = Box::new(MerkleTree::<Poseidon>::new(
                    account_compression::utils::constants::STATE_MERKLE_TREE_HEIGHT as usize,
                    account_compression::utils::constants::STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
                ));
                (1, merkle_tree)
            };

            state_merkle_trees.push(StateMerkleTreeBundle {
                accounts: *state_merkle_tree_account,
                merkle_tree,
                rollover_fee: FeeConfig::default().state_merkle_tree_rollover as i64,
                version,
                output_queue_elements: vec![],
                input_leaf_indices: vec![],
            });
        }

        let mut address_merkle_trees = Vec::new();
        for address_merkle_tree_account in address_merkle_tree_accounts {
            address_merkle_trees.push(Self::add_address_merkle_tree_bundle(
                address_merkle_tree_account,
            ));
        }

        Self {
            state_merkle_trees,
            address_merkle_trees,
            payer,
            compressed_accounts: vec![],
            nullified_compressed_accounts: vec![],
            events: vec![],
            token_compressed_accounts: vec![],
            token_nullified_compressed_accounts: vec![],
            prover_config,
            phantom: Default::default(),
            group_pda,
        }
    }

    pub fn add_address_merkle_tree_bundle(
        address_merkle_tree_accounts: AddressMerkleTreeAccounts,
        // TODO: add config here
    ) -> AddressMerkleTreeBundle {
        let (height, canopy) =
            if address_merkle_tree_accounts.merkle_tree == address_merkle_tree_accounts.queue {
                (40, 0)
            } else {
                (26, STATE_MERKLE_TREE_CANOPY_DEPTH)
            };
        let mut merkle_tree =
            Box::new(IndexedMerkleTree::<Poseidon, usize>::new(height, canopy).unwrap());
        merkle_tree.init().unwrap();
        let mut indexed_array = Box::<IndexedArray<Poseidon, usize>>::default();
        indexed_array.init().unwrap();
        AddressMerkleTreeBundle {
            merkle_tree,
            indexed_array,
            accounts: address_merkle_tree_accounts,
            rollover_fee: FeeConfig::default().address_queue_rollover as i64,
            queue_elements: vec![],
        }
    }

    async fn add_address_merkle_tree_v1(
        &mut self,
        rpc: &mut R,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
    ) -> AddressMerkleTreeAccounts {
        create_address_merkle_tree_and_queue_account_with_assert(
            &self.payer,
            true,
            rpc,
            merkle_tree_keypair,
            queue_keypair,
            owning_program_id,
            None,
            &AddressMerkleTreeConfig::default(),
            &AddressQueueConfig::default(),
            0,
        )
        .await
        .unwrap();
        self.add_address_merkle_tree_accounts(merkle_tree_keypair, queue_keypair, owning_program_id)
    }

    async fn add_address_merkle_tree_v2(
        &mut self,
        rpc: &mut R,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
    ) -> AddressMerkleTreeAccounts {
        info!(
            "Adding address merkle tree accounts v2 {:?}",
            merkle_tree_keypair.pubkey()
        );

        let params = InitAddressTreeAccountsInstructionData::test_default();

        info!(
            "Creating batched address merkle tree {:?}",
            merkle_tree_keypair.pubkey()
        );
        create_batch_address_merkle_tree(rpc, &self.payer, merkle_tree_keypair, params)
            .await
            .unwrap();
        info!(
            "Batched address merkle tree created {:?}",
            merkle_tree_keypair.pubkey()
        );

        self.add_address_merkle_tree_accounts(merkle_tree_keypair, queue_keypair, owning_program_id)
    }

    pub async fn add_address_merkle_tree(
        &mut self,
        rpc: &mut R,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
        version: u64,
    ) -> AddressMerkleTreeAccounts {
        if version == 1 {
            self.add_address_merkle_tree_v1(
                rpc,
                merkle_tree_keypair,
                queue_keypair,
                owning_program_id,
            )
            .await
        } else if version == 2 {
            self.add_address_merkle_tree_v2(
                rpc,
                merkle_tree_keypair,
                queue_keypair,
                owning_program_id,
            )
            .await
        } else {
            panic!(
                "add_address_merkle_tree: Version not supported, {}. Versions: 1, 2",
                version
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn add_state_merkle_tree(
        &mut self,
        rpc: &mut R,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        cpi_context_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
        forester: Option<Pubkey>,
        version: u64,
    ) {
        let (rollover_fee, merkle_tree) = match version {
            1 => {
                create_state_merkle_tree_and_queue_account(
                    &self.payer,
                    true,
                    rpc,
                    merkle_tree_keypair,
                    queue_keypair,
                    Some(cpi_context_keypair),
                    owning_program_id,
                    forester,
                    self.state_merkle_trees.len() as u64,
                    &StateMerkleTreeConfig::default(),
                    &NullifierQueueConfig::default(),
                )
                    .await
                    .unwrap();
                let merkle_tree = Box::new(MerkleTree::<Poseidon>::new(
                    account_compression::utils::constants::STATE_MERKLE_TREE_HEIGHT as usize,
                    account_compression::utils::constants::STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
                ));
                (FeeConfig::default().state_merkle_tree_rollover as i64,merkle_tree)
            }
            2 => {
                let params = InitStateTreeAccountsInstructionData::test_default();

                create_batched_state_merkle_tree(
                    &self.payer,
                    true,
                    rpc,
                    merkle_tree_keypair,
                    queue_keypair,
                    cpi_context_keypair,
                    params,
                ).await.unwrap();
                let merkle_tree = Box::new(MerkleTree::<Poseidon>::new(
                    DEFAULT_BATCH_STATE_TREE_HEIGHT as usize,
                    0
                ));
                (FeeConfig::test_batched().state_merkle_tree_rollover as i64,merkle_tree)
            }
            _ => panic!(
                "add_state_merkle_tree: Version not supported, {}. Versions: 1 concurrent, 2 batched",
                version
            ),
        };
        let state_merkle_tree_account = StateMerkleTreeAccounts {
            merkle_tree: merkle_tree_keypair.pubkey(),
            nullifier_queue: queue_keypair.pubkey(),
            cpi_context: cpi_context_keypair.pubkey(),
        };

        self.state_merkle_trees.push(StateMerkleTreeBundle {
            merkle_tree,
            accounts: state_merkle_tree_account,
            rollover_fee,
            version,
            output_queue_elements: vec![],
            input_leaf_indices: vec![],
        });
    }

    async fn process_inclusion_proofs(
        &self,
        merkle_tree_pubkeys: &[Pubkey],
        accounts: &[[u8; 32]],
        rpc: &mut R,
    ) -> (
        Option<BatchInclusionJsonStruct>,
        Option<BatchInclusionJsonStructLegacy>,
        Vec<u16>,
    ) {
        let mut inclusion_proofs = Vec::new();
        let mut root_indices = Vec::new();
        let mut height = 0;

        // Collect all proofs first before any await points
        let proof_data: Vec<_> = accounts
            .iter()
            .zip(merkle_tree_pubkeys.iter())
            .map(|(account, &pubkey)| {
                let bundle = &self
                    .state_merkle_trees
                    .iter()
                    .find(|x| x.accounts.merkle_tree == pubkey)
                    .unwrap();
                let merkle_tree = &bundle.merkle_tree;
                let leaf_index = merkle_tree.get_leaf_index(account).unwrap();
                let proof = merkle_tree.get_proof_of_leaf(leaf_index, true).unwrap();

                // Convert proof to owned data that implements Send
                let proof: Vec<BigInt> = proof.iter().map(|x| BigInt::from_be_bytes(x)).collect();

                if height == 0 {
                    height = merkle_tree.height;
                } else {
                    assert_eq!(height, merkle_tree.height);
                }

                (
                    bundle.version,
                    pubkey,
                    leaf_index,
                    proof,
                    merkle_tree.root(),
                )
            })
            .collect();

        // Now handle the async operations with the collected data
        for (i, (version, pubkey, leaf_index, proof, merkle_root)) in
            proof_data.into_iter().enumerate()
        {
            inclusion_proofs.push(InclusionMerkleProofInputs {
                root: BigInt::from_be_bytes(merkle_root.as_slice()),
                leaf: BigInt::from_be_bytes(&accounts[i]),
                path_index: BigInt::from_be_bytes(leaf_index.to_be_bytes().as_slice()),
                path_elements: proof,
            });

            let (root_index, root) = if version == 1 {
                let fetched_merkle_tree =
                    get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
                        rpc, pubkey,
                    )
                    .await;
                (
                    fetched_merkle_tree.root_index() as u32,
                    fetched_merkle_tree.root(),
                )
            } else {
                let mut merkle_tree_account = rpc.get_account(pubkey).await.unwrap().unwrap();
                let merkle_tree = BatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                    merkle_tree_account.data.as_mut_slice(),
                )
                .unwrap();
                (
                    merkle_tree.get_root_index(),
                    merkle_tree.get_root().unwrap(),
                )
            };

            assert_eq!(merkle_root, root, "Merkle tree root mismatch");
            root_indices.push(root_index as u16);
        }

        let (batch_inclusion_proof_inputs, legacy) = if height
            == DEFAULT_BATCH_STATE_TREE_HEIGHT as usize
        {
            let inclusion_proof_inputs =
                InclusionProofInputs::new(inclusion_proofs.as_slice()).unwrap();
            (
                Some(BatchInclusionJsonStruct::from_inclusion_proof_inputs(
                    &inclusion_proof_inputs,
                )),
                None,
            )
        } else if height == account_compression::utils::constants::STATE_MERKLE_TREE_HEIGHT as usize
        {
            let inclusion_proof_inputs = InclusionProofInputsLegacy(inclusion_proofs.as_slice());
            (
                None,
                Some(BatchInclusionJsonStructLegacy::from_inclusion_proof_inputs(
                    &inclusion_proof_inputs,
                )),
            )
        } else {
            panic!("Unsupported tree height")
        };

        (batch_inclusion_proof_inputs, legacy, root_indices)
    }

    async fn process_non_inclusion_proofs(
        &self,
        address_merkle_tree_pubkeys: &[Pubkey],
        addresses: &[[u8; 32]],
        rpc: &mut R,
    ) -> (
        Option<BatchNonInclusionJsonStruct>,
        Option<BatchNonInclusionJsonStructLegacy>,
        Vec<u16>,
    ) {
        let mut non_inclusion_proofs = Vec::new();
        let mut address_root_indices = Vec::new();
        let mut tree_heights = Vec::new();
        for tree in self.address_merkle_trees.iter() {
            println!("height {:?}", tree.merkle_tree.merkle_tree.height);
            println!("accounts {:?}", tree.accounts);
        }
        println!("process_non_inclusion_proofs: addresses {:?}", addresses);
        for (i, address) in addresses.iter().enumerate() {
            let address_tree = &self
                .address_merkle_trees
                .iter()
                .find(|x| x.accounts.merkle_tree == address_merkle_tree_pubkeys[i])
                .unwrap();
            tree_heights.push(address_tree.merkle_tree.merkle_tree.height);

            let proof_inputs = get_non_inclusion_proof_inputs(
                address,
                &address_tree.merkle_tree,
                &address_tree.indexed_array,
            );
            non_inclusion_proofs.push(proof_inputs);

            // We don't have address queues in v2 (batch) address Merkle trees
            // hence both accounts in this struct are the same.
            let is_v2 = address_tree.accounts.merkle_tree == address_tree.accounts.queue;
            println!("is v2 {:?}", is_v2);
            println!(
                "address_merkle_tree_pubkeys[i] {:?}",
                address_merkle_tree_pubkeys[i]
            );
            println!("address_tree.accounts {:?}", address_tree.accounts);
            if is_v2 {
                let account = rpc
                    .get_account(address_merkle_tree_pubkeys[i])
                    .await
                    .unwrap();
                if let Some(mut account) = account {
                    let account = BatchedMerkleTreeAccount::address_tree_from_bytes_mut(
                        account.data.as_mut_slice(),
                    )
                    .unwrap();
                    address_root_indices.push(account.get_root_index() as u16);
                } else {
                    panic!(
                        "TestIndexer.process_non_inclusion_proofs(): Address tree account not found."
                    );
                }
            } else {
                let fetched_address_merkle_tree = get_indexed_merkle_tree::<
                    AddressMerkleTreeAccount,
                    R,
                    Poseidon,
                    usize,
                    26,
                    16,
                >(
                    rpc, address_merkle_tree_pubkeys[i]
                )
                .await;
                address_root_indices.push(fetched_address_merkle_tree.root_index() as u16);
            }
        }
        // if tree heights are not the same, panic
        if tree_heights.iter().any(|&x| x != tree_heights[0]) {
            panic!(
                "All address merkle trees must have the same height {:?}",
                tree_heights
            );
        }
        let (batch_non_inclusion_proof_inputs, batch_non_inclusion_proof_inputs_legacy) =
            if tree_heights[0] == 26 {
                let non_inclusion_proof_inputs =
                    NonInclusionProofInputsLegacy::new(non_inclusion_proofs.as_slice());
                (
                    None,
                    Some(
                        BatchNonInclusionJsonStructLegacy::from_non_inclusion_proof_inputs(
                            &non_inclusion_proof_inputs,
                        ),
                    ),
                )
            } else if tree_heights[0] == 40 {
                let non_inclusion_proof_inputs =
                    NonInclusionProofInputs::new(non_inclusion_proofs.as_slice()).unwrap();
                (
                    Some(
                        BatchNonInclusionJsonStruct::from_non_inclusion_proof_inputs(
                            &non_inclusion_proof_inputs,
                        ),
                    ),
                    None,
                )
            } else {
                panic!("Unsupported tree height")
            };
        (
            batch_non_inclusion_proof_inputs,
            batch_non_inclusion_proof_inputs_legacy,
            address_root_indices,
        )
    }

    /// deserializes an event
    /// adds the output_compressed_accounts to the compressed_accounts
    /// removes the input_compressed_accounts from the compressed_accounts
    /// adds the input_compressed_accounts to the nullified_compressed_accounts
    pub fn add_lamport_compressed_accounts(&mut self, slot: u64, event_bytes: Vec<u8>) {
        let event_bytes = event_bytes.clone();
        let event = PublicTransactionEvent::deserialize(&mut event_bytes.as_slice()).unwrap();
        // TODO: map event type
        self.add_event_and_compressed_accounts(slot, &event);
    }

    /// returns the compressed sol balance of the owner pubkey
    pub fn get_compressed_balance(&self, owner: &Pubkey) -> u64 {
        self.compressed_accounts
            .iter()
            .filter(|x| x.compressed_account.owner == *owner)
            .map(|x| x.compressed_account.lamports)
            .sum()
    }

    /// returns the compressed token balance of the owner pubkey for a token by mint
    pub fn get_compressed_token_balance(&self, owner: &Pubkey, mint: &Pubkey) -> u64 {
        self.token_compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.compressed_account.owner == *owner
                    && x.token_data.mint == *mint
            })
            .map(|x| x.token_data.amount)
            .sum()
    }

    fn process_v1_compressed_account(
        &mut self,
        slot: u64,
        event: &PublicTransactionEvent,
        i: usize,
        token_compressed_accounts: &mut Vec<TokenDataWithMerkleContext>,
        compressed_accounts: &mut Vec<CompressedAccountWithMerkleContext>,
    ) {
        let mut input_addresses = vec![];
        if event.input_compressed_account_hashes.len() > i {
            let tx_hash: [u8; 32] = create_tx_hash(
                &event.input_compressed_account_hashes,
                &event.output_compressed_account_hashes,
                slot,
            )
            .unwrap();
            println!("tx_hash {:?}", tx_hash);
            println!("slot {:?}", slot);
            let hash = event.input_compressed_account_hashes[i];
            let index = self.compressed_accounts.iter().position(|x| {
                x.compressed_account
                    .hash::<Poseidon>(
                        &x.merkle_context.merkle_tree_pubkey,
                        &x.merkle_context.leaf_index,
                    )
                    .unwrap()
                    == hash
            });
            let (leaf_index, merkle_tree_pubkey) = if let Some(index) = index {
                self.nullified_compressed_accounts
                    .push(self.compressed_accounts[index].clone());
                let leaf_index = self.compressed_accounts[index].merkle_context.leaf_index;
                let merkle_tree_pubkey = self.compressed_accounts[index]
                    .merkle_context
                    .merkle_tree_pubkey;
                if let Some(address) = self.compressed_accounts[index].compressed_account.address {
                    input_addresses.push(address);
                }
                self.compressed_accounts.remove(index);
                (leaf_index, merkle_tree_pubkey)
            } else {
                let index = self
                    .token_compressed_accounts
                    .iter()
                    .position(|x| {
                        x.compressed_account
                            .compressed_account
                            .hash::<Poseidon>(
                                &x.compressed_account.merkle_context.merkle_tree_pubkey,
                                &x.compressed_account.merkle_context.leaf_index,
                            )
                            .unwrap()
                            == hash
                    })
                    .expect("input compressed account not found");
                self.token_nullified_compressed_accounts
                    .push(self.token_compressed_accounts[index].clone());
                let leaf_index = self.token_compressed_accounts[index]
                    .compressed_account
                    .merkle_context
                    .leaf_index;
                let merkle_tree_pubkey = self.token_compressed_accounts[index]
                    .compressed_account
                    .merkle_context
                    .merkle_tree_pubkey;
                self.token_compressed_accounts.remove(index);
                (leaf_index, merkle_tree_pubkey)
            };
            let bundle = &mut self
                .get_state_merkle_trees_mut()
                .iter_mut()
                .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
                .unwrap();
            // Store leaf indices of input accounts for batched trees
            if bundle.version == 2 {
                let leaf_hash = event.input_compressed_account_hashes[i];
                bundle
                    .input_leaf_indices
                    .push(LeafIndexInfo {
                        leaf_index,
                        leaf: leaf_hash,
                        tx_hash,
                    });
            }
        }
        let mut new_addresses = vec![];
        if event.output_compressed_accounts.len() > i {
            let compressed_account = &event.output_compressed_accounts[i];
            println!("output compressed account {:?}", compressed_account);
            if let Some(address) = compressed_account.compressed_account.address {
                if !input_addresses.iter().any(|x| x == &address) {
                    new_addresses.push(address);
                }
            }

            let merkle_tree = self.state_merkle_trees.iter().find(|x| {
                x.accounts.merkle_tree
                    == event.pubkey_array
                        [event.output_compressed_accounts[i].merkle_tree_index as usize]
            });
            // Check for output queue
            let merkle_tree = if let Some(merkle_tree) = merkle_tree {
                merkle_tree
            } else {
                self.state_merkle_trees
                    .iter()
                    .find(|x| {
                        x.accounts.nullifier_queue
                            == event.pubkey_array
                                [event.output_compressed_accounts[i].merkle_tree_index as usize]
                    })
                    .unwrap()
            };
            println!("found merkle tree {:?}", merkle_tree.accounts.merkle_tree);
            let nullifier_queue_pubkey = merkle_tree.accounts.nullifier_queue;
            let merkle_tree_pubkey = merkle_tree.accounts.merkle_tree;
            // if data is some, try to deserialize token data, if it fails, add to compressed_accounts
            // if data is none add to compressed_accounts
            // new accounts are inserted in front so that the newest accounts are found first
            match compressed_account.compressed_account.data.as_ref() {
                Some(data) => {
                    if compressed_account.compressed_account.owner == light_compressed_token::ID
                        && data.discriminator == light_compressed_token::constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR
                    {
                        if let Ok(token_data) = TokenData::deserialize(&mut data.data.as_slice()) {
                            let token_account = TokenDataWithMerkleContext {
                                token_data,
                                compressed_account: CompressedAccountWithMerkleContext {
                                    compressed_account: compressed_account
                                        .compressed_account
                                        .clone(),
                                    merkle_context: MerkleContext {
                                        leaf_index: event.output_leaf_indices[i],
                                        merkle_tree_pubkey,
                                        nullifier_queue_pubkey,
                                        queue_index: None,
                                    },
                                },
                            };
                            token_compressed_accounts.push(token_account.clone());
                            self.token_compressed_accounts.insert(0, token_account);
                        }
                    } else {
                        let compressed_account = CompressedAccountWithMerkleContext {
                            compressed_account: compressed_account.compressed_account.clone(),
                            merkle_context: MerkleContext {
                                leaf_index: event.output_leaf_indices[i],
                                merkle_tree_pubkey,
                                nullifier_queue_pubkey,
                                queue_index: None,
                            },
                        };
                        compressed_accounts.push(compressed_account.clone());
                        self.compressed_accounts.insert(0, compressed_account);
                    }
                }
                None => {
                    let compressed_account = CompressedAccountWithMerkleContext {
                        compressed_account: compressed_account.compressed_account.clone(),
                        merkle_context: MerkleContext {
                            leaf_index: event.output_leaf_indices[i],
                            merkle_tree_pubkey,
                            nullifier_queue_pubkey,
                            queue_index: None,
                        },
                    };
                    compressed_accounts.push(compressed_account.clone());
                    self.compressed_accounts.insert(0, compressed_account);
                }
            };
            let seq = event
                .sequence_numbers
                .iter()
                .find(|x| x.pubkey == merkle_tree_pubkey);
            let seq = if let Some(seq) = seq {
                seq
            } else {
                event
                    .sequence_numbers
                    .iter()
                    .find(|x| x.pubkey == nullifier_queue_pubkey)
                    .unwrap()
            };
            let is_batched = seq.seq == u64::MAX;

            println!("Output is batched {:?}", is_batched);
            if !is_batched {
                let merkle_tree = &mut self
                    .state_merkle_trees
                    .iter_mut()
                    .find(|x| {
                        x.accounts.merkle_tree
                            == event.pubkey_array
                                [event.output_compressed_accounts[i].merkle_tree_index as usize]
                    })
                    .unwrap();
                merkle_tree
                    .merkle_tree
                    .append(
                        &compressed_account
                            .compressed_account
                            .hash::<Poseidon>(
                                &event.pubkey_array[event.output_compressed_accounts[i]
                                    .merkle_tree_index
                                    as usize],
                                &event.output_leaf_indices[i],
                            )
                            .unwrap(),
                    )
                    .expect("insert failed");
            } else {
                let merkle_tree = &mut self
                    .state_merkle_trees
                    .iter_mut()
                    .find(|x| {
                        x.accounts.nullifier_queue
                            == event.pubkey_array
                                [event.output_compressed_accounts[i].merkle_tree_index as usize]
                    })
                    .unwrap();

                merkle_tree
                    .output_queue_elements
                    .push(event.output_compressed_account_hashes[i]);
            }
        }
        println!("new addresses {:?}", new_addresses);
        println!("event.pubkey_array {:?}", event.pubkey_array);
        println!(
            "address merkle trees {:?}",
            self.address_merkle_trees
                .iter()
                .map(|x| x.accounts.merkle_tree)
                .collect::<Vec<_>>()
        );
        // checks whether there are addresses in outputs which don't exist in inputs.
        // if so check pubkey_array for the first address Merkle tree and append to the bundles queue elements.
        // Note:
        // - creating addresses in multiple address Merkle trees in one tx is not supported
        // TODO: reimplement this is not a good solution
        // - take addresses and address Merkle tree pubkeys from cpi to account compression program
        if !new_addresses.is_empty() {
            for pubkey in event.pubkey_array.iter() {
                if let Some((_, address_merkle_tree)) = self
                    .address_merkle_trees
                    .iter_mut()
                    .enumerate()
                    .find(|(_, x)| x.accounts.merkle_tree == *pubkey)
                {
                    address_merkle_tree
                        .queue_elements
                        .append(&mut new_addresses);
                }
            }
        }
    }

    async fn _get_multiple_new_address_proofs<const NET_HEIGHT: usize>(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
        full: bool,
    ) -> Result<Vec<NewAddressProofWithContext<NET_HEIGHT>>, IndexerError> {
        let mut proofs: Vec<NewAddressProofWithContext<NET_HEIGHT>> = Vec::new();

        for address in addresses.iter() {
            info!("Getting new address proof for {:?}", address);
            let pubkey = Pubkey::from(merkle_tree_pubkey);
            let address_tree_bundle = self
                .address_merkle_trees
                .iter()
                .find(|x| x.accounts.merkle_tree == pubkey)
                .unwrap();

            let address_biguint = BigUint::from_bytes_be(address.as_slice());
            let (old_low_address, _old_low_address_next_value) = address_tree_bundle
                .indexed_array
                .find_low_element_for_nonexistent(&address_biguint)
                .unwrap();
            let address_bundle = address_tree_bundle
                .indexed_array
                .new_element_with_low_element_index(old_low_address.index, &address_biguint)
                .unwrap();

            let (old_low_address, old_low_address_next_value) = address_tree_bundle
                .indexed_array
                .find_low_element_for_nonexistent(&address_biguint)
                .unwrap();

            // Get the Merkle proof for updating low element.
            let low_address_proof = address_tree_bundle
                .merkle_tree
                .get_proof_of_leaf(old_low_address.index, full)
                .unwrap();

            let low_address_index: u64 = old_low_address.index as u64;
            let low_address_value: [u8; 32] =
                bigint_to_be_bytes_array(&old_low_address.value).unwrap();
            let low_address_next_index: u64 = old_low_address.next_index as u64;
            let low_address_next_value: [u8; 32] =
                bigint_to_be_bytes_array(&old_low_address_next_value).unwrap();
            let low_address_proof: [[u8; 32]; NET_HEIGHT] = low_address_proof.to_array().unwrap();
            let proof = NewAddressProofWithContext::<NET_HEIGHT> {
                merkle_tree: merkle_tree_pubkey,
                low_address_index,
                low_address_value,
                low_address_next_index,
                low_address_next_value,
                low_address_proof,
                root: address_tree_bundle.merkle_tree.root(),
                root_seq: address_tree_bundle.merkle_tree.merkle_tree.sequence_number as u64,
                new_low_element: Some(address_bundle.new_low_element),
                new_element: Some(address_bundle.new_element),
                new_element_next_value: Some(address_bundle.new_element_next_value),
            };
            proofs.push(proof);
        }
        Ok(proofs)
    }
}
