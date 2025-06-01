use std::{fmt::Debug, marker::Send};

use async_trait::async_trait;
use borsh::BorshDeserialize;
use light_client::{
    fee::{assert_transaction_params, TransactionParams},
    rpc::{RpcConnection, RpcError, SolanaRpcConnection},
};
use light_compressed_account::indexer_event::event::{
    BatchPublicTransactionEvent, PublicTransactionEvent,
};
use solana_sdk::{
    account::AccountSharedData,
    clock::Slot,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
};

use crate::program_test::LightProgramTest;
#[async_trait]
pub trait TestRpc: RpcConnection + Sized {
    async fn create_and_send_transaction_with_batched_event(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<light_client::fee::TransactionParams>,
    ) -> Result<Option<(Vec<BatchPublicTransactionEvent>, Signature, Slot)>, RpcError> {
        let pre_balance = self.get_balance(payer).await?;

        let event = <Self as RpcConnection>::create_and_send_transaction_with_batched_event(
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

        let result = <Self as RpcConnection>::create_and_send_transaction_with_event::<T>(
            self,
            instructions,
            payer,
            signers,
        )
        .await?;
        assert_transaction_params(self, payer, signers, pre_balance, transaction_params).await?;

        Ok(result)
    }

    async fn create_and_send_transaction_with_public_event(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(PublicTransactionEvent, Signature, Slot)>, RpcError> {
        let pre_balance = self.get_balance(payer).await?;

        let res = <Self as RpcConnection>::create_and_send_transaction_with_batched_event(
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

    fn set_account(&mut self, address: &Pubkey, account: &AccountSharedData);
    fn warp_to_slot(&mut self, slot: Slot) -> Result<(), RpcError>;
}

// Implementation required for E2ETestEnv.
#[async_trait]
impl TestRpc for SolanaRpcConnection {
    fn set_account(&mut self, _address: &Pubkey, _account: &AccountSharedData) {
        unimplemented!()
    }

    fn warp_to_slot(&mut self, _slot: Slot) -> Result<(), RpcError> {
        unimplemented!()
    }
}

#[async_trait]
impl TestRpc for LightProgramTest {
    fn set_account(&mut self, address: &Pubkey, account: &AccountSharedData) {
        self.context.set_account(address, account);
    }

    fn warp_to_slot(&mut self, slot: Slot) -> Result<(), RpcError> {
        self.context
            .warp_to_slot(slot)
            .map_err(|_| RpcError::InvalidWarpSlot)
    }
}
