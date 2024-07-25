use std::fmt::{Debug, Formatter};

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::clock::Slot;
use anchor_lang::solana_program::hash::Hash;
use anchor_lang::solana_program::system_instruction;
use anchor_lang::AnchorDeserialize;
use solana_program_test::{BanksClientError, ProgramTestContext};
use solana_sdk::account::{Account, AccountSharedData};
use solana_sdk::instruction::{Instruction, InstructionError};
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

use crate::rpc::errors::RpcError;
use crate::rpc::rpc_connection::RpcConnection;
use crate::transaction_params::TransactionParams;

pub struct ProgramTestRpcConnection {
    pub context: ProgramTestContext,
}

impl Debug for ProgramTestRpcConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProgramTestRpcConnection")
    }
}

impl Clone for ProgramTestRpcConnection {
    fn clone(&self) -> Self {
        unimplemented!()
    }
}

impl RpcConnection for ProgramTestRpcConnection {
    async fn process_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<Signature, RpcError> {
        let sig = *transaction.signatures.first().unwrap();
        let result = self
            .context
            .banks_client
            .process_transaction_with_metadata(transaction)
            .await
            .map_err(RpcError::from)?;
        result.result.map_err(RpcError::TransactionError)?;
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
        instruction: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(T, Signature, Slot)>, RpcError>
    where
        T: AnchorDeserialize,
    {
        let pre_balance = self
            .context
            .banks_client
            .get_account(*payer)
            .await?
            .unwrap()
            .lamports;

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

        let slot = self.context.banks_client.get_root_slot().await?;
        let result = event.map(|event| (event, signature, slot));
        Ok(result)
    }

    async fn confirm_transaction(&mut self, _transaction: Signature) -> Result<bool, RpcError> {
        Ok(true)
    }

    fn get_payer(&self) -> &Keypair {
        &self.context.payer
    }

    async fn get_account(&mut self, address: Pubkey) -> Result<Option<Account>, RpcError> {
        self.context
            .banks_client
            .get_account(address)
            .await
            .map_err(RpcError::from)
    }

    fn set_account(&mut self, address: &Pubkey, account: &AccountSharedData) {
        self.context.set_account(address, account);
    }

    async fn get_minimum_balance_for_rent_exemption(
        &mut self,
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

    async fn get_latest_blockhash(&mut self) -> Result<Hash, RpcError> {
        self.context
            .get_new_latest_blockhash()
            .await
            .map_err(|e| RpcError::from(BanksClientError::from(e)))
    }

    async fn get_slot(&mut self) -> Result<u64, RpcError> {
        self.context
            .banks_client
            .get_root_slot()
            .await
            .map_err(RpcError::from)
    }

    async fn airdrop_lamports(
        &mut self,
        to: &Pubkey,
        lamports: u64,
    ) -> Result<Signature, RpcError> {
        // Create a transfer instruction
        let transfer_instruction =
            system_instruction::transfer(&self.context.payer.pubkey(), to, lamports);
        let latest_blockhash = self.get_latest_blockhash().await.unwrap();
        // Create and sign a transaction
        let transaction = Transaction::new_signed_with_payer(
            &[transfer_instruction],
            Some(&self.get_payer().pubkey()),
            &vec![&self.get_payer()],
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

    async fn get_balance(&mut self, pubkey: &Pubkey) -> Result<u64, RpcError> {
        self.context
            .banks_client
            .get_balance(*pubkey)
            .await
            .map_err(RpcError::from)
    }

    fn warp_to_slot(&mut self, slot: Slot) -> Result<(), RpcError> {
        self.context.warp_to_slot(slot).map_err(RpcError::from)
    }
}
