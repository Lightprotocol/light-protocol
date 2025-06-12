use std::{fmt::Debug, marker::Send};

use anchor_lang::pubkey;
use async_trait::async_trait;
use borsh::BorshDeserialize;
use light_client::{
    indexer::{Indexer, TreeInfo},
    rpc::{LightClientConfig, Rpc, RpcError},
};
use light_compressed_account::{
    indexer_event::{
        error::ParseIndexerEventError,
        event::{BatchPublicTransactionEvent, PublicTransactionEvent},
        parse::event_from_light_transaction,
    },
    TreeType,
};
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_sdk::{
    account::Account,
    clock::{Clock, Slot},
    hash::Hash,
    instruction::Instruction,
    pubkey::Pubkey,
    rent::Rent,
    signature::{Keypair, Signature},
    transaction::Transaction,
};
use solana_transaction_status_client_types::TransactionStatus;

use crate::{
    indexer::{TestIndexer, TestIndexerExtensions},
    program_test::LightProgramTest,
};

#[async_trait]
impl Rpc for LightProgramTest {
    async fn new(_config: LightClientConfig) -> Result<Self, RpcError>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn get_payer(&self) -> &Keypair {
        &self.payer
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
        Ok(self.context.get_account(&address))
    }

    async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> Result<u64, RpcError> {
        let rent = self.context.get_sysvar::<Rent>();

        Ok(rent.minimum_balance(data_len))
    }

    async fn airdrop_lamports(
        &mut self,
        to: &Pubkey,
        lamports: u64,
    ) -> Result<Signature, RpcError> {
        let res = self.context.airdrop(to, lamports).map_err(|e| e.err)?;
        Ok(res.signature)
    }

    async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64, RpcError> {
        Ok(self.context.get_balance(pubkey).unwrap())
    }

    async fn get_latest_blockhash(&mut self) -> Result<(Hash, u64), RpcError> {
        let slot = self.get_slot().await?;
        let hash = self.context.latest_blockhash();
        Ok((hash, slot))
    }

    async fn get_slot(&self) -> Result<u64, RpcError> {
        Ok(self.context.get_sysvar::<Clock>().slot)
    }

    async fn get_transaction_slot(&self, _signature: &Signature) -> Result<u64, RpcError> {
        unimplemented!();
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
            let _res = self.context.send_transaction(transaction).map_err(|x| {
                if self.config.log_failed_tx {
                    println!("{}", x.meta.pretty_logs());
                }

                RpcError::TransactionError(x.err)
            })?;
            #[cfg(debug_assertions)]
            {
                if std::env::var("RUST_BACKTRACE").is_ok() {
                    println!("{}", _res.pretty_logs());
                }
            }
        }
        Ok(sig)
    }

    async fn process_transaction_with_context(
        &mut self,
        transaction: Transaction,
    ) -> Result<(Signature, Slot), RpcError> {
        let sig = *transaction.signatures.first().unwrap();
        let _res = self.context.send_transaction(transaction).map_err(|x| {
            #[cfg(not(debug_assertions))]
            {
                if self.config.log_failed_tx {
                    println!("{}", x.meta.pretty_logs());
                }
            }
            RpcError::TransactionError(x.err)
        })?;
        #[cfg(debug_assertions)]
        {
            if std::env::var("RUST_BACKTRACE").is_ok() {
                println!("{}", _res.pretty_logs());
            }
        }

        let slot = self.context.get_sysvar::<Clock>().slot;
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

    /// Fetch the latest state tree addresses from the cluster.
    async fn get_latest_active_state_trees(&mut self) -> Result<Vec<TreeInfo>, RpcError> {
        #[cfg(not(feature = "v2"))]
        return Ok(self
            .test_accounts
            .v1_state_trees
            .to_vec()
            .into_iter()
            .map(|tree| tree.into())
            .collect());
        #[cfg(feature = "v2")]
        return Ok(self
            .test_accounts
            .v2_state_trees
            .iter()
            .map(|tree| (*tree).into())
            .collect());
    }

    /// Fetch the latest state tree addresses from the cluster.
    fn get_state_tree_infos(&self) -> Vec<TreeInfo> {
        #[cfg(not(feature = "v2"))]
        return self
            .test_accounts
            .v1_state_trees
            .to_vec()
            .into_iter()
            .map(|tree| tree.into())
            .collect();
        #[cfg(feature = "v2")]
        return self
            .test_accounts
            .v2_state_trees
            .iter()
            .map(|tree| (*tree).into())
            .collect();
    }

    /// Gets a random active state tree.
    /// State trees are cached and have to be fetched or set.
    fn get_random_state_tree_info(&self) -> Result<TreeInfo, RpcError> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        #[cfg(not(feature = "v2"))]
        {
            if self.test_accounts.v1_state_trees.is_empty() {
                return Err(RpcError::NoStateTreesAvailable);
            }
            Ok(self.test_accounts.v1_state_trees
                [rng.gen_range(0..self.test_accounts.v1_state_trees.len())]
            .into())
        }
        #[cfg(feature = "v2")]
        {
            if self.test_accounts.v2_state_trees.is_empty() {
                return Err(RpcError::NoStateTreesAvailable);
            }
            Ok(self.test_accounts.v2_state_trees
                [rng.gen_range(0..self.test_accounts.v2_state_trees.len())]
            .into())
        }
    }

    fn get_address_tree_v1(&self) -> TreeInfo {
        TreeInfo {
            tree: pubkey!("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2"),
            queue: pubkey!("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F"),
            cpi_context: None,
            next_tree_info: None,
            tree_type: TreeType::AddressV1,
        }
    }
}

impl LightProgramTest {
    #[cfg(feature = "v2")]
    pub fn get_address_tree_v2(&self) -> TreeInfo {
        TreeInfo {
            tree: pubkey!("EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK"),
            queue: pubkey!("EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK"),
            cpi_context: None,
            next_tree_info: None,
            tree_type: TreeType::AddressV2,
        }
    }

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
            .simulate_transaction(transaction.clone())
            .map_err(|x| {
                if self.config.log_failed_tx {
                    println!("{}", x.meta.pretty_logs());
                }

                RpcError::TransactionError(x.err)
            })?;

        // Try old event deserialization.
        let event = simulation_result
            .meta
            .inner_instructions
            .iter()
            .flatten()
            .find_map(|inner_instruction| {
                PublicTransactionEvent::try_from_slice(&inner_instruction.instruction.data).ok()
            });
        let event = if let Some(event) = event {
            Some(vec![BatchPublicTransactionEvent {
                event,
                ..Default::default()
            }])
        } else {
            // If PublicTransactionEvent wasn't successful deserialize new event.
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
                .meta
                .inner_instructions
                .iter()
                .flatten()
                .find_map(|inner_instruction| {
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
                });

            event_from_light_transaction(
                &program_ids.iter().map(|x| (*x).into()).collect::<Vec<_>>(),
                vec.as_slice(),
                vec_accounts
                    .iter()
                    .map(|inner_vec| inner_vec.iter().map(|x| (*x).into()).collect())
                    .collect(),
            )
            .or(Ok::<
                Option<Vec<BatchPublicTransactionEvent>>,
                ParseIndexerEventError,
            >(None))?
        };

        // Transaction was successful, execute it.
        let _res = self.context.send_transaction(transaction).map_err(|x| {
            // Prevent duplicate prints for failing tx.

            if self.config.log_failed_tx {
                println!("{}", x.meta.pretty_logs());
            }

            RpcError::TransactionError(x.err)
        })?;
        #[cfg(debug_assertions)]
        {
            if std::env::var("RUST_BACKTRACE").is_ok() {
                // Print all tx logs and events.
                println!("{}", _res.pretty_logs());
                println!("event:\n {:?}", event);
            }
        }
        let slot = self.context.get_sysvar::<Clock>().slot;
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
            self.context.latest_blockhash(),
        );

        let signature = transaction.signatures[0];
        // Simulate the transaction. Currently, in banks-client/server, only
        // simulations are able to track CPIs. Therefore, simulating is the
        // only way to retrieve the event.
        let simulation_result = self
            .context
            .simulate_transaction(transaction.clone())
            .map_err(|x| RpcError::from(x.err))?;

        let event = simulation_result
            .meta
            .inner_instructions
            .iter()
            .flatten()
            .find_map(|inner_instruction| {
                T::try_from_slice(&inner_instruction.instruction.data).ok()
            });
        // If transaction was successful, execute it.
        let _res = self.context.send_transaction(transaction).map_err(|x| {
            #[cfg(not(debug_assertions))]
            {
                if self.config.log_failed_tx {
                    println!("{}", x.meta.pretty_logs());
                }
            }
            RpcError::TransactionError(x.err)
        })?;
        #[cfg(debug_assertions)]
        {
            if std::env::var("RUST_BACKTRACE").is_ok() {
                println!("{}", _res.pretty_logs());
            }
        }

        let slot = self.get_slot().await?;
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
            self.context.latest_blockhash(),
        );

        self._send_transaction_with_batched_event(transaction).await
    }
}
