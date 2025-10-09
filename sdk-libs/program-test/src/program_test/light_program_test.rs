use std::fmt::{self, Debug, Formatter};

use account_compression::{AddressMerkleTreeAccount, QueueAccount};
use light_client::{
    indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts},
    rpc::{merkle_tree::MerkleTreeExt, RpcError},
};
use light_compressed_account::hash_to_bn254_field_size_be;
use light_prover_client::prover::spawn_prover;
use litesvm::LiteSVM;
use solana_account::WritableAccount;
use solana_sdk::signature::{Keypair, Signer};

use crate::{
    accounts::{
        initialize::initialize_accounts, test_accounts::TestAccounts, test_keypairs::TestKeypairs,
    },
    indexer::TestIndexer,
    program_test::TestRpc,
    utils::setup_light_programs::setup_light_programs,
    ProgramTestConfig,
};

pub struct LightProgramTest {
    pub config: ProgramTestConfig,
    pub context: LiteSVM,
    pub indexer: Option<TestIndexer>,
    pub test_accounts: TestAccounts,
    pub payer: Keypair,
    pub transaction_counter: usize,
}

impl LightProgramTest {
    /// Creates ProgramTestContext with light protocol and additional programs.
    ///
    /// Programs:
    /// 1. light program
    /// 2. account_compression program
    /// 3. light_compressed_token program
    /// 4. light_system_program program
    ///
    /// Light Protocol accounts:
    /// 5. creates and initializes governance authority
    /// 6. creates and initializes group authority
    /// 7. registers the light_system_program program with the group authority
    /// 8. initializes Merkle tree owned by
    /// Note:
    /// - registers a forester
    /// - advances to the active phase slot 2
    /// - active phase doesn't end
    pub async fn new(config: ProgramTestConfig) -> Result<LightProgramTest, RpcError> {
        let mut context = setup_light_programs(config.additional_programs.clone())?;
        let payer = Keypair::new();
        context
            .airdrop(&payer.pubkey(), 100_000_000_000_000)
            .expect("Payer airdrop failed.");
        let mut context = Self {
            context,
            indexer: None,
            test_accounts: TestAccounts::get_program_test_test_accounts(),
            payer,
            config: config.clone(),
            transaction_counter: 0,
        };
        let keypairs = TestKeypairs::program_test_default();

        context
            .context
            .airdrop(&keypairs.governance_authority.pubkey(), 100_000_000_000_000)
            .expect("governance_authority airdrop failed.");
        context
            .context
            .airdrop(&keypairs.forester.pubkey(), 10_000_000_000)
            .expect("forester airdrop failed.");

        if !config.skip_protocol_init {
            let restore_logs = context.config.no_logs;
            if context.config.skip_startup_logs {
                context.config.no_logs = true;
            }
            initialize_accounts(&mut context, &config, &keypairs).await?;
            if context.config.skip_startup_logs {
                context.config.no_logs = restore_logs;
            }
            let batch_size = config
                .v2_state_tree_config
                .as_ref()
                .map(|config| config.output_queue_batch_size as usize);
            let test_accounts = context.test_accounts.clone();
            context.add_indexer(&test_accounts, batch_size).await?;

            // ensure that address tree pubkey is amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2
            {
                let address_mt = context.test_accounts.v1_address_trees[0].merkle_tree;
                let address_queue_pubkey = context.test_accounts.v1_address_trees[0].queue;
                let mut account = context
                    .context
                    .get_account(&keypairs.address_merkle_tree.pubkey())
                    .unwrap();
                let merkle_tree_account = bytemuck::from_bytes_mut::<AddressMerkleTreeAccount>(
                    &mut account.data_as_mut_slice()[8..AddressMerkleTreeAccount::LEN],
                );
                merkle_tree_account.metadata.associated_queue = address_queue_pubkey.into();
                context.set_account(address_mt, account);

                let mut account = context
                    .context
                    .get_account(&keypairs.address_merkle_tree_queue.pubkey())
                    .unwrap();
                let queue_account = bytemuck::from_bytes_mut::<QueueAccount>(
                    &mut account.data_as_mut_slice()[8..QueueAccount::LEN],
                );
                queue_account.metadata.associated_merkle_tree = address_mt.into();
                context.set_account(address_queue_pubkey, account);
            }
        }
        // Copy v1 state merkle tree accounts to devnet pubkeys
        {
            let tree_account = context
                .context
                .get_account(&keypairs.state_merkle_tree.pubkey());
            let queue_account = context
                .context
                .get_account(&keypairs.nullifier_queue.pubkey());
            let cpi_account = context
                .context
                .get_account(&keypairs.cpi_context_account.pubkey());

            if let (Some(tree_acc), Some(queue_acc), Some(cpi_acc)) =
                (tree_account, queue_account, cpi_account)
            {
                for i in 0..context.test_accounts.v1_state_trees.len() {
                    let state_mt = context.test_accounts.v1_state_trees[i].merkle_tree;
                    let nullifier_queue_pubkey =
                        context.test_accounts.v1_state_trees[i].nullifier_queue;
                    let cpi_context_pubkey = context.test_accounts.v1_state_trees[i].cpi_context;

                    // Update tree account with correct associated queue
                    let mut tree_account_data = tree_acc.clone();
                    {
                        let merkle_tree_account = bytemuck::from_bytes_mut::<
                            account_compression::StateMerkleTreeAccount,
                        >(
                            &mut tree_account_data.data_as_mut_slice()
                                [8..account_compression::StateMerkleTreeAccount::LEN],
                        );
                        merkle_tree_account.metadata.associated_queue =
                            nullifier_queue_pubkey.into();
                    }
                    context.set_account(state_mt, tree_account_data);

                    // Update queue account with correct associated merkle tree
                    let mut queue_account_data = queue_acc.clone();
                    {
                        let queue_account = bytemuck::from_bytes_mut::<QueueAccount>(
                            &mut queue_account_data.data_as_mut_slice()[8..QueueAccount::LEN],
                        );
                        queue_account.metadata.associated_merkle_tree = state_mt.into();
                    }
                    context.set_account(nullifier_queue_pubkey, queue_account_data);

                    // Update CPI context account with correct associated merkle tree and queue
                    let mut cpi_account_data = cpi_acc.clone();
                    {
                        let associated_merkle_tree_offset = 8 + 32; // discriminator + fee_payer
                        let associated_queue_offset = 8 + 32 + 32; // discriminator + fee_payer + associated_merkle_tree
                        cpi_account_data.data_as_mut_slice()
                            [associated_merkle_tree_offset..associated_merkle_tree_offset + 32]
                            .copy_from_slice(&state_mt.to_bytes());
                        cpi_account_data.data_as_mut_slice()
                            [associated_queue_offset..associated_queue_offset + 32]
                            .copy_from_slice(&nullifier_queue_pubkey.to_bytes());
                    }
                    context.set_account(cpi_context_pubkey, cpi_account_data);
                }
            }
        }
        {
            let address_mt = context.test_accounts.v2_address_trees[0];
            let account = context
                .context
                .get_account(&keypairs.batch_address_merkle_tree.pubkey());
            if let Some(account) = account {
                context.set_account(address_mt, account);
            }
        }
        // Copy batched state merkle tree accounts to devnet pubkeys
        {
            let tree_account = context
                .context
                .get_account(&keypairs.batched_state_merkle_tree.pubkey());
            let queue_account = context
                .context
                .get_account(&keypairs.batched_output_queue.pubkey());
            let cpi_account = context
                .context
                .get_account(&keypairs.batched_cpi_context.pubkey());

            if let (Some(tree_acc), Some(queue_acc), Some(cpi_acc)) =
                (tree_account, queue_account, cpi_account)
            {
                use light_batched_merkle_tree::{
                    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
                };

                for i in 0..context.test_accounts.v2_state_trees.len() {
                    let merkle_tree_pubkey = context.test_accounts.v2_state_trees[i].merkle_tree;
                    let output_queue_pubkey = context.test_accounts.v2_state_trees[i].output_queue;
                    let cpi_context_pubkey = context.test_accounts.v2_state_trees[i].cpi_context;

                    // Update tree account with correct associated queue and hashed pubkey
                    let mut tree_account_data = tree_acc.clone();
                    {
                        let mut tree = BatchedMerkleTreeAccount::state_from_bytes(
                            tree_account_data.data_as_mut_slice(),
                            &merkle_tree_pubkey.into(),
                        )
                        .unwrap();
                        let metadata = tree.get_metadata_mut();
                        metadata.metadata.associated_queue = output_queue_pubkey.into();
                        metadata.hashed_pubkey =
                            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes());
                    }
                    context.set_account(merkle_tree_pubkey, tree_account_data);

                    // Update queue account with correct associated merkle tree and hashed pubkeys
                    let mut queue_account_data = queue_acc.clone();
                    {
                        let mut queue = BatchedQueueAccount::output_from_bytes(
                            queue_account_data.data_as_mut_slice(),
                        )
                        .unwrap();
                        let metadata = queue.get_metadata_mut();
                        metadata.metadata.associated_merkle_tree = merkle_tree_pubkey.into();
                        metadata.hashed_merkle_tree_pubkey =
                            hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes());
                        metadata.hashed_queue_pubkey =
                            hash_to_bn254_field_size_be(&output_queue_pubkey.to_bytes());
                    }
                    context.set_account(output_queue_pubkey, queue_account_data);

                    // Update CPI context account with correct associated merkle tree and queue
                    let mut cpi_account_data = cpi_acc.clone();
                    {
                        let associated_merkle_tree_offset = 8 + 32; // discriminator + fee_payer
                        let associated_queue_offset = 8 + 32 + 32; // discriminator + fee_payer + associated_merkle_tree
                        cpi_account_data.data_as_mut_slice()
                            [associated_merkle_tree_offset..associated_merkle_tree_offset + 32]
                            .copy_from_slice(&merkle_tree_pubkey.to_bytes());
                        cpi_account_data.data_as_mut_slice()
                            [associated_queue_offset..associated_queue_offset + 32]
                            .copy_from_slice(&output_queue_pubkey.to_bytes());
                    }
                    context.set_account(cpi_context_pubkey, cpi_account_data);
                }
            }
        }

        // reset tx counter after program setup.
        context.transaction_counter = 0;

        #[cfg(feature = "devenv")]
        {
            spawn_prover().await;
        }
        #[cfg(not(feature = "devenv"))]
        if config.with_prover {
            spawn_prover().await;
        }

        Ok(context)
    }

    pub fn indexer(&self) -> Result<&TestIndexer, RpcError> {
        self.indexer.as_ref().ok_or(RpcError::IndexerNotInitialized)
    }

    pub fn indexer_mut(&mut self) -> Result<&mut TestIndexer, RpcError> {
        self.indexer.as_mut().ok_or(RpcError::IndexerNotInitialized)
    }

    pub fn test_accounts(&self) -> &TestAccounts {
        &self.test_accounts
    }

    /// Get account pubkeys of one state Merkle tree.
    pub fn get_state_merkle_tree_account(&self) -> StateMerkleTreeAccounts {
        self.test_accounts.v1_state_trees[0]
    }

    pub fn get_address_merkle_tree(&self) -> AddressMerkleTreeAccounts {
        self.test_accounts.v1_address_trees[0]
    }

    pub async fn add_indexer(
        &mut self,
        test_accounts: &TestAccounts,
        batch_size: Option<usize>,
    ) -> Result<(), RpcError> {
        let indexer = TestIndexer::init_from_acounts(
            &self.payer,
            test_accounts,
            batch_size.unwrap_or_default(),
        )
        .await;
        self.indexer = Some(indexer);
        Ok(())
    }

    pub fn clone_indexer(&self) -> Result<TestIndexer, RpcError> {
        Ok((*self
            .indexer
            .as_ref()
            .ok_or(RpcError::IndexerNotInitialized)?)
        .clone())
    }
}

impl MerkleTreeExt for LightProgramTest {}

impl Debug for LightProgramTest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("LightProgramTest")
            .field("context", &"ProgramTestContext")
            .field("indexer", &self.indexer)
            .field("test_accounts", &self.test_accounts)
            .finish()
    }
}
