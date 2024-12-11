use crate::indexer::TestIndexerExtensions;
use crate::test_env::{
    create_address_merkle_tree_and_queue_account, create_state_merkle_tree_and_queue_account,
    EnvAccounts,
};
use account_compression::{
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeConfig,
};
use borsh::BorshDeserialize;
use forester_utils::{
    get_concurrent_merkle_tree, get_indexed_merkle_tree, AddressMerkleTreeAccounts,
    AddressMerkleTreeBundle, StateMerkleTreeAccounts, StateMerkleTreeBundle,
};
use light_client::indexer::error::IndexerError;
use light_client::indexer::{Indexer, MerkleProof, NewAddressProofWithContext};
use light_client::rpc::RpcConnection;
use light_client::transaction_params::FeeConfig;
use light_compressed_token::constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR;
use light_prover_client::gnark::combined_json_formatter::CombinedJsonStruct;
use light_prover_client::gnark::constants::{PROVE_PATH, SERVER_ADDRESS};
use light_prover_client::gnark::helpers::ProverConfig;
use light_prover_client::gnark::proof_helpers::{
    compress_proof, deserialize_gnark_proof_json, proof_from_json_struct,
};
use light_sdk::compressed_account::CompressedAccountWithMerkleContext;
use light_sdk::event::PublicTransactionEvent;
use light_sdk::merkle_context::MerkleContext;
use light_sdk::proof::{CompressedProof, ProofRpcResult};
use light_sdk::token::{TokenData, TokenDataWithMerkleContext};
use light_utils::bigint::bigint_to_be_bytes_array;
use log::{info, warn};
use num_bigint::BigUint;
use reqwest::Client;
use solana_sdk::bs58;
use std::marker::PhantomData;
use std::time::Duration;
use {
    account_compression::{
        utils::constants::{STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT},
        AddressMerkleTreeAccount, StateMerkleTreeAccount,
    },
    light_hasher::Poseidon,
    light_indexed_merkle_tree::{array::IndexedArray, reference::IndexedMerkleTree},
    light_merkle_tree_reference::MerkleTree,
    light_prover_client::{
        gnark::{
            helpers::spawn_prover, inclusion_json_formatter::BatchInclusionJsonStruct,
            non_inclusion_json_formatter::BatchNonInclusionJsonStruct,
        },
        inclusion::merkle_inclusion_proof_inputs::{
            InclusionMerkleProofInputs, InclusionProofInputs,
        },
        non_inclusion::merkle_non_inclusion_proof_inputs::{
            get_non_inclusion_proof_inputs, NonInclusionProofInputs,
        },
    },
    num_bigint::BigInt,
    num_traits::ops::bytes::FromBytes,
    solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer},
};

// TODO: find a different way to init Indexed array on the heap so that it doesn't break the stack
#[derive(Debug)]
pub struct TestIndexer<R>
where
    R: RpcConnection,
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

impl<R> Indexer<R> for TestIndexer<R>
where
    R: RpcConnection + Send + Sync + 'static,
{
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
    ) -> Result<Vec<NewAddressProofWithContext>, IndexerError> {
        let mut proofs: Vec<NewAddressProofWithContext> = Vec::new();

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
                .get_proof_of_leaf(old_low_address.index, false)
                .unwrap();

            let low_address_index: u64 = old_low_address.index as u64;
            let low_address_value: [u8; 32] =
                bigint_to_be_bytes_array(&old_low_address.value).unwrap();
            let low_address_next_index: u64 = old_low_address.next_index as u64;
            let low_address_next_value: [u8; 32] =
                bigint_to_be_bytes_array(&old_low_address_next_value).unwrap();
            let low_address_proof: [[u8; 32]; 16] = low_address_proof.to_array().unwrap();
            let proof = NewAddressProofWithContext {
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

impl<R: RpcConnection> TestIndexerExtensions<R> for TestIndexer<R> {
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
        context: &NewAddressProofWithContext,
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
                    .find(|y| y.accounts.merkle_tree == *x)
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

    fn get_address_merkle_trees(&self) -> &Vec<AddressMerkleTreeBundle> {
        &self.address_merkle_trees
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

    async fn create_proof_for_compressed_accounts(
        &mut self,
        compressed_accounts: Option<&[[u8; 32]]>,
        state_merkle_tree_pubkeys: Option<&[Pubkey]>,
        new_addresses: Option<&[[u8; 32]]>,
        address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
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
                if let Some(ref prover_config) = self.prover_config {
                    spawn_prover(true, prover_config.clone()).await;
                }
                retries -= 1;
            }
        }
        panic!("Failed to get proof from server");
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

    /// Returns compressed accounts owned by the given `owner`.
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
                    if compressed_account.compressed_account.owner == light_compressed_token::ID
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
}

impl<R> TestIndexer<R>
where
    R: RpcConnection,
{
    pub async fn init_from_env(
        payer: &Keypair,
        env: &EnvAccounts,
        prover_config: Option<ProverConfig>,
    ) -> Self {
        Self::new(
            &[StateMerkleTreeAccounts {
                merkle_tree: env.merkle_tree_pubkey,
                nullifier_queue: env.nullifier_queue_pubkey,
                cpi_context: env.cpi_context_account_pubkey,
            }],
            &[AddressMerkleTreeAccounts {
                merkle_tree: env.address_merkle_tree_pubkey,
                queue: env.address_merkle_tree_queue_pubkey,
            }],
            payer.insecure_clone(),
            env.group_pda,
            prover_config,
        )
        .await
    }

    pub async fn new(
        state_merkle_tree_accounts: &[StateMerkleTreeAccounts],
        address_merkle_tree_accounts: &[AddressMerkleTreeAccounts],
        payer: Keypair,
        group_pda: Pubkey,
        prover_config: Option<ProverConfig>,
    ) -> Self {
        if let Some(ref prover_config) = prover_config {
            spawn_prover(true, prover_config.clone()).await;
        }
        let mut state_merkle_trees = Vec::new();
        for state_merkle_tree_account in state_merkle_tree_accounts.iter() {
            let merkle_tree = Box::new(MerkleTree::<Poseidon>::new(
                STATE_MERKLE_TREE_HEIGHT as usize,
                STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
            ));
            state_merkle_trees.push(StateMerkleTreeBundle {
                accounts: *state_merkle_tree_account,
                merkle_tree,
                rollover_fee: FeeConfig::default().state_merkle_tree_rollover as i64,
            });
        }

        let mut address_merkle_trees = Vec::new();
        for address_merkle_tree_account in address_merkle_tree_accounts {
            address_merkle_trees.push(Self::add_address_merkle_tree_bundle(
                *address_merkle_tree_account,
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
        let mut merkle_tree = Box::new(
            IndexedMerkleTree::<Poseidon, usize>::new(
                STATE_MERKLE_TREE_HEIGHT as usize,
                STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
            )
            .unwrap(),
        );
        merkle_tree.init().unwrap();
        let mut indexed_array = Box::<IndexedArray<Poseidon, usize>>::default();
        indexed_array.init().unwrap();
        AddressMerkleTreeBundle {
            merkle_tree,
            indexed_array,
            accounts: address_merkle_tree_accounts,
            rollover_fee: FeeConfig::default().address_queue_rollover as i64,
        }
    }

    pub async fn add_address_merkle_tree(
        &mut self,
        rpc: &mut R,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
    ) -> AddressMerkleTreeAccounts {
        create_address_merkle_tree_and_queue_account(
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

    pub async fn add_state_merkle_tree(
        &mut self,
        rpc: &mut R,
        merkle_tree_keypair: &Keypair,
        nullifier_queue_keypair: &Keypair,
        cpi_context_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
        forester: Option<Pubkey>,
    ) {
        create_state_merkle_tree_and_queue_account(
            &self.payer,
            true,
            rpc,
            merkle_tree_keypair,
            nullifier_queue_keypair,
            Some(cpi_context_keypair),
            owning_program_id,
            forester,
            self.state_merkle_trees.len() as u64,
            &StateMerkleTreeConfig::default(),
            &NullifierQueueConfig::default(),
        )
        .await
        .unwrap();

        let state_merkle_tree_account = StateMerkleTreeAccounts {
            merkle_tree: merkle_tree_keypair.pubkey(),
            nullifier_queue: nullifier_queue_keypair.pubkey(),
            cpi_context: cpi_context_keypair.pubkey(),
        };
        let merkle_tree = Box::new(MerkleTree::<Poseidon>::new(
            STATE_MERKLE_TREE_HEIGHT as usize,
            STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
        ));

        self.state_merkle_trees.push(StateMerkleTreeBundle {
            merkle_tree,
            accounts: state_merkle_tree_account,
            rollover_fee: FeeConfig::default().state_merkle_tree_rollover as i64,
        });
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
            let fetched_merkle_tree = {
                get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
                    rpc,
                    merkle_tree_pubkeys[i],
                )
                .await
            };
            for i in 0..fetched_merkle_tree.roots.len() {
                info!("roots {:?} {:?}", i, fetched_merkle_tree.roots[i]);
            }
            info!(
                "sequence number {:?}",
                fetched_merkle_tree.sequence_number()
            );
            info!("root index {:?}", fetched_merkle_tree.root_index());
            info!("local sequence number {:?}", merkle_tree.sequence_number);

            assert_eq!(
                merkle_tree.root(),
                fetched_merkle_tree.root(),
                "Merkle tree root mismatch"
            );

            root_indices.push(fetched_merkle_tree.root_index() as u16);
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
            let fetched_address_merkle_tree = {
                get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
                    rpc,
                    address_merkle_tree_pubkeys[i],
                )
                .await
            };
            address_root_indices.push(fetched_address_merkle_tree.root_index() as u16);
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
    pub fn add_lamport_compressed_accounts(&mut self, event_bytes: Vec<u8>) {
        let event_bytes = event_bytes.clone();
        let event = PublicTransactionEvent::deserialize(&mut event_bytes.as_slice()).unwrap();
        self.add_event_and_compressed_accounts(&event);
    }

    /// deserializes an event
    /// adds the output_compressed_accounts to the compressed_accounts
    /// removes the input_compressed_accounts from the compressed_accounts
    /// adds the input_compressed_accounts to the nullified_compressed_accounts
    /// deserializes token data from the output_compressed_accounts
    /// adds the token_compressed_accounts to the token_compressed_accounts
    pub fn add_compressed_accounts_with_token_data(&mut self, event: &PublicTransactionEvent) {
        self.add_event_and_compressed_accounts(event);
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
}