use std::{fmt::Debug, marker::Send};

use async_trait::async_trait;
use borsh::BorshDeserialize;
use light_client::{
    indexer::Indexer,
    rpc::{rpc_connection::RpcConnectionConfig, RpcConnection, RpcError},
};
use light_compressed_account::indexer_event::{
    event::{BatchPublicTransactionEvent, PublicTransactionEvent},
    parse::event_from_light_transaction,
};
use solana_banks_client::BanksClientError;
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_sdk::{
    account::Account,
    clock::Slot,
    hash::Hash,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    system_instruction,
    transaction::{Transaction, TransactionError},
};
use solana_transaction_status::TransactionStatus;

use crate::{
    indexer::{TestIndexer, TestIndexerExtensions},
    program_test::LightProgramTest,
};

#[async_trait]
impl RpcConnection for LightProgramTest {
    fn new(_config: RpcConnectionConfig) -> Self
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn get_payer(&self) -> &Keypair {
        &self.context.payer
    }

    fn get_url(&self) -> String {
        "get_url doesn't make sense for LightProgramTest".to_string()
    }

    async fn health(&self) -> Result<(), RpcError> {
        Ok(())
    }

    async fn get_program_accounts(
        &self,
        _program_id: &Pubkey,
    ) -> Result<Vec<(Pubkey, Account)>, RpcError> {
        unimplemented!("get_program_accounts")
    }

    async fn confirm_transaction(&self, _transaction: Signature) -> Result<bool, RpcError> {
        Ok(true)
    }

    async fn get_account(&self, address: Pubkey) -> Result<Option<Account>, RpcError> {
        self.context
            .banks_client
            .get_account(address)
            .await
            .map_err(RpcError::from)
    }

    async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> Result<u64, RpcError> {
        let rent = self
            .context
            .banks_client
            .get_rent()
            .await
            .map_err(RpcError::from);

        Ok(rent?.minimum_balance(data_len))
    }

    async fn airdrop_lamports(
        &mut self,
        to: &Pubkey,
        lamports: u64,
    ) -> Result<Signature, RpcError> {
        // Create a transfer instruction
        let transfer_instruction =
            system_instruction::transfer(&self.context.payer.pubkey(), to, lamports);
        let latest_blockhash = self.get_latest_blockhash().await?.0;

        // Use the RpcConnection implementation of get_payer to avoid ambiguity
        let payer = <Self as RpcConnection>::get_payer(self);

        // Create and sign a transaction
        let transaction = Transaction::new_signed_with_payer(
            &[transfer_instruction],
            Some(&payer.pubkey()),
            &vec![payer],
            latest_blockhash,
        );
        let sig = *transaction.signatures.first().unwrap();

        // Send the transaction
        self.context
            .banks_client
            .process_transaction(transaction)
            .await?;

        Ok(sig)
    }

    async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64, RpcError> {
        self.context
            .banks_client
            .get_balance(*pubkey)
            .await
            .map_err(RpcError::from)
    }

    async fn get_latest_blockhash(&mut self) -> Result<(Hash, u64), RpcError> {
        let slot = self.get_slot().await?;
        let hash = self
            .context
            .get_new_latest_blockhash()
            .await
            .map_err(|e| RpcError::from(BanksClientError::from(e)))?;
        Ok((hash, slot))
    }

    async fn get_slot(&self) -> Result<u64, RpcError> {
        self.context
            .banks_client
            .get_root_slot()
            .await
            .map_err(RpcError::from)
    }

    async fn get_transaction_slot(&self, signature: &Signature) -> Result<u64, RpcError> {
        self.context
            .banks_client
            .get_transaction_status(*signature)
            .await
            .map_err(RpcError::from)
            .and_then(|status| {
                status
                    .ok_or(RpcError::TransactionError(
                        TransactionError::SignatureFailure,
                    ))
                    .map(|status| status.slot)
            })
    }

    async fn get_signature_statuses(
        &self,
        _signatures: &[Signature],
    ) -> Result<Vec<Option<TransactionStatus>>, RpcError> {
        Err(RpcError::CustomError(
            "get_signature_statuses is unimplemented for LightProgramTest".to_string(),
        ))
    }

    async fn send_transaction(&self, _transaction: &Transaction) -> Result<Signature, RpcError> {
        Err(RpcError::CustomError(
            "send_transaction is unimplemented for ProgramTestConnection".to_string(),
        ))
    }

    async fn send_transaction_with_config(
        &self,
        _transaction: &Transaction,
        _config: RpcSendTransactionConfig,
    ) -> Result<Signature, RpcError> {
        Err(RpcError::CustomError(
            "send_transaction_with_config is unimplemented for ProgramTestConnection".to_string(),
        ))
    }

    async fn process_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<Signature, RpcError> {
        let sig = *transaction.signatures.first().unwrap();
        if self.indexer.is_some() {
            self._send_transaction_with_batched_event(transaction)
                .await?;
        } else {
            self.context
                .banks_client
                .process_transaction(transaction)
                .await
                .map_err(RpcError::from)?;
        }

        Ok(sig)
    }

    async fn process_transaction_with_context(
        &mut self,
        transaction: Transaction,
    ) -> Result<(Signature, Slot), RpcError> {
        let sig = *transaction.signatures.first().unwrap();
        let result = self
            .context
            .banks_client
            .process_transaction_with_metadata(transaction)
            .await
            .map_err(RpcError::from)?;
        result.result.map_err(RpcError::TransactionError)?;
        let slot = self.context.banks_client.get_root_slot().await?;
        Ok((sig, slot))
    }

    async fn create_and_send_transaction_with_event<T>(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Option<(T, Signature, u64)>, RpcError>
    where
        T: BorshDeserialize + Send + Debug,
    {
        self._create_and_send_transaction_with_event::<T>(instructions, payer, signers)
            .await
    }

    async fn create_and_send_transaction_with_batched_event(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Option<(Vec<BatchPublicTransactionEvent>, Signature, Slot)>, RpcError> {
        self._create_and_send_transaction_with_batched_event(instructions, payer, signers)
            .await
    }

    async fn create_and_send_transaction_with_public_event(
        &mut self,
        instruction: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Option<(PublicTransactionEvent, Signature, Slot)>, RpcError> {
        let event = self
            ._create_and_send_transaction_with_batched_event(instruction, payer, signers)
            .await?;
        let event = event.map(|e| (e.0[0].event.clone(), e.1, e.2));

        Ok(event)
    }

    fn indexer(&self) -> Result<&impl Indexer, RpcError> {
        self.indexer.as_ref().ok_or(RpcError::IndexerNotInitialized)
    }

    fn indexer_mut(&mut self) -> Result<&mut impl Indexer, RpcError> {
        self.indexer.as_mut().ok_or(RpcError::IndexerNotInitialized)
    }
}

impl LightProgramTest {
    async fn _send_transaction_with_batched_event(
        &mut self,
        transaction: Transaction,
    ) -> Result<Option<(Vec<BatchPublicTransactionEvent>, Signature, Slot)>, RpcError> {
        let mut vec = Vec::new();

        let signature = transaction.signatures[0];
        // Simulate the transaction. Currently, in banks-client/server, only
        // simulations are able to track CPIs. Therefore, simulating is the
        // only way to retrieve the event.
        let simulation_result = self
            .context
            .banks_client
            .simulate_transaction(transaction.clone())
            .await?;
        // Handle an error nested in the simulation result.
        if let Some(Err(e)) = simulation_result.result {
            let error = match e {
                TransactionError::InstructionError(_, _) => RpcError::TransactionError(e),
                _ => RpcError::from(BanksClientError::TransactionError(e)),
            };
            return Err(error);
        }
        let mut vec_accounts = Vec::<Vec<Pubkey>>::new();
        let mut program_ids = Vec::new();

        transaction.message.instructions.iter().for_each(|i| {
            program_ids.push(transaction.message.account_keys[i.program_id_index as usize]);
            vec.push(i.data.clone());
            vec_accounts.push(
                i.accounts
                    .iter()
                    .map(|x| transaction.message.account_keys[*x as usize])
                    .collect(),
            );
        });
        simulation_result
            .simulation_details
            .and_then(|details| details.inner_instructions)
            .and_then(|instructions| {
                instructions.iter().flatten().find_map(|inner_instruction| {
                    vec.push(inner_instruction.instruction.data.clone());
                    program_ids.push(
                        transaction.message.account_keys
                            [inner_instruction.instruction.program_id_index as usize],
                    );
                    vec_accounts.push(
                        inner_instruction
                            .instruction
                            .accounts
                            .iter()
                            .map(|x| transaction.message.account_keys[*x as usize])
                            .collect(),
                    );
                    None::<PublicTransactionEvent>
                })
            });

        let event = event_from_light_transaction(
            program_ids.as_slice(),
            vec.as_slice(),
            vec_accounts.to_vec(),
        )
        .unwrap();
        println!("event: {:?}", event);
        // If transaction was successful, execute it.
        if let Some(Ok(())) = simulation_result.result {
            let result = self
                .context
                .banks_client
                .process_transaction(transaction)
                .await;
            if let Err(e) = result {
                let error = RpcError::from(e);
                return Err(error);
            }
        }

        let slot = self.context.banks_client.get_root_slot().await?;
        let event = event.map(|e| (e, signature, slot));

        if let Some(indexer) = self.indexer.as_mut() {
            if let Some(events) = event.as_ref() {
                for event in events.0.iter() {
                    <TestIndexer as TestIndexerExtensions>::add_compressed_accounts_with_token_data(
                        indexer,
                        slot,
                        &event.event,
                    );
                }
            }
        }

        Ok(event)
    }

    async fn _create_and_send_transaction_with_event<T>(
        &mut self,
        instruction: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Option<(T, Signature, Slot)>, RpcError>
    where
        T: BorshDeserialize + Send + Debug,
    {
        let transaction = Transaction::new_signed_with_payer(
            instruction,
            Some(payer),
            signers,
            self.context.get_new_latest_blockhash().await?,
        );

        let signature = transaction.signatures[0];
        // Simulate the transaction. Currently, in banks-client/server, only
        // simulations are able to track CPIs. Therefore, simulating is the
        // only way to retrieve the event.
        let simulation_result = self
            .context
            .banks_client
            .simulate_transaction(transaction.clone())
            .await?;
        // Handle an error nested in the simulation result.
        if let Some(Err(e)) = simulation_result.result {
            let error = match e {
                TransactionError::InstructionError(_, _) => RpcError::TransactionError(e),
                _ => RpcError::from(BanksClientError::TransactionError(e)),
            };
            return Err(error);
        }
        let event = simulation_result
            .simulation_details
            .and_then(|details| details.inner_instructions)
            .and_then(|instructions| {
                instructions.iter().flatten().find_map(|inner_instruction| {
                    T::try_from_slice(&inner_instruction.instruction.data).ok()
                })
            });
        // If transaction was successful, execute it.
        if let Some(Ok(())) = simulation_result.result {
            let result = self
                .context
                .banks_client
                .process_transaction(transaction)
                .await;
            if let Err(e) = result {
                let error = RpcError::from(e);
                return Err(error);
            }
        }

        let slot = self.context.banks_client.get_root_slot().await?;
        let result = event.map(|event| (event, signature, slot));
        Ok(result)
    }

    async fn _create_and_send_transaction_with_batched_event(
        &mut self,
        instruction: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Option<(Vec<BatchPublicTransactionEvent>, Signature, Slot)>, RpcError> {
        let transaction = Transaction::new_signed_with_payer(
            instruction,
            Some(payer),
            signers,
            self.context.get_new_latest_blockhash().await?,
        );

        self._send_transaction_with_batched_event(transaction).await
    }
}
