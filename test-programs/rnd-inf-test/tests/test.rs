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
// - bundle trees, indexer etc in a InfTestEnv struct
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

use account_compression::utils::constants::{
    STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT,
};
use anchor_spl::token;
use light_hasher::Poseidon;
use light_test_utils::airdrop_lamports;
use light_test_utils::spl::{
    create_token_account, decompress_test, mint_tokens_helper, perform_compressed_transfer_test,
};
use light_test_utils::test_env::create_state_merkle_tree_and_queue_account;
use light_test_utils::test_indexer::{
    create_mint_helper, AddressMerkleTreeAccounts, StateMerkleTreeAccounts, TokenDataWithContext,
};
use light_test_utils::{test_env::setup_test_programs_with_accounts, test_indexer::TestIndexer};
use rand::distributions::uniform::{SampleRange, SampleUniform};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use solana_program_test::ProgramTestContext;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::{SeedDerivable, Signer};

#[tokio::test]
async fn test_10() {
    let mut env = InfTestEnv::new(
        KeypairActionConfig::spl_default(),
        Default::default(),
        10000,
        None,
    )
    .await;
    env.execute_rounds().await;
}

pub struct User {
    pub keypair: Keypair,
    // Vector of (mint, token account)
    pub token_accounts: Vec<(Pubkey, Pubkey)>,
}

pub struct InfTestEnv {
    pub payer: Keypair,
    pub indexer: TestIndexer,
    pub users: Vec<User>,
    pub mints: Vec<Pubkey>,
    pub context: ProgramTestContext,
    pub keypair_action_config: KeypairActionConfig,
    pub general_action_config: GeneralActionConfig,
    pub round: u64,
    pub rounds: u64,
    pub rng: StdRng,
}

impl InfTestEnv {
    pub async fn new(
        keypair_action_config: KeypairActionConfig,
        general_action_config: GeneralActionConfig,
        rounds: u64,
        seed: Option<u64>,
    ) -> Self {
        let (mut context, env_accounts) = setup_test_programs_with_accounts(None).await;
        let inclusion = keypair_action_config.transfer_sol.is_some()
            || keypair_action_config.transfer_spl.is_some();
        let non_inclusion = keypair_action_config.create_address.is_some();
        let mut indexer = TestIndexer::new(
            vec![StateMerkleTreeAccounts {
                merkle_tree: env_accounts.merkle_tree_pubkey,
                nullifier_queue: env_accounts.nullifier_queue_pubkey,
                cpi_context: env_accounts.cpi_signature_account_pubkey,
            }],
            vec![AddressMerkleTreeAccounts {
                merkle_tree: env_accounts.address_merkle_tree_pubkey,
                queue: env_accounts.address_merkle_tree_queue_pubkey,
            }],
            context.payer.insecure_clone(),
            inclusion,
            non_inclusion,
            "../../circuit-lib/circuitlib-rs/scripts/prover.sh",
        )
        .await;

        let seed: u64 = match seed {
            Some(seed) => seed,
            None => 42,
        };
        let mut rng = StdRng::seed_from_u64(seed);
        let user = Self::create_user(&mut rng, &mut context).await;
        let payer = context.payer.insecure_clone();
        let mint = create_mint_helper(&mut context, &payer).await;
        mint_tokens_helper(
            &mut context,
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
            context,
            keypair_action_config,
            general_action_config,
            round: 0,
            rounds,
            rng,
            mints: vec![],
        }
    }

    /// Creates a new user with a random keypair and 100 sol
    pub async fn create_user(rng: &mut StdRng, context: &mut ProgramTestContext) -> User {
        let keypair: Keypair = Keypair::from_seed(&[rng.gen_range(0..255); 32]).unwrap();
        airdrop_lamports(context, &keypair.pubkey(), 100_000_000_000)
            .await
            .unwrap();
        User {
            keypair,
            token_accounts: vec![],
        }
    }

    pub async fn execute_rounds(&mut self) {
        for _ in 0..self.rounds {
            self.execute_round().await;
        }
    }

    pub async fn execute_round(&mut self) {
        println!("Round: {}", self.round);
        println!("Users: {}", self.users.len());
        self.activate_general_actions().await;
        let len = self.users.len();
        for i in 0..len {
            self.activate_keypair_actions(&self.users[i].keypair.pubkey())
                .await;
        }
        self.round += 1;
    }

    /// 1. Add a new keypair
    /// 2. Create a new state Merkle tree
    pub async fn activate_general_actions(&mut self) {
        if self
            .rng
            .gen_bool(self.general_action_config.add_keypair.unwrap_or_default())
        {
            let user = Self::create_user(&mut self.rng, &mut self.context).await;
            self.users.push(user);
        }

        if self.rng.gen_bool(
            self.general_action_config
                .create_state_mt
                .unwrap_or_default(),
        ) {
            self.create_state_tree().await;
        }
    }

    async fn create_state_tree(&mut self) {
        let merkle_tree_keypair = Keypair::new(); //from_seed(&[self.rng.gen_range(0..255); 32]).unwrap();
        let nullifier_queue_keypair = Keypair::new(); //from_seed(&[self.rng.gen_range(0..255); 32]).unwrap();
        create_state_merkle_tree_and_queue_account(
            &self.payer,
            &mut self.context,
            &merkle_tree_keypair,
            &nullifier_queue_keypair,
            None,
            1,
        )
        .await;
        let merkle_tree = light_merkle_tree_reference::MerkleTree::<Poseidon>::new(
            STATE_MERKLE_TREE_HEIGHT as usize,
            STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
        );
        self.indexer.state_merkle_trees.push((
            StateMerkleTreeAccounts {
                merkle_tree: merkle_tree_keypair.pubkey(),
                nullifier_queue: nullifier_queue_keypair.pubkey(),
                cpi_context: Pubkey::new_unique(),
            },
            merkle_tree,
        ));
        // TODO: Add assert
    }

    pub fn safe_gen_range<T, R>(rng: &mut StdRng, range: R, empty_fallback: T) -> T
    where
        T: SampleUniform + Copy,
        R: SampleRange<T> + Sized,
    {
        if range.is_empty() {
            return empty_fallback;
        }
        rng.gen_range(range)
    }

    /// 1. Transfer spl tokens between random users
    pub async fn activate_keypair_actions(&mut self, user: &Pubkey) {
        // compress spl
        // check sufficient spl balance

        // decompress spl
        // check sufficient compressed spl balance
        if self
            .rng
            .gen_bool(self.keypair_action_config.decompress_spl.unwrap_or(0.0))
        {
            let (mint, token_accounts) = self.select_random_spl_token_accounts(user).await;
            let user_index = self
                .users
                .iter()
                .position(|u| &u.keypair.pubkey() == user)
                .unwrap();
            let token_account = match self.users[user_index]
                .token_accounts
                .iter()
                .find(|t| t.0 == mint)
            {
                Some(token_account) => token_account.1,
                None => {
                    let token_account_keypair = Keypair::new();
                    create_token_account(
                        &mut self.context,
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
            // decompress
            decompress_test(
                &self.users[user_index].keypair,
                &mut self.context,
                &mut self.indexer,
                token_accounts,
                amount,
                &output_merkle_tree_account[0],
                &token_account,
                None,
            )
            .await;
        }

        // transfer spl
        // check sufficient compressed spl balance
        if self
            .rng
            .gen_bool(self.keypair_action_config.transfer_spl.unwrap_or(0.0))
        {
            let (mint, token_accounts) = self.select_random_spl_token_accounts(user).await;

            let recipients = token_accounts
                .iter()
                .map(|_| {
                    self.users[Self::safe_gen_range(
                        &mut self.rng,
                        0..std::cmp::min(self.users.len(), 6),
                        0,
                    )]
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
            // println!(
            //     "input token accounts: {:?}",
            //     token_accounts
            //         .iter()
            //         .map(|t| t.compressed_account.merkle_context.merkle_tree_pubkey)
            //         .collect::<Vec<_>>()
            // );
            // println!(
            //     "Output Merkle tree pubkeys: {:?}",
            //     output_merkle_tree_pubkeys
            // );
            perform_compressed_transfer_test(
                &self.context.payer.insecure_clone(),
                &mut self.context,
                &mut self.indexer,
                &mint,
                &self
                    .users
                    .iter()
                    .find(|u| &u.keypair.pubkey() == user)
                    .unwrap()
                    .keypair
                    .insecure_clone(),
                &recipients,
                &amounts,
                &token_accounts,
                &output_merkle_tree_pubkeys,
                None,
            )
            .await;
        }
    }

    pub fn get_merkle_tree_pubkeys(&mut self, num: u64) -> Vec<Pubkey> {
        let mut pubkeys = vec![];
        for _ in 0..num {
            let index =
                Self::safe_gen_range(&mut self.rng, 0..self.indexer.state_merkle_trees.len(), 0);
            pubkeys.push(self.indexer.state_merkle_trees[index].0.merkle_tree);
        }
        pubkeys.sort();
        pubkeys
    }

    pub async fn select_random_spl_token_accounts(
        &mut self,
        user: &Pubkey,
    ) -> (Pubkey, Vec<TokenDataWithContext>) {
        let user_token_accounts = &mut self.indexer.get_compressed_token_accounts_by_owner(&user);
        // clean up dust so that we don't run into issues that account balances are too low
        user_token_accounts.retain(|t| t.token_data.amount > 1000);
        let token_accounts_with_mint;
        let mint;
        if user_token_accounts.is_empty() {
            mint = self.indexer.token_compressed_accounts[self
                .rng
                .gen_range(0..self.indexer.token_compressed_accounts.len())]
            .token_data
            .mint;
            let number_of_compressed_accounts = Self::safe_gen_range(&mut self.rng, 1..8, 1);
            let mt_pubkey = self.indexer.state_merkle_trees[0].0.merkle_tree;
            mint_tokens_helper(
                &mut self.context,
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
                .map(|token_account| token_account.clone())
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
        let range_end = if token_accounts_with_mint.len() == 1 {
            1
        } else {
            self.rng
                .gen_range(1..std::cmp::min(token_accounts_with_mint.len(), 4))
        };
        let mut get_random_subset_of_token_accounts = token_accounts_with_mint[0..range_end]
            .iter()
            .map(|token_account| token_account.clone())
            .collect::<Vec<_>>();
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
}

impl KeypairActionConfig {
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
        }
    }

    pub fn spl_default() -> Self {
        Self {
            compress_sol: None,
            decompress_sol: None,
            transfer_sol: None,
            create_address: None,
            compress_spl: None,
            decompress_spl: Some(0.5),
            mint_spl: None,
            transfer_spl: Some(0.5),
        }
    }
}

// Configures probabilities for general actions
// Default is all enabled, with 0.3, 0.1, 0.1 probabilities
pub struct GeneralActionConfig {
    pub add_keypair: Option<f64>,
    pub create_state_mt: Option<f64>,
    pub create_address_mt: Option<f64>,
}
impl Default for GeneralActionConfig {
    fn default() -> Self {
        Self {
            add_keypair: Some(0.0),
            create_state_mt: Some(0.0),
            create_address_mt: Some(0.1),
        }
    }
}
