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

    fn process_transaction(
        &mut self,
        transaction: Transaction,
    ) -> impl std::future::Future<Output = Result<Signature, RpcError>> + Send;

    fn process_transaction_with_context(
        &mut self,
        transaction: Transaction,
    ) -> impl std::future::Future<Output = Result<(Signature, Slot), RpcError>> + Send;

    fn create_and_send_transaction_with_event<T>(
        &mut self,
        instruction: &[Instruction],
        authority: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> impl std::future::Future<Output = Result<Option<(T, Signature, Slot)>, RpcError>> + Send
    where
        T: AnchorDeserialize + Send + Debug;

    fn create_and_send_transaction<'a>(
        &'a mut self,
        instruction: &'a [Instruction],
        payer: &'a Pubkey,
        signers: &'a [&'a Keypair],
    ) -> impl std::future::Future<Output = Result<Signature, RpcError>> + Send + 'a {
        async move {
            let blockhash = self.get_latest_blockhash().await?;
            let transaction = Transaction::new_signed_with_payer(
                instruction,
                Some(payer),
                &signers.to_vec(),
                blockhash,
            );
            let signature = transaction.signatures[0];
            self.process_transaction(transaction).await?;
            Ok(signature)
        }
    }

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

    fn get_slot(&mut self) -> impl std::future::Future<Output = Result<u64, RpcError>> + Send;

    fn airdrop_lamports(
        &mut self,
        to: &Pubkey,
        lamports: u64,
    ) -> impl std::future::Future<Output = Result<Signature, RpcError>> + Send;

    fn get_anchor_account<'a, T: AnchorDeserialize + 'static>(
        &'a mut self,
        pubkey: &'a Pubkey,
    ) -> impl std::future::Future<Output = Result<Option<T>, RpcError>> + Send + 'a {
        async move {
            match self.get_account(*pubkey).await? {
                Some(account) => {
                    let data = T::deserialize(&mut &account.data[8..]).map_err(RpcError::from)?;
                    Ok(Some(data))
                }
                None => Ok(None),
            }
        }
    }

    fn get_balance(
        &mut self,
        pubkey: &Pubkey,
    ) -> impl std::future::Future<Output = Result<u64, RpcError>> + Send;

    fn warp_to_slot(&mut self, slot: Slot) -> Result<(), RpcError>;
}
