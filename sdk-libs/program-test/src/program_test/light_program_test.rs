use std::fmt::{self, Debug, Formatter};

use account_compression::{AddressMerkleTreeAccount, QueueAccount};
use light_client::{
    indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts},
    rpc::{merkle_tree::MerkleTreeExt, RpcError},
};
use light_prover_client::prover::{spawn_prover, ProverConfig};
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

            // TODO: add the same for v2 trees once we have grinded a mainnet keypair.
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
        // Will always start a prover server.
        #[cfg(feature = "devenv")]
        let prover_config = if config.prover_config.is_none() {
            Some(ProverConfig::default())
        } else {
            config.prover_config
        };
        #[cfg(not(feature = "devenv"))]
        let prover_config = if config.with_prover && config.prover_config.is_none() {
            Some(ProverConfig::default())
        } else {
            config.prover_config
        };
        if let Some(ref prover_config) = prover_config {
            spawn_prover(prover_config.clone()).await;
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

    #[cfg(feature = "v2")]
    pub fn get_address_merkle_tree_v2(&self) -> solana_sdk::pubkey::Pubkey {
        self.test_accounts.v2_address_trees[0]
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
