use std::fmt::{self, Debug, Formatter};

use forester_utils::utils::airdrop_lamports;
use light_client::{
    indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts},
    rpc::{merkle_tree::MerkleTreeExt, RpcError},
};
use light_prover_client::gnark::helpers::{spawn_prover, ProverConfig};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::Signer;

use crate::{
    accounts::{
        initialize::initialize_accounts, test_accounts::TestAccounts, test_keypairs::TestKeypairs,
    },
    indexer::TestIndexer,
    utils::setup_light_programs::setup_light_programs,
    ProgramTestConfig,
};

pub struct LightProgramTest {
    pub context: ProgramTestContext,
    pub indexer: Option<TestIndexer>,
    pub test_accounts: TestAccounts,
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
        let context = setup_light_programs(config.additional_programs.clone()).await?;
        let mut context = Self {
            context,
            indexer: None,
            test_accounts: TestAccounts::get_program_test_test_accounts(),
        };
        let keypairs = TestKeypairs::program_test_default();
        airdrop_lamports(
            &mut context,
            &keypairs.governance_authority.pubkey(),
            100_000_000_000,
        )
        .await?;
        airdrop_lamports(&mut context, &keypairs.forester.pubkey(), 10_000_000_000).await?;
        let test_accounts = initialize_accounts(&mut context, &config, keypairs).await?;
        let batch_size = config
            .v2_state_tree_config
            .as_ref()
            .map(|config| config.output_queue_batch_size as usize);
        context.add_indexer(&test_accounts, batch_size).await?;
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
    pub fn get_state_merkle_tree(&self) -> StateMerkleTreeAccounts {
        self.test_accounts.v1_state_trees[0]
    }

    #[cfg(feature = "v2")]
    pub fn get_state_merkle_tree_v2(
        &self,
    ) -> crate::accounts::test_accounts::StateMerkleTreeAccountsV2 {
        self.test_accounts.v2_state_trees[0]
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
            &self.context.payer,
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
