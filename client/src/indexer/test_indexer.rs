use std::{marker::PhantomData, time::Duration};

use borsh::BorshDeserialize;
use light_concurrent_merkle_tree::light_hasher::Poseidon;
use light_indexed_merkle_tree::{array::IndexedArray, reference::IndexedMerkleTree};
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::{
    gnark::{
        combined_json_formatter::CombinedJsonStruct,
        constants::{PROVE_PATH, SERVER_ADDRESS},
        inclusion_json_formatter::BatchInclusionJsonStruct,
        non_inclusion_json_formatter::BatchNonInclusionJsonStruct,
        proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    },
    inclusion::merkle_inclusion_proof_inputs::{InclusionMerkleProofInputs, InclusionProofInputs},
    non_inclusion::merkle_non_inclusion_proof_inputs::{
        get_non_inclusion_proof_inputs, NonInclusionProofInputs,
    },
};
use light_sdk::{
    compressed_account::CompressedAccountWithMerkleContext,
    event::PublicTransactionEvent,
    merkle_context::MerkleContext,
    proof::{CompressedProof, ProofRpcResult},
    token::{TokenData, TokenDataWithMerkleContext},
    ADDRESS_MERKLE_TREE_CANOPY_DEPTH, ADDRESS_MERKLE_TREE_HEIGHT, PROGRAM_ID_LIGHT_SYSTEM,
    STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT,
    TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
};
use log::warn;
use num_bigint::BigInt;
use num_traits::FromBytes;
use reqwest::Client;
use solana_sdk::pubkey::Pubkey;
use tokio::time::sleep;

use crate::{
    indexer::Indexer,
    rpc::{merkle_tree::MerkleTreeExt, RpcConnection},
    transaction_params::FeeConfig,
};

use super::{
    AddressMerkleTreeAccounts, AddressMerkleTreeBundle, StateMerkleTreeAccounts,
    StateMerkleTreeBundle,
};

#[derive(Debug)]
pub struct TestIndexer<R>
where
    R: RpcConnection + MerkleTreeExt,
{
    pub state_merkle_trees: Vec<StateMerkleTreeBundle>,
    pub address_merkle_trees: Vec<AddressMerkleTreeBundle>,
    pub compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub nullified_compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub token_compressed_accounts: Vec<TokenDataWithMerkleContext>,
    pub token_nullified_compressed_accounts: Vec<TokenDataWithMerkleContext>,
    pub events: Vec<PublicTransactionEvent>,
    _rpc: PhantomData<R>,
}

impl<R> Indexer<R> for TestIndexer<R>
where
    R: RpcConnection + MerkleTreeExt,
{
    fn add_event_and_compressed_accounts(
        &mut self,
        event: &PublicTransactionEvent,
    ) -> (
        Vec<CompressedAccountWithMerkleContext>,
        Vec<TokenDataWithMerkleContext>,
    ) {
        for hash in event.input_compressed_account_hashes.iter() {
            let index = self.compressed_accounts.iter().position(|x| {
                x.compressed_account
                    .hash::<Poseidon>(
                        &x.merkle_context.merkle_tree_pubkey,
                        &x.merkle_context.leaf_index,
                    )
                    .unwrap()
                    == *hash
            });
            if let Some(index) = index {
                self.nullified_compressed_accounts
                    .push(self.compressed_accounts[index].clone());
                self.compressed_accounts.remove(index);
                continue;
            };
            if index.is_none() {
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
                            == *hash
                    })
                    .expect("input compressed account not found");
                self.token_nullified_compressed_accounts
                    .push(self.token_compressed_accounts[index].clone());
                self.token_compressed_accounts.remove(index);
            }
        }

        let mut compressed_accounts = Vec::new();
        let mut token_compressed_accounts = Vec::new();
        for (i, compressed_account) in event.output_compressed_accounts.iter().enumerate() {
            let nullifier_queue_pubkey = self
                .state_merkle_trees
                .iter()
                .find(|x| {
                    x.accounts.merkle_tree
                        == event.pubkey_array
                            [event.output_compressed_accounts[i].merkle_tree_index as usize]
                })
                .unwrap()
                .accounts
                .nullifier_queue;
            // if data is some, try to deserialize token data, if it fails, add to compressed_accounts
            // if data is none add to compressed_accounts
            // new accounts are inserted in front so that the newest accounts are found first
            match compressed_account.compressed_account.data.as_ref() {
                Some(data) => {
                    if compressed_account.compressed_account.owner == PROGRAM_ID_LIGHT_SYSTEM
                        && data.discriminator == TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR
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
                                        merkle_tree_pubkey: event.pubkey_array[event
                                            .output_compressed_accounts[i]
                                            .merkle_tree_index
                                            as usize],
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
                                merkle_tree_pubkey: event.pubkey_array[event
                                    .output_compressed_accounts[i]
                                    .merkle_tree_index
                                    as usize],
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
                            merkle_tree_pubkey: event.pubkey_array
                                [event.output_compressed_accounts[i].merkle_tree_index as usize],
                            nullifier_queue_pubkey,
                            queue_index: None,
                        },
                    };
                    compressed_accounts.push(compressed_account.clone());
                    self.compressed_accounts.insert(0, compressed_account);
                }
            };
            let merkle_tree = &mut self
                .state_merkle_trees
                .iter_mut()
                .find(|x| {
                    x.accounts.merkle_tree
                        == event.pubkey_array
                            [event.output_compressed_accounts[i].merkle_tree_index as usize]
                })
                .unwrap()
                .merkle_tree;
            merkle_tree
                .append(
                    &compressed_account
                        .compressed_account
                        .hash::<Poseidon>(
                            &event.pubkey_array
                                [event.output_compressed_accounts[i].merkle_tree_index as usize],
                            &event.output_leaf_indices[i],
                        )
                        .unwrap(),
                )
                .expect("insert failed");
        }

        self.events.push(event.clone());
        (compressed_accounts, token_compressed_accounts)
    }

    async fn create_proof_for_compressed_accounts(
        &mut self,
        compressed_accounts: Option<&[[u8; 32]]>,
        state_merkle_tree_pubkeys: Option<&[solana_sdk::pubkey::Pubkey]>,
        new_addresses: Option<&[[u8; 32]]>,
        address_merkle_tree_pubkeys: Option<Vec<solana_sdk::pubkey::Pubkey>>,
        rpc: &mut R,
    ) -> ProofRpcResult {
        if compressed_accounts.is_some()
            && ![1usize, 2usize, 3usize, 4usize, 8usize]
                .contains(&compressed_accounts.unwrap().len())
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
                    let (payload, indices) = self
                        .process_inclusion_proofs(state_merkle_tree_pubkeys.unwrap(), accounts, rpc)
                        .await;
                    (indices, Vec::new(), payload.to_string())
                }
                (None, Some(addresses)) => {
                    let (payload, indices) = self
                        .process_non_inclusion_proofs(
                            address_merkle_tree_pubkeys.unwrap().as_slice(),
                            addresses,
                            rpc,
                        )
                        .await;
                    (Vec::<u16>::new(), indices, payload.to_string())
                }
                (Some(accounts), Some(addresses)) => {
                    let (inclusion_payload, inclusion_indices) = self
                        .process_inclusion_proofs(state_merkle_tree_pubkeys.unwrap(), accounts, rpc)
                        .await;
                    let (non_inclusion_payload, non_inclusion_indices) = self
                        .process_non_inclusion_proofs(
                            address_merkle_tree_pubkeys.unwrap().as_slice(),
                            addresses,
                            rpc,
                        )
                        .await;

                    let combined_payload = CombinedJsonStruct {
                        inclusion: inclusion_payload.inputs,
                        non_inclusion: non_inclusion_payload.inputs,
                    }
                    .to_string();
                    (inclusion_indices, non_inclusion_indices, combined_payload)
                }
                _ => {
                    panic!("At least one of compressed_accounts or new_addresses must be provided")
                }
            };

        let mut retries = 3;
        while retries > 0 {
            let response_result = client
                .post(&format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(json_payload.clone())
                .send()
                .await
                .expect("Failed to execute request.");
            if response_result.status().is_success() {
                let body = response_result.text().await.unwrap();
                let proof_json = deserialize_gnark_proof_json(&body).unwrap();
                let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
                let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
                return ProofRpcResult {
                    root_indices,
                    address_root_indices,
                    proof: CompressedProof {
                        a: proof_a,
                        b: proof_b,
                        c: proof_c,
                    },
                };
            } else {
                warn!("Error: {}", response_result.text().await.unwrap());
                tokio::time::sleep(Duration::from_secs(1)).await;
                // spawn_prover(true, self.proof_types.as_slice()).await;

                // std::process::Command::new("light")
                //     .arg("start-prover")
                //     .spawn()
                //     .expect("Failed to start prover");
                // sleep(Duration::from_secs(10)).await;
                retries -= 1;
            }
        }
        panic!("Failed to get proof from server");
    }

    /// Returns compressed accounts owned by the given `owner`.
    fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext> {
        self.compressed_accounts
            .iter()
            .filter(|x| x.compressed_account.owner == *owner)
            .cloned()
            .collect()
    }
}

impl<R> TestIndexer<R>
where
    R: RpcConnection + MerkleTreeExt,
{
    pub async fn new(
        state_merkle_tree_accounts: &[StateMerkleTreeAccounts],
        address_merkle_tree_accounts: &[AddressMerkleTreeAccounts],
        inclusion: bool,
        non_inclusion: bool,
    ) -> Self {
        let state_merkle_trees = state_merkle_tree_accounts
            .iter()
            .map(|accounts| {
                let merkle_tree = Box::new(MerkleTree::<Poseidon>::new(
                    STATE_MERKLE_TREE_HEIGHT,
                    STATE_MERKLE_TREE_CANOPY_DEPTH,
                ));
                StateMerkleTreeBundle {
                    accounts: *accounts,
                    merkle_tree,
                    rollover_fee: FeeConfig::default().state_merkle_tree_rollover,
                }
            })
            .collect::<Vec<_>>();

        let address_merkle_trees = address_merkle_tree_accounts
            .iter()
            .map(|accounts| Self::add_address_merkle_tree_bundle(accounts))
            .collect::<Vec<_>>();

        // let mut proof_types = vec![];
        // if inclusion {
        //     proof_types.push(ProofType::Inclusion);
        // }
        // if non_inclusion {
        //     proof_types.push(ProofType::NonInclusion);
        // }
        // if !non_inclusion || !inclusion {
        // spawn_prover(true, proof_types.as_slice()).await;
        let mut types = vec!["start-prover"];
        if !inclusion {
            types.push("-c");
        }
        if !non_inclusion {
            types.push("-n");
        }
        let project_root = light_prover_client::gnark::helpers::get_project_root();
        std::process::Command::new("light")
            .args(types.as_slice())
            .current_dir(project_root.unwrap().trim_end_matches("\n"))
            .spawn()
            .expect("Failed to start prover");
        sleep(Duration::from_secs(5)).await;

        Self {
            state_merkle_trees,
            address_merkle_trees,
            compressed_accounts: Vec::new(),
            nullified_compressed_accounts: Vec::new(),
            token_compressed_accounts: Vec::new(),
            token_nullified_compressed_accounts: Vec::new(),
            events: Vec::new(),
            _rpc: PhantomData,
        }
    }

    pub fn add_address_merkle_tree_bundle(
        accounts: &AddressMerkleTreeAccounts,
        // TODO: add config here
    ) -> AddressMerkleTreeBundle {
        let mut merkle_tree = Box::new(
            IndexedMerkleTree::<Poseidon, usize>::new(
                ADDRESS_MERKLE_TREE_HEIGHT,
                ADDRESS_MERKLE_TREE_CANOPY_DEPTH,
            )
            .unwrap(),
        );
        merkle_tree.init().unwrap();
        let mut indexed_array = Box::<IndexedArray<Poseidon, usize>>::default();
        indexed_array.init().unwrap();
        AddressMerkleTreeBundle {
            merkle_tree,
            indexed_array,
            accounts: *accounts,
            rollover_fee: FeeConfig::default().address_queue_rollover,
        }
    }

    async fn process_inclusion_proofs(
        &self,
        merkle_tree_pubkeys: &[Pubkey],
        accounts: &[[u8; 32]],
        rpc: &mut R,
    ) -> (BatchInclusionJsonStruct, Vec<u16>) {
        let mut inclusion_proofs = Vec::new();
        let mut root_indices = Vec::new();

        for (i, account) in accounts.iter().enumerate() {
            let merkle_tree = &self
                .state_merkle_trees
                .iter()
                .find(|x| x.accounts.merkle_tree == merkle_tree_pubkeys[i])
                .unwrap()
                .merkle_tree;
            let leaf_index = merkle_tree.get_leaf_index(account).unwrap();
            let proof = merkle_tree.get_proof_of_leaf(leaf_index, true).unwrap();
            inclusion_proofs.push(InclusionMerkleProofInputs {
                root: BigInt::from_be_bytes(merkle_tree.root().as_slice()),
                leaf: BigInt::from_be_bytes(account),
                path_index: BigInt::from_be_bytes(leaf_index.to_be_bytes().as_slice()),
                path_elements: proof.iter().map(|x| BigInt::from_be_bytes(x)).collect(),
            });
            let onchain_merkle_tree = rpc
                .get_state_merkle_tree(merkle_tree_pubkeys[i])
                .await
                .unwrap();

            root_indices.push(onchain_merkle_tree.root_index() as u16);
        }

        let inclusion_proof_inputs = InclusionProofInputs(inclusion_proofs.as_slice());
        let batch_inclusion_proof_inputs =
            BatchInclusionJsonStruct::from_inclusion_proof_inputs(&inclusion_proof_inputs);

        (batch_inclusion_proof_inputs, root_indices)
    }

    async fn process_non_inclusion_proofs(
        &self,
        address_merkle_tree_pubkeys: &[Pubkey],
        addresses: &[[u8; 32]],
        rpc: &mut R,
    ) -> (BatchNonInclusionJsonStruct, Vec<u16>) {
        let mut non_inclusion_proofs = Vec::new();
        let mut address_root_indices = Vec::new();
        for (i, address) in addresses.iter().enumerate() {
            let address_tree = &self
                .address_merkle_trees
                .iter()
                .find(|x| x.accounts.merkle_tree == address_merkle_tree_pubkeys[i])
                .unwrap();
            let proof_inputs = get_non_inclusion_proof_inputs(
                address,
                &address_tree.merkle_tree,
                &address_tree.indexed_array,
            );
            non_inclusion_proofs.push(proof_inputs);
            let onchain_address_merkle_tree = rpc
                .get_address_merkle_tree(address_merkle_tree_pubkeys[i])
                .await
                .unwrap();
            address_root_indices.push(onchain_address_merkle_tree.root_index() as u16);
        }

        let non_inclusion_proof_inputs = NonInclusionProofInputs(non_inclusion_proofs.as_slice());
        let batch_non_inclusion_proof_inputs =
            BatchNonInclusionJsonStruct::from_non_inclusion_proof_inputs(
                &non_inclusion_proof_inputs,
            );
        (batch_non_inclusion_proof_inputs, address_root_indices)
    }

    /// deserializes an event
    /// adds the output_compressed_accounts to the compressed_accounts
    /// removes the input_compressed_accounts from the compressed_accounts
    /// adds the input_compressed_accounts to the nullified_compressed_accounts
    /// deserialiazes token data from the output_compressed_accounts
    /// adds the token_compressed_accounts to the token_compressed_accounts
    pub fn add_compressed_accounts_with_token_data(&mut self, event: &PublicTransactionEvent) {
        self.add_event_and_compressed_accounts(event);
    }
}
