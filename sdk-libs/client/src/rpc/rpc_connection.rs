use std::fmt::Debug;

use async_trait::async_trait;
use borsh::BorshDeserialize;
use light_utils::instruction::event::PublicTransactionEvent;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_program::{clock::Slot, instruction::Instruction};
use solana_sdk::{
    account::{Account, AccountSharedData},
    commitment_config::CommitmentConfig,
    epoch_info::EpochInfo,
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    transaction::Transaction,
};
use solana_transaction_status::TransactionStatus;

use crate::{
    rate_limiter::RateLimiter, rpc::errors::RpcError, transaction_params::TransactionParams,
};

#[async_trait]
pub trait RpcConnection: Send + Sync + Debug + 'static {
    fn new<U: ToString>(url: U, commitment_config: Option<CommitmentConfig>) -> Self
    where
        Self: Sized;

    fn set_rpc_rate_limiter(&mut self, rate_limiter: RateLimiter);
    fn set_send_tx_rate_limiter(&mut self, rate_limiter: RateLimiter);

    fn rpc_rate_limiter(&self) -> Option<&RateLimiter>;
    fn send_tx_rate_limiter(&self) -> Option<&RateLimiter>;

    async fn check_rpc_rate_limit(&self) {
        if let Some(limiter) = self.rpc_rate_limiter() {
            limiter.acquire_with_wait().await;
        }
    }

    async fn check_send_tx_rrate_limit(&self) {
        if let Some(limiter) = self.send_tx_rate_limiter() {
            limiter.acquire_with_wait().await;
        }
    }

    fn get_payer(&self) -> &Keypair;
    fn get_url(&self) -> String;

    async fn health(&self) -> Result<(), RpcError>;
    async fn get_block_time(&self, slot: u64) -> Result<i64, RpcError>;
    async fn get_epoch_info(&self) -> Result<EpochInfo, RpcError>;

    async fn get_program_accounts(
        &self,
        program_id: &Pubkey,
    ) -> Result<Vec<(Pubkey, Account)>, RpcError>;
    async fn process_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<Signature, RpcError>;
    async fn process_transaction_with_context(
        &mut self,
        transaction: Transaction,
    ) -> Result<(Signature, Slot), RpcError>;

    async fn process_transaction_with_config(
        &mut self,
        transaction: Transaction,
        config: RpcSendTransactionConfig,
    ) -> Result<Signature, RpcError>;

    async fn create_and_send_transaction_with_event<T>(
        &mut self,
        instructions: &[Instruction],
        authority: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(T, Signature, Slot)>, RpcError>
    where
        T: BorshDeserialize + Send + Debug;

    async fn create_and_send_transaction<'a>(
        &'a mut self,
        instructions: &'a [Instruction],
        payer: &'a Pubkey,
        signers: &'a [&'a Keypair],
    ) -> Result<Signature, RpcError> {
        let blockhash = self.get_latest_blockhash().await?;
        let transaction =
            Transaction::new_signed_with_payer(instructions, Some(payer), signers, blockhash);
        self.process_transaction(transaction).await
    }

    async fn confirm_transaction(&self, signature: Signature) -> Result<bool, RpcError>;
    async fn get_account(&mut self, address: Pubkey) -> Result<Option<Account>, RpcError>;
    fn set_account(&mut self, address: &Pubkey, account: &AccountSharedData);
    async fn get_minimum_balance_for_rent_exemption(
        &mut self,
        data_len: usize,
    ) -> Result<u64, RpcError>;
    async fn airdrop_lamports(&mut self, to: &Pubkey, lamports: u64)
        -> Result<Signature, RpcError>;

    async fn get_anchor_account<T: BorshDeserialize>(
        &mut self,
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

    async fn get_balance(&mut self, pubkey: &Pubkey) -> Result<u64, RpcError>;
    async fn get_latest_blockhash(&mut self) -> Result<Hash, RpcError>;
    async fn get_slot(&mut self) -> Result<u64, RpcError>;
    async fn warp_to_slot(&mut self, slot: Slot) -> Result<(), RpcError>;
    async fn send_transaction(&self, transaction: &Transaction) -> Result<Signature, RpcError>;
    async fn send_transaction_with_config(
        &self,
        transaction: &Transaction,
        config: RpcSendTransactionConfig,
    ) -> Result<Signature, RpcError>;
    async fn get_transaction_slot(&mut self, signature: &Signature) -> Result<u64, RpcError>;
    async fn get_signature_statuses(
        &self,
        signatures: &[Signature],
    ) -> Result<Vec<Option<TransactionStatus>>, RpcError>;
    async fn get_block_height(&mut self) -> Result<u64, RpcError>;

    async fn create_and_send_transaction_with_public_event(
        &mut self,
        _instruction: &[Instruction],
        _payer: &Pubkey,
        _signers: &[&Keypair],
        _transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(PublicTransactionEvent, Signature, Slot)>, RpcError> {
        unimplemented!()
    }
}
