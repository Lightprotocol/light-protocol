use async_trait::async_trait;
use light_client::rpc::{LightClient, Rpc, RpcError};
use solana_account::Account;
use solana_sdk::{clock::Slot, pubkey::Pubkey};
#[cfg(feature = "devenv")]
use {
    borsh::BorshDeserialize,
    light_client::fee::{assert_transaction_params, TransactionParams},
    light_compressed_account::indexer_event::event::{
        BatchPublicTransactionEvent, PublicTransactionEvent,
    },
    solana_sdk::{
        instruction::Instruction,
        signature::{Keypair, Signature},
    },
    std::{fmt::Debug, marker::Send},
};

use crate::program_test::LightProgramTest;

#[async_trait]
pub trait TestRpc: Rpc + Sized {
    #[cfg(feature = "devenv")]
    async fn create_and_send_transaction_with_batched_event(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<light_client::fee::TransactionParams>,
    ) -> Result<Option<(Vec<BatchPublicTransactionEvent>, Signature, Slot)>, RpcError> {
        let pre_balance = self.get_balance(payer).await?;

        let event = <Self as Rpc>::create_and_send_transaction_with_batched_event(
            self,
            instructions,
            payer,
            signers,
        )
        .await?;

        light_client::fee::assert_transaction_params(
            self,
            payer,
            signers,
            pre_balance,
            transaction_params,
        )
        .await?;

        Ok(event)
    }

    #[cfg(feature = "devenv")]
    async fn create_and_send_transaction_with_event<T>(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(T, Signature, Slot)>, RpcError>
    where
        T: BorshDeserialize + Send + Debug,
    {
        let pre_balance = self.get_balance(payer).await?;

        let result = <Self as Rpc>::create_and_send_transaction_with_event::<T>(
            self,
            instructions,
            payer,
            signers,
        )
        .await?;
        assert_transaction_params(self, payer, signers, pre_balance, transaction_params).await?;

        Ok(result)
    }

    #[cfg(feature = "devenv")]
    async fn create_and_send_transaction_with_public_event(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(PublicTransactionEvent, Signature, Slot)>, RpcError> {
        let pre_balance = self.get_balance(payer).await?;

        let res = <Self as Rpc>::create_and_send_transaction_with_batched_event(
            self,
            instructions,
            payer,
            signers,
        )
        .await?;
        assert_transaction_params(self, payer, signers, pre_balance, transaction_params).await?;

        let event = res.map(|e| (e.0[0].event.clone(), e.1, e.2));

        Ok(event)
    }

    fn set_account(&mut self, address: &Pubkey, account: &Account);
    fn warp_to_slot(&mut self, slot: Slot) -> Result<(), RpcError>;
}

// Implementation required for E2ETestEnv.
#[async_trait]
impl TestRpc for LightClient {
    fn set_account(&mut self, _address: &Pubkey, _account: &Account) {
        unimplemented!()
    }

    fn warp_to_slot(&mut self, _slot: Slot) -> Result<(), RpcError> {
        unimplemented!()
    }
}

#[async_trait]
impl TestRpc for LightProgramTest {
    fn set_account(&mut self, address: &Pubkey, account: &Account) {
        self.context
            .set_account(*address, account.clone())
            .expect("Setting account failed.");
    }

    fn warp_to_slot(&mut self, slot: Slot) -> Result<(), RpcError> {
        self.context.warp_to_slot(slot);
        Ok(())
    }
}
