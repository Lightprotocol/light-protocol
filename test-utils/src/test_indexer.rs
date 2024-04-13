#![cfg(feature = "test_indexer")]

use anchor_lang::AnchorDeserialize;
use num_bigint::{BigInt, BigUint};
use num_traits::ops::bytes::FromBytes;
use num_traits::Num;
use reqwest::Client;
use solana_program_test::ProgramTestContext;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer};
use spl_token::instruction::initialize_mint;

use account_compression::{
    utils::constants::{STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT},
    AddressMerkleTreeAccount, StateMerkleTreeAccount,
};
use light_circuitlib_rs::gnark::combined_json_formatter::CombinedJsonStruct;
use light_circuitlib_rs::gnark::helpers::spawn_prover;
use light_circuitlib_rs::gnark::inclusion_json_formatter::BatchInclusionJsonStruct;
use light_circuitlib_rs::gnark::non_inclusion_json_formatter::BatchNonInclusionJsonStruct;
use light_circuitlib_rs::non_inclusion::merkle_non_inclusion_proof_inputs::{
    get_non_inclusion_proof_inputs, NonInclusionMerkleProofInputs, NonInclusionProofInputs,
};
use light_circuitlib_rs::{
    gnark::{
        constants::{PROVE_PATH, SERVER_ADDRESS},
        proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    },
    inclusion::merkle_inclusion_proof_inputs::{InclusionMerkleProofInputs, InclusionProofInputs},
};
use light_compressed_pda::{
    compressed_account::CompressedAccountWithMerkleContext, event::PublicTransactionEvent,
    utils::CompressedProof,
};
use light_compressed_token::{
    get_token_authority_pda, get_token_pool_pda,
    mint_sdk::{create_initialize_mint_instruction, create_mint_to_instruction},
    TokenData,
};
use light_hasher::Poseidon;
use light_indexed_merkle_tree::array::IndexedArray;

use crate::{
    create_account_instruction, create_and_send_transaction,
    create_and_send_transaction_with_event, get_hash_set, AccountZeroCopy,
};

#[derive(Debug)]
pub struct ProofRpcResult {
    pub proof: CompressedProof,
    pub root_indices: Vec<u16>,
    pub address_root_indices: Vec<u16>,
}

#[derive(Debug)]
pub struct TestIndexer {
    pub address_merkle_tree_pubkey: Pubkey,
    pub merkle_tree_pubkey: Pubkey,
    pub nullifier_queue_pubkey: Pubkey,
    pub payer: Keypair,
    pub compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub nullified_compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub token_compressed_accounts: Vec<TokenDataWithContext>,
    pub token_nullified_compressed_accounts: Vec<TokenDataWithContext>,
    pub events: Vec<PublicTransactionEvent>,
    pub merkle_tree: light_merkle_tree_reference::MerkleTree<Poseidon>,
    pub address_merkle_tree:
        light_indexed_merkle_tree::reference::IndexedMerkleTree<Poseidon, usize>,
    pub indexing_array: IndexedArray<Poseidon, usize, 1000>,
}

#[derive(Debug, Clone)]
pub struct TokenDataWithContext {
    pub index: usize,
    pub token_data: TokenData,
}

impl TestIndexer {
    pub async fn new(
        merkle_tree_pubkey: Pubkey,
        nullifier_queue_pubkey: Pubkey,
        address_merkle_tree_pubkey: Pubkey,
        payer: Keypair,
    ) -> Self {
        // TODO: add path to gnark bin as parameter
        // we should have a release and download the binary to target
        spawn_prover().await;

        let merkle_tree = light_merkle_tree_reference::MerkleTree::<Poseidon>::new(
            STATE_MERKLE_TREE_HEIGHT as usize,
            STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
        );

        let mut address_merkle_tree = light_indexed_merkle_tree::reference::IndexedMerkleTree::new(
            STATE_MERKLE_TREE_HEIGHT as usize,
            STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
        )
        .unwrap();
        let mut indexed_array = IndexedArray::<Poseidon, usize, 1000>::default();

        let init_value = BigUint::from_str_radix(
            &"21888242871839275222246405745257275088548364400416034343698204186575808495616",
            10,
        )
        .unwrap();
        address_merkle_tree
            .append(&init_value, &mut indexed_array)
            .unwrap();
        Self {
            merkle_tree_pubkey,
            nullifier_queue_pubkey,
            address_merkle_tree_pubkey,
            payer,
            compressed_accounts: vec![],
            nullified_compressed_accounts: vec![],
            events: vec![],
            merkle_tree,
            address_merkle_tree,
            indexing_array: indexed_array,
            token_compressed_accounts: vec![],
            token_nullified_compressed_accounts: vec![],
        }
    }

    pub async fn create_proof_for_compressed_accounts(
        &mut self,
        compressed_accounts: Option<&[[u8; 32]]>,
        new_addresses: Option<&[[u8; 32]]>,
        context: &mut ProgramTestContext,
    ) -> ProofRpcResult {
        println!("compressed_accounts {:?}", compressed_accounts);
        println!("new_addresses {:?}", new_addresses);
        println!("self.merkle_tree.root() {:?}", self.merkle_tree.root());

        let (root_indices, address_root_indices, json_payload) =
            match (compressed_accounts, new_addresses) {
                (Some(accounts), None) => {
                    let (payload, indices) = self.process_inclusion_proofs(accounts, context).await;
                    (indices, Vec::new(), payload.to_string())
                }
                (None, Some(addresses)) => {
                    let (payload, indices) =
                        self.process_non_inclusion_proofs(addresses, context).await;
                    (Vec::<u16>::new(), indices, payload.to_string())
                }
                (Some(accounts), Some(addresses)) => {
                    let (inclusion_payload, inclusion_indices) =
                        self.process_inclusion_proofs(accounts, context).await;
                    let (non_inclusion_payload, non_inclusion_indices) =
                        self.process_non_inclusion_proofs(addresses, context).await;

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

        let client = Client::new();
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
        }
        println!(
            "response_result {:?}",
            response_result.text().await.unwrap()
        );
        panic!("Failed to get proof from server");
    }

    async fn process_inclusion_proofs(
        &self,
        accounts: &[[u8; 32]],
        context: &mut ProgramTestContext,
    ) -> (BatchInclusionJsonStruct, Vec<u16>) {
        let mut inclusion_proofs = Vec::<InclusionMerkleProofInputs>::new();
        for compressed_account in accounts.iter() {
            let leaf_index = self.merkle_tree.get_leaf_index(compressed_account).unwrap();
            let proof = self
                .merkle_tree
                .get_proof_of_leaf(leaf_index, true)
                .unwrap();
            inclusion_proofs.push(InclusionMerkleProofInputs {
                root: BigInt::from_be_bytes(self.merkle_tree.root().as_slice()),
                leaf: BigInt::from_be_bytes(compressed_account),
                path_index: BigInt::from_be_bytes(leaf_index.to_be_bytes().as_slice()),
                path_elements: proof.iter().map(|x| BigInt::from_be_bytes(x)).collect(),
            });
        }

        let inclusion_proof_inputs = InclusionProofInputs(inclusion_proofs.as_slice());
        let batch_inclusion_proof_inputs =
            BatchInclusionJsonStruct::from_inclusion_proof_inputs(&inclusion_proof_inputs);

        let merkle_tree_account =
            AccountZeroCopy::<StateMerkleTreeAccount>::new(context, self.merkle_tree_pubkey).await;
        let merkle_tree = merkle_tree_account
            .deserialized()
            .copy_merkle_tree()
            .unwrap();
        assert_eq!(
            self.merkle_tree.root(),
            merkle_tree.root().unwrap(),
            "Merkle tree root mismatch"
        );
        let root_indices = vec![merkle_tree.current_root_index as u16; accounts.len()];
        (batch_inclusion_proof_inputs, root_indices)
    }

    async fn process_non_inclusion_proofs(
        &self,
        addresses: &[[u8; 32]],
        context: &mut ProgramTestContext,
    ) -> (BatchNonInclusionJsonStruct, Vec<u16>) {
        let mut non_inclusion_proofs = Vec::<NonInclusionMerkleProofInputs>::new();

        for address in addresses.iter() {
            let proof_inputs = get_non_inclusion_proof_inputs(
                address,
                &self.address_merkle_tree,
                &self.indexing_array,
            );
            non_inclusion_proofs.push(proof_inputs);
        }

        let non_inclusion_proof_inputs = NonInclusionProofInputs(non_inclusion_proofs.as_slice());
        let batch_non_inclusion_proof_inputs =
            BatchNonInclusionJsonStruct::from_non_inclusion_proof_inputs(
                &non_inclusion_proof_inputs,
            );

        let merkle_tree_account = AccountZeroCopy::<AddressMerkleTreeAccount>::new(
            context,
            self.address_merkle_tree_pubkey,
        )
        .await;
        let address_merkle_tree = merkle_tree_account
            .deserialized()
            .copy_merkle_tree()
            .unwrap();
        let address_root_indices =
            vec![address_merkle_tree.current_root_index as u16; addresses.len()];

        (batch_non_inclusion_proof_inputs, address_root_indices)
    }

    /// deserializes an event
    /// adds the output_compressed_accounts to the compressed_accounts
    /// removes the input_compressed_accounts from the compressed_accounts
    /// adds the input_compressed_accounts to the nullified_compressed_accounts
    pub fn add_lamport_compressed_accounts(&mut self, event_bytes: Vec<u8>) {
        let event_bytes = event_bytes.clone();
        let event = PublicTransactionEvent::deserialize(&mut event_bytes.as_slice()).unwrap();
        self.add_event_and_compressed_accounts(event);
    }

    pub fn add_event_and_compressed_accounts(
        &mut self,
        event: PublicTransactionEvent,
    ) -> Vec<usize> {
        for compressed_account in event.input_compressed_accounts.iter() {
            let index = self
                .compressed_accounts
                .iter()
                .position(|x| x.compressed_account == compressed_account.compressed_account)
                .expect("compressed_account not found");
            self.compressed_accounts.remove(index);
            let token_compressed_account_element = self
                .token_compressed_accounts
                .iter()
                .find(|x| x.index == index);
            if token_compressed_account_element.is_some() {
                let token_compressed_account_element =
                    token_compressed_account_element.unwrap().clone();
                self.token_compressed_accounts.remove(index);
                self.token_nullified_compressed_accounts
                    .push(token_compressed_account_element);
            }
            // TODO: nullify compressed_account in Merkle tree, not implemented yet
            self.nullified_compressed_accounts
                .push(compressed_account.clone());
            let index = self
                .compressed_accounts
                .iter()
                .position(|x| x == compressed_account);
            if let Some(index) = index {
                let token_compressed_account_element =
                    self.token_compressed_accounts[index].clone();
                self.token_compressed_accounts.remove(index);
                self.token_nullified_compressed_accounts
                    .push(token_compressed_account_element);
            }
        }
        let mut indices = Vec::with_capacity(event.output_compressed_accounts.len());
        for (i, compressed_account) in event.output_compressed_accounts.iter().enumerate() {
            self.compressed_accounts
                .push(CompressedAccountWithMerkleContext {
                    compressed_account: compressed_account.clone(),
                    leaf_index: event.output_leaf_indices[i],
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 0,
                });
            indices.push(self.compressed_accounts.len() - 1);
            self.merkle_tree
                .append(
                    &compressed_account
                        .hash(&self.merkle_tree_pubkey, &event.output_leaf_indices[i])
                        .unwrap(),
                )
                .expect("insert failed");
        }

        self.events.push(event);
        indices
    }

    /// deserializes an event
    /// adds the output_compressed_accounts to the compressed_accounts
    /// removes the input_compressed_accounts from the compressed_accounts
    /// adds the input_compressed_accounts to the nullified_compressed_accounts
    /// deserializes token data from the output_compressed_accounts
    /// adds the token_compressed_accounts to the token_compressed_accounts
    pub fn add_compressed_accounts_with_token_data(&mut self, event: PublicTransactionEvent) {
        let indices = self.add_event_and_compressed_accounts(event);
        for index in indices.iter() {
            let data = self.compressed_accounts[*index]
                .compressed_account
                .data
                .as_ref()
                .unwrap();
            let token_data = TokenData::deserialize(&mut data.data.as_slice());
            match token_data {
                Ok(token_data) => {
                    self.token_compressed_accounts.push(TokenDataWithContext {
                        index: *index,
                        token_data,
                    });
                }
                Err(_) => {}
            }
        }
    }

    /// Check compressed_accounts in the queue array which are not nullified yet
    /// Iterate over these compressed_accounts and nullify them
    pub async fn nullify_compressed_accounts(&mut self, context: &mut ProgramTestContext) {
        let nullifier_queue = unsafe {
            get_hash_set::<u16, account_compression::NullifierQueueAccount>(
                context,
                self.nullifier_queue_pubkey,
            )
            .await
        };
        let merkle_tree_account =
            AccountZeroCopy::<StateMerkleTreeAccount>::new(context, self.merkle_tree_pubkey).await;
        let merkle_tree = merkle_tree_account
            .deserialized()
            .copy_merkle_tree()
            .unwrap();
        let change_log_index = merkle_tree.current_changelog_index as u64;

        let mut compressed_account_to_nullify = Vec::new();

        for (i, element) in nullifier_queue.iter() {
            if element.sequence_number().is_none() {
                compressed_account_to_nullify.push((i, element.value_bytes()));
            }
        }

        for (index_in_nullifier_queue, compressed_account) in compressed_account_to_nullify.iter() {
            let leaf_index = self.merkle_tree.get_leaf_index(compressed_account).unwrap();
            let proof: Vec<[u8; 32]> = self
                .merkle_tree
                .get_proof_of_leaf(leaf_index, false)
                .unwrap()
                .to_array::<16>()
                .unwrap()
                .to_vec();

            let instructions = [
                account_compression::nullify_leaves::sdk_nullify::create_nullify_instruction(
                    vec![change_log_index].as_slice(),
                    vec![(*index_in_nullifier_queue) as u16].as_slice(),
                    vec![0u64].as_slice(),
                    vec![proof].as_slice(),
                    &context.payer.pubkey(),
                    &self.merkle_tree_pubkey,
                    &self.nullifier_queue_pubkey,
                ),
            ];

            create_and_send_transaction(
                context,
                &instructions,
                &self.payer.pubkey(),
                &[&self.payer],
            )
            .await
            .unwrap();

            let nullifier_queue = unsafe {
                get_hash_set::<u16, account_compression::NullifierQueueAccount>(
                    context,
                    self.nullifier_queue_pubkey,
                )
                .await
            };
            let array_element = nullifier_queue
                .by_value_index(*index_in_nullifier_queue, Some(merkle_tree.sequence_number))
                .unwrap();
            assert_eq!(&array_element.value_bytes(), compressed_account);
            let merkle_tree_account =
                AccountZeroCopy::<StateMerkleTreeAccount>::new(context, self.merkle_tree_pubkey)
                    .await;
            assert_eq!(
                array_element.sequence_number(),
                Some(
                    merkle_tree_account
                        .deserialized()
                        .load_merkle_tree()
                        .unwrap()
                        .sequence_number
                        + account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS as usize
                )
            );
        }
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
        anchor_spl::token::Mint::LEN,
        rent,
        &anchor_spl::token::ID,
        Some(mint_keypair),
    );

    let mint_pubkey = mint_keypair.pubkey();
    let mint_authority = get_token_authority_pda(authority, &mint_pubkey);
    let create_mint_instruction = initialize_mint(
        &anchor_spl::token::ID,
        &mint_keypair.pubkey(),
        &mint_authority,
        None,
        decimals,
    )
    .unwrap();
    let transfer_ix =
        anchor_lang::solana_program::system_instruction::transfer(payer, &mint_pubkey, rent);

    let instruction = create_initialize_mint_instruction(payer, authority, &mint_pubkey);
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

pub async fn create_mint_helper(context: &mut ProgramTestContext, payer: &Keypair) -> Pubkey {
    let payer_pubkey = payer.pubkey();
    let rent = context
        .banks_client
        .get_rent()
        .await
        .unwrap()
        .minimum_balance(anchor_spl::token::Mint::LEN);
    let mint = Keypair::new();

    let (instructions, _): ([Instruction; 4], Pubkey) =
        create_initialize_mint_instructions(&payer_pubkey, &payer_pubkey, rent, 2, &mint);

    create_and_send_transaction(context, &instructions, &payer_pubkey, &[&payer, &mint])
        .await
        .unwrap();

    mint.pubkey()
}

pub async fn mint_tokens_helper(
    context: &mut ProgramTestContext,
    test_indexer: &mut TestIndexer,
    merkle_tree_pubkey: &Pubkey,
    mint_authority: &Keypair,
    mint: &Pubkey,
    amounts: Vec<u64>,
    recipients: Vec<Pubkey>,
) {
    let payer_pubkey = mint_authority.pubkey();
    let instruction = create_mint_to_instruction(
        &payer_pubkey,
        &payer_pubkey,
        mint,
        merkle_tree_pubkey,
        amounts,
        recipients,
    );
    let event = create_and_send_transaction_with_event::<PublicTransactionEvent>(
        context,
        &[instruction],
        &payer_pubkey,
        &[&mint_authority],
    )
    .await
    .unwrap()
    .unwrap();
    test_indexer.add_compressed_accounts_with_token_data(event);
}
