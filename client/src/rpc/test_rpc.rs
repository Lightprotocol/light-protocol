use crate::transaction_params::TransactionParams;
use async_trait::async_trait;
use borsh::BorshDeserialize;
use solana_banks_client::BanksClientError;
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    account::{Account, AccountSharedData},
    clock::Slot,
    commitment_config::CommitmentConfig,
    epoch_info::EpochInfo,
    hash::Hash,
    instruction::{Instruction, InstructionError},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    system_instruction,
    transaction::{Transaction, TransactionError},
};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{merkle_tree::MerkleTreeExt, RpcConnection, RpcError};

pub struct ProgramTestRpcConnection {
    pub context: Arc<RwLock<ProgramTestContext>>,
}

#[async_trait]
impl RpcConnection for ProgramTestRpcConnection {
    fn new<U: ToString>(_url: U, _commitment_config: Option<CommitmentConfig>) -> Self
    where
        Self: Sized,
    {
        unimplemented!()
    }

    async fn get_payer(&self) -> Keypair {
        let context = self.context.read().await;
        context.payer.insecure_clone()
    }

    fn get_url(&self) -> String {
        unimplemented!("get_url doesn't make sense for ProgramTestRpcConnection")
    }

    async fn health(&self) -> Result<(), RpcError> {
        unimplemented!()
    }

    async fn get_block_time(&self, _slot: u64) -> Result<i64, RpcError> {
        unimplemented!()
    }

    async fn get_epoch_info(&self) -> Result<EpochInfo, RpcError> {
        unimplemented!()
    }

    async fn get_program_accounts(
        &self,
        _program_id: &Pubkey,
    ) -> Result<Vec<(Pubkey, Account)>, RpcError> {
        unimplemented!("get_program_accounts")
    }

    async fn process_transaction(&self, transaction: Transaction) -> Result<Signature, RpcError> {
        let sig = *transaction.signatures.first().unwrap();
        let result = {
            let mut context = self.context.write().await;
            context
                .banks_client
                .process_transaction_with_metadata(transaction)
                .await
        };
        result
            .map_err(RpcError::from)?
            .result
            .map_err(RpcError::TransactionError)?;
        Ok(sig)
    }

    async fn process_transaction_with_context(
        &self,
        transaction: Transaction,
    ) -> Result<(Signature, Slot), RpcError> {
        let sig = *transaction.signatures.first().unwrap();
        let (result, slot) = {
            let mut context = self.context.write().await;
            let result = context
                .banks_client
                .process_transaction_with_metadata(transaction)
                .await;
            let slot = context.banks_client.get_root_slot().await?;
            (result, slot)
        };
        result
            .map_err(RpcError::from)?
            .result
            .map_err(RpcError::TransactionError)?;
        Ok((sig, slot))
    }

    async fn create_and_send_transaction_with_event<T>(
        &self,
        instruction: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(T, Signature, Slot)>, RpcError>
    where
        T: BorshDeserialize + Send + Debug,
    {
        let mut context = self.context.write().await;
        let pre_balance = context
            .banks_client
            .get_account(*payer)
            .await?
            .unwrap()
            .lamports;

        let transaction = Transaction::new_signed_with_payer(
            instruction,
            Some(payer),
            signers,
            context.get_new_latest_blockhash().await?,
        );
        drop(context);

        let signature = transaction.signatures[0];
        // Simulate the transaction. Currently, in banks-client/server, only
        // simulations are able to track CPIs. Therefore, simulating is the
        // only way to retrieve the event.

        let simulation_result = {
            let mut context = self.context.write().await;
            context
                .banks_client
                .simulate_transaction(transaction.clone())
                .await?
        };

        // Handle an error nested in the simulation result.
        if let Some(Err(e)) = simulation_result.result {
            let error = match e {
                TransactionError::InstructionError(_, _) => RpcError::TransactionError(e),
                _ => RpcError::from(BanksClientError::TransactionError(e)),
            };
            return Err(error);
        }

        // Retrieve the event.
        let event = simulation_result
            .simulation_details
            .and_then(|details| details.inner_instructions)
            .and_then(|instructions| {
                instructions.iter().flatten().find_map(|inner_instruction| {
                    T::try_from_slice(inner_instruction.instruction.data.as_slice()).ok()
                })
            });
        // If transaction was successful, execute it.
        if let Some(Ok(())) = simulation_result.result {
            let result = {
                let mut context = self.context.write().await;
                context.banks_client.process_transaction(transaction).await
            };

            if let Err(e) = result {
                let error = RpcError::from(e);
                return Err(error);
            }
        }

        // assert correct rollover fee and network_fee distribution
        if let Some(transaction_params) = transaction_params {
            let mut deduped_signers = signers.to_vec();
            deduped_signers.dedup();
            let post_balance = self.get_account(*payer).await?.unwrap().lamports;

            // a network_fee is charged if there are input compressed accounts or new addresses
            let mut network_fee: i64 = 0;
            if transaction_params.num_input_compressed_accounts != 0 {
                network_fee += transaction_params.fee_config.network_fee as i64;
            }
            if transaction_params.num_new_addresses != 0 {
                network_fee += transaction_params.fee_config.address_network_fee as i64;
            }
            let expected_post_balance = pre_balance as i64
                - i64::from(transaction_params.num_new_addresses)
                    * transaction_params.fee_config.address_queue_rollover as i64
                - i64::from(transaction_params.num_output_compressed_accounts)
                    * transaction_params.fee_config.state_merkle_tree_rollover as i64
                - transaction_params.compress
                - transaction_params.fee_config.solana_network_fee * deduped_signers.len() as i64
                - network_fee;

            if post_balance as i64 != expected_post_balance {
                println!("transaction_params: {:?}", transaction_params);
                println!("pre_balance: {}", pre_balance);
                println!("post_balance: {}", post_balance);
                println!("expected post_balance: {}", expected_post_balance);
                println!(
                    "diff post_balance: {}",
                    post_balance as i64 - expected_post_balance
                );
                println!(
                    "rollover fee: {}",
                    transaction_params.fee_config.state_merkle_tree_rollover
                );
                println!(
                    "address_network_fee: {}",
                    transaction_params.fee_config.address_network_fee
                );
                println!("network_fee: {}", network_fee);
                println!("num signers {}", deduped_signers.len());
                return Err(RpcError::from(BanksClientError::TransactionError(
                    TransactionError::InstructionError(0, InstructionError::Custom(11111)),
                )));
            }
        }

        let slot = {
            let mut context = self.context.write().await;
            context.banks_client.get_root_slot().await?
        };
        let result = event.map(|event| (event, signature, slot));
        Ok(result)
    }

    async fn confirm_transaction(&self, _transaction: Signature) -> Result<bool, RpcError> {
        Ok(true)
    }

    async fn get_account(&self, address: Pubkey) -> Result<Option<Account>, RpcError> {
        let mut context = self.context.write().await;
        context
            .banks_client
            .get_account(address)
            .await
            .map_err(RpcError::from)
    }

    async fn set_account(&self, address: &Pubkey, account: &AccountSharedData) {
        let mut context = self.context.write().await;
        context.set_account(address, account);
    }

    async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> Result<u64, RpcError> {
        let mut context = self.context.write().await;
        let rent = context
            .banks_client
            .get_rent()
            .await
            .map_err(RpcError::from);

        Ok(rent?.minimum_balance(data_len))
    }

    async fn airdrop_lamports(&self, to: &Pubkey, lamports: u64) -> Result<Signature, RpcError> {
        // Create a transfer instruction
        let transfer_instruction = {
            let context = self.context.read().await;
            system_instruction::transfer(&context.payer.pubkey(), to, lamports)
        };
        let latest_blockhash = self.get_latest_blockhash().await?;
        // Create and sign a transaction
        let payer = self.get_payer().await;
        let transaction = Transaction::new_signed_with_payer(
            &[transfer_instruction],
            Some(&payer.pubkey()),
            &vec![&payer],
            latest_blockhash,
        );
        let sig = *transaction.signatures.first().unwrap();

        // Send the transaction
        {
            let mut context = self.context.write().await;
            context
                .banks_client
                .process_transaction(transaction)
                .await?;
        }

        Ok(sig)
    }

    async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64, RpcError> {
        let mut context = self.context.write().await;
        context
            .banks_client
            .get_balance(*pubkey)
            .await
            .map_err(RpcError::from)
    }

    async fn get_latest_blockhash(&self) -> Result<Hash, RpcError> {
        let mut context = self.context.write().await;
        context
            .get_new_latest_blockhash()
            .await
            .map_err(|e| RpcError::from(BanksClientError::from(e)))
    }

    async fn get_slot(&self) -> Result<u64, RpcError> {
        let mut context = self.context.write().await;
        context
            .banks_client
            .get_root_slot()
            .await
            .map_err(RpcError::from)
    }

    async fn warp_to_slot(&self, slot: Slot) -> Result<(), RpcError> {
        let mut context = self.context.write().await;
        context
            .warp_to_slot(slot)
            .map_err(|_| RpcError::InvalidWarpSlot)
    }

    async fn send_transaction(&self, _transaction: &Transaction) -> Result<Signature, RpcError> {
        unimplemented!("send transaction is unimplemented for ProgramTestRpcConnection")
    }
}

impl MerkleTreeExt for ProgramTestRpcConnection {}
