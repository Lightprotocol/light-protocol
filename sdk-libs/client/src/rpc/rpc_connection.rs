use std::fmt::Debug;

use async_trait::async_trait;
use borsh::BorshDeserialize;
use light_compressed_account::indexer_event::event::{
    BatchPublicTransactionEvent, PublicTransactionEvent,
};
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
use solana_transaction_status::TransactionStatus;

use super::solana_rpc::SolanaRpcUrl;
use crate::{indexer::Indexer, rpc::errors::RpcError};

#[derive(Debug, Clone)]
pub struct RpcConnectionConfig {
    pub url: String,
    pub commitment_config: Option<CommitmentConfig>,
    pub with_indexer: bool,
}

impl RpcConnectionConfig {
    pub fn new(url: String) -> Self {
        Self {
            url,
            commitment_config: Some(CommitmentConfig::confirmed()),
            with_indexer: true,
        }
    }
    pub fn local_no_indexer() -> Self {
        Self {
            url: SolanaRpcUrl::Localnet.to_string(),
            commitment_config: Some(CommitmentConfig::confirmed()),
            with_indexer: false,
        }
    }

    pub fn local() -> Self {
        Self {
            url: SolanaRpcUrl::Localnet.to_string(),
            commitment_config: Some(CommitmentConfig::confirmed()),
            with_indexer: true,
        }
    }

    pub fn devnet() -> Self {
        Self {
            url: SolanaRpcUrl::Devnet.to_string(),
            commitment_config: Some(CommitmentConfig::confirmed()),
            with_indexer: true,
        }
    }
}

#[async_trait]
pub trait RpcConnection: Send + Sync + Debug + 'static {
    fn new(config: RpcConnectionConfig) -> Self
    where
        Self: Sized;

    fn should_retry(&self, error: &RpcError) -> bool {
        match error {
            // Do not retry transaction errors.
            RpcError::ClientError(error) => error.kind.get_transaction_error().is_none(),
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
        let transaction =
            Transaction::new_signed_with_payer(instructions, Some(payer), signers, blockhash);
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
}
