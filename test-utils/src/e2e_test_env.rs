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

// TODO: implement traits for context object and indexer that we can implement with an rpc as well
// context trait: send_transaction -> return transaction result, get_account_info -> return account info
// indexer trait: get_compressed_accounts_by_owner -> return compressed accounts,
// refactor all tests to work with that so that we can run all tests with a test validator and concurrency

use light_compressed_token::token_data::AccountState;
use log::info;
use num_bigint::{BigUint, RandBigInt};
use num_traits::Num;
use rand::distributions::uniform::{SampleRange, SampleUniform};
use rand::prelude::SliceRandom;
use rand::rngs::{StdRng, ThreadRng};
use rand::{Rng, RngCore, SeedableRng};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signature;
use solana_sdk::signer::{SeedDerivable, Signer};
use spl_token::solana_program::native_token::LAMPORTS_PER_SOL;

use account_compression::utils::constants::{
    STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT,
};
use account_compression::{
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeConfig,
    SAFETY_MARGIN,
};
use light_hasher::Poseidon;
use light_indexed_merkle_tree::HIGHEST_ADDRESS_PLUS_ONE;
use light_indexed_merkle_tree::{array::IndexedArray, reference::IndexedMerkleTree};
use light_system_program::sdk::compressed_account::CompressedAccountWithMerkleContext;
use light_utils::bigint::bigint_to_be_bytes_array;
use light_utils::rand::gen_prime;

use crate::address_merkle_tree_config::{
    address_tree_ready_for_rollover, state_tree_ready_for_rollover,
};
use crate::address_tree_rollover::{
    assert_rolled_over_address_merkle_tree_and_queue,
    perform_address_merkle_tree_roll_over_forester, perform_state_merkle_tree_roll_over_forester,
};
use crate::indexer::{
    AddressMerkleTreeAccounts, AddressMerkleTreeBundle, Indexer, StateMerkleTreeAccounts,
    StateMerkleTreeBundle, TokenDataWithContext,
};
use crate::rpc::errors::RpcError;
use crate::rpc::rpc_connection::RpcConnection;
use crate::spl::{
    approve_test, burn_test, compress_test, compressed_transfer_test, create_mint_helper,
    create_token_account, decompress_test, freeze_test, mint_tokens_helper, revoke_test, thaw_test,
};
use crate::state_tree_rollover::assert_rolled_over_pair;
use crate::system_program::{
    compress_sol_test, create_addresses_test, decompress_sol_test, transfer_compressed_sol_test,
};
use crate::test_env::{
    create_address_merkle_tree_and_queue_account, create_state_merkle_tree_and_queue_account,
    EnvAccounts,
};
use crate::test_forester::{empty_address_queue_test, nullify_compressed_accounts};
use crate::transaction_params::{FeeConfig, TransactionParams};
use crate::{airdrop_lamports, AccountZeroCopy};

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
    }
}

pub struct E2ETestEnv<R: RpcConnection, I: Indexer<R>> {
    pub payer: Keypair,
    pub indexer: I,
    pub users: Vec<User>,
    pub mints: Vec<Pubkey>,
    pub rpc: R,
    pub keypair_action_config: KeypairActionConfig,
    pub general_action_config: GeneralActionConfig,
    pub round: u64,
    pub rounds: u64,
    pub rng: StdRng,
    pub stats: Stats,
}

impl<R: RpcConnection, I: Indexer<R>> E2ETestEnv<R, I>
where
    R: RpcConnection,
    I: Indexer<R>,
{
    pub async fn new(
        mut rpc: R,
        mut indexer: I,
        env_accounts: &EnvAccounts,
        keypair_action_config: KeypairActionConfig,
        general_action_config: GeneralActionConfig,
        rounds: u64,
        seed: Option<u64>,
    ) -> Self {
        let payer = rpc.get_payer().insecure_clone();

        airdrop_lamports(&mut rpc, &payer.pubkey(), 1_000_000_000_000)
            .await
            .unwrap();

        airdrop_lamports(&mut rpc, &env_accounts.forester.pubkey(), 1_000_000_000_000)
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
            &env_accounts.merkle_tree_pubkey,
            &payer,
            &mint,
            vec![100_000_000; 1],
            vec![user.keypair.pubkey()],
        )
        .await;
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

    pub async fn get_balance(&mut self, pubkey: &Pubkey) -> u64 {
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
            self.create_state_tree(rollover_threshold).await;
            self.stats.create_state_mt += 1;
        }

        if self.rng.gen_bool(
            self.general_action_config
                .create_address_mt
                .unwrap_or_default(),
        ) {
            self.create_address_tree(rollover_threshold).await;
            self.stats.create_address_mt += 1;
        }

        if self.rng.gen_bool(
            self.general_action_config
                .nullify_compressed_accounts
                .unwrap_or_default(),
        ) {
            let payer = self.indexer.get_payer().insecure_clone();
            for state_tree_bundle in self.indexer.get_state_merkle_trees_mut().iter_mut() {
                println!("\n --------------------------------------------------\n\t\t NULLIFYING LEAVES\n --------------------------------------------------");
                nullify_compressed_accounts(&mut self.rpc, &payer, state_tree_bundle).await;
            }
        }

        if self.rng.gen_bool(
            self.general_action_config
                .empty_address_queue
                .unwrap_or_default(),
        ) {
            let payer = self.indexer.get_payer().insecure_clone();
            for address_merkle_tree_bundle in self.indexer.get_address_merkle_trees_mut().iter_mut()
            {
                println!("\n --------------------------------------------------\n\t\t Empty Address Queue\n --------------------------------------------------");
                empty_address_queue_test(&payer, &mut self.rpc, address_merkle_tree_bundle, false)
                    .await
                    .unwrap();
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

                self.rollover_state_merkle_tree_and_queue(index)
                    .await
                    .unwrap();
                self.stats.rolledover_state_trees += 1;
            }
        }

        for index in 0..self.indexer.get_address_merkle_trees().len() {
            let is_read_for_rollover = address_tree_ready_for_rollover(
                &mut self.rpc,
                self.indexer.get_address_merkle_trees()[index]
                    .accounts
                    .merkle_tree,
            )
            .await;
            if self
                .rng
                .gen_bool(self.general_action_config.rollover.unwrap_or_default())
                && is_read_for_rollover
            {
                println!("\n --------------------------------------------------\n\t\t Rollover Address Merkle Tree\n --------------------------------------------------");
                self.rollover_address_merkle_tree_and_queue(index)
                    .await
                    .unwrap();
                self.stats.rolledover_address_trees += 1;
            }
        }
    }

    pub async fn create_state_tree(&mut self, rollover_threshold: Option<u64>) {
        let merkle_tree_keypair = Keypair::new(); //from_seed(&[self.rng.gen_range(0..255); 32]).unwrap();
        let nullifier_queue_keypair = Keypair::new(); //from_seed(&[self.rng.gen_range(0..255); 32]).unwrap();
        let cpi_context_keypair = Keypair::new();
        let rollover_threshold = if let Some(rollover_threshold) = rollover_threshold {
            Some(rollover_threshold)
        } else if self.rng.gen_bool(0.5) {
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
            let capacity: u32 = gen_prime(&mut self.rng, 1..10000).unwrap();
            NullifierQueueConfig {
                capacity: capacity as u16,
                sequence_threshold: merkle_tree_config.roots_size + SAFETY_MARGIN,
                network_fee: None,
            }
        } else {
            NullifierQueueConfig::default()
        };
        println!("queue config: {:?}", queue_config);
        create_state_merkle_tree_and_queue_account(
            &self.payer,
            true,
            &mut self.rpc,
            &merkle_tree_keypair,
            &nullifier_queue_keypair,
            None,
            1,
            &merkle_tree_config,
            &queue_config,
        )
        .await;
        let merkle_tree = Box::new(light_merkle_tree_reference::MerkleTree::<Poseidon>::new(
            STATE_MERKLE_TREE_HEIGHT as usize,
            STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
        ));
        crate::test_env::init_cpi_context_account(
            &mut self.rpc,
            &merkle_tree_keypair.pubkey(),
            &cpi_context_keypair,
            &self.payer,
        )
        .await;
        let state_tree_account =
            AccountZeroCopy::<account_compression::StateMerkleTreeAccount>::new(
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
                },
                merkle_tree,
            });
        // TODO: Add assert
    }

    pub async fn create_address_tree(&mut self, rollover_threshold: Option<u64>) {
        let merkle_tree_keypair = Keypair::new();
        let nullifier_queue_keypair = Keypair::new();
        let rollover_threshold = if let Some(rollover_threshold) = rollover_threshold {
            Some(rollover_threshold)
        } else if self.rng.gen_bool(0.5) {
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
        } else {
            (
                AddressMerkleTreeConfig::default(),
                AddressQueueConfig::default(),
            )
        };
        println!("config: {:?}", config);

        create_address_merkle_tree_and_queue_account(
            &self.payer,
            true,
            &mut self.rpc,
            &merkle_tree_keypair,
            &nullifier_queue_keypair,
            None,
            &config,
            &address_config,
            self.indexer.get_address_merkle_trees().len() as u64,
        )
        .await;
        let init_value = BigUint::from_str_radix(HIGHEST_ADDRESS_PLUS_ONE, 10).unwrap();
        let mut merkle_tree = Box::new(
            IndexedMerkleTree::<Poseidon, usize>::new(
                STATE_MERKLE_TREE_HEIGHT as usize,
                STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
            )
            .unwrap(),
        );
        let mut indexed_array = Box::<IndexedArray<Poseidon, usize>>::default();
        merkle_tree.append(&init_value, &mut indexed_array).unwrap();

        let queue_account = AccountZeroCopy::<account_compression::QueueAccount>::new(
            &mut self.rpc,
            nullifier_queue_keypair.pubkey(),
        )
        .await;
        self.indexer
            .get_address_merkle_trees_mut()
            .push(AddressMerkleTreeBundle {
                rollover_fee: queue_account
                    .deserialized()
                    .metadata
                    .rollover_metadata
                    .rollover_fee as i64,
                accounts: AddressMerkleTreeAccounts {
                    merkle_tree: merkle_tree_keypair.pubkey(),
                    queue: nullifier_queue_keypair.pubkey(),
                },
                merkle_tree,
                indexed_array,
            });
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
            self.create_address(None).await;
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

    pub async fn transfer_sol_deterministic(
        &mut self,
        from: &Keypair,
        to: &Pubkey,
        tree_index: Option<usize>,
    ) -> Result<Signature, RpcError> {
        let input_compressed_accounts = self.get_compressed_sol_accounts(&from.pubkey());
        let output_merkle_tree = self.indexer.get_state_merkle_trees()[tree_index.unwrap_or(0)]
            .accounts
            .merkle_tree;
        let recipients = vec![*to];
        transfer_compressed_sol_test(
            &mut self.rpc,
            &mut self.indexer,
            from,
            input_compressed_accounts.as_slice(),
            recipients.as_slice(),
            &[output_merkle_tree],
            Some(TransactionParams {
                num_new_addresses: 0,
                num_input_compressed_accounts: input_compressed_accounts.len() as u8,
                num_output_compressed_accounts: 1u8,
                compress: 0,
                fee_config: FeeConfig::default(),
            }),
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
                    num_input_compressed_accounts: input_compressed_accounts.len() as u8,
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
                    num_input_compressed_accounts: input_compressed_accounts.len() as u8,
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
            self.stats.sol_decompress += 1;
        }
    }

    pub async fn compress_sol_deterministic(
        &mut self,
        from: &Keypair,
        amount: u64,
        tree_index: Option<usize>,
    ) {
        let input_compressed_accounts = self.get_compressed_sol_accounts(&from.pubkey());
        let output_merkle_tree = self.indexer.get_state_merkle_trees()[tree_index.unwrap_or(0)]
            .accounts
            .merkle_tree;
        let transaction_parameters = if self.keypair_action_config.fee_assert {
            Some(TransactionParams {
                num_new_addresses: 0,
                num_input_compressed_accounts: input_compressed_accounts.len() as u8,
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
            from,
            input_compressed_accounts.as_slice(),
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
                num_input_compressed_accounts: input_compressed_accounts.len() as u8,
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

    pub async fn create_address(&mut self, optional_addresses: Option<Vec<Pubkey>>) -> Vec<Pubkey> {
        println!("\n --------------------------------------------------\n\t\t Create Address\n --------------------------------------------------");
        // select number of addresses to create
        let num_addresses = self.rng.gen_range(1..=2);
        // select random address Merkle tree(s)
        let (address_merkle_tree_pubkeys, address_queue_pubkeys) =
            self.get_address_merkle_tree_pubkeys(num_addresses);
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
                num_input_compressed_accounts: 0u8,
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
                num_input_compressed_accounts: token_accounts.len() as u8,
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
                num_input_compressed_accounts: token_accounts.len() as u8,
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
                num_input_compressed_accounts: token_accounts.len() as u8,
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
                num_input_compressed_accounts: token_accounts.len() as u8,
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
        let transaction_paramets = if self.keypair_action_config.fee_assert {
            Some(TransactionParams {
                num_new_addresses: 0u8,
                num_input_compressed_accounts: token_accounts.len() as u8,
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
            transaction_paramets,
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
                num_input_compressed_accounts: token_accounts.len() as u8,
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
                    num_input_compressed_accounts: 0u8,
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
                num_input_compressed_accounts: token_accounts.len() as u8,
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
        )
        .await;
        self.stats.spl_decompress += 1;
    }

    pub async fn rollover_state_merkle_tree_and_queue(
        &mut self,
        index: usize,
    ) -> Result<(), RpcError> {
        let bundle = self.indexer.get_state_merkle_trees()[index].accounts;
        let new_nullifier_queue_keypair = Keypair::new();
        let new_merkle_tree_keypair = Keypair::new();
        // TODO: move into registry program
        let new_cpi_signature_keypair = Keypair::new();
        let fee_payer_balance = self
            .rpc
            .get_balance(&self.indexer.get_payer().pubkey())
            .await
            .unwrap();
        let rollover_signature_and_slot = perform_state_merkle_tree_roll_over_forester(
            self.indexer.get_payer(),
            &mut self.rpc,
            &new_nullifier_queue_keypair,
            &new_merkle_tree_keypair,
            &new_cpi_signature_keypair,
            &bundle.merkle_tree,
            &bundle.nullifier_queue,
        )
        .await
        .unwrap();
        info!("Rollover signature: {:?}", rollover_signature_and_slot.0);
        assert_rolled_over_pair(
            &self.indexer.get_payer().pubkey(),
            &mut self.rpc,
            &fee_payer_balance,
            &bundle.merkle_tree,
            &bundle.nullifier_queue,
            &new_merkle_tree_keypair.pubkey(),
            &new_nullifier_queue_keypair.pubkey(),
            rollover_signature_and_slot.1,
        )
        .await;
        crate::test_env::init_cpi_context_account(
            &mut self.rpc,
            &new_merkle_tree_keypair.pubkey(),
            &new_cpi_signature_keypair,
            &self.payer,
        )
        .await;
        self.indexer
            .get_state_merkle_trees_mut()
            .push(StateMerkleTreeBundle {
                // TODO: fetch correct fee when this property is used
                rollover_fee: 0,
                accounts: StateMerkleTreeAccounts {
                    merkle_tree: new_merkle_tree_keypair.pubkey(),
                    nullifier_queue: new_nullifier_queue_keypair.pubkey(),
                    cpi_context: new_cpi_signature_keypair.pubkey(),
                },
                merkle_tree: Box::new(light_merkle_tree_reference::MerkleTree::<Poseidon>::new(
                    STATE_MERKLE_TREE_HEIGHT as usize,
                    STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
                )),
            });
        Ok(())
    }

    pub async fn rollover_address_merkle_tree_and_queue(
        &mut self,
        index: usize,
    ) -> Result<(), RpcError> {
        let bundle = self.indexer.get_address_merkle_trees()[index].accounts;
        let new_nullifier_queue_keypair = Keypair::new();
        let new_merkle_tree_keypair = Keypair::new();
        let fee_payer_balance = self
            .rpc
            .get_balance(&self.indexer.get_payer().pubkey())
            .await
            .unwrap();
        println!("prior balance {}", fee_payer_balance);
        perform_address_merkle_tree_roll_over_forester(
            self.indexer.get_payer(),
            &mut self.rpc,
            &new_nullifier_queue_keypair,
            &new_merkle_tree_keypair,
            &bundle.merkle_tree,
            &bundle.queue,
        )
        .await?;
        assert_rolled_over_address_merkle_tree_and_queue(
            &self.indexer.get_payer().pubkey(),
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

    pub fn get_random_compressed_sol_accounts(
        &mut self,
        user_index: usize,
    ) -> Vec<CompressedAccountWithMerkleContext> {
        let input_compressed_accounts = self
            .indexer
            .get_compressed_accounts_by_owner(&self.users[user_index].keypair.pubkey());
        let range = std::cmp::min(input_compressed_accounts.len(), 4);
        let number_of_compressed_accounts = Self::safe_gen_range(&mut self.rng, 0..=range, 0);
        input_compressed_accounts[0..number_of_compressed_accounts].to_vec()
    }

    pub fn get_compressed_sol_accounts(
        &self,
        pubkey: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext> {
        self.indexer.get_compressed_accounts_by_owner(pubkey)
    }

    pub fn get_merkle_tree_pubkeys(&mut self, num: u64) -> Vec<Pubkey> {
        let mut pubkeys = vec![];
        for _ in 0..num {
            let range_max: usize = std::cmp::min(
                self.keypair_action_config
                    .max_output_accounts
                    .unwrap_or(self.indexer.get_state_merkle_trees().len() as u64),
                self.indexer.get_state_merkle_trees().len() as u64,
            ) as usize;

            let index = Self::safe_gen_range(&mut self.rng, 0..range_max, 0);
            pubkeys.push(
                self.indexer.get_state_merkle_trees()[index]
                    .accounts
                    .merkle_tree,
            );
        }
        pubkeys.sort();
        pubkeys
    }

    pub fn get_address_merkle_tree_pubkeys(&mut self, num: u64) -> (Vec<Pubkey>, Vec<Pubkey>) {
        let mut pubkeys = vec![];
        let mut queue_pubkeys = vec![];
        for _ in 0..num {
            let index = Self::safe_gen_range(
                &mut self.rng,
                0..self.indexer.get_address_merkle_trees().len(),
                0,
            );
            pubkeys.push(
                self.indexer.get_address_merkle_trees()[index]
                    .accounts
                    .merkle_tree,
            );
            queue_pubkeys.push(
                self.indexer.get_address_merkle_trees()[index]
                    .accounts
                    .queue,
            );
        }
        (pubkeys, queue_pubkeys)
    }

    pub async fn select_random_compressed_token_accounts(
        &mut self,
        user: &Pubkey,
    ) -> (Pubkey, Vec<TokenDataWithContext>) {
        self.select_random_compressed_token_accounts_delegated(user, false, None, false)
            .await
    }

    pub async fn select_random_compressed_token_accounts_frozen(
        &mut self,
        user: &Pubkey,
    ) -> (Pubkey, Vec<TokenDataWithContext>) {
        self.select_random_compressed_token_accounts_delegated(user, false, None, true)
            .await
    }

    pub async fn select_random_compressed_token_accounts_delegated(
        &mut self,
        user: &Pubkey,
        delegated: bool,
        delegate: Option<Pubkey>,
        frozen: bool,
    ) -> (Pubkey, Vec<TokenDataWithContext>) {
        let user_token_accounts = &mut self.indexer.get_compressed_token_accounts_by_owner(user);
        // clean up dust so that we don't run into issues that account balances are too low
        user_token_accounts.retain(|t| t.token_data.amount > 1000);
        let mut token_accounts_with_mint;
        let mint;
        if user_token_accounts.is_empty() {
            mint = self.indexer.get_token_compressed_accounts()[self
                .rng
                .gen_range(0..self.indexer.get_token_compressed_accounts().len())]
            .token_data
            .mint;
            let number_of_compressed_accounts = Self::safe_gen_range(&mut self.rng, 1..8, 1);
            let mt_pubkey = self.indexer.get_state_merkle_trees()[0]
                .accounts
                .merkle_tree;
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
            token_accounts_with_mint = self
                .indexer
                .get_compressed_token_accounts_by_owner(user)
                .iter()
                .filter(|token_account| token_account.token_data.mint == mint)
                .cloned()
                .collect::<Vec<_>>();
        } else {
            mint = user_token_accounts
                [Self::safe_gen_range(&mut self.rng, 0..user_token_accounts.len(), 0)]
            .token_data
            .mint;
            token_accounts_with_mint = user_token_accounts
                .iter()
                .filter(|token_account| token_account.token_data.mint == mint)
                .map(|token_account| (*token_account).clone())
                .collect::<Vec<TokenDataWithContext>>();
        }
        if delegated {
            token_accounts_with_mint = token_accounts_with_mint
                .iter()
                .filter(|token_account| token_account.token_data.delegate.is_some())
                .map(|token_account| (*token_account).clone())
                .collect::<Vec<TokenDataWithContext>>();
            if token_accounts_with_mint.is_empty() {
                return (mint, Vec::new());
            }
        }
        if let Some(delegate) = delegate {
            token_accounts_with_mint = token_accounts_with_mint
                .iter()
                .filter(|token_account| token_account.token_data.delegate.unwrap() == delegate)
                .map(|token_account| (*token_account).clone())
                .collect::<Vec<TokenDataWithContext>>();
        }
        if frozen {
            token_accounts_with_mint = token_accounts_with_mint
                .iter()
                .filter(|token_account| token_account.token_data.state == AccountState::Frozen)
                .map(|token_account| (*token_account).clone())
                .collect::<Vec<TokenDataWithContext>>();
            if token_accounts_with_mint.is_empty() {
                return (mint, Vec::new());
            }
        } else {
            token_accounts_with_mint = token_accounts_with_mint
                .iter()
                .filter(|token_account| token_account.token_data.state == AccountState::Initialized)
                .map(|token_account| (*token_account).clone())
                .collect::<Vec<TokenDataWithContext>>();
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
                .cmp(&b.compressed_account.merkle_context.merkle_tree_pubkey)
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
            transfer_spl: Some(0.0),
            max_output_accounts: Some(10),
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
        }
    }
    pub fn test_with_rollover() -> Self {
        Self {
            add_keypair: Some(0.3),
            create_state_mt: None,
            create_address_mt: None,
            nullify_compressed_accounts: None,
            empty_address_queue: None,
            rollover: None,
        }
    }
}
