use log::{debug, info, warn};
use num_bigint::BigUint;
use solana_sdk::bs58;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use account_compression::{AddressMerkleTreeConfig, AddressQueueConfig, StateMerkleTreeConfig};
use light_compressed_token::constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR;
use light_compressed_token::mint_sdk::create_create_token_pool_instruction;
use light_compressed_token::{get_token_pool_pda, TokenData};
use light_utils::bigint::bigint_to_be_bytes_array;
use {
    crate::{
        create_account_instruction,
        test_env::{create_state_merkle_tree_and_queue_account, EnvAccounts},
    },
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

use crate::indexer::{Indexer, IndexerError, MerkleProof, NewAddressProofWithContext};
use crate::transaction_params::FeeConfig;
use crate::{get_concurrent_merkle_tree, get_indexed_merkle_tree};
use crate::{
    rpc::rpc_connection::RpcConnection, test_env::create_address_merkle_tree_and_queue_account,
};

#[derive(Debug)]
pub struct ProofRpcResult {
    pub proof: CompressedProof,
    pub root_indices: Vec<u16>,
    pub address_root_indices: Vec<u16>,
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct StateMerkleTreeAccounts {
    pub merkle_tree: Pubkey,
    pub nullifier_queue: Pubkey,
    pub cpi_context: Pubkey,
}

#[derive(Debug, Clone, Copy)]
pub struct AddressMerkleTreeAccounts {
    pub merkle_tree: Pubkey,
    pub queue: Pubkey,
}

#[derive(Debug, Clone)]
pub struct StateMerkleTreeBundle {
    pub rollover_fee: i64,
    pub merkle_tree: Box<MerkleTree<Poseidon>>,
    pub accounts: StateMerkleTreeAccounts,
}

#[derive(Debug, Clone)]
pub struct AddressMerkleTreeBundle<const INDEXED_ARRAY_SIZE: usize> {
    pub rollover_fee: i64,
    pub merkle_tree: Box<IndexedMerkleTree<Poseidon, usize>>,
    pub indexed_array: Box<IndexedArray<Poseidon, usize, INDEXED_ARRAY_SIZE>>,
    pub accounts: AddressMerkleTreeAccounts,
}

// TODO: find a different way to init Indexed array on the heap so that it doesn't break the stack
#[derive(Debug)]
pub struct TestIndexer<const INDEXED_ARRAY_SIZE: usize, R: RpcConnection> {
    pub state_merkle_trees: Vec<StateMerkleTreeBundle>,
    pub address_merkle_trees: Vec<AddressMerkleTreeBundle<INDEXED_ARRAY_SIZE>>,
    pub payer: Keypair,
    pub group_pda: Pubkey,
    pub compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub nullified_compressed_accounts: Arc<Mutex<Vec<CompressedAccountWithMerkleContext>>>,
    pub token_compressed_accounts: Vec<TokenDataWithContext>,
    pub token_nullified_compressed_accounts: Vec<TokenDataWithContext>,
    pub events: Vec<PublicTransactionEvent>,
    pub proof_types: Vec<ProofType>,
    phantom: PhantomData<R>,
}

impl<const INDEXED_ARRAY_SIZE: usize, R: RpcConnection> Clone
    for TestIndexer<INDEXED_ARRAY_SIZE, R>
{
    fn clone(&self) -> Self {
        Self {
            state_merkle_trees: self.state_merkle_trees.clone(),
            address_merkle_trees: self.address_merkle_trees.clone(),
            payer: self.payer.insecure_clone(),
            group_pda: self.group_pda,
            compressed_accounts: self.compressed_accounts.clone(),
            nullified_compressed_accounts: self.nullified_compressed_accounts.clone(),
            token_compressed_accounts: self.token_compressed_accounts.clone(),
            token_nullified_compressed_accounts: self.token_nullified_compressed_accounts.clone(),
            events: self.events.clone(),
            proof_types: self.proof_types.clone(),
            phantom: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenDataWithContext {
    pub token_data: TokenData,
    pub compressed_account: CompressedAccountWithMerkleContext,
}

impl<const INDEXED_ARRAY_SIZE: usize, R: RpcConnection + Send + Sync + 'static> Indexer
    for TestIndexer<INDEXED_ARRAY_SIZE, R>
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
                        leaf_index: leaf_index as u32,
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
        let result = self.get_compressed_accounts_by_owner(owner);
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
        address: [u8; 32],
    ) -> Result<NewAddressProofWithContext, IndexerError> {
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
        let low_address_value: [u8; 32] = bigint_to_be_bytes_array(&old_low_address.value).unwrap();
        let low_address_next_index: u64 = old_low_address.next_index as u64;
        let low_address_next_value: [u8; 32] =
            bigint_to_be_bytes_array(&old_low_address_next_value).unwrap();
        let low_address_proof: [[u8; 32]; 16] = low_address_proof.to_array().unwrap();
        info!("merkle_tree_pubkey: {:?}", merkle_tree_pubkey);
        info!("address: {:?}", address);
        info!("low_address_index: {:?}", low_address_index);
        info!("low_address_value: {:?}", low_address_value);
        info!("low_address_next_index: {:?}", low_address_next_index);
        info!("low_address_next_value: {:?}", low_address_next_value);
        info!("low_address_proof: {:?}", low_address_proof);

        Ok(NewAddressProofWithContext {
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
        })
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
        merkle_tree_pubkey: [u8; 32],
        context: NewAddressProofWithContext,
    ) {
        let pubkey = Pubkey::from(merkle_tree_pubkey);
        let mut address_tree_bundle: &mut AddressMerkleTreeBundle<{ INDEXED_ARRAY_SIZE }> = self
            .address_merkle_trees
            .iter_mut()
            .find(|x| x.accounts.merkle_tree == pubkey)
            .unwrap();

        let new_low_element = context.new_low_element.unwrap();
        let new_element = context.new_element.unwrap();
        let new_element_next_value = context.new_element_next_value.unwrap();
        address_tree_bundle
            .merkle_tree
            .update(&new_low_element, &new_element, &new_element_next_value)
            .unwrap();
        address_tree_bundle
            .indexed_array
            .append_with_low_element_index(new_low_element.index, &new_element.value)
            .unwrap();
    }
}

impl<const INDEXED_ARRAY_SIZE: usize, R: RpcConnection> TestIndexer<INDEXED_ARRAY_SIZE, R> {
    fn count_matching_hashes(&self, query_hashes: &[String]) -> usize {
        self.nullified_compressed_accounts
            .lock()
            .unwrap()
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

    pub fn get_state_merkle_tree_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Vec<StateMerkleTreeAccounts> {
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
        let nullified_compressed_accounts = Arc::new(Mutex::new(Vec::new()));

        Self {
            state_merkle_trees,
            address_merkle_trees,
            payer,
            compressed_accounts: vec![],
            nullified_compressed_accounts,
            events: vec![],
            token_compressed_accounts: vec![],
            token_nullified_compressed_accounts: vec![],
            proof_types: vec_proof_types,
            phantom: Default::default(),
            group_pda,
        }
    }

    pub fn add_address_merkle_tree_bundle(
        address_merkle_tree_accounts: AddressMerkleTreeAccounts,
        // TODO: add config here
    ) -> AddressMerkleTreeBundle<INDEXED_ARRAY_SIZE> {
        let mut merkle_tree = Box::new(
            IndexedMerkleTree::<Poseidon, usize>::new(
                STATE_MERKLE_TREE_HEIGHT as usize,
                STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
            )
            .unwrap(),
        );
        merkle_tree.init().unwrap();
        let mut indexed_array = Box::<IndexedArray<Poseidon, usize, INDEXED_ARRAY_SIZE>>::default();
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
            &self.group_pda,
            rpc,
            merkle_tree_keypair,
            queue_keypair,
            owning_program_id,
            &AddressMerkleTreeConfig::default(),
            &AddressQueueConfig::default(),
            self.address_merkle_trees.len() as u64,
        )
        .await;
        let address_merkle_tree_accounts = AddressMerkleTreeAccounts {
            merkle_tree: merkle_tree_keypair.pubkey(),
            queue: queue_keypair.pubkey(),
        };
        self.address_merkle_trees
            .push(Self::add_address_merkle_tree_bundle(
                address_merkle_tree_accounts,
            ));
        address_merkle_tree_accounts
    }

    pub async fn add_state_merkle_tree(
        &mut self,
        rpc: &mut R,
        merkle_tree_keypair: &Keypair,
        nullifier_queue_keypair: &Keypair,
        cpi_context_keypair: &Keypair,
        owning_program_id: Option<Pubkey>,
    ) {
        create_state_merkle_tree_and_queue_account(
            &self.payer,
            &self.group_pda,
            rpc,
            merkle_tree_keypair,
            nullifier_queue_keypair,
            owning_program_id,
            self.state_merkle_trees.len() as u64,
            StateMerkleTreeConfig::default(),
        )
        .await;
        crate::test_env::init_cpi_context_account(
            rpc,
            &merkle_tree_keypair.pubkey(),
            cpi_context_keypair,
            &self.payer,
        )
        .await;

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

    pub async fn create_proof_for_compressed_accounts(
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
                spawn_prover(true, self.proof_types.as_slice()).await;
                retries -= 1;
            }
        }
        panic!("Failed to get proof from server");
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
            let fetched_merkle_tree = unsafe {
                get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
                    rpc,
                    merkle_tree_pubkeys[i],
                )
                .await
            };
            for i in 0..fetched_merkle_tree.roots.len() {
                debug!("roots {:?} {:?}", i, fetched_merkle_tree.roots[i]);
            }
            debug!(
                "sequence number {:?}",
                fetched_merkle_tree.sequence_number()
            );
            debug!("root index {:?}", fetched_merkle_tree.root_index());
            debug!("local sequence number {:?}", merkle_tree.sequence_number);

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
    pub fn add_lamport_compressed_accounts(&mut self, event_bytes: Vec<u8>) {
        let event_bytes = event_bytes.clone();
        let event = PublicTransactionEvent::deserialize(&mut event_bytes.as_slice()).unwrap();
        self.add_event_and_compressed_accounts(&event);
    }

    pub fn add_event_and_compressed_accounts(
        &mut self,
        event: &PublicTransactionEvent,
    ) -> (
        Vec<CompressedAccountWithMerkleContext>,
        Vec<TokenDataWithContext>,
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
                    .lock()
                    .unwrap()
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
                            let token_account = TokenDataWithContext {
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

    /// deserializes an event
    /// adds the output_compressed_accounts to the compressed_accounts
    /// removes the input_compressed_accounts from the compressed_accounts
    /// adds the input_compressed_accounts to the nullified_compressed_accounts
    /// deserialiazes token data from the output_compressed_accounts
    /// adds the token_compressed_accounts to the token_compressed_accounts
    pub fn add_compressed_accounts_with_token_data(&mut self, event: &PublicTransactionEvent) {
        self.add_event_and_compressed_accounts(event);
    }

    /// returns compressed_accounts with the owner pubkey
    /// does not return token accounts.
    pub fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext> {
        self.compressed_accounts
            .iter()
            .filter(|x| x.compressed_account.owner == *owner)
            .cloned()
            .collect()
    }

    pub fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Vec<TokenDataWithContext> {
        self.token_compressed_accounts
            .iter()
            .filter(|x| x.token_data.owner == *owner)
            .cloned()
            .collect()
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

pub fn create_initialize_mint_instructions(
    payer: &Pubkey,
    authority: &Pubkey,
    rent: u64,
    decimals: u8,
    mint_keypair: &Keypair,
) -> ([Instruction; 4], Pubkey) {
    let account_create_ix = create_account_instruction(
        payer,
        spl_token::state::Mint::LEN,
        rent,
        &spl_token::ID,
        Some(mint_keypair),
    );

    let mint_pubkey = mint_keypair.pubkey();
    let create_mint_instruction = initialize_mint(
        &spl_token::ID,
        &mint_keypair.pubkey(),
        authority,
        None,
        decimals,
    )
    .unwrap();
    let transfer_ix =
        anchor_lang::solana_program::system_instruction::transfer(payer, &mint_pubkey, rent);

    let instruction = create_create_token_pool_instruction(payer, &mint_pubkey);
    let pool_pubkey = get_token_pool_pda(&mint_pubkey);
    (
        [
            account_create_ix,
            create_mint_instruction,
            transfer_ix,
            instruction,
        ],
        pool_pubkey,
    )
}

pub async fn create_mint_helper<R: RpcConnection>(rpc: &mut R, payer: &Keypair) -> Pubkey {
    let payer_pubkey = payer.pubkey();
    let rent = rpc
        .get_minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN)
        .await
        .unwrap();
    let mint = Keypair::new();

    let (instructions, _): ([Instruction; 4], Pubkey) =
        create_initialize_mint_instructions(&payer_pubkey, &payer_pubkey, rent, 2, &mint);

    rpc.create_and_send_transaction(&instructions, &payer_pubkey, &[payer, &mint])
        .await
        .unwrap();

    mint.pubkey()
}
