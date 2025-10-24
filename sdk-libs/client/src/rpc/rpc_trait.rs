use std::fmt::Debug;

use async_trait::async_trait;
use borsh::BorshDeserialize;
use light_event::event::{BatchPublicTransactionEvent, PublicTransactionEvent};
use solana_account::Account;
use solana_clock::Slot;
use solana_commitment_config::CommitmentConfig;
use solana_hash::Hash;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_signature::Signature;
use solana_transaction::Transaction;
use solana_transaction_status_client_types::TransactionStatus;

use super::client::RpcUrl;
use crate::{
    indexer::{Indexer, TreeInfo},
    rpc::errors::RpcError,
};

#[derive(Debug, Clone)]
pub struct LightClientConfig {
    pub url: String,
    pub commitment_config: Option<CommitmentConfig>,
    pub photon_url: Option<String>,
    pub api_key: Option<String>,
    pub fetch_active_tree: bool,
}

impl LightClientConfig {
    pub fn new(url: String, photon_url: Option<String>, api_key: Option<String>) -> Self {
        Self {
            url,
            photon_url,
            api_key,
            commitment_config: Some(CommitmentConfig::confirmed()),
            fetch_active_tree: true,
        }
    }
    pub fn local_no_indexer() -> Self {
        Self {
            url: RpcUrl::Localnet.to_string(),
            commitment_config: Some(CommitmentConfig::confirmed()),
            photon_url: None,
            api_key: None,
            fetch_active_tree: false,
        }
    }

    pub fn local() -> Self {
        Self {
            url: RpcUrl::Localnet.to_string(),
            commitment_config: Some(CommitmentConfig::confirmed()),
            photon_url: Some("http://127.0.0.1:8784".to_string()),
            api_key: None,
            fetch_active_tree: false,
        }
    }

    pub fn devnet(photon_url: Option<String>, api_key: Option<String>) -> Self {
        Self {
            url: RpcUrl::Devnet.to_string(),
            photon_url,
            api_key,
            commitment_config: Some(CommitmentConfig::confirmed()),
            fetch_active_tree: true,
        }
    }
}

#[async_trait]
pub trait Rpc: Send + Sync + Debug + 'static {
    async fn new(config: LightClientConfig) -> Result<Self, RpcError>
    where
        Self: Sized;

    fn should_retry(&self, error: &RpcError) -> bool {
        match error {
            // Do not retry transaction errors.
            RpcError::ClientError(error) => error.kind.get_transaction_error().is_none(),
            // Do not retry signing errors.
            RpcError::SigningError(_) => false,
            _ => true,
        }
    }

    fn get_payer(&self) -> &Keypair;
    fn get_url(&self) -> String;

    async fn health(&self) -> Result<(), RpcError>;

    async fn get_program_accounts(
        &self,
        program_id: &Pubkey,
    ) -> Result<Vec<(Pubkey, Account)>, RpcError>;
    // TODO: add send transaction with config

    async fn confirm_transaction(&self, signature: Signature) -> Result<bool, RpcError>;

    /// Returns an account struct.
    async fn get_account(&self, address: Pubkey) -> Result<Option<Account>, RpcError>;

    /// Returns an a borsh deserialized account.
    /// Deserialization skips the discriminator.
    async fn get_anchor_account<T: BorshDeserialize>(
        &self,
        pubkey: &Pubkey,
    ) -> Result<Option<T>, RpcError> {
        match self.get_account(*pubkey).await? {
            Some(account) => {
                let data = T::deserialize(&mut &account.data[8..]).map_err(RpcError::from)?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> Result<u64, RpcError>;

    async fn airdrop_lamports(&mut self, to: &Pubkey, lamports: u64)
        -> Result<Signature, RpcError>;

    async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64, RpcError>;
    async fn get_latest_blockhash(&mut self) -> Result<(Hash, u64), RpcError>;
    async fn get_slot(&self) -> Result<u64, RpcError>;
    async fn get_transaction_slot(&self, signature: &Signature) -> Result<u64, RpcError>;
    async fn get_signature_statuses(
        &self,
        signatures: &[Signature],
    ) -> Result<Vec<Option<TransactionStatus>>, RpcError>;

    async fn send_transaction(&self, transaction: &Transaction) -> Result<Signature, RpcError>;

    async fn send_transaction_with_config(
        &self,
        transaction: &Transaction,
        config: RpcSendTransactionConfig,
    ) -> Result<Signature, RpcError>;

    async fn process_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<Signature, RpcError>;

    async fn process_transaction_with_context(
        &mut self,
        transaction: Transaction,
    ) -> Result<(Signature, Slot), RpcError>;

    async fn create_and_send_transaction_with_event<T>(
        &mut self,
        instructions: &[Instruction],
        authority: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Option<(T, Signature, Slot)>, RpcError>
    where
        T: BorshDeserialize + Send + Debug;

    async fn create_and_send_transaction<'a>(
        &'a mut self,
        instructions: &'a [Instruction],
        payer: &'a Pubkey,
        signers: &'a [&'a Keypair],
    ) -> Result<Signature, RpcError> {
        let blockhash = self.get_latest_blockhash().await?.0;
        let mut transaction = Transaction::new_with_payer(instructions, Some(payer));
        transaction
            .try_sign(signers, blockhash)
            .map_err(|e| RpcError::SigningError(e.to_string()))?;
        self.process_transaction(transaction).await
    }

    async fn create_and_send_transaction_with_public_event(
        &mut self,
        instruction: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Option<(PublicTransactionEvent, Signature, Slot)>, RpcError>;

    async fn create_and_send_transaction_with_batched_event(
        &mut self,
        instruction: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Option<(Vec<BatchPublicTransactionEvent>, Signature, Slot)>, RpcError>;

    fn indexer(&self) -> Result<&impl Indexer, RpcError>;
    fn indexer_mut(&mut self) -> Result<&mut impl Indexer, RpcError>;

    /// Fetch the latest state tree addresses from the cluster.
    async fn get_latest_active_state_trees(&mut self) -> Result<Vec<TreeInfo>, RpcError>;

    /// Gets state tree infos.
    /// State trees are cached and have to be fetched or set.
    fn get_state_tree_infos(&self) -> Vec<TreeInfo>;

    /// Gets a random state tree info.
    /// State trees are cached and have to be fetched or set.
    /// Returns v1 state trees by default, v2 state trees when v2 feature is enabled.
    fn get_random_state_tree_info(&self) -> Result<TreeInfo, RpcError>;

    /// Gets a random v1 state tree info.
    /// State trees are cached and have to be fetched or set.
    fn get_random_state_tree_info_v1(&self) -> Result<TreeInfo, RpcError>;

    fn get_address_tree_v1(&self) -> TreeInfo;

    fn get_address_tree_v2(&self) -> TreeInfo;
}
