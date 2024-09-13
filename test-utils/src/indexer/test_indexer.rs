use crate::e2e_test_env::KeypairActionConfig;
use crate::{
    spl::create_initialize_mint_instructions,
    test_env::create_address_merkle_tree_and_queue_account,
};
use account_compression::{
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeConfig,
};
use async_trait::async_trait;
use forester_utils::indexer::{
    AddressMerkleTreeAccounts, AddressMerkleTreeBundle, Indexer, IndexerError, MerkleProof,
    NewAddressProofWithContext, ProofRpcResult, StateMerkleTreeAccounts, StateMerkleTreeBundle,
    TokenDataWithContext,
};
use forester_utils::{get_concurrent_merkle_tree, get_indexed_merkle_tree};
use light_client::rpc::RpcConnection;
use light_client::transaction_params::FeeConfig;
use light_compressed_token::constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR;
use light_compressed_token::mint_sdk::create_create_token_pool_instruction;
use light_compressed_token::{get_token_pool_pda, TokenData};
use light_utils::bigint::bigint_to_be_bytes_array;
use log::{debug, info, warn};
use num_bigint::BigUint;
use solana_sdk::bs58;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use {
    crate::test_env::{create_state_merkle_tree_and_queue_account, EnvAccounts},
    account_compression::{
        utils::constants::{STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT},
        AddressMerkleTreeAccount, StateMerkleTreeAccount,
    },
    anchor_lang::AnchorDeserialize,
    light_hasher::Poseidon,
    light_indexed_merkle_tree::{array::IndexedArray, reference::IndexedMerkleTree},
    light_merkle_tree_reference::MerkleTree,
    light_prover_client::{
        gnark::{
            combined_json_formatter::CombinedJsonStruct,
            constants::{PROVE_PATH, SERVER_ADDRESS},
            helpers::{spawn_prover, ProofType},
            inclusion_json_formatter::BatchInclusionJsonStruct,
            non_inclusion_json_formatter::BatchNonInclusionJsonStruct,
            proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
        },
        inclusion::merkle_inclusion_proof_inputs::{
            InclusionMerkleProofInputs, InclusionProofInputs,
        },
        non_inclusion::merkle_non_inclusion_proof_inputs::{
            get_non_inclusion_proof_inputs, NonInclusionProofInputs,
        },
    },
    light_system_program::{
        invoke::processor::CompressedProof,
        sdk::{
            compressed_account::{CompressedAccountWithMerkleContext, MerkleContext},
            event::PublicTransactionEvent,
        },
    },
    num_bigint::BigInt,
    num_traits::ops::bytes::FromBytes,
    reqwest::Client,
    solana_sdk::{
        instruction::Instruction, program_pack::Pack, pubkey::Pubkey, signature::Keypair,
        signer::Signer,
    },
    spl_token::instruction::initialize_mint,
    std::time::Duration,
};

// TODO: find a different way to init Indexed array on the heap so that it doesn't break the stack
pub struct TestIndexer<R: RpcConnection> {
    pub state: TestIndexerState,
    pub payer: Keypair,
    pub group_pda: Pubkey,
    pub proof_types: Vec<ProofType>,
    phantom: PhantomData<R>,
}
pub struct TestIndexerState {
    pub compressed_accounts: Arc<RwLock<Vec<CompressedAccountWithMerkleContext>>>,
    pub token_compressed_accounts: Arc<RwLock<Vec<TokenDataWithContext>>>,
    pub nullified_compressed_accounts: Arc<RwLock<Vec<CompressedAccountWithMerkleContext>>>,
    pub token_nullified_compressed_accounts: Arc<RwLock<Vec<TokenDataWithContext>>>,
    pub state_merkle_trees: Arc<RwLock<Vec<StateMerkleTreeBundle>>>,
    pub address_merkle_trees: Arc<RwLock<Vec<AddressMerkleTreeBundle>>>,
    pub events: Arc<RwLock<Vec<PublicTransactionEvent>>>,
}

impl Default for TestIndexerState {
    fn default() -> Self {
        Self {
            compressed_accounts: Arc::new(RwLock::new(Vec::new())),
            token_compressed_accounts: Arc::new(RwLock::new(Vec::new())),
            nullified_compressed_accounts: Arc::new(RwLock::new(Vec::new())),
            token_nullified_compressed_accounts: Arc::new(RwLock::new(Vec::new())),
            state_merkle_trees: Arc::new(RwLock::new(Vec::new())),
            address_merkle_trees: Arc::new(RwLock::new(Vec::new())),
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

unsafe impl<R: RpcConnection + Send + Sync> Send for TestIndexer<R> {}
unsafe impl<R: RpcConnection + Send + Sync> Sync for TestIndexer<R> {}

#[async_trait]
impl<R: RpcConnection> Indexer<R> for TestIndexer<R> {
    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<MerkleProof>, IndexerError> {
        info!("Getting proofs for {:?}", hashes);
        let mut proofs: Vec<MerkleProof> = Vec::new();
        let state_merkle_trees = self.state.state_merkle_trees.read().await;
        hashes.iter().for_each(|hash| {
            let hash_array: [u8; 32] = bs58::decode(hash)
                .into_vec()
                .unwrap()
                .as_slice()
                .try_into()
                .unwrap();

            state_merkle_trees.iter().for_each(|tree| {
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

    async fn get_rpc_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<String>, IndexerError> {
        let compressed_accounts = self.get_compressed_accounts_by_owner(owner).await;
        let hashes = compressed_accounts
            .iter()
            .map(|account| {
                let hash = account.hash()?;
                Ok(bs58::encode(hash).into_string())
            })
            .collect::<Result<Vec<String>, IndexerError>>()?;
        Ok(hashes)
    }

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext>, IndexerError> {
        let mut proofs: Vec<NewAddressProofWithContext> = Vec::new();
        let address_merkle_trees = self.state.address_merkle_trees.read().await;
        for address in addresses.iter() {
            info!("Getting new address proof for {:?}", address);
            let pubkey = Pubkey::from(merkle_tree_pubkey);
            let address_tree_bundle = address_merkle_trees
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

    async fn account_nullified(&self, merkle_tree_pubkey: Pubkey, account_hash: &str) {
        let decoded_hash: [u8; 32] = bs58::decode(account_hash)
            .into_vec()
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap();

        let mut state_merkle_trees = self.state.state_merkle_trees.write().await;
        if let Some(state_tree_bundle) = state_merkle_trees
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

    async fn address_tree_updated(
        &self,
        merkle_tree_pubkey: Pubkey,
        context: &NewAddressProofWithContext,
    ) {
        info!("Updating address tree...");
        let mut address_merkle_trees = self.state.address_merkle_trees.write().await;
        let mut address_tree_bundle: &mut AddressMerkleTreeBundle = address_merkle_trees
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

    async fn get_state_merkle_tree_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Vec<StateMerkleTreeAccounts> {
        let state_merkle_trees = self.state.state_merkle_trees.read().await;
        pubkeys
            .iter()
            .map(|x| {
                state_merkle_trees
                    .iter()
                    .find(|y| y.accounts.merkle_tree == *x)
                    .unwrap()
                    .accounts
            })
            .collect::<Vec<_>>()
    }

    async fn add_event_and_compressed_accounts(
        &self,
        event: &PublicTransactionEvent,
    ) -> (
        Vec<CompressedAccountWithMerkleContext>,
        Vec<TokenDataWithContext>,
    ) {
        println!("Adding event {:?}", event);
        let mut to_nullify = Vec::new();
        let mut token_to_nullify = Vec::new();

        // Process input compressed account hashes
        {
            let compressed_accounts = self.state.compressed_accounts.read().await;
            let token_compressed_accounts = self.state.token_compressed_accounts.read().await;

            for hash in &event.input_compressed_account_hashes {
                if let Some(account) = compressed_accounts.iter().find(|x| {
                    x.compressed_account
                        .hash::<Poseidon>(
                            &x.merkle_context.merkle_tree_pubkey,
                            &x.merkle_context.leaf_index,
                        )
                        .unwrap()
                        == *hash
                }) {
                    to_nullify.push(account.clone());
                } else if let Some(account) = token_compressed_accounts.iter().find(|x| {
                    x.compressed_account
                        .compressed_account
                        .hash::<Poseidon>(
                            &x.compressed_account.merkle_context.merkle_tree_pubkey,
                            &x.compressed_account.merkle_context.leaf_index,
                        )
                        .unwrap()
                        == *hash
                }) {
                    token_to_nullify.push(account.clone());
                }
            }
        }

        println!("Nullifying accounts");
        {
            let mut compressed_accounts = self.state.compressed_accounts.write().await;
            let mut token_compressed_accounts = self.state.token_compressed_accounts.write().await;
            let mut nullified_compressed_accounts =
                self.state.nullified_compressed_accounts.write().await;
            let mut token_nullified_compressed_accounts =
                self.state.token_nullified_compressed_accounts.write().await;

            compressed_accounts.retain(|x| !to_nullify.contains(x));
            nullified_compressed_accounts.extend(to_nullify);

            token_compressed_accounts.retain(|x| !token_to_nullify.contains(x));
            token_nullified_compressed_accounts.extend(token_to_nullify);
        } // Write locks are released here

        println!("Processing output compressed accounts");
        let mut new_compressed_accounts = Vec::new();
        let mut new_token_compressed_accounts = Vec::new();

        let state_merkle_trees = self.state.state_merkle_trees.read().await;

        for (i, compressed_account) in event.output_compressed_accounts.iter().enumerate() {
            let nullifier_queue_pubkey = state_merkle_trees
                .iter()
                .find(|x| {
                    x.accounts.merkle_tree
                        == event.pubkey_array
                            [event.output_compressed_accounts[i].merkle_tree_index as usize]
                })
                .unwrap()
                .accounts
                .nullifier_queue;

            let new_account = CompressedAccountWithMerkleContext {
                compressed_account: compressed_account.compressed_account.clone(),
                merkle_context: MerkleContext {
                    leaf_index: event.output_leaf_indices[i],
                    merkle_tree_pubkey: event.pubkey_array
                        [event.output_compressed_accounts[i].merkle_tree_index as usize],
                    nullifier_queue_pubkey,
                    queue_index: None,
                },
            };

            // if data is some, try to deserialize token data, if it fails, add to compressed_accounts
            // if data is none add to compressed_accounts
            // new accounts are inserted in front so that the newest accounts are found first
            match compressed_account.compressed_account.data.as_ref() {
                Some(data)
                    if compressed_account.compressed_account.owner
                        == light_compressed_token::ID
                        && data.discriminator == TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR =>
                {
                    if let Ok(token_data) = TokenData::deserialize(&mut data.data.as_slice()) {
                        let token_account = TokenDataWithContext {
                            token_data,
                            compressed_account: new_account.clone(),
                        };
                        new_token_compressed_accounts.push(token_account.clone());
                        self.state
                            .token_compressed_accounts
                            .write()
                            .await
                            .insert(0, token_account);
                    }
                }
                _ => {
                    new_compressed_accounts.push(new_account.clone());
                    self.state
                        .compressed_accounts
                        .write()
                        .await
                        .insert(0, new_account);
                }
            };
        }
        drop(state_merkle_trees); // Explicitly release the read lock

        println!("Updating Merkle trees");
        let mut state_merkle_trees = self.state.state_merkle_trees.write().await;
        for (i, compressed_account) in event.output_compressed_accounts.iter().enumerate() {
            let merkle_tree = state_merkle_trees
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
                            &event.pubkey_array
                                [event.output_compressed_accounts[i].merkle_tree_index as usize],
                            &event.output_leaf_indices[i],
                        )
                        .unwrap(),
                )
                .expect("insert failed");
        }

        // Update events
        self.state.events.write().await.push(event.clone());
        (new_compressed_accounts, new_token_compressed_accounts)
    }

    async fn get_state_merkle_trees(&self) -> Vec<StateMerkleTreeBundle> {
        self.state.state_merkle_trees.read().await.clone()
    }

    async fn get_address_merkle_trees(&self) -> Vec<AddressMerkleTreeBundle> {
        self.state.address_merkle_trees.read().await.clone()
    }

    async fn get_token_compressed_accounts(&self) -> Vec<TokenDataWithContext> {
        self.state.token_compressed_accounts.read().await.clone()
    }

    fn get_payer(&self) -> &Keypair {
        &self.payer
    }

    fn get_group_pda(&self) -> &Pubkey {
        &self.group_pda
    }

    async fn create_proof_for_compressed_accounts(
        &self,
        compressed_accounts: Option<&[[u8; 32]]>,
        state_merkle_tree_pubkeys: Option<&[Pubkey]>,
        new_addresses: Option<&[[u8; 32]]>,
        address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        rpc: &R,
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
                spawn_prover(true, self.proof_types.as_slice()).await;
                retries -= 1;
            }
        }
        panic!("Failed to get proof from server");
    }

    async fn add_address_merkle_tree_accounts(
        &self,
        merkle_tree_keypair: &Keypair,
        queue_keypair: &Keypair,
        _owning_program_id: Option<Pubkey>,
    ) -> AddressMerkleTreeAccounts {
        info!("Adding address merkle tree accounts...");
        let address_merkle_tree_accounts = AddressMerkleTreeAccounts {
            merkle_tree: merkle_tree_keypair.pubkey(),
            queue: queue_keypair.pubkey(),
        };
        let mut address_merkle_trees = self.state.address_merkle_trees.write().await;
        address_merkle_trees.push(Self::add_address_merkle_tree_bundle(
            address_merkle_tree_accounts,
        ));
        info!(
            "Address merkle tree accounts added. Total: {}",
            address_merkle_trees.len()
        );
        address_merkle_tree_accounts
    }

    /// returns compressed_accounts with the owner pubkey
    /// does not return token accounts.
    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext> {
        let compressed_accounts = self.state.compressed_accounts.read().await;
        compressed_accounts
            .iter()
            .filter(|x| x.compressed_account.owner == *owner)
            .cloned()
            .collect()
    }

    async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<TokenDataWithContext> {
        let token_compressed_accounts = self.state.token_compressed_accounts.read().await;
        token_compressed_accounts
            .iter()
            .filter(|x| x.token_data.owner == *owner)
            .cloned()
            .collect()
    }

    async fn add_state_bundle(&self, state_bundle: StateMerkleTreeBundle) {
        let mut state_merkle_trees = self.state.state_merkle_trees.write().await;
        state_merkle_trees.push(state_bundle);
    }

    async fn add_address_bundle(&self, address_bundle: AddressMerkleTreeBundle) {
        let mut address_merkle_trees = self.state.address_merkle_trees.write().await;
        address_merkle_trees.push(address_bundle);
    }

    async fn clear_state_trees(&self) {
        let mut state_merkle_trees = self.state.state_merkle_trees.write().await;
        state_merkle_trees.clear();
    }
}

impl<R: RpcConnection> TestIndexer<R> {
    async fn count_matching_hashes(&self, query_hashes: &[String]) -> usize {
        let nullified_compressed_accounts = self.state.nullified_compressed_accounts.read().await;
        nullified_compressed_accounts
            .iter()
            .map(|account| self.compute_hash(account))
            .filter(|bs58_hash| query_hashes.contains(bs58_hash))
            .count()
    }

    fn compute_hash(&self, account: &CompressedAccountWithMerkleContext) -> String {
        // replace AccountType with actual type
        let hash = account
            .compressed_account
            .hash::<Poseidon>(
                &account.merkle_context.merkle_tree_pubkey,
                &account.merkle_context.leaf_index,
            )
            .unwrap();
        bs58::encode(hash).into_string()
    }

    pub async fn init_from_env(
        payer: &Keypair,
        env: &EnvAccounts,
        inclusion: bool,
        non_inclusion: bool,
    ) -> Self {
        Self::new(
            vec![StateMerkleTreeAccounts {
                merkle_tree: env.merkle_tree_pubkey,
                nullifier_queue: env.nullifier_queue_pubkey,
                cpi_context: env.cpi_context_account_pubkey,
            }],
            vec![AddressMerkleTreeAccounts {
                merkle_tree: env.address_merkle_tree_pubkey,
                queue: env.address_merkle_tree_queue_pubkey,
            }],
            payer.insecure_clone(),
            env.group_pda,
            inclusion,
            non_inclusion,
        )
        .await
    }

    pub async fn new(
        state_merkle_tree_accounts: Vec<StateMerkleTreeAccounts>,
        address_merkle_tree_accounts: Vec<AddressMerkleTreeAccounts>,
        payer: Keypair,
        group_pda: Pubkey,
        inclusion: bool,
        non_inclusion: bool,
    ) -> Self {
        let mut vec_proof_types = vec![];
        if inclusion {
            vec_proof_types.push(ProofType::Inclusion);
        }
        if non_inclusion {
            vec_proof_types.push(ProofType::NonInclusion);
        }
        if !vec_proof_types.is_empty() {
            spawn_prover(true, vec_proof_types.as_slice()).await;
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
                address_merkle_tree_account,
            ));
        }

        Self {
            state: TestIndexerState {
                state_merkle_trees: Arc::new(RwLock::new(state_merkle_trees)),
                address_merkle_trees: Arc::new(RwLock::new(address_merkle_trees)),
                ..Default::default()
            },
            payer,
            proof_types: vec_proof_types,
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
        &self,
        rpc: &R,
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
            .await
    }

    pub async fn add_state_merkle_tree(
        &self,
        rpc: &R,
        merkle_tree_keypair: &Keypair,
        nullifier_queue_keypair: &Keypair,
        cpi_context_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
        forester: Option<Pubkey>,
    ) {
        {
            let state_merkle_trees = self.state.state_merkle_trees.read().await;
            create_state_merkle_tree_and_queue_account(
                &self.payer,
                true,
                rpc,
                merkle_tree_keypair,
                nullifier_queue_keypair,
                Some(cpi_context_keypair),
                owning_program_id,
                forester,
                state_merkle_trees.len() as u64,
                &StateMerkleTreeConfig::default(),
                &NullifierQueueConfig::default(),
            )
            .await
            .unwrap();
        }

        let state_merkle_tree_account = StateMerkleTreeAccounts {
            merkle_tree: merkle_tree_keypair.pubkey(),
            nullifier_queue: nullifier_queue_keypair.pubkey(),
            cpi_context: cpi_context_keypair.pubkey(),
        };
        let merkle_tree = Box::new(MerkleTree::<Poseidon>::new(
            STATE_MERKLE_TREE_HEIGHT as usize,
            STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
        ));

        let mut state_merkle_trees = self.state.state_merkle_trees.write().await;
        state_merkle_trees.push(StateMerkleTreeBundle {
            merkle_tree,
            accounts: state_merkle_tree_account,
            rollover_fee: FeeConfig::default().state_merkle_tree_rollover as i64,
        });
    }

    async fn process_inclusion_proofs(
        &self,
        merkle_tree_pubkeys: &[Pubkey],
        accounts: &[[u8; 32]],
        rpc: &R,
    ) -> (BatchInclusionJsonStruct, Vec<u16>) {
        let state_merkle_trees = self.state.state_merkle_trees.read().await;

        // Step 1: Generate all proof-related data without any await
        let proof_data: Vec<_> = merkle_tree_pubkeys
            .iter()
            .zip(accounts)
            .map(|(&pubkey, account)| {
                let merkle_tree = &state_merkle_trees
                    .iter()
                    .find(|x| x.accounts.merkle_tree == pubkey)
                    .unwrap()
                    .merkle_tree;
                let leaf_index = merkle_tree.get_leaf_index(account).unwrap();
                let proof = merkle_tree.get_proof_of_leaf(leaf_index, true).unwrap();
                let root = merkle_tree.root();

                InclusionMerkleProofInputs {
                    root: BigInt::from_be_bytes(root.as_slice()),
                    leaf: BigInt::from_be_bytes(account),
                    path_index: BigInt::from_be_bytes(leaf_index.to_be_bytes().as_slice()),
                    path_elements: proof.iter().map(|x| BigInt::from_be_bytes(x)).collect(),
                }
            })
            .collect();

        // Step 2: Make RPC calls and perform checks
        let mut root_indices = Vec::new();
        for (&pubkey, proof_input) in merkle_tree_pubkeys.iter().zip(proof_data.iter()) {
            let fetched_merkle_tree = unsafe {
                get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(rpc, pubkey)
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

            assert_eq!(
                proof_input.root,
                BigInt::from_be_bytes(fetched_merkle_tree.root().as_slice()),
                "Merkle tree root mismatch"
            );

            root_indices.push(fetched_merkle_tree.root_index() as u16);
        }

        let inclusion_proof_inputs = InclusionProofInputs(&proof_data);
        let batch_inclusion_proof_inputs =
            BatchInclusionJsonStruct::from_inclusion_proof_inputs(&inclusion_proof_inputs);

        (batch_inclusion_proof_inputs, root_indices)
    }

    async fn process_non_inclusion_proofs(
        &self,
        address_merkle_tree_pubkeys: &[Pubkey],
        addresses: &[[u8; 32]],
        rpc: &R,
    ) -> (BatchNonInclusionJsonStruct, Vec<u16>) {
        let mut non_inclusion_proofs = Vec::new();
        let mut address_root_indices = Vec::new();
        let address_merkle_trees = self.state.address_merkle_trees.read().await;
        for (i, address) in addresses.iter().enumerate() {
            let address_tree = address_merkle_trees
                .iter()
                .find(|x| x.accounts.merkle_tree == address_merkle_tree_pubkeys[i])
                .unwrap();
            let proof_inputs = get_non_inclusion_proof_inputs(
                address,
                &address_tree.merkle_tree,
                &address_tree.indexed_array,
            );
            non_inclusion_proofs.push(proof_inputs);
            let fetched_address_merkle_tree = unsafe {
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
    pub async fn add_lamport_compressed_accounts(&mut self, event_bytes: Vec<u8>) {
        let event_bytes = event_bytes.clone();
        let event = PublicTransactionEvent::deserialize(&mut event_bytes.as_slice()).unwrap();
        self.add_event_and_compressed_accounts(&event).await;
    }

    /// deserializes an event
    /// adds the output_compressed_accounts to the compressed_accounts
    /// removes the input_compressed_accounts from the compressed_accounts
    /// adds the input_compressed_accounts to the nullified_compressed_accounts
    /// deserialiazes token data from the output_compressed_accounts
    /// adds the token_compressed_accounts to the token_compressed_accounts
    pub async fn add_compressed_accounts_with_token_data(&self, event: &PublicTransactionEvent) {
        self.add_event_and_compressed_accounts(event).await;
    }

    /// returns the compressed sol balance of the owner pubkey
    pub async fn get_compressed_balance(&self, owner: &Pubkey) -> u64 {
        let compressed_accounts = self.state.compressed_accounts.read().await;
        compressed_accounts
            .iter()
            .filter(|x| x.compressed_account.owner == *owner)
            .map(|x| x.compressed_account.lamports)
            .sum()
    }

    /// returns the compressed token balance of the owner pubkey for a token by mint
    pub async fn get_compressed_token_balance(&self, owner: &Pubkey, mint: &Pubkey) -> u64 {
        let token_compressed_accounts = self.state.token_compressed_accounts.read().await;
        token_compressed_accounts
            .iter()
            .filter(|x| {
                x.compressed_account.compressed_account.owner == *owner
                    && x.token_data.mint == *mint
            })
            .map(|x| x.token_data.amount)
            .sum()
    }
}
