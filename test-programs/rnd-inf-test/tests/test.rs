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

// Debug strategy:
// - recreate the same Merkle trees in a test
//   - with appends
//   - with appends and nullifications
const LEAVES: [[u8; 32]; 16] = [
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    [
        11, 36, 94, 177, 195, 5, 4, 35, 75, 253, 31, 235, 68, 201, 79, 197, 199, 23, 214, 86, 196,
        2, 41, 249, 246, 138, 184, 248, 245, 66, 184, 244,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    [
        34, 229, 118, 4, 68, 219, 118, 228, 117, 70, 150, 93, 208, 215, 51, 243, 123, 48, 39, 228,
        206, 194, 200, 232, 35, 133, 166, 222, 118, 217, 122, 228,
    ],
    [
        24, 61, 159, 11, 70, 12, 177, 252, 244, 238, 130, 73, 202, 69, 102, 83, 33, 103, 82, 66,
        83, 191, 149, 187, 141, 111, 253, 110, 49, 5, 47, 151,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    [
        36, 131, 231, 53, 12, 14, 62, 144, 170, 248, 90, 226, 125, 178, 99, 87, 101, 226, 179, 43,
        110, 130, 233, 194, 112, 209, 74, 219, 154, 48, 41, 148,
    ],
    [
        12, 110, 79, 229, 117, 215, 178, 45, 227, 65, 183, 14, 91, 45, 170, 232, 126, 71, 37, 211,
        160, 77, 148, 223, 50, 144, 134, 232, 83, 159, 131, 62,
    ],
    [
        28, 57, 110, 171, 41, 144, 47, 162, 132, 221, 102, 100, 30, 69, 249, 176, 87, 134, 133,
        207, 250, 166, 139, 16, 73, 39, 11, 139, 158, 182, 43, 68,
    ],
    [
        25, 88, 170, 121, 91, 234, 185, 213, 24, 92, 209, 146, 109, 134, 118, 242, 74, 218, 69, 28,
        87, 154, 207, 86, 218, 48, 182, 206, 8, 9, 35, 240,
    ],
];
#[test]
fn merkle_tree_append_test() {
    let mut ref_mt = light_merkle_tree_reference::MerkleTree::<light_hasher::Poseidon>::new(26, 10);
    let mut con_mt =
        light_concurrent_merkle_tree::ConcurrentMerkleTree26::<Poseidon>::new(26, 1400, 2400, 10)
            .unwrap();
    con_mt.init().unwrap();
    assert_eq!(ref_mt.root(), con_mt.root().unwrap());
    for leaf in LEAVES.iter() {
        ref_mt.append(leaf).unwrap();
        // let change_log_index = con_mt.changelog_index();
        con_mt.append(leaf).unwrap();
        assert_eq!(ref_mt.root(), con_mt.root().unwrap());
    }
}
// leaves with nullification
// Option: is none means append, Some(1) means nullify leaf in index 1
const LEAVES_NON_NULL: [([u8; 32], Option<usize>); 25] = [
    (
        [
            9, 207, 75, 159, 247, 170, 46, 154, 178, 197, 60, 83, 191, 240, 137, 41, 36, 54, 242,
            50, 43, 48, 56, 220, 154, 217, 138, 19, 152, 123, 86, 8,
        ],
        None,
    ),
    (
        [
            40, 10, 138, 159, 12, 188, 226, 84, 188, 92, 250, 11, 94, 240, 77, 158, 69, 219, 175,
            48, 248, 181, 216, 200, 54, 38, 12, 224, 155, 40, 23, 32,
        ],
        None,
    ),
    (
        [
            11, 36, 94, 177, 195, 5, 4, 35, 75, 253, 31, 235, 68, 201, 79, 197, 199, 23, 214, 86,
            196, 2, 41, 249, 246, 138, 184, 248, 245, 66, 184, 244,
        ],
        None,
    ),
    (
        [
            29, 3, 221, 195, 235, 46, 139, 171, 137, 7, 36, 118, 178, 198, 52, 20, 10, 131, 164, 5,
            116, 187, 118, 186, 34, 193, 46, 6, 5, 144, 82, 4,
        ],
        None,
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(0),
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(1),
    ),
    (
        [
            6, 146, 149, 76, 49, 159, 84, 164, 203, 159, 181, 165, 21, 204, 111, 149, 87, 255, 46,
            82, 162, 181, 99, 178, 247, 27, 166, 174, 212, 39, 163, 106,
        ],
        None,
    ),
    (
        [
            19, 135, 28, 172, 63, 129, 175, 101, 201, 97, 135, 147, 18, 78, 152, 243, 15, 154, 120,
            153, 92, 46, 245, 82, 67, 32, 224, 141, 89, 149, 162, 228,
        ],
        None,
    ),
    (
        [
            4, 93, 251, 40, 246, 136, 132, 20, 175, 98, 3, 186, 159, 251, 128, 159, 219, 172, 67,
            20, 69, 19, 66, 193, 232, 30, 121, 19, 193, 177, 143, 6,
        ],
        None,
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(3),
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(4),
    ),
    (
        [
            34, 229, 118, 4, 68, 219, 118, 228, 117, 70, 150, 93, 208, 215, 51, 243, 123, 48, 39,
            228, 206, 194, 200, 232, 35, 133, 166, 222, 118, 217, 122, 228,
        ],
        None,
    ),
    (
        [
            24, 61, 159, 11, 70, 12, 177, 252, 244, 238, 130, 73, 202, 69, 102, 83, 33, 103, 82,
            66, 83, 191, 149, 187, 141, 111, 253, 110, 49, 5, 47, 151,
        ],
        None,
    ),
    (
        [
            29, 239, 118, 17, 75, 98, 148, 167, 142, 190, 223, 175, 98, 255, 153, 111, 127, 169,
            62, 234, 90, 89, 90, 70, 218, 161, 233, 150, 89, 173, 19, 1,
        ],
        None,
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(6),
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(5),
    ),
    (
        [
            45, 31, 195, 30, 201, 235, 73, 88, 57, 130, 35, 53, 202, 191, 20, 156, 125, 123, 37,
            49, 154, 194, 124, 157, 198, 236, 233, 25, 195, 174, 157, 31,
        ],
        None,
    ),
    (
        [
            5, 59, 32, 123, 40, 100, 50, 132, 2, 194, 104, 95, 21, 23, 52, 56, 125, 198, 102, 210,
            24, 44, 99, 255, 185, 255, 151, 249, 67, 167, 189, 85,
        ],
        None,
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(9),
    ),
    (
        [
            36, 131, 231, 53, 12, 14, 62, 144, 170, 248, 90, 226, 125, 178, 99, 87, 101, 226, 179,
            43, 110, 130, 233, 194, 112, 209, 74, 219, 154, 48, 41, 148,
        ],
        None,
    ),
    (
        [
            12, 110, 79, 229, 117, 215, 178, 45, 227, 65, 183, 14, 91, 45, 170, 232, 126, 71, 37,
            211, 160, 77, 148, 223, 50, 144, 134, 232, 83, 159, 131, 62,
        ],
        None,
    ),
    (
        [
            28, 57, 110, 171, 41, 144, 47, 162, 132, 221, 102, 100, 30, 69, 249, 176, 87, 134, 133,
            207, 250, 166, 139, 16, 73, 39, 11, 139, 158, 182, 43, 68,
        ],
        None,
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(11),
    ),
    (
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        Some(10),
    ),
    (
        [
            25, 88, 170, 121, 91, 234, 185, 213, 24, 92, 209, 146, 109, 134, 118, 242, 74, 218, 69,
            28, 87, 154, 207, 86, 218, 48, 182, 206, 8, 9, 35, 240,
        ],
        None,
    ),
];

// Its not the canopy
const HEIGHT: usize = 26;
#[test]
fn merkle_tree_test_with_nullification() {
    let mut ref_mt =
        light_merkle_tree_reference::MerkleTree::<light_hasher::Keccak>::new(HEIGHT, 0);
    let mut con_mt =
        light_concurrent_merkle_tree::ConcurrentMerkleTree26::<light_hasher::Keccak>::new(
            HEIGHT, 1400, 2400, 0,
        )
        .unwrap();
    let mut spl_concurrent_mt =
        spl_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree::<HEIGHT, 256>::new();
    spl_concurrent_mt.initialize().unwrap();
    con_mt.init().unwrap();
    assert_eq!(ref_mt.root(), con_mt.root().unwrap());
    for (i, leaf) in LEAVES_NON_NULL.iter().enumerate() {
        match leaf.1 {
            Some(index) => {
                let change_log_index = con_mt.changelog_index();
                let mut proof = ref_mt.get_proof_of_leaf(index, false).unwrap();
                let old_leaf = ref_mt.leaf(index);
                let current_root = con_mt.root().unwrap();
                spl_concurrent_mt
                    .set_leaf(
                        current_root,
                        old_leaf,
                        [0u8; 32],
                        proof.to_array::<HEIGHT>().unwrap().as_slice(),
                        index.try_into().unwrap(),
                    )
                    .unwrap();
                println!("\n\nconcurrent update --------------------------------------------");

                con_mt
                    .update(
                        change_log_index,
                        &old_leaf,
                        &[0u8; 32],
                        index,
                        &mut proof,
                        true,
                    )
                    .unwrap();
                println!("\n\n reference update --------------------------------------------");

                ref_mt.update(&[0u8; 32], index).unwrap();
            }
            None => {
                println!("\n\nconcurrent append --------------------------------------------");
                con_mt.append(&leaf.0).unwrap();
                println!("\n\n reference append --------------------------------------------");
                ref_mt.append(&leaf.0).unwrap();
                spl_concurrent_mt.append(leaf.0).unwrap();
            }
        }
        println!("i = {}", i);
        assert_eq!(spl_concurrent_mt.get_root(), ref_mt.root());
        assert_eq!(spl_concurrent_mt.get_root(), con_mt.root().unwrap());
        assert_eq!(ref_mt.root(), con_mt.root().unwrap());
    }
}

use account_compression::utils::constants::{
    STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT,
};

use light_hasher::Poseidon;
use light_test_utils::airdrop_lamports;
use light_test_utils::spl::{
    create_token_account, decompress_test, mint_tokens_helper, perform_compressed_transfer_test,
};
use light_test_utils::test_env::create_state_merkle_tree_and_queue_account;
use light_test_utils::test_forester::nullify_compressed_accounts;
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

        if self.rng.gen_bool(
            self.general_action_config
                .nullify_compressed_accounts
                .unwrap_or_default(),
        ) {
            for (state_merkle_tree_accounts, merkle_tree) in
                self.indexer.state_merkle_trees.iter_mut()
            {
                nullify_compressed_accounts(
                    &mut self.context,
                    &self.payer,
                    state_merkle_tree_accounts,
                    merkle_tree,
                )
                .await;
            }
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
            println!("\n --------------------------------------------------\n\t\t Decompress Spl\n --------------------------------------------------");
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
            println!("\n --------------------------------------------------\n\t\t Tranfer Spl\n --------------------------------------------------");
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
    pub nullify_compressed_accounts: Option<f64>,
}
impl Default for GeneralActionConfig {
    fn default() -> Self {
        Self {
            add_keypair: Some(0.0),
            create_state_mt: Some(0.0),
            create_address_mt: Some(0.1),
            nullify_compressed_accounts: Some(1.0),
        }
    }
}
