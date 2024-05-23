use crate::rpc::errors::RpcError;
use crate::rpc::rpc_connection::RpcConnection;
use crate::transaction_params::TransactionParams;
use account_compression::initialize_address_merkle_tree::Rent;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::clock::Slot;
use anchor_lang::solana_program::hash::Hash;
use anchor_lang::AnchorDeserialize;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_program_test::BanksClientError;
use solana_sdk::account::{Account, AccountSharedData};
use solana_sdk::instruction::{Instruction, InstructionError};
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::transaction::{Transaction, TransactionError};

#[allow(dead_code)]
struct SolanaRpcConnection {
    client: RpcClient,
}

impl RpcConnection for SolanaRpcConnection {
    async fn create_and_send_transaction_with_event<T>(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<T>, RpcError>
    where
        T: AnchorDeserialize,
    {
        let pre_balance = self.client.get_balance(payer)?;
        let config = RpcSendTransactionConfig {
            skip_preflight: true,
            ..RpcSendTransactionConfig::default()
        };
        let latest_blockhash = self.client.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            instructions,
            Some(payer),
            signers,
            latest_blockhash,
        );
        let signature = self
            .client
            .send_transaction_with_config(&transaction, config)?;
        let mut confirmed = false;
        while !confirmed {
            confirmed = self.client.confirm_transaction(&signature)?;
        }

        let event = transaction
            .message
            .instructions
            .iter()
            .find_map(|instruction| T::try_from_slice(instruction.data.as_slice()).ok());
        // assert correct rollover fee and tip distribution
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
                - 5000 * deduped_signers.len() as i64
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
                println!("network_fee: {}", network_fee);
                return Err(RpcError::from(BanksClientError::TransactionError(
                    TransactionError::InstructionError(0, InstructionError::Custom(11111)),
                )));
            }
        }
        Ok(event)
    }

    async fn create_and_send_transaction(
        &mut self,
        _instruction: &[Instruction],
        _payer: &Pubkey,
        _signers: &[&Keypair],
    ) -> Result<Signature, RpcError> {
        todo!()
    }

    fn get_payer(&self) -> &Keypair {
        todo!()
    }

    async fn get_account(&mut self, _address: Pubkey) -> Result<Option<Account>, RpcError> {
        todo!()
    }

    fn set_account(&mut self, _address: &Pubkey, _account: &AccountSharedData) {
        todo!()
    }

    async fn get_rent(&mut self) -> Result<Rent, RpcError> {
        todo!()
    }

    async fn get_latest_blockhash(&mut self) -> Result<Hash, RpcError> {
        todo!()
    }

    async fn process_transaction(&mut self, _transaction: Transaction) -> Result<(), RpcError> {
        todo!()
    }

    async fn get_slot(&mut self) -> Result<u64, RpcError> {
        todo!()
    }

    async fn airdrop_lamports(
        &mut self,
        _destination_pubkey: &Pubkey,
        _lamports: u64,
    ) -> Result<(), RpcError> {
        todo!()
    }

    async fn get_anchor_account<T: AnchorDeserialize>(&mut self, _pubkey: &Pubkey) -> T {
        todo!()
    }

    async fn get_balance(&mut self, pubkey: &Pubkey) -> Result<u64, RpcError> {
        self.client.get_balance(pubkey).map_err(RpcError::from)
    }

    fn warp_to_slot(&mut self, _slot: Slot) -> Result<(), RpcError> {
        todo!()
    }
}
