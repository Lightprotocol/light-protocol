// Flow:
// init indexer
// init first keypair
// init crank
// vec of public Merkle tree NF queue pairs
// vec of public address Mt and queue pairs
// for i in rounds
//   randomly add new keypair
// for every keypair randomly select whether it does an action

// Architecture:
// - bundle trees, indexer etc in a E2ETestEnv struct
// - methods:
// 	// bundles all general actions
//   - activate general actions
//   // bundles all keypair actions
//   - activate keypair actions
// 	// calls general and keypair actions
//   - execute round
//   // every action takes a probability as input
//   // if you want to execute the action on purpose pass 1
//   - method for every action
//  - add action activation config with default configs
//    - all enabled
//    - only spl, only sol, etc
//  Forester struct
//  - payer keypair, authority keypair
//  -methods
//   - empty nullifier queue
//   - empty address queue
//   - rollover Merkle tree
//   - rollover address Merkle tree

// keypair actions:
// safeguard every action in case of no balance
// 1. compress sol
// 2. decompress sol
// 2. transfer sol
// 3. compress spl
// 4. decompress spl
// 5. mint spl
// 6. transfer spl

// general actions:
// add keypair
// create new state Mt
// create new address Mt

// extension:
// keypair actions:
// - create pda
// - escrow tokens
// - delegate, revoke, delegated transaction

// general actions:
// - create new program owned state Merkle tree and queue
// - create new program owned address Merkle tree and queue

// minimal start
// struct with env and test-indexer
// only spl transactions

// second pr
// refactor sol tests to functions that can be reused

use std::collections::HashMap;

use account_compression::{
    utils::constants::{STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT},
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeConfig,
    SAFETY_MARGIN,
};
use anchor_lang::{prelude::AccountMeta, AnchorSerialize, Discriminator};
use create_address_test_program::create_invoke_cpi_instruction;
use forester_utils::{
    account_zero_copy::AccountZeroCopy,
    address_merkle_tree_config::{address_tree_ready_for_rollover, state_tree_ready_for_rollover},
    forester_epoch::{Epoch, Forester, TreeAccounts},
    utils::airdrop_lamports,
};
use light_batched_merkle_tree::{
    batch::BatchState,
    constants::{DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, TEST_DEFAULT_BATCH_SIZE},
    merkle_tree::{BatchedMerkleTreeAccount, InstructionDataBatchNullifyInputs},
    queue::BatchedQueueAccount,
};
use light_client::{
    fee::{FeeConfig, TransactionParams},
    indexer::{AddressMerkleTreeAccounts, AddressWithTree, Indexer, StateMerkleTreeAccounts},
    rpc::{errors::RpcError, merkle_tree::MerkleTreeExt, Rpc},
};
// TODO: implement traits for context object and indexer that we can implement with an rpc as well
// context trait: send_transaction -> return transaction result, get_account_info -> return account info
// indexer trait: get_compressed_accounts_by_owner -> return compressed accounts,
// refactor all tests to work with that so that we can run all tests with a test validator and concurrency
use light_compressed_account::{
    address::{
        derive_address,
        // pack_new_address_params, pack_read_only_accounts,
        // pack_read_only_address_params,
    },
    compressed_account::{
        // pack_compressed_accounts, pack_output_compressed_accounts,
        CompressedAccount,
        CompressedAccountData,
        CompressedAccountWithMerkleContext,

        ReadOnlyCompressedAccount,
    },
    instruction_data::{
        compressed_proof::CompressedProof,
        cpi_context::CompressedCpiContext,
        data::{NewAddressParams, ReadOnlyAddress},
        invoke_cpi::InstructionDataInvokeCpi,
        with_readonly::InstructionDataInvokeCpiWithReadOnly,
    },
    TreeType,
};
use light_compressed_token::process_transfer::transfer_sdk::to_account_metas;
use light_hasher::{bigint::bigint_to_be_bytes_array, Poseidon};
use light_indexed_merkle_tree::{
    array::IndexedArray, reference::IndexedMerkleTree, HIGHEST_ADDRESS_PLUS_ONE,
};
use light_program_test::{
    accounts::{
        state_tree::create_state_merkle_tree_and_queue_account, test_accounts::TestAccounts,
    },
    indexer::{
        address_tree::AddressMerkleTreeBundle, state_tree::StateMerkleTreeBundle, TestIndexer,
        TestIndexerExtensions,
    },
    program_test::{LightProgramTest, TestRpc},
    utils::register_test_forester::register_test_forester,
};
use light_prover_client::{
    constants::{PROVE_PATH, SERVER_ADDRESS},
    proof::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    proof_types::batch_address_append::{get_batch_address_append_circuit_inputs, to_json},
};
use light_registry::{
    account_compression_cpi::sdk::create_batch_update_address_tree_instruction,
    protocol_config::state::{ProtocolConfig, ProtocolConfigPda},
    sdk::create_finalize_registration_instruction,
    utils::get_protocol_config_pda_address,
    ForesterConfig,
};
use light_sdk::{
    address::NewAddressParamsAssignedPacked,
    constants::{ADDRESS_MERKLE_TREE_ROOTS, CPI_AUTHORITY_PDA_SEED, STATE_MERKLE_TREE_ROOTS},
    token::{AccountState, TokenDataWithMerkleContext},
};
use light_sparse_merkle_tree::{
    changelog::ChangelogEntry, indexed_changelog::IndexedChangelogEntry, SparseMerkleTree,
};
use log::info;
use num_bigint::{BigUint, RandBigInt};
use num_traits::Num;
use rand::{
    distributions::uniform::{SampleRange, SampleUniform},
    prelude::SliceRandom,
    rngs::{StdRng, ThreadRng},
    Rng, RngCore, SeedableRng,
};
use reqwest::Client;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::{SeedDerivable, Signer},
};
use spl_token::solana_program::native_token::LAMPORTS_PER_SOL;

use crate::{
    address_tree_rollover::{
        assert_rolled_over_address_merkle_tree_and_queue,
        perform_address_merkle_tree_roll_over_forester,
        perform_state_merkle_tree_roll_over_forester,
    },
    assert_epoch::{
        assert_finalized_epoch_registration, assert_report_work, fetch_epoch_and_forester_pdas,
    },
    create_address_merkle_tree_and_queue_account_with_assert,
    pack::*,
    spl::{
        approve_test, burn_test, compress_test, compressed_transfer_test, create_mint_helper,
        create_token_account, decompress_test, freeze_test, mint_tokens_helper, revoke_test,
        thaw_test,
    },
    state_tree_rollover::assert_rolled_over_pair,
    system_program::{
        compress_sol_test, create_addresses_test, decompress_sol_test, transfer_compressed_sol_test,
    },
    test_batch_forester::{perform_batch_append, perform_batch_nullify},
    test_forester::{empty_address_queue_test, nullify_compressed_accounts},
};

pub struct User {
    pub keypair: Keypair,
    // Vector of (mint, token account)
    pub token_accounts: Vec<(Pubkey, Pubkey)>,
}

#[derive(Debug, Default)]
pub struct Stats {
    pub spl_transfers: u64,
    pub mints: u64,
    pub spl_decompress: u64,
    pub spl_compress: u64,
    pub sol_transfers: u64,
    pub sol_decompress: u64,
    pub sol_compress: u64,
    pub create_address: u64,
    pub create_pda: u64,
    pub create_state_mt: u64,
    pub create_address_mt: u64,
    pub rolledover_state_trees: u64,
    pub rolledover_address_trees: u64,
    pub spl_approved: u64,
    pub spl_revoked: u64,
    pub spl_burned: u64,
    pub spl_frozen: u64,
    pub spl_thawed: u64,
    pub registered_foresters: u64,
    pub created_foresters: u64,
    pub work_reported: u64,
    pub finalized_registrations: u64,
}

impl Stats {
    pub fn print(&self, users: u64) {
        println!("Stats:");
        println!("Users {}", users);
        println!("Mints {}", self.mints);
        println!("Spl transfers {}", self.spl_transfers);
        println!("Spl decompress {}", self.spl_decompress);
        println!("Spl compress {}", self.spl_compress);
        println!("Sol transfers {}", self.sol_transfers);
        println!("Sol decompress {}", self.sol_decompress);
        println!("Sol compress {}", self.sol_compress);
        println!("Create address {}", self.create_address);
        println!("Create pda {}", self.create_pda);
        println!("Create state mt {}", self.create_state_mt);
        println!("Create address mt {}", self.create_address_mt);
        println!("Rolled over state trees {}", self.rolledover_state_trees);
        println!(
            "Rolled over address trees {}",
            self.rolledover_address_trees
        );
        println!("Spl approved {}", self.spl_approved);
        println!("Spl revoked {}", self.spl_revoked);
        println!("Spl burned {}", self.spl_burned);
        println!("Spl frozen {}", self.spl_frozen);
        println!("Spl thawed {}", self.spl_thawed);
        println!("Registered foresters {}", self.registered_foresters);
        println!("Created foresters {}", self.created_foresters);
        println!("Work reported {}", self.work_reported);
        println!("Finalized registrations {}", self.finalized_registrations);
    }
}

pub async fn init_program_test_env<R: Rpc + MerkleTreeExt + Indexer + TestRpc>(
    rpc: R,
    test_accounts: &TestAccounts,
    batch_size: usize,
) -> E2ETestEnv<R, TestIndexer> {
    let indexer: TestIndexer = TestIndexer::init_from_acounts(
        &test_accounts.protocol.forester.insecure_clone(),
        test_accounts,
        batch_size,
    )
    .await;

    E2ETestEnv::<R, TestIndexer>::new(
        rpc,
        indexer,
        test_accounts,
        KeypairActionConfig::all_default(),
        GeneralActionConfig::default(),
        10,
        None,
    )
    .await
}

pub async fn init_program_test_env_forester(
    rpc: LightProgramTest,
    test_accounts: &TestAccounts,
    batch_size: usize,
) -> E2ETestEnv<LightProgramTest, TestIndexer> {
    let indexer = TestIndexer::init_from_acounts(
        &test_accounts.protocol.forester.insecure_clone(),
        test_accounts,
        batch_size,
    )
    .await;

    E2ETestEnv::<LightProgramTest, TestIndexer>::new(
        rpc,
        indexer,
        test_accounts,
        KeypairActionConfig::all_default(),
        GeneralActionConfig::default(),
        10,
        None,
    )
    .await
}

#[derive(Debug, PartialEq)]
pub struct TestForester {
    keypair: Keypair,
    forester: Forester,
    is_registered: Option<u64>,
}

pub struct E2ETestEnv<R: Rpc + TestRpc + Indexer, I: Indexer + Clone + TestIndexerExtensions> {
    pub payer: Keypair,
    pub governance_keypair: Keypair,
    pub indexer: I,
    pub users: Vec<User>,
    pub mints: Vec<Pubkey>,
    pub foresters: Vec<TestForester>,
    pub rpc: R,
    pub keypair_action_config: KeypairActionConfig,
    pub general_action_config: GeneralActionConfig,
    pub round: u64,
    pub rounds: u64,
    pub rng: StdRng,
    pub stats: Stats,
    pub epoch: u64,
    pub slot: u64,
    /// Forester struct is reused but not used for foresting here
    /// Epoch config keeps track of the ongong epochs.
    pub epoch_config: Forester,
    pub protocol_config: ProtocolConfig,
    pub registration_epoch: u64,
}

impl<R: Rpc + TestRpc + Indexer, I: Indexer + Clone + TestIndexerExtensions> E2ETestEnv<R, I>
where
    R: Rpc,
    I: Indexer,
{
    pub async fn new(
        mut rpc: R,
        mut indexer: I,
        test_accounts: &TestAccounts,
        keypair_action_config: KeypairActionConfig,
        general_action_config: GeneralActionConfig,
        rounds: u64,
        seed: Option<u64>,
    ) -> Self {
        let payer = rpc.get_payer().insecure_clone();

        airdrop_lamports(&mut rpc, &payer.pubkey(), 1_000_000_000_000)
            .await
            .unwrap();

        airdrop_lamports(
            &mut rpc,
            &test_accounts.protocol.forester.pubkey(),
            1_000_000_000_000,
        )
        .await
        .unwrap();
        let mut thread_rng = ThreadRng::default();
        let random_seed = thread_rng.next_u64();
        let seed: u64 = seed.unwrap_or(random_seed);
        // Keep this print so that in case the test fails
        // we can use the seed to reproduce the error.
        println!("\n\ne2e test seed {}\n\n", seed);
        let mut rng = StdRng::seed_from_u64(seed);
        let user = Self::create_user(&mut rng, &mut rpc).await;
        let mint = create_mint_helper(&mut rpc, &payer).await;
        mint_tokens_helper(
            &mut rpc,
            &mut indexer,
            &test_accounts.v1_state_trees[0].merkle_tree,
            &payer,
            &mint,
            vec![100_000_000; 1],
            vec![user.keypair.pubkey()],
        )
        .await;
        let protocol_config_pda_address = get_protocol_config_pda_address().0;
        println!("here");
        let protocol_config = rpc
            .get_anchor_account::<ProtocolConfigPda>(&protocol_config_pda_address)
            .await
            .unwrap()
            .unwrap()
            .config;
        // TODO: add clear test env enum
        // register foresters is only compatible with ProgramTest environment
        // Default forester setup - tests that need forester functionality should call
        // setup_forester_and_advance_to_epoch explicitly and manage their own forester state
        let (foresters, epoch_config) = (Vec::<TestForester>::new(), Forester::default());
        Self {
            payer,
            indexer,
            users: vec![user],
            rpc,
            keypair_action_config,
            general_action_config,
            round: 0,
            rounds,
            rng,
            mints: vec![],
            stats: Stats::default(),
            foresters,
            registration_epoch: 0,
            epoch: 0,
            slot: 0,
            epoch_config,
            protocol_config,
            governance_keypair: test_accounts.protocol.governance_authority.insecure_clone(),
        }
    }

    /// Creates a new user with a random keypair and 100 sol
    pub async fn create_user(rng: &mut StdRng, rpc: &mut R) -> User {
        let keypair: Keypair = Keypair::from_seed(&[rng.gen_range(0..255); 32]).unwrap();

        rpc.airdrop_lamports(&keypair.pubkey(), LAMPORTS_PER_SOL * 5000)
            .await
            .unwrap();
        User {
            keypair,
            token_accounts: vec![],
        }
    }

    pub async fn get_balance(&self, pubkey: &Pubkey) -> u64 {
        self.rpc.get_balance(pubkey).await.unwrap()
    }

    pub async fn execute_rounds(&mut self) {
        for _ in 0..=self.rounds {
            self.execute_round().await;
        }
    }

    pub async fn execute_round(&mut self) {
        println!("\n------------------------------------------------------\n");
        println!("Round: {}", self.round);
        self.stats.print(self.users.len() as u64);

        // TODO: check at the beginning of the round that the Merkle trees are in sync
        let len = self.users.len();
        for i in 0..len {
            self.activate_keypair_actions(&self.users[i].keypair.pubkey())
                .await;
        }
        self.activate_general_actions().await;
        self.round += 1;
    }

    /// 1. Add a new keypair
    /// 2. Create a new state Merkle tree
    pub async fn activate_general_actions(&mut self) {
        // If we want to test rollovers we set the threshold to 0 for all newly created trees
        let rollover_threshold = if self.general_action_config.rollover.is_some() {
            Some(0)
        } else {
            None
        };
        if self
            .rng
            .gen_bool(self.general_action_config.add_keypair.unwrap_or_default())
        {
            let user = Self::create_user(&mut self.rng, &mut self.rpc).await;
            self.users.push(user);
        }

        if self.rng.gen_bool(
            self.general_action_config
                .create_state_mt
                .unwrap_or_default(),
        ) {
            println!("\n------------------------------------------------------\n");
            println!("Creating new state Merkle tree");
            self.create_state_tree(rollover_threshold).await;
            self.stats.create_state_mt += 1;
        }

        if self.rng.gen_bool(
            self.general_action_config
                .create_address_mt
                .unwrap_or_default(),
        ) {
            println!("\n------------------------------------------------------\n");
            println!("Creating new address Merkle tree");

            self.create_address_tree(rollover_threshold).await;
            self.stats.create_address_mt += 1;
        }

        if self.rng.gen_bool(
            self.general_action_config
                .nullify_compressed_accounts
                .unwrap_or_default(),
        ) {
            for state_tree_bundle in self.indexer.get_state_merkle_trees_mut().iter_mut() {
                println!("state tree bundle version {}", state_tree_bundle.tree_type);
                match state_tree_bundle.tree_type {
                    TreeType::StateV1 => {
                        println!("\n --------------------------------------------------\n\t\t NULLIFYING LEAVES v1\n --------------------------------------------------");
                        // find forester which is eligible this slot for this tree
                        if let Some(payer) = Self::get_eligible_forester_for_queue(
                            &state_tree_bundle.accounts.nullifier_queue,
                            &self.foresters,
                            self.slot,
                        ) {
                            // TODO: add newly addeded trees to foresters
                            nullify_compressed_accounts(
                                &mut self.rpc,
                                &payer,
                                state_tree_bundle,
                                self.epoch,
                                false,
                            )
                            .await
                            .unwrap();
                        } else {
                            println!("No forester found for nullifier queue");
                        };
                    }
                    TreeType::StateV2 => {
                        let merkle_tree_pubkey = state_tree_bundle.accounts.merkle_tree;
                        let queue_pubkey = state_tree_bundle.accounts.nullifier_queue;
                        // Check input queue
                        if let Some(payer) = Self::get_eligible_forester_for_queue(
                            &state_tree_bundle.accounts.merkle_tree,
                            &self.foresters,
                            self.slot,
                        ) {
                            let mut merkle_tree_account_data = self
                                .rpc
                                .get_account(merkle_tree_pubkey)
                                .await
                                .unwrap()
                                .unwrap()
                                .data;
                            let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
                                merkle_tree_account_data.as_mut_slice(),
                                &merkle_tree_pubkey.into(),
                            )
                            .unwrap();
                            let pending_batch_index = merkle_tree.queue_batches.pending_batch_index;
                            let batch = merkle_tree
                                .queue_batches
                                .batches
                                .get(pending_batch_index as usize)
                                .unwrap();
                            let batch_state = batch.get_state();
                            println!(
                                "output batch_state {:?}, {}, batch index {}",
                                batch_state,
                                batch.get_num_inserted_zkp_batch()
                                    + batch.get_current_zkp_batch_index() * batch.zkp_batch_size,
                                pending_batch_index
                            );
                            println!("input batch_state {:?}", batch_state);
                            if batch_state == BatchState::Full {
                                println!("\n --------------------------------------------------\n\t\t NULLIFYING LEAVES batched (v2)\n --------------------------------------------------");
                                for _ in 0..TEST_DEFAULT_BATCH_SIZE {
                                    perform_batch_nullify(
                                        &mut self.rpc,
                                        state_tree_bundle,
                                        &payer,
                                        self.epoch,
                                        false,
                                        None,
                                    )
                                    .await
                                    .unwrap();
                                }
                            }
                        }
                        // Check output queue
                        if let Some(payer) = Self::get_eligible_forester_for_queue(
                            &state_tree_bundle.accounts.merkle_tree,
                            &self.foresters,
                            self.slot,
                        ) {
                            println!("\n --------------------------------------------------\n\t\t Appending LEAVES batched (v2)\n --------------------------------------------------");
                            let mut queue_account_data = self
                                .rpc
                                .get_account(queue_pubkey)
                                .await
                                .unwrap()
                                .unwrap()
                                .data;

                            let output_queue = BatchedQueueAccount::output_from_bytes(
                                queue_account_data.as_mut_slice(),
                            )
                            .unwrap();
                            let pending_batch_index =
                                output_queue.batch_metadata.pending_batch_index;
                            let batch = output_queue
                                .batch_metadata
                                .batches
                                .get(pending_batch_index as usize)
                                .unwrap();
                            let batch_state = batch.get_state();
                            println!(
                                "output batch_state {:?}, {}, batch index {}",
                                batch_state,
                                batch.get_num_inserted_zkp_batch()
                                    + batch.get_current_zkp_batch_index() * batch.zkp_batch_size,
                                pending_batch_index
                            );
                            if batch_state == BatchState::Full {
                                for _ in 0..output_queue.batch_metadata.get_num_zkp_batches() {
                                    perform_batch_append(
                                        &mut self.rpc,
                                        state_tree_bundle,
                                        &payer,
                                        self.epoch,
                                        false,
                                        None,
                                    )
                                    .await
                                    .unwrap();
                                }
                            }
                        }
                    }
                    _ => {
                        println!("Version skipped {}", state_tree_bundle.tree_type);
                    }
                }
            }
        }

        if self.rng.gen_bool(
            self.general_action_config
                .empty_address_queue
                .unwrap_or_default(),
        ) {
            for address_merkle_tree_bundle in self
                .indexer
                .get_address_merkle_trees_mut()
                .iter_mut()
                .filter(|x| x.accounts.merkle_tree != x.accounts.queue)
                .collect::<Vec<_>>()
                .iter_mut()
            {
                // find forester which is eligible this slot for this tree
                if let Some(payer) = Self::get_eligible_forester_for_queue(
                    &address_merkle_tree_bundle.accounts.queue,
                    &self.foresters,
                    self.slot,
                ) {
                    println!("\n --------------------------------------------------\n\t\t Empty v1 Address Queue\n --------------------------------------------------");
                    println!("epoch {}", self.epoch);
                    println!("forester {}", payer.pubkey());
                    if address_merkle_tree_bundle.accounts.queue
                        != address_merkle_tree_bundle.accounts.merkle_tree
                    {
                        // TODO: add newly addeded trees to foresters
                        empty_address_queue_test(
                            &payer,
                            &mut self.rpc,
                            address_merkle_tree_bundle,
                            false,
                            self.epoch,
                            false,
                        )
                        .await
                        .unwrap();
                    }
                }
            }
            let pubkeys = self
                .indexer
                .get_address_merkle_trees_mut()
                .iter_mut()
                .filter(|x| x.accounts.merkle_tree == x.accounts.queue)
                .map(|x| x.accounts.merkle_tree)
                .collect::<Vec<_>>();
            for merkle_tree_pubkey in pubkeys {
                // find forester which is eligible this slot for this tree
                if let Some(payer) = Self::get_eligible_forester_for_queue(
                    &merkle_tree_pubkey,
                    &self.foresters,
                    self.slot,
                ) {
                    println!("\n --------------------------------------------------\n\t\t Empty Address Queue\n --------------------------------------------------");
                    println!("epoch {}", self.epoch);
                    println!("forester {}", payer.pubkey());

                    let mut merkle_tree_account_data = self
                        .rpc
                        .get_account(merkle_tree_pubkey)
                        .await
                        .unwrap()
                        .unwrap()
                        .data;
                    let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
                        merkle_tree_account_data.as_mut_slice(),
                        &merkle_tree_pubkey.into(),
                    )
                    .unwrap();
                    let pending_batch_index = merkle_tree.queue_batches.pending_batch_index;
                    let batch = merkle_tree
                        .queue_batches
                        .batches
                        .get(pending_batch_index as usize)
                        .unwrap();
                    let batch_state = batch.get_state();
                    println!(
                        "output batch_state {:?}, {}, batch index {}",
                        batch_state,
                        batch.get_num_inserted_zkp_batch()
                            + batch.get_current_zkp_batch_index() * batch.zkp_batch_size,
                        pending_batch_index
                    );
                    println!("input batch_state {:?}", batch_state);
                    if batch_state == BatchState::Full {
                        println!("\n --------------------------------------------------\n\t\t Empty Address Queue (v2)\n --------------------------------------------------");
                        let mut merkle_tree_account = self
                            .rpc
                            .get_account(merkle_tree_pubkey)
                            .await
                            .unwrap()
                            .unwrap();
                        let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
                            merkle_tree_account.data.as_mut_slice(),
                            &merkle_tree_pubkey.into(),
                        )
                        .unwrap();
                        for _ in 0..merkle_tree.queue_batches.get_num_zkp_batches() {
                            let instruction_data = {
                                let full_batch_index =
                                    merkle_tree.queue_batches.pending_batch_index;
                                let batch =
                                    &merkle_tree.queue_batches.batches[full_batch_index as usize];
                                let zkp_batch_index = batch.get_num_inserted_zkps();
                                let leaves_hash_chain = merkle_tree.hash_chain_stores
                                    [full_batch_index as usize]
                                    [zkp_batch_index as usize];

                                let addresses = self
                                    .indexer
                                    .get_queue_elements(
                                        merkle_tree_pubkey.to_bytes(),
                                        None,
                                        Some(batch.batch_size as u16),
                                        None,
                                        None,
                                        None,
                                    )
                                    .await
                                    .unwrap();
                                let addresses =
                                    addresses.value.output_queue_elements.unwrap_or_default().iter().map(|x| x.account_hash).collect::<Vec<_>>();
                                // // local_leaves_hash_chain is only used for a test assertion.
                                // let local_nullifier_hash_chain = create_hash_chain_from_array(&addresses);
                                // assert_eq!(leaves_hash_chain, local_nullifier_hash_chain);
                                let start_index = merkle_tree.next_index as usize;
                                assert!(
                                    start_index >= 2,
                                    "start index should be greater than 2 else tree is not inited"
                                );
                                let current_root = *merkle_tree.root_history.last().unwrap();
                                let mut low_element_values = Vec::new();
                                let mut low_element_indices = Vec::new();
                                let mut low_element_next_indices = Vec::new();
                                let mut low_element_next_values = Vec::new();
                                let mut low_element_proofs: Vec<Vec<[u8; 32]>> = Vec::new();
                                let non_inclusion_proofs = self
                                    .indexer
                                    .get_multiple_new_address_proofs(
                                        merkle_tree_pubkey.to_bytes(),
                                        addresses.clone(),
                                        None,
                                    )
                                    .await
                                    .unwrap();
                                for non_inclusion_proof in &non_inclusion_proofs.value.items {
                                    low_element_values.push(non_inclusion_proof.low_address_value);
                                    low_element_indices
                                        .push(non_inclusion_proof.low_address_index as usize);
                                    low_element_next_indices
                                        .push(non_inclusion_proof.low_address_next_index as usize);
                                    low_element_next_values
                                        .push(non_inclusion_proof.low_address_next_value);

                                    low_element_proofs
                                        .push(non_inclusion_proof.low_address_proof.to_vec());
                                }

                                let subtrees =   self.indexer
                                    .get_subtrees(merkle_tree_pubkey.to_bytes(), None)
                                    .await
                                    .unwrap();
                                let mut sparse_merkle_tree = SparseMerkleTree::<Poseidon, { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize }>::new(<[[u8; 32]; DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize]>::try_from(subtrees.value.items).unwrap(), start_index);

                                let mut changelog: Vec<ChangelogEntry<{ DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize }>> = Vec::new();
                                let mut indexed_changelog: Vec<IndexedChangelogEntry<usize, { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize }>> = Vec::new();

                                let inputs = get_batch_address_append_circuit_inputs::<
                                    { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize },
                                >(
                                    start_index,
                                    current_root,
                                    low_element_values,
                                    low_element_next_values,
                                    low_element_indices,
                                    low_element_next_indices,
                                    low_element_proofs,
                                    addresses,
                                    &mut sparse_merkle_tree,
                                    leaves_hash_chain,
                                    batch.zkp_batch_size as usize,
                                    &mut changelog,
                                    &mut indexed_changelog,
                                )
                                .unwrap();
                                let client = Client::new();
                                let circuit_inputs_new_root =
                                    bigint_to_be_bytes_array::<32>(&inputs.new_root).unwrap();
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
                                    let (proof_a, proof_b, proof_c) =
                                        proof_from_json_struct(proof_json);
                                    let (proof_a, proof_b, proof_c) =
                                        compress_proof(&proof_a, &proof_b, &proof_c);
                                    let instruction_data = InstructionDataBatchNullifyInputs {
                                        new_root: circuit_inputs_new_root,
                                        compressed_proof: CompressedProof {
                                            a: proof_a,
                                            b: proof_b,
                                            c: proof_c,
                                        },
                                    };
                                    Ok(instruction_data)
                                } else {
                                    Err(RpcError::CustomError(
                                        "Prover failed to generate proof".to_string(),
                                    ))
                                }
                            }
                            .unwrap();

                            let instruction = create_batch_update_address_tree_instruction(
                                payer.pubkey(),
                                payer.pubkey(),
                                merkle_tree_pubkey,
                                self.epoch,
                                instruction_data.try_to_vec().unwrap(),
                            );
                            self.rpc
                                .create_and_send_transaction(
                                    &[instruction],
                                    &payer.pubkey(),
                                    &[&payer],
                                )
                                .await
                                .unwrap();
                        }
                        let mut account = self
                            .rpc
                            .get_account(merkle_tree_pubkey)
                            .await
                            .unwrap()
                            .unwrap();
                        self.indexer
                            .finalize_batched_address_tree_update(
                                merkle_tree_pubkey,
                                account.data.as_mut_slice(),
                            )
                            .await;
                    }
                } else {
                    println!("No forester found for address queue");
                };
            }
        }

        for index in 0..self.indexer.get_state_merkle_trees().len() {
            let is_read_for_rollover = state_tree_ready_for_rollover(
                &mut self.rpc,
                self.indexer.get_state_merkle_trees()[index]
                    .accounts
                    .merkle_tree,
            )
            .await;
            if self
                .rng
                .gen_bool(self.general_action_config.rollover.unwrap_or_default())
                && is_read_for_rollover
            {
                println!("\n --------------------------------------------------\n\t\t Rollover State Merkle Tree\n --------------------------------------------------");
                // find forester which is eligible this slot for this tree
                if let Some(payer) = Self::get_eligible_forester_for_queue(
                    &self.indexer.get_state_merkle_trees()[index]
                        .accounts
                        .nullifier_queue,
                    &self.foresters,
                    self.slot,
                ) {
                    self.rollover_state_merkle_tree_and_queue(index, &payer, self.epoch)
                        .await
                        .unwrap();
                    self.stats.rolledover_state_trees += 1;
                }
            }
        }

        for index in 0..self
            .indexer
            .get_address_merkle_trees()
            .iter()
            .filter(|x| x.accounts.merkle_tree != x.accounts.queue)
            .collect::<Vec<_>>()
            .len()
        {
            let is_read_for_rollover = address_tree_ready_for_rollover(
                &mut self.rpc,
                self.indexer
                    .get_address_merkle_trees()
                    .iter()
                    .filter(|x| x.accounts.merkle_tree != x.accounts.queue)
                    .collect::<Vec<_>>()[index]
                    .accounts
                    .merkle_tree,
            )
            .await;
            if self
                .rng
                .gen_bool(self.general_action_config.rollover.unwrap_or_default())
                && is_read_for_rollover
            {
                // find forester which is eligible this slot for this tree
                if let Some(payer) = Self::get_eligible_forester_for_queue(
                    &self
                        .indexer
                        .get_address_merkle_trees()
                        .iter()
                        .filter(|x| x.accounts.merkle_tree != x.accounts.queue)
                        .collect::<Vec<_>>()[index]
                        .accounts
                        .queue,
                    &self.foresters,
                    self.slot,
                ) {
                    println!("\n --------------------------------------------------\n\t\t Rollover Address Merkle Tree\n --------------------------------------------------");
                    self.rollover_address_merkle_tree_and_queue(index, &payer, self.epoch)
                        .await
                        .unwrap();
                    self.stats.rolledover_address_trees += 1;
                }
            }
        }

        if self
            .rng
            .gen_bool(self.general_action_config.add_forester.unwrap_or_default())
        {
            println!("\n --------------------------------------------------\n\t\t Add Forester\n --------------------------------------------------");
            let forester = TestForester {
                keypair: Keypair::new(),
                forester: Forester::default(),
                is_registered: None,
            };
            let forester_config = ForesterConfig {
                fee: self.rng.gen_range(0..=100),
            };
            register_test_forester(
                &mut self.rpc,
                &self.governance_keypair,
                &forester.keypair.pubkey(),
                forester_config,
            )
            .await
            .unwrap();
            self.foresters.push(forester);
            self.stats.created_foresters += 1;
        }

        // advance to next light slot and perform forester epoch actions
        if !self.general_action_config.disable_epochs {
            println!("\n --------------------------------------------------\n\t\t Start Epoch Actions \n --------------------------------------------------");

            let current_solana_slot = self.rpc.get_slot().await.unwrap();
            let current_light_slot = self
                .protocol_config
                .get_current_active_epoch_progress(current_solana_slot)
                / self.protocol_config.slot_length;
            // If slot didn't change, advance to next slot
            // if current_light_slot != self.slot {
            let new_slot = current_solana_slot + self.protocol_config.slot_length;
            println!("advanced slot from {} to {}", self.slot, current_light_slot);
            println!("solana slot from {} to {}", current_solana_slot, new_slot);
            self.rpc.warp_to_slot(new_slot).unwrap();

            self.slot = current_light_slot + 1;

            let current_solana_slot = self.rpc.get_slot().await.unwrap();
            // need to detect whether new registration phase started
            let current_registration_epoch = self
                .protocol_config
                .get_latest_register_epoch(current_solana_slot)
                .unwrap();
            // If reached new registration phase register all foresters
            if current_registration_epoch != self.registration_epoch {
                println!("\n --------------------------------------------------\n\t\t Register Foresters for new Epoch \n --------------------------------------------------");

                self.registration_epoch = current_registration_epoch;
                println!("new register epoch {}", self.registration_epoch);
                println!("num foresters {}", self.foresters.len());
                for forester in self.foresters.iter_mut() {
                    println!(
                        "registered forester {} for epoch {}",
                        forester.keypair.pubkey(),
                        self.registration_epoch
                    );

                    let registered_epoch = Epoch::register(
                        &mut self.rpc,
                        &self.protocol_config,
                        &forester.keypair,
                        &forester.keypair.pubkey(),
                    )
                    .await
                    .unwrap()
                    .unwrap();
                    println!("registered_epoch {:?}", registered_epoch.phases);
                    forester.forester.registration = registered_epoch;
                    if forester.is_registered.is_none() {
                        forester.is_registered = Some(self.registration_epoch);
                    }
                    self.stats.registered_foresters += 1;
                }
            }

            let current_active_epoch = self
                .protocol_config
                .get_current_active_epoch(current_solana_slot)
                .unwrap();
            // If reached new active epoch
            // 1. move epoch in every forester to report work epoch
            // 2. report work for every forester
            // 3. finalize registration for every forester
            #[allow(clippy::comparison_chain)]
            if current_active_epoch > self.epoch {
                self.slot = current_light_slot;
                self.epoch = current_active_epoch;
                // 1. move epoch in every forester to report work epoch
                for forester in self.foresters.iter_mut() {
                    if forester.is_registered.is_none() {
                        continue;
                    }
                    forester.forester.switch_to_report_work();
                }
                println!("\n --------------------------------------------------\n\t\t Report Work \n --------------------------------------------------");

                // 2. report work for every forester
                for forester in self.foresters.iter_mut() {
                    if forester.is_registered.is_none() {
                        continue;
                    }
                    println!("report work for forester {}", forester.keypair.pubkey());
                    println!(
                        "forester.forester.report_work.forester_epoch_pda {}",
                        forester.forester.report_work.forester_epoch_pda
                    );
                    println!(
                        "forester.forester.report_work.epoch_pda {}",
                        forester.forester.report_work.epoch_pda
                    );

                    let (pre_forester_epoch_pda, pre_epoch_pda) = fetch_epoch_and_forester_pdas(
                        &mut self.rpc,
                        &forester.forester.report_work.forester_epoch_pda,
                        &forester.forester.report_work.epoch_pda,
                    )
                    .await;
                    forester
                        .forester
                        .report_work(&mut self.rpc, &forester.keypair, &forester.keypair.pubkey())
                        .await
                        .unwrap();
                    println!("reported work");
                    assert_report_work(
                        &mut self.rpc,
                        &forester.forester.report_work.forester_epoch_pda,
                        &forester.forester.report_work.epoch_pda,
                        pre_forester_epoch_pda,
                        pre_epoch_pda,
                    )
                    .await;
                    self.stats.work_reported += 1;
                }

                // 3. finalize registration for every forester
                println!("\n --------------------------------------------------\n\t\t Finalize Registration \n --------------------------------------------------");

                // 3.1 get tree accounts
                // TODO: use TreeAccounts in TestIndexer
                let mut tree_accounts = self
                    .indexer
                    .get_state_merkle_trees()
                    .iter()
                    .map(|state_merkle_tree_bundle| {
                        let tree_type = match state_merkle_tree_bundle.tree_type {
                            TreeType::StateV1 => TreeType::StateV1,
                            TreeType::StateV2 => TreeType::StateV2,
                            _ => {
                                panic!("unsupported version {}", state_merkle_tree_bundle.tree_type)
                            }
                        };

                        TreeAccounts {
                            tree_type,
                            merkle_tree: state_merkle_tree_bundle.accounts.merkle_tree,
                            queue: state_merkle_tree_bundle.accounts.nullifier_queue,
                            is_rolledover: false,
                        }
                    })
                    .collect::<Vec<TreeAccounts>>();
                self.indexer.get_address_merkle_trees().iter().for_each(
                    |address_merkle_tree_bundle| {
                        tree_accounts.push(TreeAccounts {
                            tree_type: TreeType::AddressV1,
                            merkle_tree: address_merkle_tree_bundle.accounts.merkle_tree,
                            queue: address_merkle_tree_bundle.accounts.queue,
                            is_rolledover: false,
                        });
                    },
                );
                // 3.2 finalize registration for every forester
                for forester in self.foresters.iter_mut() {
                    if forester.is_registered.is_none() {
                        continue;
                    }
                    println!(
                        "registered forester {} for epoch {}",
                        forester.keypair.pubkey(),
                        self.epoch
                    );
                    println!(
                        "forester.forester registration epoch {:?}",
                        forester.forester.registration.epoch
                    );
                    println!(
                        "forester.forester active epoch {:?}",
                        forester.forester.active.epoch
                    );
                    println!(
                        "forester.forester report_work epoch {:?}",
                        forester.forester.report_work.epoch
                    );

                    forester
                        .forester
                        .active
                        .fetch_account_and_add_trees_with_schedule(&mut self.rpc, &tree_accounts)
                        .await
                        .unwrap();
                    let ix = create_finalize_registration_instruction(
                        &forester.keypair.pubkey(),
                        &forester.keypair.pubkey(),
                        forester.forester.active.epoch,
                    );
                    self.rpc
                        .create_and_send_transaction(
                            &[ix],
                            &forester.keypair.pubkey(),
                            &[&forester.keypair],
                        )
                        .await
                        .unwrap();
                    assert_finalized_epoch_registration(
                        &mut self.rpc,
                        &forester.forester.active.forester_epoch_pda,
                        &forester.forester.active.epoch_pda,
                    )
                    .await;
                    self.stats.finalized_registrations += 1;
                }
            } else if current_active_epoch < self.epoch {
                panic!(
                    "current_active_epoch {} is less than self.epoch {}",
                    current_active_epoch, self.epoch
                );
            }
        }
    }

    pub async fn create_state_tree(&mut self, rollover_threshold: Option<u64>) {
        let merkle_tree_keypair = Keypair::new(); //from_seed(&[self.rng.gen_range(0..255); 32]).unwrap();
        let nullifier_queue_keypair = Keypair::new(); //from_seed(&[self.rng.gen_range(0..255); 32]).unwrap();
        let cpi_context_keypair = Keypair::new();
        let rollover_threshold = if let Some(rollover_threshold) = rollover_threshold {
            Some(rollover_threshold)
        } else if self.rng.gen_bool(0.5) && !self.keypair_action_config.fee_assert {
            Some(self.rng.gen_range(1..100))
        } else {
            None
        };
        let merkle_tree_config = if !self.keypair_action_config.fee_assert {
            StateMerkleTreeConfig {
                height: 26,
                changelog_size: self.rng.gen_range(1..5000),
                roots_size: self.rng.gen_range(1..10000),
                canopy_depth: 10,
                network_fee: Some(5000),
                close_threshold: None,
                rollover_threshold,
            }
        } else {
            StateMerkleTreeConfig::default()
        };
        println!("merkle tree config: {:?}", merkle_tree_config);
        let queue_config = if !self.keypair_action_config.fee_assert {
            let capacity: u32 = self.rng.gen_range(1..10000);
            NullifierQueueConfig {
                capacity: capacity as u16,
                sequence_threshold: merkle_tree_config.roots_size + SAFETY_MARGIN,
                network_fee: None,
            }
        } else if rollover_threshold.is_some() {
            panic!("rollover_threshold should not be set when fee_assert is set (keypair_action_config.fee_assert)");
        } else {
            NullifierQueueConfig::default()
        };
        let forester = Pubkey::new_unique();
        println!("queue config: {:?}", queue_config);
        create_state_merkle_tree_and_queue_account(
            &self.payer,
            true,
            &mut self.rpc,
            &merkle_tree_keypair,
            &nullifier_queue_keypair,
            Some(&cpi_context_keypair),
            None,
            Some(forester),
            1,
            &merkle_tree_config,
            &queue_config,
        )
        .await
        .unwrap();
        let merkle_tree = Box::new(
            light_merkle_tree_reference::MerkleTree::<Poseidon>::new_with_history(
                STATE_MERKLE_TREE_HEIGHT as usize,
                STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
                0,
                STATE_MERKLE_TREE_ROOTS,
            ),
        );
        let state_tree_account = AccountZeroCopy::<account_compression::QueueAccount>::new(
            &mut self.rpc,
            nullifier_queue_keypair.pubkey(),
        )
        .await;
        self.indexer
            .get_state_merkle_trees_mut()
            .push(StateMerkleTreeBundle {
                rollover_fee: state_tree_account
                    .deserialized()
                    .metadata
                    .rollover_metadata
                    .rollover_fee as i64,
                accounts: StateMerkleTreeAccounts {
                    merkle_tree: merkle_tree_keypair.pubkey(),
                    nullifier_queue: nullifier_queue_keypair.pubkey(),
                    cpi_context: cpi_context_keypair.pubkey(),
                    tree_type: TreeType::StateV1,
                },
                tree_type: TreeType::StateV1,
                merkle_tree,
                output_queue_elements: vec![],
                input_leaf_indices: vec![],
                num_inserted_batches: 0,
                output_queue_batch_size: None,
            });
        // TODO: Add assert
    }

    /// Only supports v1 address trees.
    pub async fn create_address_tree(&mut self, rollover_threshold: Option<u64>) {
        let merkle_tree_keypair = Keypair::new();
        let nullifier_queue_keypair = Keypair::new();
        let rollover_threshold = if let Some(rollover_threshold) = rollover_threshold {
            Some(rollover_threshold)
        } else if self.rng.gen_bool(0.5) && !self.keypair_action_config.fee_assert {
            Some(self.rng.gen_range(1..100))
        } else {
            None
        };

        let (config, address_config) = if !self.keypair_action_config.fee_assert {
            let root_history = self.rng.gen_range(1..10000);
            (
                AddressMerkleTreeConfig {
                    height: 26,
                    changelog_size: self.rng.gen_range(1..5000),
                    roots_size: root_history,
                    canopy_depth: 10,
                    address_changelog_size: self.rng.gen_range(1..5000),
                    rollover_threshold,
                    network_fee: Some(5000),
                    close_threshold: None,
                    // TODO: double check that close threshold cannot be set
                },
                AddressQueueConfig {
                    sequence_threshold: root_history + SAFETY_MARGIN,
                    ..Default::default()
                },
            )
        } else if rollover_threshold.is_some() {
            panic!("rollover_threshold should not be set when fee_assert is set (keypair_action_config.fee_assert)");
        } else {
            (
                AddressMerkleTreeConfig::default(),
                AddressQueueConfig::default(),
            )
        };

        create_address_merkle_tree_and_queue_account_with_assert(
            &self.payer,
            true,
            &mut self.rpc,
            &merkle_tree_keypair,
            &nullifier_queue_keypair,
            None,
            None,
            &config,
            &address_config,
            0,
        )
        .await
        .unwrap();
        let init_value = BigUint::from_str_radix(HIGHEST_ADDRESS_PLUS_ONE, 10).unwrap();
        let mut merkle_tree = Box::new(
            IndexedMerkleTree::<Poseidon, usize>::new(
                STATE_MERKLE_TREE_HEIGHT as usize,
                STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
            )
            .unwrap(),
        );
        merkle_tree.merkle_tree.root_history_array_len = Some(ADDRESS_MERKLE_TREE_ROOTS);
        let mut indexed_array = Box::<IndexedArray<Poseidon, usize>>::default();
        merkle_tree.append(&init_value, &mut indexed_array).unwrap();

        let queue_account = AccountZeroCopy::<account_compression::QueueAccount>::new(
            &mut self.rpc,
            nullifier_queue_keypair.pubkey(),
        )
        .await;
        let mut bundle = AddressMerkleTreeBundle::new_v1(AddressMerkleTreeAccounts {
            merkle_tree: merkle_tree_keypair.pubkey(),
            queue: nullifier_queue_keypair.pubkey(),
        })
        .unwrap();
        bundle.rollover_fee = queue_account
            .deserialized()
            .metadata
            .rollover_metadata
            .rollover_fee as i64;
        self.indexer.get_address_merkle_trees_mut().push(bundle);
        // TODO: Add assert
    }

    pub fn safe_gen_range<T, RR>(rng: &mut StdRng, range: RR, empty_fallback: T) -> T
    where
        T: SampleUniform + Copy,
        RR: SampleRange<T> + Sized,
    {
        if range.is_empty() {
            return empty_fallback;
        }
        rng.gen_range(range)
    }

    /// 1. Transfer spl tokens between random users
    pub async fn activate_keypair_actions(&mut self, user: &Pubkey) {
        let user_index = self
            .users
            .iter()
            .position(|u| &u.keypair.pubkey() == user)
            .unwrap();
        // compress spl
        // check sufficient spl balance
        if self
            .rng
            .gen_bool(self.keypair_action_config.compress_spl.unwrap_or(0.0))
            && self.users[user_index].token_accounts.is_empty()
        // TODO: enable compress spl test
        {
            self.compress_spl(user_index).await;
        }
        // decompress spl
        // check sufficient compressed spl balance
        if self
            .rng
            .gen_bool(self.keypair_action_config.decompress_spl.unwrap_or(0.0))
        {
            self.decompress_spl(user_index).await;
        }

        // transfer spl
        // check sufficient compressed spl balance
        if self
            .rng
            .gen_bool(self.keypair_action_config.transfer_spl.unwrap_or(0.0))
        {
            self.transfer_spl(user_index).await;
        }
        // create address
        if self
            .rng
            .gen_bool(self.keypair_action_config.create_address.unwrap_or(0.0))
        {
            self.create_address(None, None).await;
        }

        // compress sol
        // check sufficient sol balance
        let balance = self
            .rpc
            .get_balance(&self.users[user_index].keypair.pubkey())
            .await
            .unwrap();
        if self
            .rng
            .gen_bool(self.keypair_action_config.compress_sol.unwrap_or(0.0))
            && balance > 1000
        {
            self.compress_sol(user_index, balance).await;
        } else {
            println!("Not enough balance to compress sol. Balance: {}", balance);
        }

        // decompress sol
        // check sufficient compressed sol balance
        if self
            .rng
            .gen_bool(self.keypair_action_config.decompress_sol.unwrap_or(0.0))
        {
            self.decompress_sol(user_index).await;
        }

        // transfer sol
        if self
            .rng
            .gen_bool(self.keypair_action_config.transfer_sol.unwrap_or(0.0))
        {
            self.transfer_sol(user_index).await;
        }
        // approve spl
        if self
            .rng
            .gen_bool(self.keypair_action_config.approve_spl.unwrap_or(0.0))
            && !self.users[user_index].token_accounts.is_empty()
        {
            self.approve_spl(user_index).await;
        }
        // revoke spl
        if self
            .rng
            .gen_bool(self.keypair_action_config.revoke_spl.unwrap_or(0.0))
            && !self.users[user_index].token_accounts.is_empty()
        {
            self.revoke_spl(user_index).await;
        }
        // burn spl
        if self
            .rng
            .gen_bool(self.keypair_action_config.burn_spl.unwrap_or(0.0))
            && !self.users[user_index].token_accounts.is_empty()
        {
            self.burn_spl(user_index).await;
        }
        // freeze spl
        if self
            .rng
            .gen_bool(self.keypair_action_config.freeze_spl.unwrap_or(0.0))
            && !self.users[user_index].token_accounts.is_empty()
        {
            self.freeze_spl(user_index).await;
        }
        // thaw spl
        if self
            .rng
            .gen_bool(self.keypair_action_config.thaw_spl.unwrap_or(0.0))
            && !self.users[user_index].token_accounts.is_empty()
        {
            self.thaw_spl(user_index).await;
        }
    }

    pub fn get_eligible_forester_for_queue(
        queue_pubkey: &Pubkey,
        foresters: &[TestForester],
        light_slot: u64,
    ) -> Option<Keypair> {
        for f in foresters.iter() {
            let tree = f.forester.active.merkle_trees.iter().find(|mt| {
                if mt.tree_accounts.tree_type == TreeType::StateV2 {
                    mt.tree_accounts.merkle_tree == *queue_pubkey
                } else {
                    mt.tree_accounts.queue == *queue_pubkey
                }
            });
            if let Some(tree) = tree {
                if tree.is_eligible(light_slot) {
                    return Some(f.keypair.insecure_clone());
                }
            }
        }
        None
    }
    pub async fn transfer_sol_deterministic(
        &mut self,
        from: &Keypair,
        to: &Pubkey,
        tree_index: Option<usize>,
    ) -> Result<Signature, RpcError> {
        let input_compressed_accounts = self.get_compressed_sol_accounts(&from.pubkey());
        let bundle = self.indexer.get_state_merkle_trees()[tree_index.unwrap_or(0)].clone();
        let rollover_fee = bundle.rollover_fee;
        let output_merkle_tree = match bundle.tree_type {
            TreeType::StateV1 => bundle.accounts.merkle_tree,
            // Output queue for batched trees
            TreeType::StateV2 => bundle.accounts.nullifier_queue,
            _ => panic!("Unsupported version"),
        };
        let recipients = vec![*to];
        let transaction_params = if self.keypair_action_config.fee_assert {
            let (inputs, is_v2) = if bundle.tree_type == TreeType::StateV2 {
                (0u8, true)
            } else {
                (input_compressed_accounts.len() as u8, false)
            };
            Some(TransactionParams {
                num_new_addresses: 0,
                v1_input_compressed_accounts: inputs,
                v2_input_compressed_accounts: is_v2,
                num_output_compressed_accounts: 1u8,
                compress: 0,
                fee_config: FeeConfig {
                    state_merkle_tree_rollover: rollover_fee as u64,
                    ..Default::default()
                },
            })
        } else {
            None
        };
        transfer_compressed_sol_test(
            &mut self.rpc,
            &mut self.indexer,
            from,
            input_compressed_accounts.as_slice(),
            recipients.as_slice(),
            &[output_merkle_tree],
            transaction_params,
        )
        .await
    }

    pub async fn transfer_sol(&mut self, user_index: usize) {
        let input_compressed_accounts = self.get_random_compressed_sol_accounts(user_index);

        if !input_compressed_accounts.is_empty() {
            println!("\n --------------------------------------------------\n\t\t Transfer Sol\n --------------------------------------------------");
            let recipients = self
                .users
                .iter()
                .map(|u| u.keypair.pubkey())
                .collect::<Vec<Pubkey>>();
            let num_output_merkle_trees = Self::safe_gen_range(
                &mut self.rng,
                1..std::cmp::min(
                    self.keypair_action_config
                        .max_output_accounts
                        .unwrap_or(recipients.len() as u64),
                    recipients.len() as u64,
                ),
                1,
            );
            let recipients = recipients
                .choose_multiple(&mut self.rng, num_output_merkle_trees as usize)
                .copied()
                .collect::<Vec<_>>();
            let output_merkle_trees = self.get_merkle_tree_pubkeys(num_output_merkle_trees);
            let transaction_parameters = if self.keypair_action_config.fee_assert {
                Some(TransactionParams {
                    num_new_addresses: 0,
                    v1_input_compressed_accounts: input_compressed_accounts.len() as u8,
                    v2_input_compressed_accounts: false,
                    num_output_compressed_accounts: num_output_merkle_trees as u8,
                    compress: 0,
                    fee_config: FeeConfig::default(),
                })
            } else {
                None
            };
            transfer_compressed_sol_test(
                &mut self.rpc,
                &mut self.indexer,
                &self.users[user_index].keypair,
                input_compressed_accounts.as_slice(),
                recipients.as_slice(),
                output_merkle_trees.as_slice(),
                transaction_parameters,
            )
            .await
            .unwrap();
            self.stats.sol_transfers += 1;
        }
    }

    pub async fn decompress_sol(&mut self, user_index: usize) {
        let input_compressed_accounts = self.get_random_compressed_sol_accounts(user_index);

        if !input_compressed_accounts.is_empty() {
            println!("\n --------------------------------------------------\n\t\t Decompress Sol\n --------------------------------------------------");
            let output_merkle_tree = self.get_merkle_tree_pubkeys(1)[0];
            let recipient = self.users
                [Self::safe_gen_range(&mut self.rng, 0..std::cmp::min(self.users.len(), 6), 0)]
            .keypair
            .pubkey();
            let balance = input_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.lamports)
                .sum::<u64>();
            let decompress_amount = Self::safe_gen_range(&mut self.rng, 1000..balance, balance / 2);
            let transaction_paramets = if self.keypair_action_config.fee_assert {
                Some(TransactionParams {
                    num_new_addresses: 0,
                    v1_input_compressed_accounts: input_compressed_accounts.len() as u8,
                    v2_input_compressed_accounts: false,
                    num_output_compressed_accounts: 1u8,
                    compress: 0,
                    fee_config: FeeConfig::default(),
                })
            } else {
                None
            };
            decompress_sol_test(
                &mut self.rpc,
                &mut self.indexer,
                &self.users[user_index].keypair,
                &input_compressed_accounts,
                &recipient,
                decompress_amount,
                &output_merkle_tree,
                transaction_paramets,
            )
            .await
            .unwrap();
            println!("post decompress_sol_test");
            self.stats.sol_decompress += 1;
        }
    }

    pub async fn compress_sol_deterministic(
        &mut self,
        from: &Keypair,
        amount: u64,
        tree_index: Option<usize>,
    ) {
        self.compress_sol_deterministic_opt_inputs(from, amount, tree_index, true)
            .await;
    }

    pub async fn compress_sol_deterministic_opt_inputs(
        &mut self,
        from: &Keypair,
        amount: u64,
        tree_index: Option<usize>,
        inputs: bool,
    ) {
        let input_compressed_accounts = if inputs {
            self.get_compressed_sol_accounts(&from.pubkey())
        } else {
            vec![]
        };
        let bundle = self.indexer.get_state_merkle_trees()[tree_index.unwrap_or(0)].clone();
        let rollover_fee = bundle.rollover_fee;
        let output_merkle_tree = match bundle.tree_type {
            TreeType::StateV1 => bundle.accounts.merkle_tree,
            // Output queue for batched trees
            TreeType::StateV2 => bundle.accounts.nullifier_queue,
            _ => panic!("Unsupported version"),
        };

        let transaction_parameters = if self.keypair_action_config.fee_assert {
            let (inputs, is_v2) = if bundle.tree_type == TreeType::StateV2 {
                (0u8, true)
            } else {
                (input_compressed_accounts.len() as u8, false)
            };
            Some(TransactionParams {
                num_new_addresses: 0,
                v1_input_compressed_accounts: inputs,
                v2_input_compressed_accounts: is_v2,
                num_output_compressed_accounts: 1u8,
                compress: amount as i64,
                fee_config: FeeConfig {
                    state_merkle_tree_rollover: rollover_fee as u64,
                    ..Default::default()
                },
            })
        } else {
            None
        };

        compress_sol_test(
            &mut self.rpc,
            &mut self.indexer,
            from,
            &input_compressed_accounts[..std::cmp::min(input_compressed_accounts.len(), 4)],
            false,
            amount,
            &output_merkle_tree,
            transaction_parameters,
        )
        .await
        .unwrap();
    }

    pub async fn compress_sol(&mut self, user_index: usize, balance: u64) {
        println!("\n --------------------------------------------------\n\t\t Compress Sol\n --------------------------------------------------");
        // Limit max compress amount to 1 sol so that context.payer doesn't get depleted by airdrops.
        let max_amount = std::cmp::min(balance, 1_000_000_000);
        let amount = Self::safe_gen_range(&mut self.rng, 1000..max_amount, max_amount / 2);
        let input_compressed_accounts = self.get_random_compressed_sol_accounts(user_index);
        let create_output_compressed_accounts_for_input_accounts = false;
        // TODO: debug Merkle trees in wrong order
        // if input_compressed_accounts.is_empty() {
        //     false
        // } else {
        //     self.rng.gen_bool(0.5)
        // };
        let output_merkle_tree = self.get_merkle_tree_pubkeys(1)[0];
        let transaction_parameters = if self.keypair_action_config.fee_assert {
            Some(TransactionParams {
                num_new_addresses: 0,
                v1_input_compressed_accounts: input_compressed_accounts.len() as u8,
                v2_input_compressed_accounts: false,
                num_output_compressed_accounts: 1u8,
                compress: amount as i64,
                fee_config: FeeConfig::default(),
            })
        } else {
            None
        };
        compress_sol_test(
            &mut self.rpc,
            &mut self.indexer,
            &self.users[user_index].keypair,
            input_compressed_accounts.as_slice(),
            create_output_compressed_accounts_for_input_accounts,
            amount,
            &output_merkle_tree,
            transaction_parameters,
        )
        .await
        .unwrap();
        airdrop_lamports(
            &mut self.rpc,
            &self.users[user_index].keypair.pubkey(),
            amount,
        )
        .await
        .unwrap();
        self.stats.sol_compress += 1;
    }

    pub async fn create_address(
        &mut self,
        optional_addresses: Option<Vec<Pubkey>>,
        address_tree_index: Option<usize>,
    ) -> Vec<Pubkey> {
        println!("\n --------------------------------------------------\n\t\t Create Address\n --------------------------------------------------");
        // select number of addresses to create
        let num_addresses = self.rng.gen_range(1..=2);
        let (address_merkle_tree_pubkeys, address_queue_pubkeys) =
            if let Some(address_tree_index) = address_tree_index {
                (
                    vec![
                        self.indexer
                            .get_address_merkle_trees()
                            .iter()
                            .filter(|x| x.accounts.merkle_tree != x.accounts.queue)
                            .collect::<Vec<_>>()[address_tree_index]
                            .accounts
                            .merkle_tree;
                        num_addresses as usize
                    ],
                    vec![
                        self.indexer
                            .get_address_merkle_trees()
                            .iter()
                            .filter(|x| x.accounts.merkle_tree != x.accounts.queue)
                            .collect::<Vec<_>>()[address_tree_index]
                            .accounts
                            .queue;
                        num_addresses as usize
                    ],
                )
            } else {
                // select random address Merkle tree(s)
                self.get_address_merkle_tree_pubkeys(num_addresses, false)
            };
        let mut address_seeds = Vec::new();
        let mut created_addresses = Vec::new();

        if let Some(addresses) = optional_addresses {
            for address in addresses {
                let address_seed: [u8; 32] = address.to_bytes();
                address_seeds.push(address_seed);
                created_addresses.push(address);
            }
        } else {
            for _ in 0..num_addresses {
                let address_seed: [u8; 32] =
                    bigint_to_be_bytes_array::<32>(&self.rng.gen_biguint(256)).unwrap();
                address_seeds.push(address_seed);
                created_addresses.push(Pubkey::from(address_seed));
            }
        }

        let output_compressed_accounts = self.get_merkle_tree_pubkeys(num_addresses);
        let transaction_parameters = if self.keypair_action_config.fee_assert {
            Some(TransactionParams {
                num_new_addresses: num_addresses as u8,
                v1_input_compressed_accounts: 0u8,
                v2_input_compressed_accounts: false,
                num_output_compressed_accounts: num_addresses as u8,
                compress: 0,
                fee_config: FeeConfig::default(),
            })
        } else {
            None
        };
        // TODO: add other input compressed accounts
        // (to test whether the address generation degrades performance)
        create_addresses_test(
            &mut self.rpc,
            &mut self.indexer,
            address_merkle_tree_pubkeys.as_slice(),
            address_queue_pubkeys.as_slice(),
            output_compressed_accounts,
            address_seeds.as_slice(),
            &Vec::new(),
            false,
            transaction_parameters,
        )
        .await
        .unwrap();
        self.stats.create_address += num_addresses;
        created_addresses
    }

    pub async fn transfer_spl(&mut self, user_index: usize) {
        let user = &self.users[user_index].keypair.pubkey();
        println!("\n --------------------------------------------------\n\t\t Tranfer Spl\n --------------------------------------------------");
        let (mint, mut token_accounts) = self.select_random_compressed_token_accounts(user).await;
        if token_accounts.is_empty() {
            let mt_pubkeys = self.get_merkle_tree_pubkeys(1);
            mint_tokens_helper(
                &mut self.rpc,
                &mut self.indexer,
                &mt_pubkeys[0],
                &self.payer,
                &mint,
                vec![Self::safe_gen_range(&mut self.rng, 100_000..1_000_000, 100_000); 1],
                vec![*user; 1],
            )
            .await;
            let (_, _token_accounts) = self.select_random_compressed_token_accounts(user).await;
            token_accounts = _token_accounts;
        }
        let recipients = token_accounts
            .iter()
            .map(|_| {
                self.users
                    [Self::safe_gen_range(&mut self.rng, 0..std::cmp::min(self.users.len(), 6), 0)]
                .keypair
                .pubkey()
            })
            .collect::<Vec<_>>();
        println!("Recipients: {:?}", recipients.len());
        let max_amount = token_accounts
            .iter()
            .map(|token_account| token_account.token_data.amount)
            .sum::<u64>();
        let amount = Self::safe_gen_range(&mut self.rng, 1000..max_amount, max_amount / 2);
        let equal_amount = amount / recipients.len() as u64;
        let num_output_compressed_accounts = if max_amount - amount != 0 {
            recipients.len() + 1
        } else {
            recipients.len()
        };
        // get different amounts for each recipient so that every compressed account is unique
        let amounts = recipients
            .iter()
            .enumerate()
            .map(|(i, _)| equal_amount - i as u64)
            .collect::<Vec<u64>>();

        let output_merkle_tree_pubkeys =
            self.get_merkle_tree_pubkeys(num_output_compressed_accounts as u64);
        let transaction_paramets = if self.keypair_action_config.fee_assert {
            Some(TransactionParams {
                num_new_addresses: 0u8,
                v1_input_compressed_accounts: token_accounts.len() as u8,
                v2_input_compressed_accounts: false,
                num_output_compressed_accounts: output_merkle_tree_pubkeys.len() as u8,
                compress: 0,
                fee_config: FeeConfig::default(),
            })
        } else {
            None
        };
        compressed_transfer_test(
            &self.rpc.get_payer().insecure_clone(),
            &mut self.rpc,
            &mut self.indexer,
            &mint,
            &self.users[user_index].keypair.insecure_clone(),
            &recipients,
            &amounts,
            None,
            &token_accounts,
            &output_merkle_tree_pubkeys,
            None,
            false,
            transaction_paramets,
        )
        .await;
        self.stats.spl_transfers += 1;
    }

    pub async fn approve_spl(&mut self, user_index: usize) {
        let user = &self.users[user_index].keypair.pubkey();
        println!("\n --------------------------------------------------\n\t\t Approve Spl\n --------------------------------------------------");
        let (mint, mut token_accounts) = self.select_random_compressed_token_accounts(user).await;
        if token_accounts.is_empty() {
            let mt_pubkeys = self.get_merkle_tree_pubkeys(1);
            mint_tokens_helper(
                &mut self.rpc,
                &mut self.indexer,
                &mt_pubkeys[0],
                &self.payer,
                &mint,
                vec![Self::safe_gen_range(&mut self.rng, 100_000..1_000_000, 100_000); 1],
                vec![*user; 1],
            )
            .await;
            let (_, _token_accounts) = self.select_random_compressed_token_accounts(user).await;
            token_accounts = _token_accounts;
        }
        println!("token_accounts: {:?}", token_accounts);
        let rnd_user_index = self.rng.gen_range(0..self.users.len());
        let delegate = self.users[rnd_user_index].keypair.pubkey();
        let max_amount = token_accounts
            .iter()
            .map(|token_account| token_account.token_data.amount)
            .sum::<u64>();
        let delegate_amount = Self::safe_gen_range(&mut self.rng, 0..max_amount, max_amount / 2);
        let num_output_compressed_accounts = if delegate_amount != max_amount { 2 } else { 1 };
        let output_merkle_tree_pubkeys = self.get_merkle_tree_pubkeys(2);
        let transaction_paramets = if self.keypair_action_config.fee_assert {
            Some(TransactionParams {
                num_new_addresses: 0u8,
                v1_input_compressed_accounts: token_accounts.len() as u8,
                v2_input_compressed_accounts: false,
                num_output_compressed_accounts,
                compress: 0,
                fee_config: FeeConfig::default(),
            })
        } else {
            None
        };
        approve_test(
            &self.users[user_index].keypair,
            &mut self.rpc,
            &mut self.indexer,
            token_accounts,
            delegate_amount,
            None,
            &delegate,
            &output_merkle_tree_pubkeys[0],
            &output_merkle_tree_pubkeys[1],
            transaction_paramets,
        )
        .await;
        self.stats.spl_approved += 1;
    }

    pub async fn revoke_spl(&mut self, user_index: usize) {
        let user = &self.users[user_index].keypair.pubkey();
        println!("\n --------------------------------------------------\n\t\t Revoke Spl\n --------------------------------------------------");
        let (mint, mut token_accounts) = self
            .select_random_compressed_token_accounts_delegated(user, true, None, false)
            .await;
        if token_accounts.is_empty() {
            let mt_pubkeys = self.get_merkle_tree_pubkeys(1);
            mint_tokens_helper(
                &mut self.rpc,
                &mut self.indexer,
                &mt_pubkeys[0],
                &self.payer,
                &mint,
                vec![Self::safe_gen_range(&mut self.rng, 100_000..1_000_000, 100_000); 1],
                vec![*user; 1],
            )
            .await;
            self.approve_spl(user_index).await;
            let (_, _token_accounts) = self
                .select_random_compressed_token_accounts_delegated(user, true, None, false)
                .await;
            token_accounts = _token_accounts;
        }
        let num_output_compressed_accounts = 1;
        let output_merkle_tree_pubkeys = self.get_merkle_tree_pubkeys(1);
        let transaction_paramets = if self.keypair_action_config.fee_assert {
            Some(TransactionParams {
                num_new_addresses: 0u8,
                v1_input_compressed_accounts: token_accounts.len() as u8,
                v2_input_compressed_accounts: false,
                num_output_compressed_accounts,
                compress: 0,
                fee_config: FeeConfig::default(),
            })
        } else {
            None
        };
        revoke_test(
            &self.users[user_index].keypair,
            &mut self.rpc,
            &mut self.indexer,
            token_accounts,
            &output_merkle_tree_pubkeys[0],
            transaction_paramets,
        )
        .await;
        self.stats.spl_revoked += 1;
    }

    pub async fn burn_spl(&mut self, user_index: usize) {
        let user = &self.users[user_index].keypair.pubkey();
        println!("\n --------------------------------------------------\n\t\t Burn Spl\n --------------------------------------------------");
        let (mint, mut token_accounts) = self.select_random_compressed_token_accounts(user).await;
        if token_accounts.is_empty() {
            let mt_pubkeys = self.get_merkle_tree_pubkeys(1);
            mint_tokens_helper(
                &mut self.rpc,
                &mut self.indexer,
                &mt_pubkeys[0],
                &self.payer,
                &mint,
                vec![Self::safe_gen_range(&mut self.rng, 100_000..1_000_000, 100_000); 1],
                vec![*user; 1],
            )
            .await;
            let (_, _token_accounts) = self.select_random_compressed_token_accounts(user).await;
            token_accounts = _token_accounts;
        }
        let max_amount = token_accounts
            .iter()
            .map(|token_account| token_account.token_data.amount)
            .sum::<u64>();
        let burn_amount = Self::safe_gen_range(&mut self.rng, 0..max_amount, max_amount / 2);
        let num_output_compressed_accounts = if burn_amount != max_amount { 1 } else { 0 };
        let output_merkle_tree_pubkeys = self.get_merkle_tree_pubkeys(1);
        let transaction_paramets = if self.keypair_action_config.fee_assert {
            Some(TransactionParams {
                num_new_addresses: 0u8,
                v1_input_compressed_accounts: token_accounts.len() as u8,
                v2_input_compressed_accounts: false,
                num_output_compressed_accounts,
                compress: 0,
                fee_config: FeeConfig::default(),
            })
        } else {
            None
        };

        burn_test(
            &self.users[user_index].keypair,
            &mut self.rpc,
            &mut self.indexer,
            token_accounts,
            &output_merkle_tree_pubkeys[0],
            burn_amount,
            false,
            transaction_paramets,
            false,
            0,
        )
        .await;
        self.stats.spl_burned += 1;
    }

    pub async fn freeze_spl(&mut self, user_index: usize) {
        let user = &self.users[user_index].keypair.pubkey();
        println!("\n --------------------------------------------------\n\t\t Freeze Spl\n --------------------------------------------------");
        let (mint, mut token_accounts) = self.select_random_compressed_token_accounts(user).await;
        if token_accounts.is_empty() {
            let mt_pubkeys = self.get_merkle_tree_pubkeys(1);
            mint_tokens_helper(
                &mut self.rpc,
                &mut self.indexer,
                &mt_pubkeys[0],
                &self.payer,
                &mint,
                vec![Self::safe_gen_range(&mut self.rng, 100_000..1_000_000, 100_000); 1],
                vec![*user; 1],
            )
            .await;
            let (_, _token_accounts) = self
                .select_random_compressed_token_accounts_delegated(user, false, None, false)
                .await;
            token_accounts = _token_accounts;
        }
        let output_merkle_tree_pubkeys = self.get_merkle_tree_pubkeys(1);
        let transaction_parameters = if self.keypair_action_config.fee_assert {
            Some(TransactionParams {
                num_new_addresses: 0u8,
                v1_input_compressed_accounts: token_accounts.len() as u8,
                v2_input_compressed_accounts: false,
                num_output_compressed_accounts: token_accounts.len() as u8,
                compress: 0,
                fee_config: FeeConfig::default(),
            })
        } else {
            None
        };
        freeze_test(
            &self.rpc.get_payer().insecure_clone(),
            &mut self.rpc,
            &mut self.indexer,
            token_accounts,
            &output_merkle_tree_pubkeys[0],
            transaction_parameters,
        )
        .await;
        self.stats.spl_frozen += 1;
    }

    pub async fn thaw_spl(&mut self, user_index: usize) {
        let user = &self.users[user_index].keypair.pubkey();
        println!("\n --------------------------------------------------\n\t\t Thaw Spl\n --------------------------------------------------");
        let (_, mut token_accounts) = self
            .select_random_compressed_token_accounts_frozen(user)
            .await;
        if token_accounts.is_empty() {
            self.freeze_spl(user_index).await;

            let (_, _token_accounts) = self
                .select_random_compressed_token_accounts_frozen(user)
                .await;
            token_accounts = _token_accounts;
        }
        let output_merkle_tree_pubkeys = self.get_merkle_tree_pubkeys(1);
        let transaction_paramets = if self.keypair_action_config.fee_assert {
            Some(TransactionParams {
                num_new_addresses: 0u8,
                v1_input_compressed_accounts: token_accounts.len() as u8,
                v2_input_compressed_accounts: false,
                num_output_compressed_accounts: token_accounts.len() as u8,
                compress: 0,
                fee_config: FeeConfig::default(),
            })
        } else {
            None
        };

        thaw_test(
            &self.rpc.get_payer().insecure_clone(),
            &mut self.rpc,
            &mut self.indexer,
            token_accounts,
            &output_merkle_tree_pubkeys[0],
            transaction_paramets,
        )
        .await;
        self.stats.spl_thawed += 1;
    }

    pub async fn compress_spl(&mut self, user_index: usize) {
        println!("\n --------------------------------------------------\n\t\t Compress Spl\n --------------------------------------------------");
        let mut balance = 0;
        let mut mint = Pubkey::default();
        let mut token_account = Pubkey::default();
        for _ in 0..self.users[user_index].token_accounts.len() {
            let (_mint, _token_account) = self.users[user_index].token_accounts[self
                .rng
                .gen_range(0..self.users[user_index].token_accounts.len())];
            token_account = _token_account;
            mint = _mint;
            self.rpc.get_account(_token_account).await.unwrap();
            use solana_sdk::program_pack::Pack;
            let account = spl_token::state::Account::unpack(
                &self
                    .rpc
                    .get_account(_token_account)
                    .await
                    .unwrap()
                    .unwrap()
                    .data,
            )
            .unwrap();
            balance = account.amount;
            if balance != 0 {
                break;
            }
        }
        if balance != 0 {
            self.users[user_index]
                .token_accounts
                .push((mint, token_account));
            let output_merkle_tree_account = self.get_merkle_tree_pubkeys(1);

            let amount = Self::safe_gen_range(&mut self.rng, 1000..balance, balance / 2);
            let transaction_paramets = if self.keypair_action_config.fee_assert {
                Some(TransactionParams {
                    num_new_addresses: 0u8,
                    v1_input_compressed_accounts: 0u8,
                    v2_input_compressed_accounts: false,
                    num_output_compressed_accounts: 1u8,
                    compress: 0, // sol amount this is a spl compress test
                    fee_config: FeeConfig::default(),
                })
            } else {
                None
            };
            compress_test(
                &self.users[user_index].keypair,
                &mut self.rpc,
                &mut self.indexer,
                amount,
                &mint,
                &output_merkle_tree_account[0],
                &token_account,
                transaction_paramets,
                false,
                0, // TODO: make random
                None,
            )
            .await;
            self.stats.spl_compress += 1;
        }
    }

    pub async fn decompress_spl(&mut self, user_index: usize) {
        let user = &self.users[user_index].keypair.pubkey();
        println!("\n --------------------------------------------------\n\t\t Decompress Spl\n --------------------------------------------------");
        let (mint, mut token_accounts) = self.select_random_compressed_token_accounts(user).await;
        if token_accounts.is_empty() {
            let mt_pubkeys = self.get_merkle_tree_pubkeys(1);
            mint_tokens_helper(
                &mut self.rpc,
                &mut self.indexer,
                &mt_pubkeys[0],
                &self.payer,
                &mint,
                vec![Self::safe_gen_range(&mut self.rng, 100_000..1_000_000, 100_000); 1],
                vec![*user; 1],
            )
            .await;
            let (_, _token_accounts) = self.select_random_compressed_token_accounts(user).await;
            token_accounts = _token_accounts;
        }
        let token_account = match self.users[user_index]
            .token_accounts
            .iter()
            .find(|t| t.0 == mint)
        {
            Some(token_account) => token_account.1,
            None => {
                let token_account_keypair = Keypair::new();
                create_token_account(
                    &mut self.rpc,
                    &mint,
                    &token_account_keypair,
                    &self.users[user_index].keypair,
                )
                .await
                .unwrap();

                token_account_keypair.pubkey()
            }
        };

        self.users[user_index]
            .token_accounts
            .push((mint, token_account));
        let output_merkle_tree_account = self.get_merkle_tree_pubkeys(1);
        let max_amount = token_accounts
            .iter()
            .map(|token_account| token_account.token_data.amount)
            .sum::<u64>();
        let amount = Self::safe_gen_range(&mut self.rng, 1000..max_amount, max_amount / 2);
        let transaction_paramets = if self.keypair_action_config.fee_assert {
            Some(TransactionParams {
                num_new_addresses: 0u8,
                v1_input_compressed_accounts: token_accounts.len() as u8,
                v2_input_compressed_accounts: false,
                num_output_compressed_accounts: 1u8,
                compress: 0,
                fee_config: FeeConfig::default(),
            })
        } else {
            None
        };
        // decompress
        decompress_test(
            &self.users[user_index].keypair,
            &mut self.rpc,
            &mut self.indexer,
            token_accounts.clone(),
            amount,
            &output_merkle_tree_account[0],
            &token_account,
            transaction_paramets,
            false,
            0, // TODO: make random
            None,
        )
        .await;
        self.stats.spl_decompress += 1;
    }

    pub async fn rollover_state_merkle_tree_and_queue(
        &mut self,
        index: usize,
        payer: &Keypair,
        epoch: u64,
    ) -> Result<(), RpcError> {
        let bundle = self.indexer.get_state_merkle_trees()[index].accounts;
        let new_nullifier_queue_keypair = Keypair::new();
        let new_merkle_tree_keypair = Keypair::new();
        // TODO: move into registry program
        let new_cpi_signature_keypair = Keypair::new();
        let fee_payer_balance = self
            .rpc
            .get_balance(&self.rpc.get_payer().pubkey())
            .await
            .unwrap();
        let rollover_signature_and_slot = perform_state_merkle_tree_roll_over_forester(
            payer,
            &mut self.rpc,
            &new_nullifier_queue_keypair,
            &new_merkle_tree_keypair,
            &new_cpi_signature_keypair,
            &bundle.merkle_tree,
            &bundle.nullifier_queue,
            epoch,
            false,
        )
        .await
        .unwrap();
        info!("Rollover signature: {:?}", rollover_signature_and_slot.0);
        let additional_rent = self
            .rpc
            .get_minimum_balance_for_rent_exemption(
                ProtocolConfig::default().cpi_context_size as usize,
            )
            .await
            .unwrap();
        info!("additional_rent: {:?}", additional_rent);
        assert_rolled_over_pair(
            &self.rpc.get_payer().pubkey(),
            &mut self.rpc,
            &fee_payer_balance,
            &bundle.merkle_tree,
            &bundle.nullifier_queue,
            &new_merkle_tree_keypair.pubkey(),
            &new_nullifier_queue_keypair.pubkey(),
            rollover_signature_and_slot.1,
            additional_rent,
            4,
        )
        .await;
        let output_queue_batch_size =
            self.indexer.get_state_merkle_trees()[index].output_queue_batch_size;
        self.indexer
            .get_state_merkle_trees_mut()
            .push(StateMerkleTreeBundle {
                // TODO: fetch correct fee when this property is used
                rollover_fee: 0,
                accounts: StateMerkleTreeAccounts {
                    merkle_tree: new_merkle_tree_keypair.pubkey(),
                    nullifier_queue: new_nullifier_queue_keypair.pubkey(),
                    cpi_context: new_cpi_signature_keypair.pubkey(),
                    tree_type: TreeType::StateV1,
                },
                tree_type: TreeType::StateV1,
                merkle_tree: Box::new(light_merkle_tree_reference::MerkleTree::<Poseidon>::new(
                    STATE_MERKLE_TREE_HEIGHT as usize,
                    STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
                )),
                output_queue_elements: vec![],
                input_leaf_indices: vec![],
                num_inserted_batches: 0,
                output_queue_batch_size,
            });
        Ok(())
    }

    pub async fn rollover_address_merkle_tree_and_queue(
        &mut self,
        index: usize,
        payer: &Keypair,
        epoch: u64,
    ) -> Result<(), RpcError> {
        let bundle = self
            .indexer
            .get_address_merkle_trees()
            .iter()
            .filter(|x| x.accounts.merkle_tree != x.accounts.queue)
            .collect::<Vec<_>>()[index]
            .accounts;
        let new_nullifier_queue_keypair = Keypair::new();
        let new_merkle_tree_keypair = Keypair::new();
        let fee_payer_balance = self
            .rpc
            .get_balance(&self.rpc.get_payer().pubkey())
            .await
            .unwrap();
        perform_address_merkle_tree_roll_over_forester(
            payer,
            &mut self.rpc,
            &new_nullifier_queue_keypair,
            &new_merkle_tree_keypair,
            &bundle.merkle_tree,
            &bundle.queue,
            epoch,
            false,
        )
        .await?;
        assert_rolled_over_address_merkle_tree_and_queue(
            &self.rpc.get_payer().pubkey(),
            &mut self.rpc,
            &fee_payer_balance,
            &bundle.merkle_tree,
            &bundle.queue,
            &new_merkle_tree_keypair.pubkey(),
            &new_nullifier_queue_keypair.pubkey(),
        )
        .await;
        self.indexer.add_address_merkle_tree_accounts(
            &new_merkle_tree_keypair,
            &new_nullifier_queue_keypair,
            None,
        );
        Ok(())
    }

    pub async fn invoke_cpi_test(&mut self, user_index: usize) -> Result<(), RpcError> {
        // addresses that can be used in output accounts
        let mut addresses = vec![];

        let mut proof_input_accounts = vec![];
        // TODO: filter for Merkle tree type to enable mixed tree tests
        let input_accounts = {
            let mut program_accounts = self
                .indexer
                .get_compressed_accounts_with_merkle_context_by_owner(
                    &create_address_test_program::ID,
                )
                .clone();
            let num_accounts = program_accounts.len();
            let max_num = std::cmp::min(num_accounts, 1);
            let select_num_accounts = Self::safe_gen_range(&mut self.rng, 0..=max_num, 0);
            let mut accounts = vec![];
            for _ in 0..select_num_accounts {
                let num_accounts = program_accounts.len();
                let index = Self::safe_gen_range(&mut self.rng, 0..num_accounts, 0);
                accounts.push(program_accounts.remove(index));
            }

            accounts.iter().for_each(|account| {
                let hash = account.hash().unwrap();

                proof_input_accounts.push((hash, account.merkle_context.merkle_tree_pubkey));
            });

            accounts
        };

        let mut read_only_accounts = {
            let program_accounts = self
                .indexer
                .get_compressed_accounts_with_merkle_context_by_owner(
                    &create_address_test_program::ID,
                );
            let num_accounts = program_accounts.len();

            let max_num = std::cmp::min(num_accounts, 4 - input_accounts.len());

            let select_num_accounts = Self::safe_gen_range(&mut self.rng, 0..max_num, 0);
            let mut accounts = vec![];
            for _ in 0..select_num_accounts {
                let index = Self::safe_gen_range(&mut self.rng, 0..num_accounts, 0);
                let account = program_accounts[index].clone();
                if !input_accounts.contains(&account) {
                    accounts.push(account);
                }
            }
            accounts
                .iter()
                .map(|account| {
                    let account_hash = account.hash().unwrap();
                    proof_input_accounts
                        .push((account_hash, account.merkle_context.merkle_tree_pubkey));

                    ReadOnlyCompressedAccount {
                        account_hash,
                        merkle_context: account.merkle_context,
                        root_index: 0, // set after proof generation
                    }
                })
                .collect::<Vec<_>>()
        };

        let mut proof_input_addresses = vec![];
        let mut new_address_params = {
            let num_addresses = Self::safe_gen_range(&mut self.rng, 0..=1, 0);
            let (address_merkle_tree, queues) =
                self.get_address_merkle_tree_pubkeys(num_addresses, true);

            let seeds = (0..num_addresses)
                .map(|_| {
                    let seed: [u8; 32] =
                        bigint_to_be_bytes_array::<32>(&self.rng.gen_biguint(256)).unwrap();
                    seed
                })
                .collect::<Vec<_>>();
            seeds.iter().enumerate().for_each(|(index, seed)| {
                println!("seed {:?}", seed);
                println!(
                    "address_merkle_tree {:?}",
                    address_merkle_tree[index].to_bytes()
                );
                println!(
                    "create_address_test_program::ID {:?}",
                    create_address_test_program::ID.to_bytes()
                );
                let address = derive_address(
                    seed,
                    &address_merkle_tree[index].to_bytes(),
                    &create_address_test_program::ID.to_bytes(),
                );
                addresses.push(address);
                proof_input_addresses.push((address, address_merkle_tree[index]));
            });
            seeds
                .iter()
                .enumerate()
                .map(|(index, seed)| {
                    NewAddressParams {
                        address_merkle_tree_pubkey: address_merkle_tree[index].into(),
                        address_queue_pubkey: queues[index].into(),
                        seed: *seed,
                        address_merkle_tree_root_index: 0, // set after proof generation
                    }
                })
                .collect::<Vec<_>>()
        };

        let mut read_only_addresses = {
            let num_addresses = Self::safe_gen_range(&mut self.rng, 0..=1, 0);
            let seeds = (0..num_addresses)
                .map(|_| {
                    let seed: [u8; 32] =
                        bigint_to_be_bytes_array::<32>(&self.rng.gen_biguint(256)).unwrap();
                    seed
                })
                .collect::<Vec<_>>();
            let address_merkle_tree = self.get_address_merkle_tree_pubkeys(num_addresses, true).0;
            seeds
                .iter()
                .enumerate()
                .map(|(index, seed)| {
                    let address = derive_address(
                        seed,
                        &address_merkle_tree[index].to_bytes(),
                        &create_address_test_program::ID.to_bytes(),
                    );
                    proof_input_addresses.push((address, address_merkle_tree[index]));
                    ReadOnlyAddress {
                        address_merkle_tree_pubkey: address_merkle_tree[index].into(),
                        address,
                        address_merkle_tree_root_index: 0, // set after proof generation
                    }
                })
                .collect::<Vec<_>>()
        };
        // (address params index, output account index)
        let mut new_address_indices: Vec<(usize, u8)> = vec![];

        let (output_accounts, output_merkle_trees) = {
            let num_output_accounts = Self::safe_gen_range(&mut self.rng, 0..=8, 0);
            let mut accounts = vec![];

            for i in 0..num_output_accounts {
                let rnd_data: [u8; 32] = self.rng.gen();
                let mut data_hash = rnd_data;
                // truncate else 0x1 error
                data_hash[1] = 0;
                data_hash[0] = 0;
                let select_address: bool = self.rng.gen_bool(0.5);
                let num_addresses = addresses.len();

                let address = if select_address && num_addresses > 0 {
                    let new_address_index =
                        Self::safe_gen_range(&mut self.rng, 0..num_addresses, 0);
                    new_address_indices.push((new_address_index, i.try_into().unwrap()));
                    Some(addresses.remove(new_address_index))
                } else {
                    None
                };
                let account = CompressedAccount {
                    owner: create_address_test_program::ID.into(),
                    data: Some(CompressedAccountData {
                        data: rnd_data.to_vec(),
                        discriminator: [1; 8],
                        data_hash,
                    }),
                    address,
                    lamports: 0,
                };
                accounts.push(account);
            }
            let merkle_trees = self.get_merkle_tree_pubkeys(num_output_accounts as u64);
            (accounts, merkle_trees)
        };
        let mut root_indices = Vec::new();
        let mut proof = None;
        if !proof_input_accounts.is_empty() || !proof_input_addresses.is_empty() {
            let addresses_with_tree = proof_input_addresses
                .clone()
                .into_iter()
                .map(|(address, tree)| AddressWithTree { address, tree })
                .collect::<Vec<_>>();
            let compressed_account_input_hashes =
                proof_input_accounts.iter().map(|x| x.0).collect::<Vec<_>>();
            println!(
                "compressed_account_input_hashes {:?}",
                compressed_account_input_hashes
            );

            let proof_rpc_res = self
                .indexer
                .get_validity_proof(compressed_account_input_hashes, addresses_with_tree, None)
                .await
                .unwrap();

            root_indices = proof_rpc_res.value.get_root_indices();

            if let Some(proof_rpc_res) = proof_rpc_res.value.proof.0 {
                proof = Some(proof_rpc_res);
            }

            if !new_address_params.is_empty() {
                let address_root_indices: Vec<_> = proof_rpc_res
                    .value
                    .addresses
                    .iter()
                    .map(|x| x.root_index)
                    .collect();
                for (i, input_address) in new_address_params.iter_mut().enumerate() {
                    input_address.address_merkle_tree_root_index = address_root_indices[i];
                }
            }
            if !read_only_addresses.is_empty() {
                let address_root_indices: Vec<_> = proof_rpc_res
                    .value
                    .addresses
                    .iter()
                    .map(|x| x.root_index)
                    .collect();
                for (i, input_address) in read_only_addresses.iter_mut().enumerate() {
                    input_address.address_merkle_tree_root_index =
                        address_root_indices[i + new_address_params.len()];
                }
            }

            if !read_only_accounts.is_empty() {
                let account_root_indices: Vec<_> = proof_rpc_res.value.get_root_indices();
                for (i, input_account) in read_only_accounts.iter_mut().enumerate() {
                    if let Some(root_index) =
                        account_root_indices.get(i + input_accounts.len()).copied()
                    {
                        if let Some(root_index) = root_index {
                            input_account.root_index = root_index;
                        } else {
                            input_account.merkle_context.prove_by_index = true;
                        }
                    }
                }
            }
        }
        let mut remaining_accounts = HashMap::<Pubkey, usize>::new();

        let packed_new_address_params =
            pack_new_address_params(new_address_params.as_slice(), &mut remaining_accounts);

        let packed_inputs = pack_compressed_accounts(
            input_accounts.as_slice(),
            root_indices.as_slice(),
            &mut remaining_accounts,
        );

        // Pack output accounts and track original creation order
        let packed_outputs_with_index = pack_output_compressed_accounts(
            output_accounts.as_slice(),
            output_merkle_trees.as_slice(),
            &mut remaining_accounts,
        )
        .into_iter()
        .enumerate()
        .collect::<Vec<_>>();

        // Sort by merkle_tree_index while tracking original indices
        let mut sorted_outputs = packed_outputs_with_index.clone();
        sorted_outputs.sort_by_key(|(_, account)| account.merkle_tree_index);

        // Create mapping: creation_order_index -> sorted_index
        let mut creation_to_sorted_index = vec![0usize; sorted_outputs.len()];
        for (sorted_idx, (creation_idx, _)) in sorted_outputs.iter().enumerate() {
            creation_to_sorted_index[*creation_idx] = sorted_idx;
        }
        println!(
            "creation_to_sorted_index mapping: {:?}",
            creation_to_sorted_index
        );

        let output_compressed_accounts = sorted_outputs
            .into_iter()
            .map(|(_, account)| account)
            .collect::<Vec<_>>();

        println!(
            "output_compressed_accounts: {:?}",
            output_compressed_accounts
        );
        println!(
            "remaining_accounts: {:?}",
            remaining_accounts.clone().into_iter().collect::<Vec<_>>()
        );
        let invoke_cpi: InstructionDataInvokeCpi = InstructionDataInvokeCpi {
            proof,
            new_address_params: packed_new_address_params,
            input_compressed_accounts_with_merkle_context: packed_inputs,
            output_compressed_accounts,
            relay_fee: None,
            compress_or_decompress_lamports: None,
            is_compress: false,
            cpi_context: None,
        };
        let read_only_accounts =
            pack_read_only_accounts(read_only_accounts.as_slice(), &mut remaining_accounts);
        let read_only_addresses =
            pack_read_only_address_params(read_only_addresses.as_slice(), &mut remaining_accounts);
        let (_, bump) = Pubkey::find_program_address(
            &[CPI_AUTHORITY_PDA_SEED],
            &create_address_test_program::ID,
        );
        let mut new_address_params = Vec::new();
        for (index, new_address_param) in invoke_cpi.new_address_params.into_iter().enumerate() {
            if let Some((_, creation_order_idx)) = new_address_indices
                .iter()
                .find(|(address_index, _)| *address_index == index)
            {
                // Map creation order index to sorted index
                let sorted_idx = creation_to_sorted_index[*creation_order_idx as usize];
                new_address_params.push(NewAddressParamsAssignedPacked::new(
                    new_address_param,
                    Some(sorted_idx.try_into().unwrap()),
                ));
            } else {
                new_address_params
                    .push(NewAddressParamsAssignedPacked::new(new_address_param, None));
            }
        }

        let ix_data: InstructionDataInvokeCpiWithReadOnly = InstructionDataInvokeCpiWithReadOnly {
            mode: 0,
            bump,
            with_transaction_hash: self.rng.gen(),
            invoking_program_id: create_address_test_program::ID.into(),
            proof: invoke_cpi.proof,
            new_address_params,
            with_cpi_context: false,
            cpi_context: CompressedCpiContext::default(),
            input_compressed_accounts: invoke_cpi
                .input_compressed_accounts_with_merkle_context
                .iter()
                .map(|x| (*x).clone().into())
                .collect::<Vec<_>>(),
            is_compress: invoke_cpi.is_compress,
            compress_or_decompress_lamports: invoke_cpi
                .compress_or_decompress_lamports
                .unwrap_or_default(),
            output_compressed_accounts: invoke_cpi.output_compressed_accounts,
            read_only_accounts,
            read_only_addresses,
        };
        println!("ix_data: {:?}", ix_data);
        if ix_data.read_only_accounts.is_empty()
            && ix_data.read_only_addresses.is_empty()
            && ix_data.input_compressed_accounts.is_empty()
            && ix_data.output_compressed_accounts.is_empty()
        {
            return Ok(());
        }

        let user = &self.users[user_index].keypair;
        let remaining_accounts = to_account_metas(remaining_accounts);

        let instruction = create_invoke_cpi_instruction(
            user.pubkey(),
            [
                light_system_program::instruction::InvokeCpiWithReadOnly::DISCRIMINATOR.to_vec(),
                ix_data.try_to_vec().unwrap(),
            ]
            .concat(),
            remaining_accounts,
            None,
        );

        let res =
            light_program_test::program_test::TestRpc::create_and_send_transaction_with_batched_event(
                &mut self.rpc,
                &[instruction],
                &user.pubkey(),
                &[user],
                None,
            )
            .await?;
        // In case that only read only accounts exist in a transaction
        // the account compression program is not invoked -> there is no event and it is ok.
        let tx_has_read_only =
            !ix_data.read_only_accounts.is_empty() || !ix_data.read_only_addresses.is_empty();
        let tx_has_no_writable = ix_data.input_compressed_accounts.is_empty()
            && ix_data.output_compressed_accounts.is_empty()
            && ix_data.new_address_params.is_empty();
        let tx_is_read_only = tx_has_read_only && tx_has_no_writable;
        if !tx_is_read_only {
            let (events, _, slot) = res.ok_or(RpcError::CustomError(
                "invoke_cpi_test: No event".to_string(),
            ))?;
            for event in events {
                self.indexer
                    .add_event_and_compressed_accounts(slot, &event.event);
            }
            let tree_bundle = &self.indexer.get_address_merkle_trees()[0];
            println!(
                "tree_bundle queue_elements: {:?}",
                tree_bundle.queue_elements
            );
            println!(
                "new addresses proof_input_addresses: {:?}",
                proof_input_addresses
            );
        } else if res.is_some() {
            return Err(RpcError::CustomError(
                "invoke_cpi_test: Read only tx created an event.".to_string(),
            ));
        }

        Ok(())
    }

    pub fn get_random_compressed_sol_accounts(
        &mut self,
        user_index: usize,
    ) -> Vec<CompressedAccountWithMerkleContext> {
        let input_compressed_accounts = self
            .indexer
            .get_compressed_accounts_with_merkle_context_by_owner(
                &self.users[user_index].keypair.pubkey(),
            )
            .into_iter()
            .collect::<Vec<_>>();
        if input_compressed_accounts.is_empty() {
            return vec![];
        }
        let index = Self::safe_gen_range(&mut self.rng, 0..input_compressed_accounts.len(), 0);
        // pick random first account to decompress
        let first_account = &input_compressed_accounts[index];
        let first_mt = self
            .indexer
            .get_state_merkle_trees()
            .iter()
            .find(|x| {
                x.accounts.merkle_tree.to_bytes()
                    == first_account.merkle_context.merkle_tree_pubkey.to_bytes()
            })
            .unwrap()
            .tree_type;
        let input_compressed_accounts_with_same_version = input_compressed_accounts
            .iter()
            .filter(|x| {
                self.indexer
                    .get_state_merkle_trees()
                    .iter()
                    .find(|y| {
                        y.accounts.merkle_tree.to_bytes()
                            == x.merkle_context.merkle_tree_pubkey.to_bytes()
                    })
                    .unwrap()
                    .tree_type
                    == first_mt
            })
            .cloned()
            .collect::<Vec<_>>();
        let range = std::cmp::min(input_compressed_accounts_with_same_version.len(), 4);

        let number_of_compressed_accounts = Self::safe_gen_range(&mut self.rng, 0..=range, 0);
        input_compressed_accounts_with_same_version[0..number_of_compressed_accounts].to_vec()
    }

    pub fn get_compressed_sol_accounts(
        &self,
        pubkey: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext> {
        self.indexer
            .get_compressed_accounts_with_merkle_context_by_owner(pubkey)
            .into_iter()
            .collect()
    }

    pub fn get_merkle_tree_pubkeys(&mut self, num: u64) -> Vec<Pubkey> {
        let mut pubkeys = vec![];
        let range_max: usize = std::cmp::min(
            self.keypair_action_config
                .max_output_accounts
                .unwrap_or(self.indexer.get_state_merkle_trees().len() as u64),
            self.indexer.get_state_merkle_trees().len() as u64,
        ) as usize;

        for _ in 0..num {
            let index = Self::safe_gen_range(&mut self.rng, 0..range_max, 0);
            let bundle = &self.indexer.get_state_merkle_trees()[index];
            let accounts = &bundle.accounts;

            // For batched trees we need to use the output queue
            if bundle.tree_type == TreeType::StateV2 {
                pubkeys.push(accounts.nullifier_queue);
            } else {
                pubkeys.push(accounts.merkle_tree);
            }
        }
        pubkeys.sort();
        pubkeys
    }

    pub fn get_address_merkle_tree_pubkeys(
        &mut self,
        num: u64,
        batched: bool,
    ) -> (Vec<Pubkey>, Vec<Pubkey>) {
        let mut pubkeys = vec![];
        let mut queue_pubkeys = vec![];
        let mut version = 0;
        for i in 0..num {
            let index = Self::safe_gen_range(
                &mut self.rng,
                0..self
                    .indexer
                    .get_address_merkle_trees()
                    .iter()
                    .filter(|x| x.accounts.merkle_tree != x.accounts.queue)
                    .collect::<Vec<_>>()
                    .len(),
                0,
            );
            let accounts = if !batched {
                &self
                    .indexer
                    .get_address_merkle_trees()
                    .iter()
                    .filter(|x| x.accounts.merkle_tree != x.accounts.queue)
                    .collect::<Vec<_>>()[index]
                    .accounts
            } else {
                &self
                    .indexer
                    .get_address_merkle_trees()
                    .iter()
                    .filter(|x| x.accounts.merkle_tree == x.accounts.queue)
                    .collect::<Vec<_>>()[index]
                    .accounts
            };
            let local_version = if accounts.merkle_tree == accounts.queue {
                2
            } else {
                1
            };
            // Versions of all trees must be consistent
            // if selected trees version is different reuse the first tree
            if i == 0 {
                version = local_version;
            }
            if version != local_version {
                pubkeys.push(pubkeys[0]);
                queue_pubkeys.push(queue_pubkeys[0]);
            } else {
                pubkeys.push(accounts.merkle_tree);
                queue_pubkeys.push(accounts.queue);
            }
        }
        (pubkeys, queue_pubkeys)
    }

    pub async fn select_random_compressed_token_accounts(
        &mut self,
        user: &Pubkey,
    ) -> (Pubkey, Vec<TokenDataWithMerkleContext>) {
        self.select_random_compressed_token_accounts_delegated(user, false, None, false)
            .await
    }

    pub async fn select_random_compressed_token_accounts_frozen(
        &mut self,
        user: &Pubkey,
    ) -> (Pubkey, Vec<TokenDataWithMerkleContext>) {
        self.select_random_compressed_token_accounts_delegated(user, false, None, true)
            .await
    }

    pub async fn select_random_compressed_token_accounts_delegated(
        &mut self,
        user: &Pubkey,
        delegated: bool,
        delegate: Option<Pubkey>,
        frozen: bool,
    ) -> (Pubkey, Vec<TokenDataWithMerkleContext>) {
        let user_token_accounts = &mut self
            .indexer
            .get_compressed_token_accounts_by_owner(user, None, None)
            .await
            .unwrap();
        // clean up dust so that we don't run into issues that account balances are too low
        user_token_accounts
            .value
            .items
            .retain(|t| t.token.amount > 1000);
        let mut token_accounts_with_mint: Vec<TokenDataWithMerkleContext>;
        let mint;
        let tree_version;
        if user_token_accounts.value.items.is_empty() {
            mint = self.indexer.get_token_compressed_accounts()[self
                .rng
                .gen_range(0..self.indexer.get_token_compressed_accounts().len())]
            .token_data
            .mint;
            let number_of_compressed_accounts = Self::safe_gen_range(&mut self.rng, 1..8, 1);
            let bundle = &self.indexer.get_state_merkle_trees()[0];
            let mt_pubkey = bundle.accounts.merkle_tree;
            tree_version = bundle.tree_type;
            mint_tokens_helper(
                &mut self.rpc,
                &mut self.indexer,
                &mt_pubkey,
                &self.payer,
                &mint,
                vec![
                    Self::safe_gen_range(&mut self.rng, 100_000..1_000_000, 100_000);
                    number_of_compressed_accounts
                ],
                vec![*user; number_of_compressed_accounts],
            )
            .await;
            // filter for token accounts with the same version and mint
            token_accounts_with_mint = self
                .indexer
                .get_compressed_token_accounts_by_owner(user, None, None)
                .await
                .unwrap()
                .value
                .items
                .iter()
                .filter(|token_account| {
                    let version = self
                        .indexer
                        .get_state_merkle_trees()
                        .iter()
                        .find(|x| x.accounts.merkle_tree == token_account.account.tree_info.tree)
                        .unwrap()
                        .tree_type;
                    token_account.token.mint == mint && tree_version == version
                })
                .cloned()
                .map(|account| account.into())
                .collect::<Vec<TokenDataWithMerkleContext>>();
        } else {
            let token_account = &user_token_accounts.value.items
                [Self::safe_gen_range(&mut self.rng, 0..user_token_accounts.value.items.len(), 0)];
            mint = token_account.token.mint;
            tree_version = self
                .indexer
                .get_state_merkle_trees()
                .iter()
                .find(|x| x.accounts.merkle_tree == token_account.account.tree_info.tree)
                .unwrap()
                .tree_type;

            token_accounts_with_mint = user_token_accounts
                .value
                .items
                .iter()
                .filter(|token_account| {
                    let version = self
                        .indexer
                        .get_state_merkle_trees()
                        .iter()
                        .find(|x| x.accounts.merkle_tree == token_account.account.tree_info.tree)
                        .unwrap()
                        .tree_type;
                    token_account.token.mint == mint && tree_version == version
                })
                .map(|token_account| (*token_account).clone().into())
                .collect::<Vec<TokenDataWithMerkleContext>>();
        }
        if delegated {
            token_accounts_with_mint = token_accounts_with_mint
                .iter()
                .filter(|token_account| token_account.token_data.delegate.is_some())
                .map(|token_account| (*token_account).clone())
                .collect::<Vec<TokenDataWithMerkleContext>>();
            if token_accounts_with_mint.is_empty() {
                return (mint, Vec::new());
            }
        }
        if let Some(delegate) = delegate {
            token_accounts_with_mint = token_accounts_with_mint
                .iter()
                .filter(|token_account| token_account.token_data.delegate.unwrap() == delegate)
                .map(|token_account| (*token_account).clone())
                .collect::<Vec<TokenDataWithMerkleContext>>();
        }
        if frozen {
            token_accounts_with_mint = token_accounts_with_mint
                .iter()
                .filter(|token_account| token_account.token_data.state == AccountState::Frozen)
                .map(|token_account| (*token_account).clone())
                .collect::<Vec<TokenDataWithMerkleContext>>();
            if token_accounts_with_mint.is_empty() {
                return (mint, Vec::new());
            }
        } else {
            token_accounts_with_mint = token_accounts_with_mint
                .iter()
                .filter(|token_account| token_account.token_data.state == AccountState::Initialized)
                .map(|token_account| (*token_account).clone())
                .collect::<Vec<TokenDataWithMerkleContext>>();
        }
        let range_end = if token_accounts_with_mint.len() == 1 {
            1
        } else if !token_accounts_with_mint.is_empty() {
            self.rng
                .gen_range(1..std::cmp::min(token_accounts_with_mint.len(), 4))
        } else {
            return (mint, Vec::new());
        };
        let mut get_random_subset_of_token_accounts =
            token_accounts_with_mint[0..range_end].to_vec();
        // Sorting input and output Merkle tree pubkeys the same way so the pubkey indices do not get out of order
        get_random_subset_of_token_accounts.sort_by(|a, b| {
            a.compressed_account
                .merkle_context
                .merkle_tree_pubkey
                .to_bytes()
                .cmp(
                    &b.compressed_account
                        .merkle_context
                        .merkle_tree_pubkey
                        .to_bytes(),
                )
        });
        (mint, get_random_subset_of_token_accounts)
    }
}

// Configures probabilities for keypair actions
// default sol configuration is all sol actions enabled with 0.5 probability
pub struct KeypairActionConfig {
    pub compress_sol: Option<f64>,
    pub decompress_sol: Option<f64>,
    pub transfer_sol: Option<f64>,
    pub create_address: Option<f64>,
    pub compress_spl: Option<f64>,
    pub decompress_spl: Option<f64>,
    pub mint_spl: Option<f64>,
    pub transfer_spl: Option<f64>,
    pub max_output_accounts: Option<u64>,
    pub fee_assert: bool,
    pub approve_spl: Option<f64>,
    pub revoke_spl: Option<f64>,
    pub freeze_spl: Option<f64>,
    pub thaw_spl: Option<f64>,
    pub burn_spl: Option<f64>,
}

impl KeypairActionConfig {
    pub fn inclusion(&self) -> bool {
        self.transfer_sol.is_some() || self.transfer_spl.is_some()
    }

    pub fn non_inclusion(&self) -> bool {
        self.create_address.is_some()
    }

    pub fn sol_default() -> Self {
        Self {
            compress_sol: Some(0.5),
            decompress_sol: Some(0.5),
            transfer_sol: Some(0.5),
            create_address: None,
            compress_spl: None,
            decompress_spl: None,
            mint_spl: None,
            transfer_spl: None,
            max_output_accounts: None,
            fee_assert: true,
            approve_spl: None,
            revoke_spl: None,
            freeze_spl: None,
            thaw_spl: None,
            burn_spl: None,
        }
    }

    pub fn spl_default() -> Self {
        Self {
            compress_sol: None,
            decompress_sol: None,
            transfer_sol: None,
            create_address: None,
            compress_spl: Some(0.7),
            decompress_spl: Some(0.5),
            mint_spl: None,
            transfer_spl: Some(0.5),
            max_output_accounts: Some(10),
            fee_assert: true,
            approve_spl: Some(0.5),
            revoke_spl: Some(0.5),
            freeze_spl: Some(0.5),
            thaw_spl: Some(0.5),
            burn_spl: Some(0.5),
        }
    }

    pub fn all_default() -> Self {
        Self {
            compress_sol: Some(0.5),
            decompress_sol: Some(1.0),
            transfer_sol: Some(1.0),
            create_address: Some(0.2),
            compress_spl: Some(0.7),
            decompress_spl: Some(0.5),
            mint_spl: None,
            transfer_spl: Some(0.5),
            max_output_accounts: Some(10),
            fee_assert: true,
            approve_spl: Some(0.7),
            revoke_spl: Some(0.7),
            freeze_spl: Some(0.7),
            thaw_spl: Some(0.7),
            burn_spl: Some(0.7),
        }
    }
    pub fn all_default_no_fee_assert() -> Self {
        Self {
            compress_sol: Some(0.5),
            decompress_sol: Some(1.0),
            transfer_sol: Some(1.0),
            create_address: Some(0.2),
            compress_spl: Some(0.7),
            decompress_spl: Some(0.5),
            mint_spl: None,
            transfer_spl: Some(0.5),
            max_output_accounts: Some(10),
            fee_assert: false,
            approve_spl: Some(0.7),
            revoke_spl: Some(0.7),
            freeze_spl: Some(0.7),
            thaw_spl: Some(0.7),
            burn_spl: Some(0.7),
        }
    }

    pub fn test_default() -> Self {
        Self {
            compress_sol: Some(1.0),
            decompress_sol: Some(1.0),
            transfer_sol: Some(1.0),
            create_address: Some(1.0),
            compress_spl: Some(0.0),
            decompress_spl: Some(0.0),
            mint_spl: None,
            transfer_spl: Some(1.0),
            max_output_accounts: Some(3),
            fee_assert: true,
            approve_spl: None,
            revoke_spl: None,
            freeze_spl: None,
            thaw_spl: None,
            burn_spl: None,
        }
    }

    pub fn test_forester_default() -> Self {
        Self {
            compress_sol: Some(0.0),
            decompress_sol: Some(0.0),
            transfer_sol: Some(1.0),
            create_address: None,
            compress_spl: None,
            decompress_spl: None,
            mint_spl: None,
            transfer_spl: None,
            max_output_accounts: Some(3),
            fee_assert: true,
            approve_spl: None,
            revoke_spl: None,
            freeze_spl: None,
            thaw_spl: None,
            burn_spl: None,
        }
    }
}

// Configures probabilities for general actions
pub struct GeneralActionConfig {
    pub add_keypair: Option<f64>,
    pub create_state_mt: Option<f64>,
    pub create_address_mt: Option<f64>,
    pub nullify_compressed_accounts: Option<f64>,
    pub empty_address_queue: Option<f64>,
    pub rollover: Option<f64>,
    pub add_forester: Option<f64>,
    /// TODO: add this
    /// Creates one infinte epoch
    pub disable_epochs: bool,
}
impl Default for GeneralActionConfig {
    fn default() -> Self {
        Self {
            add_keypair: Some(0.3),
            create_state_mt: Some(1.0),
            create_address_mt: Some(1.0),
            nullify_compressed_accounts: Some(0.2),
            empty_address_queue: Some(0.2),
            rollover: None,
            add_forester: None,
            disable_epochs: false,
        }
    }
}

impl GeneralActionConfig {
    pub fn test_forester_default() -> Self {
        Self {
            add_keypair: None,
            create_state_mt: None,
            create_address_mt: None,
            nullify_compressed_accounts: None,
            empty_address_queue: None,
            rollover: None,
            add_forester: None,
            disable_epochs: false,
        }
    }
    pub fn test_with_rollover() -> Self {
        Self {
            add_keypair: Some(0.3),
            create_state_mt: Some(1.0),
            create_address_mt: Some(1.0),
            nullify_compressed_accounts: Some(0.2),
            empty_address_queue: Some(0.2),
            rollover: Some(0.5),
            add_forester: None,
            disable_epochs: false,
        }
    }
}

pub fn to_account_metas_light(
    remaining_accounts: HashMap<light_compressed_account::Pubkey, usize>,
) -> Vec<AccountMeta> {
    let mut remaining_accounts = remaining_accounts
        .iter()
        .map(|(k, i)| {
            (
                AccountMeta {
                    pubkey: (*k).into(),
                    is_signer: false,
                    is_writable: true,
                },
                *i,
            )
        })
        .collect::<Vec<(AccountMeta, usize)>>();
    // hash maps are not sorted so we need to sort manually and collect into a vector again
    remaining_accounts.sort_by_key(|(_, idx)| *idx);
    let remaining_accounts = remaining_accounts
        .iter()
        .map(|(k, _)| k.clone())
        .collect::<Vec<AccountMeta>>();
    remaining_accounts
}
