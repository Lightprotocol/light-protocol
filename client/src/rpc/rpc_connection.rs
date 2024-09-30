use crate::rpc::errors::RpcError;
use crate::transaction_params::TransactionParams;
use async_trait::async_trait;
use borsh::BorshDeserialize;
use solana_program::clock::Slot;
use solana_program::instruction::Instruction;
use solana_sdk::account::{Account, AccountSharedData};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::epoch_info::EpochInfo;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::transaction::Transaction;
use std::fmt::Debug;

#[async_trait]
pub trait RpcConnection: Send + Sync + 'static {
    fn new<U: ToString>(url: U, commitment_config: Option<CommitmentConfig>) -> Self
    where
        Self: Sized;

    async fn get_payer(&self) -> Keypair;
    fn get_url(&self) -> String;

    async fn health(&self) -> Result<(), RpcError>;
    async fn get_block_time(&self, slot: u64) -> Result<i64, RpcError>;
    async fn get_epoch_info(&self) -> Result<EpochInfo, RpcError>;

    async fn get_program_accounts(
        &self,
        program_id: &Pubkey,
    ) -> Result<Vec<(Pubkey, Account)>, RpcError>;
    async fn process_transaction(&self, transaction: Transaction) -> Result<Signature, RpcError>;
    async fn process_transaction_with_context(
        &self,
        transaction: Transaction,
    ) -> Result<(Signature, Slot), RpcError>;

    async fn create_and_send_transaction_with_event<T>(
        &self,
        instructions: &[Instruction],
        authority: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(T, Signature, Slot)>, RpcError>
    where
        T: BorshDeserialize + Send + Debug;

    async fn create_and_send_transaction<'a>(
        &'a self,
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
    async fn get_account(&self, address: Pubkey) -> Result<Option<Account>, RpcError>;
    async fn set_account(&self, address: &Pubkey, account: &AccountSharedData);
    async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> Result<u64, RpcError>;
    async fn airdrop_lamports(&self, to: &Pubkey, lamports: u64) -> Result<Signature, RpcError>;

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

    async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64, RpcError>;
    async fn get_latest_blockhash(&self) -> Result<Hash, RpcError>;
    async fn get_slot(&self) -> Result<u64, RpcError>;
    async fn warp_to_slot(&self, slot: Slot) -> Result<(), RpcError>;
    async fn send_transaction(&self, transaction: &Transaction) -> Result<Signature, RpcError>;
}
