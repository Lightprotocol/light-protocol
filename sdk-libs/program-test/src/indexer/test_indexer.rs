use std::{cmp::min, marker::PhantomData, time::Duration};

use account_compression::{
    AddressMerkleTreeAccount, AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig,
    StateMerkleTreeAccount, StateMerkleTreeConfig,
};
use async_trait::async_trait;
use borsh::BorshDeserialize;
use forester_utils::account_zero_copy::{get_concurrent_merkle_tree, get_indexed_merkle_tree};
use light_batched_merkle_tree::{
    constants::{DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, DEFAULT_BATCH_STATE_TREE_HEIGHT},
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount,
    queue::BatchedQueueAccount,
};
use light_client::{
    indexer::{
        Address, AddressMerkleTreeAccounts, AddressMerkleTreeBundle, AddressQueueIndex,
        AddressWithTree, BatchAddressUpdateIndexerResponse, Hash, Indexer, IndexerError,
        IntoPhotonAccount, LeafIndexInfo, MerkleProof, MerkleProofWithContext,
        NewAddressProofWithContext, StateMerkleTreeAccounts, StateMerkleTreeBundle,
    },
    rpc::{
        merkle_tree::MerkleTreeExt,
        types::{BatchedTreeProofRpcResult, ProofRpcResult},
        RpcConnection,
    },
    transaction_params::FeeConfig,
};
use light_compressed_account::{
    compressed_account::{CompressedAccountWithMerkleContext, MerkleContext},
    hash_chain::create_hash_chain_from_slice,
    indexer_event::event::PublicTransactionEvent,
    instruction_data::compressed_proof::CompressedProof,
    tx_hash::create_tx_hash,
    TreeType,
};
use light_hasher::{bigint::bigint_to_be_bytes_array, Hasher, Poseidon};
use light_merkle_tree_metadata::QueueType;
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
    non_inclusion::merkle_non_inclusion_proof_inputs::NonInclusionProofInputs,
    non_inclusion_legacy::merkle_non_inclusion_proof_inputs::NonInclusionProofInputs as NonInclusionProofInputsLegacy,
};
use light_sdk::token::{TokenData, TokenDataWithMerkleContext};
use log::{info, warn};
use num_bigint::{BigInt, BigUint};
use num_traits::FromBytes;
use photon_api::models::{Account, CompressedProofWithContextV2, TokenBalance};
use reqwest::Client;
use solana_sdk::{
    bs58,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

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
        &mut self,
        merkle_tree_pubkey: [u8; 32],
        queue_type: QueueType,
        num_elements: u16,
        _start_offset: Option<u64>,
    ) -> Result<Vec<MerkleProofWithContext>, IndexerError> {
        let pubkey = Pubkey::new_from_array(merkle_tree_pubkey);
        let address_tree_bundle = self
            .address_merkle_trees
            .iter()
            .find(|x| x.accounts.merkle_tree == pubkey);
        if let Some(address_tree_bundle) = address_tree_bundle {
            let end_offset = min(
                num_elements as usize,
                address_tree_bundle.queue_elements.len(),
            );
            let queue_elements = address_tree_bundle.queue_elements[0..end_offset].to_vec();

            let merkle_proofs_with_context = queue_elements
                .iter()
                .map(|element| MerkleProofWithContext {
                    proof: Vec::new(),
                    leaf: [0u8; 32],
                    leaf_index: 0,
                    merkle_tree: address_tree_bundle.accounts.merkle_tree.to_bytes(),
                    root: address_tree_bundle.root(),
                    tx_hash: None,
                    root_seq: 0,
                    account_hash: *element,
                })
                .collect();
            return Ok(merkle_proofs_with_context);
        }

        let state_tree_bundle = self
            .state_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == pubkey);
        if queue_type == QueueType::InputStateV2 {
            if let Some(state_tree_bundle) = state_tree_bundle {
                let end_offset = min(
                    num_elements as usize,
                    state_tree_bundle.input_leaf_indices.len(),
                );
                let queue_elements = state_tree_bundle.input_leaf_indices[0..end_offset].to_vec();
                let merkle_proofs = queue_elements
                    .iter()
                    .map(|leaf_info| {
                        match state_tree_bundle
                            .merkle_tree
                            .get_proof_of_leaf(leaf_info.leaf_index as usize, true)
                        {
                            Ok(proof) => proof.to_vec(),
                            Err(_) => {
                                let mut next_index =
                                    state_tree_bundle.merkle_tree.get_next_index() as u64;
                                while next_index < leaf_info.leaf_index as u64 {
                                    state_tree_bundle.merkle_tree.append(&[0u8; 32]).unwrap();
                                    next_index =
                                        state_tree_bundle.merkle_tree.get_next_index() as u64;
                                }
                                state_tree_bundle
                                    .merkle_tree
                                    .get_proof_of_leaf(leaf_info.leaf_index as usize, true)
                                    .unwrap()
                                    .to_vec();
                                Vec::new()
                            }
                        }
                    })
                    .collect::<Vec<_>>();
                let leaves = queue_elements
                    .iter()
                    .map(|leaf_info| {
                        state_tree_bundle
                            .merkle_tree
                            .get_leaf(leaf_info.leaf_index as usize)
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>();
                let merkle_proofs_with_context = merkle_proofs
                    .iter()
                    .zip(queue_elements.iter())
                    .zip(leaves.iter())
                    .map(|((proof, element), leaf)| MerkleProofWithContext {
                        proof: proof.clone(),
                        leaf: *leaf,
                        leaf_index: element.leaf_index as u64,
                        merkle_tree: state_tree_bundle.accounts.merkle_tree.to_bytes(),
                        root: state_tree_bundle.merkle_tree.root(),
                        tx_hash: Some(element.tx_hash),
                        root_seq: 0,
                        account_hash: element.leaf,
                    })
                    .collect();

                return Ok(merkle_proofs_with_context);
            }
        }

        if queue_type == QueueType::OutputStateV2 {
            if let Some(state_tree_bundle) = state_tree_bundle {
                let end_offset = min(
                    num_elements as usize,
                    state_tree_bundle.output_queue_elements.len(),
                );
                let queue_elements =
                    state_tree_bundle.output_queue_elements[0..end_offset].to_vec();
                let indices = queue_elements
                    .iter()
                    .map(|(_, index)| index)
                    .collect::<Vec<_>>();
                let merkle_proofs = indices
                    .iter()
                    .map(|index| {
                        match state_tree_bundle
                            .merkle_tree
                            .get_proof_of_leaf(**index as usize, true)
                        {
                            Ok(proof) => proof.to_vec(),
                            Err(_) => {
                                let mut next_index =
                                    state_tree_bundle.merkle_tree.get_next_index() as u64;
                                while next_index < **index {
                                    state_tree_bundle.merkle_tree.append(&[0u8; 32]).unwrap();
                                    next_index =
                                        state_tree_bundle.merkle_tree.get_next_index() as u64;
                                }
                                state_tree_bundle
                                    .merkle_tree
                                    .get_proof_of_leaf(**index as usize, true)
                                    .unwrap()
                                    .to_vec();
                                Vec::new()
                            }
                        }
                    })
                    .collect::<Vec<_>>();
                let leaves = indices
                    .iter()
                    .map(|index| {
                        state_tree_bundle
                            .merkle_tree
                            .get_leaf(**index as usize)
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>();
                let merkle_proofs_with_context = merkle_proofs
                    .iter()
                    .zip(queue_elements.iter())
                    .zip(leaves.iter())
                    .map(|((proof, (element, index)), leaf)| MerkleProofWithContext {
                        proof: proof.clone(),
                        leaf: *leaf,
                        leaf_index: *index,
                        merkle_tree: state_tree_bundle.accounts.merkle_tree.to_bytes(),
                        root: state_tree_bundle.merkle_tree.root(),
                        tx_hash: None,
                        root_seq: 0,
                        account_hash: *element,
                    })
                    .collect();
                return Ok(merkle_proofs_with_context);
            }
        }

        Err(IndexerError::InvalidParameters(
            "Merkle tree not found".to_string(),
        ))
    }

    async fn get_subtrees(
        &self,
        merkle_tree_pubkey: [u8; 32],
    ) -> Result<Vec<[u8; 32]>, IndexerError> {
        let merkle_tree_pubkey = Pubkey::new_from_array(merkle_tree_pubkey);
        let address_tree_bundle = self
            .address_merkle_trees
            .iter()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey);
        if let Some(address_tree_bundle) = address_tree_bundle {
            Ok(address_tree_bundle.get_subtrees())
        } else {
            let state_tree_bundle = self
                .state_merkle_trees
                .iter()
                .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey);
            if let Some(state_tree_bundle) = state_tree_bundle {
                Ok(state_tree_bundle.merkle_tree.get_subtrees())
            } else {
                Err(IndexerError::InvalidParameters(
                    "Merkle tree not found".to_string(),
                ))
            }
        }
    }

    async fn create_proof_for_compressed_accounts(
        &mut self,
        compressed_accounts: Option<Vec<[u8; 32]>>,
        state_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        new_addresses: Option<&[[u8; 32]]>,
        address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        rpc: &mut R,
    ) -> Result<ProofRpcResult, IndexerError> {
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
                        .await?;
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
                        .await?;
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

        let mut retries = 3;
        while retries > 0 {
            println!("PROVE_PATH {:?}", PROVE_PATH);
            println!("SERVER_ADDRESS {:?}", SERVER_ADDRESS);
            let response_result = client
                .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(json_payload.clone())
                .send()
                .await;
            if let Ok(response_result) = response_result {
                retries = 0;
                if response_result.status().is_success() {
                    let body = response_result.text().await.unwrap();
                    let proof_json = deserialize_gnark_proof_json(&body).unwrap();
                    let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
                    let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
                    let root_indices = root_indices.iter().map(|x| Some(*x)).collect();
                    return Ok(ProofRpcResult {
                        root_indices,
                        address_root_indices: address_root_indices.clone(),
                        proof: CompressedProof {
                            a: proof_a,
                            b: proof_b,
                            c: proof_c,
                        },
                    });
                }
            } else {
                warn!("Error: {:#?}", response_result);
                tokio::time::sleep(Duration::from_secs(5)).await;
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
                        root: *tree.merkle_tree.roots.last().unwrap(),
                    });
                }
            })
        });
        Ok(proofs)
    }

    async fn get_compressed_accounts_by_owner_v2(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<CompressedAccountWithMerkleContext>, IndexerError> {
        Ok(self.get_compressed_accounts_with_merkle_context_by_owner(owner))
    }

    async fn get_compressed_token_accounts_by_owner_v2(
        &self,
        _owner: &Pubkey,
        _mint: Option<Pubkey>,
    ) -> Result<Vec<TokenDataWithMerkleContext>, IndexerError> {
        todo!()
    }

    async fn get_compressed_account(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<Account, IndexerError> {
        let account = match (address, hash) {
            (Some(address), _) => self.compressed_accounts.iter().find(|acc| {
                acc.compressed_account
                    .address
                    .map_or(false, |acc_addr| acc_addr == address)
            }),
            (_, Some(hash)) => self
                .compressed_accounts
                .iter()
                .find(|acc| acc.hash().map_or(false, |acc_hash| acc_hash == hash)),
            (None, None) => {
                return Err(IndexerError::InvalidParameters(
                    "Either address or hash must be provided".to_string(),
                ))
            }
        };

        account
            .map(|acc| acc.clone().into_photon_account())
            .ok_or(IndexerError::AccountNotFound)
    }

    async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<Vec<TokenDataWithMerkleContext>, IndexerError> {
        let accounts = self
            .token_compressed_accounts
            .iter()
            .filter(|acc| {
                acc.token_data.owner == *owner && mint.map_or(true, |m| acc.token_data.mint == m)
            })
            .cloned()
            .collect();

        Ok(accounts)
    }

    async fn get_compressed_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<u64, IndexerError> {
        let account = self.get_compressed_account(address, hash).await?;
        Ok(account.lamports)
    }

    async fn get_compressed_token_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<u64, IndexerError> {
        let account = match (address, hash) {
            (Some(address), _) => self.token_compressed_accounts.iter().find(|acc| {
                acc.compressed_account
                    .compressed_account
                    .address
                    .map_or(false, |acc_addr| acc_addr == address)
            }),
            (_, Some(hash)) => self.token_compressed_accounts.iter().find(|acc| {
                acc.compressed_account
                    .hash()
                    .map_or(false, |acc_hash| acc_hash == hash)
            }),
            (None, None) => {
                return Err(IndexerError::InvalidParameters(
                    "Either address or hash must be provided".to_string(),
                ))
            }
        };

        account
            .map(|acc| acc.token_data.amount)
            .ok_or(IndexerError::AccountNotFound)
    }

    async fn get_multiple_compressed_accounts(
        &self,
        addresses: Option<Vec<Address>>,
        hashes: Option<Vec<Hash>>,
    ) -> Result<Vec<Account>, IndexerError> {
        match (addresses, hashes) {
            (Some(addresses), _) => {
                let accounts = self
                    .compressed_accounts
                    .iter()
                    .filter(|acc| {
                        acc.compressed_account
                            .address
                            .map_or(false, |addr| addresses.contains(&addr))
                    })
                    .map(|acc| acc.clone().into_photon_account())
                    .collect();
                Ok(accounts)
            }
            (_, Some(hashes)) => {
                let accounts = self
                    .compressed_accounts
                    .iter()
                    .filter(|acc| acc.hash().map_or(false, |hash| hashes.contains(&hash)))
                    .map(|acc| acc.clone().into_photon_account())
                    .collect();
                Ok(accounts)
            }
            (None, None) => Err(IndexerError::InvalidParameters(
                "Either addresses or hashes must be provided".to_string(),
            )),
        }
    }

    async fn get_compressed_token_balances_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<photon_api::models::token_balance_list::TokenBalanceList, IndexerError> {
        let balances: Vec<TokenBalance> = self
            .token_compressed_accounts
            .iter()
            .filter(|acc| {
                acc.token_data.owner == *owner && mint.map_or(true, |m| acc.token_data.mint == m)
            })
            .map(|acc| TokenBalance {
                balance: acc.token_data.amount,
                mint: acc.token_data.mint.to_string(),
            })
            .collect();

        Ok(photon_api::models::token_balance_list::TokenBalanceList {
            cursor: None,
            token_balances: balances,
        })
    }

    async fn get_compression_signatures_for_account(
        &self,
        _hash: Hash,
    ) -> Result<Vec<String>, IndexerError> {
        todo!()
    }

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<16>>, IndexerError> {
        self._get_multiple_new_address_proofs(merkle_tree_pubkey, addresses, false)
            .await
    }

    async fn get_multiple_new_address_proofs_h40(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<40>>, IndexerError> {
        self._get_multiple_new_address_proofs(merkle_tree_pubkey, addresses, true)
            .await
    }

    async fn get_validity_proof(
        &self,
        _hashes: Vec<Hash>,
        _new_addresses_with_trees: Vec<AddressWithTree>,
    ) -> Result<
        photon_api::models::compressed_proof_with_context::CompressedProofWithContext,
        IndexerError,
    > {
        todo!()
    }

    async fn get_validity_proof_v2(
        &self,
        _hashes: Vec<Hash>,
        _new_addresses_with_trees: Vec<AddressWithTree>,
    ) -> Result<CompressedProofWithContextV2, IndexerError> {
        todo!()
    }

    async fn get_indexer_slot(&self, rpc: &mut R) -> Result<u64, IndexerError> {
        rpc.get_slot()
            .await
            .map_err(|e| IndexerError::RpcError(e.to_string()))
    }

    fn get_address_merkle_trees(&self) -> &Vec<AddressMerkleTreeBundle> {
        &self.address_merkle_trees
    }

    async fn get_address_queue_with_proofs(
        &mut self,
        merkle_tree_pubkey: &Pubkey,
        zkp_batch_size: u16,
    ) -> Result<BatchAddressUpdateIndexerResponse, IndexerError> {
        let batch_start_index = self
            .get_address_merkle_trees()
            .iter()
            .find(|x| x.accounts.merkle_tree == *merkle_tree_pubkey)
            .unwrap()
            .get_v2_indexed_merkle_tree()
            .ok_or(IndexerError::Unknown(
                "Failed to get v2 indexed merkle tree".into(),
            ))?
            .merkle_tree
            .rightmost_index;

        let address_proofs = self
            .get_queue_elements(
                merkle_tree_pubkey.to_bytes(),
                QueueType::AddressV2,
                zkp_batch_size,
                None,
            )
            .await
            .map_err(|_| IndexerError::Unknown("Failed to get queue elements".into()))?;

        let addresses: Vec<AddressQueueIndex> = address_proofs
            .iter()
            .enumerate()
            .map(|(i, proof)| AddressQueueIndex {
                address: proof.account_hash,
                queue_index: proof.root_seq + i as u64,
            })
            .collect();
        let non_inclusion_proofs = self
            .get_multiple_new_address_proofs_h40(
                merkle_tree_pubkey.to_bytes(),
                address_proofs.iter().map(|x| x.account_hash).collect(),
            )
            .await
            .map_err(|_| {
                IndexerError::Unknown("Failed to get get_multiple_new_address_proofs_full".into())
            })?;

        let subtrees = self
            .get_subtrees(merkle_tree_pubkey.to_bytes())
            .await
            .map_err(|_| IndexerError::Unknown("Failed to get subtrees".into()))?;

        Ok(BatchAddressUpdateIndexerResponse {
            batch_start_index: batch_start_index as u64,
            addresses,
            non_inclusion_proofs,
            subtrees,
        })
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
        let pos = self
            .address_merkle_trees
            .iter()
            .position(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
            .unwrap();
        let new_low_element = context.new_low_element.clone().unwrap();
        let new_element = context.new_element.clone().unwrap();
        let new_element_next_value = context.new_element_next_value.clone().unwrap();
        // It can only be v1 address tree because proof with context has len 16.
        self.address_merkle_trees[pos]
            .get_v1_indexed_merkle_tree_mut()
            .expect("Failed to get v1 indexed merkle tree.")
            .update(&new_low_element, &new_element, &new_element_next_value)
            .unwrap();
        self.address_merkle_trees[pos]
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

    #[cfg(feature = "devenv")]
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
                        let leaf_index = accounts.merkle_tree.get_leaf_index(compressed_account);
                        if leaf_index.is_none() {
                            let output_queue_pubkey = accounts.accounts.nullifier_queue;
                            let mut queue =
                                forester_utils::account_zero_copy::AccountZeroCopy::<
                                    light_batched_merkle_tree::queue::BatchedQueueMetadata,
                                >::new(rpc, output_queue_pubkey)
                                .await;
                            let queue_zero_copy = BatchedQueueAccount::output_from_bytes(
                                queue.account.data.as_mut_slice(),
                            )
                            .unwrap();
                            for value_array in queue_zero_copy.value_vecs.iter() {
                                let index =
                                    value_array.iter().position(|x| *x == *compressed_account);
                                if index.is_some() {
                                    indices_to_remove.push(i);
                                }
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
            rpc_result.as_ref().unwrap().address_root_indices.clone()
        } else {
            Vec::new()
        };
        let root_indices = {
            let mut root_indices = if let Some(rpc_result) = rpc_result.as_ref() {
                rpc_result.as_ref().unwrap().root_indices.clone()
            } else {
                Vec::new()
            };
            for index in indices_to_remove {
                root_indices.insert(index, None);
            }
            root_indices
        };
        BatchedTreeProofRpcResult {
            proof: rpc_result.map(|x| x.unwrap().proof),
            root_indices,
            address_root_indices,
        }
    }

    #[cfg(not(feature = "devenv"))]
    async fn create_proof_for_compressed_accounts2(
        &mut self,
        _compressed_accounts: Option<Vec<[u8; 32]>>,
        _state_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        _new_addresses: Option<&[[u8; 32]]>,
        _address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        _rpc: &mut R,
    ) -> BatchedTreeProofRpcResult {
        unimplemented!("create_proof_for_compressed_accounts2 is only implemented for feature devenv in light-protocol monorepo.")
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
            .push(Self::add_address_merkle_tree_bundle(address_merkle_tree_accounts).unwrap());
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

    #[cfg(feature = "devenv")]
    fn get_proof_by_index(&mut self, merkle_tree_pubkey: Pubkey, index: u64) -> MerkleProof {
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

        MerkleProof {
            hash: bs58::encode(leaf).into_string(),
            leaf_index: index,
            merkle_tree: merkle_tree_pubkey.to_string(),
            proof,
            root_seq: bundle.merkle_tree.sequence_number as u64,
            root: bundle.merkle_tree.root(),
        }
    }

    #[cfg(not(feature = "devenv"))]
    fn get_proof_by_index(&mut self, _merkle_tree_pubkey: Pubkey, _index: u64) -> MerkleProof {
        unimplemented!("get_proof_by_index is unimplemented.")
    }

    async fn update_test_indexer_after_append(
        &mut self,
        rpc: &mut R,
        merkle_tree_pubkey: Pubkey,
        output_queue_pubkey: Pubkey,
    ) {
        let state_merkle_tree_bundle = self
            .state_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
            .unwrap();

        let (merkle_tree_next_index, root) = {
            let mut merkle_tree_account =
                rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
            let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
                merkle_tree_account.data.as_mut_slice(),
                &merkle_tree_pubkey.into(),
            )
            .unwrap();
            (
                merkle_tree.next_index as usize,
                *merkle_tree.root_history.last().unwrap(),
            )
        };

        let zkp_batch_size = {
            let mut output_queue_account =
                rpc.get_account(output_queue_pubkey).await.unwrap().unwrap();
            let output_queue =
                BatchedQueueAccount::output_from_bytes(output_queue_account.data.as_mut_slice())
                    .unwrap();

            output_queue.batch_metadata.zkp_batch_size
        };

        let leaves = state_merkle_tree_bundle.output_queue_elements.to_vec();
        let batch_update_leaves = leaves[0..zkp_batch_size as usize].to_vec();

        for (i, (new_leaf, _)) in batch_update_leaves.iter().enumerate() {
            let index = merkle_tree_next_index + i - zkp_batch_size as usize;
            // This is dangerous it should call self.get_leaf_by_index() but it
            // can t for mutable borrow
            // TODO: call a get_leaf_by_index equivalent, we could move the method to the reference merkle tree
            let leaf = state_merkle_tree_bundle
                .merkle_tree
                .get_leaf(index)
                .unwrap_or_default();
            if leaf == [0u8; 32] {
                let result = state_merkle_tree_bundle.merkle_tree.update(new_leaf, index);
                if result.is_err() && state_merkle_tree_bundle.merkle_tree.rightmost_index == index
                {
                    state_merkle_tree_bundle
                        .merkle_tree
                        .append(new_leaf)
                        .unwrap();
                } else {
                    result.unwrap();
                }
            }
        }
        assert_eq!(
            root,
            state_merkle_tree_bundle.merkle_tree.root(),
            "update indexer after append root invalid"
        );

        for _ in 0..zkp_batch_size {
            state_merkle_tree_bundle.output_queue_elements.remove(0);
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
        let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &merkle_tree_pubkey.into(),
        )
        .unwrap();

        let batch = &merkle_tree.queue_batches.batches[batch_index];
        let batch_size = batch.zkp_batch_size;
        let leaf_indices_tx_hashes =
            state_merkle_tree_bundle.input_leaf_indices[..batch_size as usize].to_vec();
        for leaf_info in leaf_indices_tx_hashes.iter() {
            let index = leaf_info.leaf_index as usize;
            let leaf = leaf_info.leaf;
            let index_bytes = index.to_be_bytes();

            let nullifier = Poseidon::hashv(&[&leaf, &index_bytes, &leaf_info.tx_hash]).unwrap();

            state_merkle_tree_bundle.input_leaf_indices.remove(0);
            let result = state_merkle_tree_bundle
                .merkle_tree
                .update(&nullifier, index);
            if result.is_err() {
                let num_missing_leaves =
                    (index + 1) - state_merkle_tree_bundle.merkle_tree.rightmost_index;
                state_merkle_tree_bundle
                    .merkle_tree
                    .append_batch(&vec![&[0u8; 32]; num_missing_leaves])
                    .unwrap();
                state_merkle_tree_bundle
                    .merkle_tree
                    .update(&nullifier, index)
                    .unwrap();
            }
        }
    }

    #[cfg(feature = "devenv")]
    async fn finalize_batched_address_tree_update(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        account_data: &mut [u8],
    ) {
        let onchain_account =
            BatchedMerkleTreeAccount::address_from_bytes(account_data, &merkle_tree_pubkey.into())
                .unwrap();
        let address_tree = self
            .address_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
            .unwrap();
        let address_tree_index = address_tree.right_most_index();
        let onchain_next_index = onchain_account.next_index;
        let diff_onchain_indexer = onchain_next_index - address_tree_index as u64;
        let addresses = address_tree.queue_elements[0..diff_onchain_indexer as usize].to_vec();
        for _ in 0..diff_onchain_indexer {
            address_tree.queue_elements.remove(0);
        }
        for new_element_value in &addresses {
            address_tree
                .append(&BigUint::from_bytes_be(new_element_value))
                .unwrap();
        }

        let onchain_root = onchain_account.root_history.last().unwrap();
        let new_root = address_tree.root();
        assert_eq!(*onchain_root, new_root);
    }

    #[cfg(not(feature = "devenv"))]
    async fn finalize_batched_address_tree_update(
        &mut self,
        _merkle_tree_pubkey: Pubkey,
        _account_data: &mut [u8],
    ) {
        unimplemented!(
            "finalize_batched_address_tree_update is only implemented with feature devnenv."
        )
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
        #[allow(unused_mut)]
        let mut state_merkle_tree_accounts = vec![StateMerkleTreeAccounts {
            merkle_tree: env.merkle_tree_pubkey,
            nullifier_queue: env.nullifier_queue_pubkey,
            cpi_context: env.cpi_context_account_pubkey,
        }];
        #[cfg(feature = "devenv")]
        state_merkle_tree_accounts.push(StateMerkleTreeAccounts {
            merkle_tree: env.batched_state_merkle_tree,
            nullifier_queue: env.batched_output_queue,
            cpi_context: env.batched_cpi_context,
        });
        #[allow(unused_mut)]
        let mut address_merkle_tree_accounts = vec![AddressMerkleTreeAccounts {
            merkle_tree: env.address_merkle_tree_pubkey,
            queue: env.address_merkle_tree_queue_pubkey,
        }];
        #[cfg(feature = "devenv")]
        address_merkle_tree_accounts.push(AddressMerkleTreeAccounts {
            merkle_tree: env.batch_address_merkle_tree,
            queue: env.batch_address_merkle_tree,
        });
        Self::new(
            state_merkle_tree_accounts,
            address_merkle_tree_accounts,
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
            address_merkle_trees
                .push(Self::add_address_merkle_tree_bundle(address_merkle_tree_account).unwrap());
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
    ) -> Result<AddressMerkleTreeBundle, IndexerError> {
        if address_merkle_tree_accounts.merkle_tree == address_merkle_tree_accounts.queue {
            AddressMerkleTreeBundle::new_v2(address_merkle_tree_accounts)
        } else {
            AddressMerkleTreeBundle::new_v1(address_merkle_tree_accounts)
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
                let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
                    merkle_tree_account.data.as_mut_slice(),
                    &pubkey.into(),
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
    ) -> Result<
        (
            Option<BatchNonInclusionJsonStruct>,
            Option<BatchNonInclusionJsonStructLegacy>,
            Vec<u16>,
        ),
        IndexerError,
    > {
        let mut non_inclusion_proofs = Vec::new();
        let mut address_root_indices = Vec::new();
        let mut tree_heights = Vec::new();
        for (i, address) in addresses.iter().enumerate() {
            let address_tree = &self
                .address_merkle_trees
                .iter()
                .find(|x| x.accounts.merkle_tree == address_merkle_tree_pubkeys[i])
                .unwrap();
            tree_heights.push(address_tree.height());

            let proof_inputs = address_tree.get_non_inclusion_proof_inputs(address)?;
            non_inclusion_proofs.push(proof_inputs);

            // We don't have address queues in v2 (batch) address Merkle trees
            // hence both accounts in this struct are the same.
            let is_v2 = address_tree.accounts.merkle_tree == address_tree.accounts.queue;
            if is_v2 {
                let account = rpc
                    .get_account(address_merkle_tree_pubkeys[i])
                    .await
                    .unwrap();
                if let Some(mut account) = account {
                    let account = BatchedMerkleTreeAccount::address_from_bytes(
                        account.data.as_mut_slice(),
                        &address_merkle_tree_pubkeys[i].into(),
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
        Ok((
            batch_non_inclusion_proof_inputs,
            batch_non_inclusion_proof_inputs_legacy,
            address_root_indices,
        ))
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
            let hash = event.input_compressed_account_hashes[i];
            let index = self
                .compressed_accounts
                .iter()
                .position(|x| x.hash().unwrap() == hash);
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
                    .position(|x| x.compressed_account.hash().unwrap() == hash)
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
                bundle.input_leaf_indices.push(LeafIndexInfo {
                    leaf_index,
                    leaf: leaf_hash,
                    tx_hash,
                });
            }
        }
        let mut new_addresses = vec![];
        if event.output_compressed_accounts.len() > i {
            let compressed_account = &event.output_compressed_accounts[i];
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
                                        queue_pubkey: nullifier_queue_pubkey,
                                        prove_by_index: false,
                                        tree_type: if merkle_tree.version == 2 {
                                            TreeType::StateV2
                                        } else {TreeType::StateV1}
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
                                queue_pubkey: nullifier_queue_pubkey,
                                prove_by_index: false,
                                tree_type: if merkle_tree.version == 2 {
                                    TreeType::StateV2
                                } else {TreeType::StateV1}
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
                            queue_pubkey: nullifier_queue_pubkey,
                            prove_by_index: false,
                            tree_type: if merkle_tree.version == 2 {
                                TreeType::StateV2
                            } else {
                                TreeType::StateV1
                            },
                        },
                    };
                    compressed_accounts.push(compressed_account.clone());
                    self.compressed_accounts.insert(0, compressed_account);
                }
            };
            let merkle_tree = &mut self.state_merkle_trees.iter_mut().find(|x| {
                x.accounts.merkle_tree
                    == event.pubkey_array
                        [event.output_compressed_accounts[i].merkle_tree_index as usize]
            });
            if merkle_tree.is_some() {
                let merkle_tree = merkle_tree.as_mut().unwrap();
                let leaf_hash = compressed_account
                    .compressed_account
                    .hash(
                        &event.pubkey_array
                            [event.output_compressed_accounts[i].merkle_tree_index as usize],
                        &event.output_leaf_indices[i],
                        false,
                    )
                    .unwrap();
                merkle_tree
                    .merkle_tree
                    .append(&leaf_hash)
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

                merkle_tree.output_queue_elements.push((
                    event.output_compressed_account_hashes[i],
                    event.output_leaf_indices[i].into(),
                ));
            }
        }
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
            let (old_low_address, _old_low_address_next_value) =
                address_tree_bundle.find_low_element_for_nonexistent(&address_biguint)?;
            let address_bundle = address_tree_bundle
                .new_element_with_low_element_index(old_low_address.index, &address_biguint)?;

            let (old_low_address, old_low_address_next_value) =
                address_tree_bundle.find_low_element_for_nonexistent(&address_biguint)?;

            // Get the Merkle proof for updating low element.
            let low_address_proof =
                address_tree_bundle.get_proof_of_leaf(old_low_address.index, full)?;

            let low_address_index: u64 = old_low_address.index as u64;
            let low_address_value: [u8; 32] =
                bigint_to_be_bytes_array(&old_low_address.value).unwrap();
            let low_address_next_index: u64 = old_low_address.next_index as u64;
            let low_address_next_value: [u8; 32] =
                bigint_to_be_bytes_array(&old_low_address_next_value).unwrap();
            let low_address_proof: [[u8; 32]; NET_HEIGHT] = low_address_proof.try_into().unwrap();
            let proof = NewAddressProofWithContext::<NET_HEIGHT> {
                merkle_tree: merkle_tree_pubkey,
                low_address_index,
                low_address_value,
                low_address_next_index,
                low_address_next_value,
                low_address_proof,
                root: address_tree_bundle.root(),
                root_seq: address_tree_bundle.sequence_number(),
                new_low_element: Some(address_bundle.new_low_element),
                new_element: Some(address_bundle.new_element),
                new_element_next_value: Some(address_bundle.new_element_next_value),
            };
            proofs.push(proof);
        }
        Ok(proofs)
    }
}
