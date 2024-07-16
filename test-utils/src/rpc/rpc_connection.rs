use crate::rpc::errors::RpcError;
use crate::transaction_params::TransactionParams;
use account_compression::initialize_address_merkle_tree::{AnchorDeserialize, Pubkey};
use anchor_lang::solana_program::clock::Slot;
use anchor_lang::solana_program::instruction::Instruction;
use solana_sdk::account::{Account, AccountSharedData};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::transaction::Transaction;
use std::fmt::Debug;

pub trait RpcConnection: Clone + Send + Sync + Debug + 'static {
    fn new<U: ToString>(_url: U, _commitment_config: Option<CommitmentConfig>) -> Self {
        unimplemented!()
    }

    fn create_and_send_transaction_with_event<T>(
        &mut self,
        instruction: &[Instruction],
        authority: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> impl std::future::Future<Output = Result<Option<(T, Signature, u64)>, RpcError>> + Send
    where
        T: AnchorDeserialize + Send + Debug;

    fn create_and_send_transaction(
        &mut self,
        instruction: &[Instruction],
        authority: &Pubkey,
        signers: &[&Keypair],
    ) -> impl std::future::Future<Output = Result<Signature, RpcError>> + Send;

    fn confirm_transaction(
        &mut self,
        transaction: Signature,
    ) -> impl std::future::Future<Output = Result<bool, RpcError>> + Send;

    fn get_payer(&self) -> &Keypair;
    fn get_account(
        &mut self,
        address: Pubkey,
    ) -> impl std::future::Future<Output = Result<Option<Account>, RpcError>> + Send;
    fn set_account(&mut self, address: &Pubkey, account: &AccountSharedData);

    fn get_minimum_balance_for_rent_exemption(
        &mut self,
        data_len: usize,
    ) -> impl std::future::Future<Output = Result<u64, RpcError>> + Send;

    fn get_latest_blockhash(
        &mut self,
    ) -> impl std::future::Future<Output = Result<Hash, RpcError>> + Send;
    fn process_transaction(
        &mut self,
        transaction: Transaction,
    ) -> impl std::future::Future<Output = Result<Signature, RpcError>> + Send;
    fn get_slot(&mut self) -> impl std::future::Future<Output = Result<u64, RpcError>> + Send;
    fn airdrop_lamports(
        &mut self,
        to: &Pubkey,
        lamports: u64,
    ) -> impl std::future::Future<Output = Result<Signature, RpcError>> + Send;

    // TODO: return Result<T, Error>
    fn get_anchor_account<T: AnchorDeserialize>(
        &mut self,
        pubkey: &Pubkey,
    ) -> impl std::future::Future<Output = T> + Send;

    fn get_balance(
        &mut self,
        pubkey: &Pubkey,
    ) -> impl std::future::Future<Output = Result<u64, RpcError>> + Send;

    fn warp_to_slot(&mut self, slot: Slot) -> Result<(), RpcError>;
}
