use std::fmt::{self, Debug, Formatter};

#[cfg(feature = "devenv")]
use account_compression::QueueAccount;
use light_client::{
    indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts},
    rpc::{merkle_tree::MerkleTreeExt, RpcError},
};
#[cfg(feature = "devenv")]
use light_compressed_account::hash_to_bn254_field_size_be;
use light_prover_client::prover::spawn_prover;
use litesvm::LiteSVM;
#[cfg(feature = "devenv")]
use solana_account::WritableAccount;
use solana_sdk::signature::{Keypair, Signer};

#[cfg(feature = "devenv")]
use crate::accounts::initialize::initialize_accounts;
#[cfg(feature = "devenv")]
use crate::program_test::TestRpc;
use crate::{
    accounts::{test_accounts::TestAccounts, test_keypairs::TestKeypairs},
    indexer::TestIndexer,
    utils::setup_light_programs::setup_light_programs,
    ProgramTestConfig,
};

pub struct LightProgramTest {
    pub config: ProgramTestConfig,
    pub context: LiteSVM,
    pub pre_context: Option<LiteSVM>,
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
    ///   Get an account from the pre-transaction context (before the last transaction)
    pub fn get_pre_transaction_account(
        &self,
        pubkey: &solana_sdk::pubkey::Pubkey,
    ) -> Option<solana_sdk::account::Account> {
        self.pre_context
            .as_ref()
            .and_then(|ctx| ctx.get_account(pubkey))
    }

    pub async fn new(config: ProgramTestConfig) -> Result<LightProgramTest, RpcError> {
        let mut context = setup_light_programs(config.additional_programs.clone())?;
        let payer = Keypair::new();
        context
            .airdrop(&payer.pubkey(), 100_000_000_000_000)
            .expect("Payer airdrop failed.");
        let mut context = Self {
            context,
            pre_context: None,
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

        #[cfg(feature = "devenv")]
        {
            if !config.skip_protocol_init {
                let restore_logs = context.config.no_logs;
                if context.config.skip_startup_logs {
                    context.config.no_logs = true;
                }
                initialize_accounts(&mut context, &config, &keypairs).await?;
                crate::accounts::compressible_config::create_compressible_config(&mut context)
                    .await?;
                if context.config.skip_startup_logs {
                    context.config.no_logs = restore_logs;
                }
                let batch_size = config
                    .v2_state_tree_config
                    .as_ref()
                    .map(|config| config.output_queue_batch_size as usize);
                let test_accounts = context.test_accounts.clone();
                context.add_indexer(&test_accounts, batch_size).await?;

                // Load V1 address tree accounts from JSON files
                {
                    use crate::utils::load_accounts::load_account_from_dir;

                    if context.test_accounts.v1_address_trees.len() != 1 {
                        return Err(RpcError::CustomError(format!(
                            "Expected exactly 1 V1 address tree, found {}. V1 address trees are deprecated and only one is supported.",
                            context.test_accounts.v1_address_trees.len()
                        )));
                    }

                    let address_mt = context.test_accounts.v1_address_trees[0].merkle_tree;
                    let address_queue_pubkey = context.test_accounts.v1_address_trees[0].queue;

                    let tree_account =
                        load_account_from_dir(&address_mt, Some("address_merkle_tree"))?;
                    context
                        .context
                        .set_account(address_mt, tree_account)
                        .map_err(|e| {
                            RpcError::CustomError(format!(
                                "Failed to set V1 address tree account: {}",
                                e
                            ))
                        })?;

                    let queue_account = load_account_from_dir(
                        &address_queue_pubkey,
                        Some("address_merkle_tree_queue"),
                    )?;
                    context
                        .context
                        .set_account(address_queue_pubkey, queue_account)
                        .map_err(|e| {
                            RpcError::CustomError(format!(
                                "Failed to set V1 address queue account: {}",
                                e
                            ))
                        })?;
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
                        let cpi_context_pubkey =
                            context.test_accounts.v1_state_trees[i].cpi_context;

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
                        let merkle_tree_pubkey =
                            context.test_accounts.v2_state_trees[i].merkle_tree;
                        let output_queue_pubkey =
                            context.test_accounts.v2_state_trees[i].output_queue;
                        let cpi_context_pubkey =
                            context.test_accounts.v2_state_trees[i].cpi_context;

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
        }

        #[cfg(not(feature = "devenv"))]
        {
            // Load all accounts from JSON directory
            use crate::utils::load_accounts::load_all_accounts_from_dir;

            let accounts = load_all_accounts_from_dir()?;

            // Extract and verify batch_size from all V2 state tree output queues
            // BatchedQueueMetadata layout: discriminator (8) + QueueMetadata (224) + QueueBatches.num_batches (8) + QueueBatches.batch_size (8)
            const BATCH_SIZE_OFFSET: usize = 240;
            let mut batch_sizes = Vec::new();

            for v2_tree in &context.test_accounts.v2_state_trees {
                if let Some(queue_account) = accounts.get(&v2_tree.output_queue) {
                    if queue_account.data.len() >= BATCH_SIZE_OFFSET + 8 {
                        let bytes: [u8; 8] = queue_account.data
                            [BATCH_SIZE_OFFSET..BATCH_SIZE_OFFSET + 8]
                            .try_into()
                            .map_err(|_| {
                                RpcError::CustomError("Failed to read batch_size bytes".to_string())
                            })?;
                        batch_sizes.push(u64::from_le_bytes(bytes) as usize);
                    }
                }
            }

            // Verify all batch sizes are the same
            if !batch_sizes.is_empty() && !batch_sizes.windows(2).all(|w| w[0] == w[1]) {
                return Err(RpcError::CustomError(format!(
                    "Inconsistent batch_sizes found across output queues: {:?}",
                    batch_sizes
                )));
            }

            let batch_size = batch_sizes.first().copied().unwrap_or(0);

            for (pubkey, account) in accounts {
                context.context.set_account(pubkey, account).map_err(|e| {
                    RpcError::CustomError(format!("Failed to set account {}: {}", pubkey, e))
                })?;
            }

            // Initialize indexer with extracted batch size
            let test_accounts = context.test_accounts.clone();
            context
                .add_indexer(&test_accounts, Some(batch_size))
                .await?;
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
