use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::clock::Slot;
use anchor_lang::solana_program::hash::Hash;
use anchor_lang::AnchorDeserialize;
use log::debug;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_program_test::BanksClientError;
use solana_sdk::account::{Account, AccountSharedData};
use solana_sdk::bs58;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::{Instruction, InstructionError};
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::transaction::{Transaction, TransactionError};
use solana_transaction_status::option_serializer::OptionSerializer;
use solana_transaction_status::{UiInstruction, UiTransactionEncoding};
use std::fmt::{Debug, Display, Formatter};

use crate::rpc::errors::RpcError;
use crate::rpc::rpc_connection::RpcConnection;
use crate::transaction_params::TransactionParams;

pub enum SolanaRpcUrl {
    Testnet,
    Devnet,
    Localnet,
    ZKTestnet,
    Custom(String),
}

impl Display for SolanaRpcUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            SolanaRpcUrl::Testnet => "https://api.testnet.solana.com".to_string(),
            SolanaRpcUrl::Devnet => "https://api.devnet.solana.com".to_string(),
            SolanaRpcUrl::Localnet => "http://localhost:8899".to_string(),
            SolanaRpcUrl::ZKTestnet => "https://zk-testnet.helius.dev:8899".to_string(),
            SolanaRpcUrl::Custom(url) => url.clone(),
        };
        write!(f, "{}", str)
    }
}

#[allow(dead_code)]
pub struct SolanaRpcConnection {
    pub client: RpcClient,
    pub payer: Keypair,
}

impl Debug for SolanaRpcConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SolanaRpcConnection {{ client: {:?} }}",
            self.client.url()
        )
    }
}

impl SolanaRpcConnection {
    fn parse_inner_instructions<T: AnchorDeserialize>(
        &self,
        signature: Signature,
    ) -> Result<T, RpcError> {
        let rpc_transaction_config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            commitment: Some(self.client.commitment()),
            ..Default::default()
        };
        let transaction = self
            .client
            .get_transaction_with_config(&signature, rpc_transaction_config)
            .map_err(|e| RpcError::CustomError(e.to_string()))?;
        let meta = transaction.transaction.meta.as_ref().ok_or_else(|| {
            RpcError::CustomError("Transaction missing metadata information".to_string())
        })?;
        if meta.status.is_err() {
            return Err(RpcError::CustomError(
                "Transaction status indicates an error".to_string(),
            ));
        }

        let inner_instructions = match &meta.inner_instructions {
            OptionSerializer::Some(i) => i,
            OptionSerializer::None => {
                return Err(RpcError::CustomError(
                    "No inner instructions found".to_string(),
                ));
            }
            OptionSerializer::Skip => {
                return Err(RpcError::CustomError(
                    "No inner instructions found".to_string(),
                ));
            }
        };

        for ix in inner_instructions.iter() {
            for ui_instruction in ix.instructions.iter() {
                match ui_instruction {
                    UiInstruction::Compiled(ui_compiled_instruction) => {
                        let data = bs58::decode(&ui_compiled_instruction.data)
                            .into_vec()
                            .map_err(|_| {
                                RpcError::CustomError(
                                    "Failed to decode instruction data".to_string(),
                                )
                            })?;

                        if let Ok(parsed_data) = T::try_from_slice(data.as_slice()) {
                            return Ok(parsed_data);
                        }
                    }
                    UiInstruction::Parsed(_) => {
                        println!("Parsed instructions are not implemented yet");
                    }
                }
            }
        }
        Err(RpcError::CustomError(
            "Failed to find any parseable inner instructions".to_string(),
        ))
    }
}

impl Clone for SolanaRpcConnection {
    fn clone(&self) -> Self {
        unimplemented!()
    }
}

impl RpcConnection for SolanaRpcConnection {
    fn new<U: ToString>(url: U, commitment_config: Option<CommitmentConfig>) -> Self {
        let payer = Keypair::new();
        let commitment_config = commitment_config.unwrap_or(CommitmentConfig::confirmed());
        let client = RpcClient::new_with_commitment(url, commitment_config);
        Self { client, payer }
    }

    async fn create_and_send_transaction_with_event<T>(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(T, Signature, u64)>, RpcError>
    where
        T: AnchorDeserialize + Debug,
    {
        let pre_balance = self.client.get_balance(payer)?;
        let latest_blockhash = self.client.get_latest_blockhash()?;

        let mut instructions_vec = vec![
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
        ];
        instructions_vec.extend_from_slice(instructions);

        let transaction = Transaction::new_signed_with_payer(
            instructions_vec.as_slice(),
            Some(payer),
            signers,
            latest_blockhash,
        );
        let signature = self.client.send_and_confirm_transaction(&transaction)?;
        let sig_info = self.client.get_signature_statuses(&[signature]);
        let sig_info = sig_info.unwrap().value.first().unwrap().clone();
        let slot = sig_info.unwrap().slot;

        let mut event = transaction
            .message
            .instructions
            .iter()
            .find_map(|instruction| T::try_from_slice(instruction.data.as_slice()).ok());

        if event.is_none() {
            let parsed_event: Result<T, RpcError> = self.parse_inner_instructions::<T>(signature);
            event = match parsed_event {
                Ok(e) => Some(e),
                Err(e) => {
                    println!("solana_rpc: error parsing inner instructions: {:?}", e);
                    None
                }
            }
        }

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

        let result = event.map(|event| (event, signature, slot));
        Ok(result)
    }

    async fn confirm_transaction(&mut self, transaction: Signature) -> Result<bool, RpcError> {
        self.client
            .confirm_transaction(&transaction)
            .map_err(RpcError::from)
    }

    fn get_payer(&self) -> &Keypair {
        &self.payer
    }

    async fn get_account(&mut self, address: Pubkey) -> Result<Option<Account>, RpcError> {
        debug!("CommitmentConfig: {:?}", self.client.commitment());
        let result = self
            .client
            .get_account_with_commitment(&address, self.client.commitment());
        result.map(|account| account.value).map_err(RpcError::from)
    }

    fn set_account(&mut self, _address: &Pubkey, _account: &AccountSharedData) {
        todo!()
    }

    async fn get_minimum_balance_for_rent_exemption(
        &mut self,
        data_len: usize,
    ) -> Result<u64, RpcError> {
        match self.client.get_minimum_balance_for_rent_exemption(data_len) {
            Ok(result) => Ok(result),
            Err(e) => Err(RpcError::ClientError(e)),
        }
    }

    async fn get_latest_blockhash(&mut self) -> Result<Hash, RpcError> {
        self.client.get_latest_blockhash().map_err(RpcError::from)
    }

    async fn process_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<Signature, RpcError> {
        debug!("CommitmentConfig: {:?}", self.client.commitment());
        match self.client.send_and_confirm_transaction(&transaction) {
            Ok(signature) => Ok(signature),
            Err(e) => Err(RpcError::ClientError(e)),
        }
    }

    async fn process_transaction_with_context(
        &mut self,
        transaction: Transaction,
    ) -> Result<(Signature, Slot), RpcError> {
        debug!("CommitmentConfig: {:?}", self.client.commitment());
        match self.client.send_and_confirm_transaction(&transaction) {
            Ok(signature) => {
                let sig_info = self.client.get_signature_statuses(&[signature]);
                let sig_info = sig_info.unwrap().value.first().unwrap().clone();
                let slot = sig_info.unwrap().slot;
                Ok((signature, slot))
            }
            Err(e) => Err(RpcError::ClientError(e)),
        }
    }

    async fn get_slot(&mut self) -> Result<u64, RpcError> {
        self.client.get_slot().map_err(RpcError::from)
    }

    async fn airdrop_lamports(
        &mut self,
        to: &Pubkey,
        lamports: u64,
    ) -> Result<Signature, RpcError> {
        let signature = self
            .client
            .request_airdrop(to, lamports)
            .map_err(RpcError::from)?;
        // TODO: Find a different way this can result in an infinite loop
        println!("Airdrop signature: {:?}", signature);
        loop {
            let confirmed = self
                .client
                .confirm_transaction_with_commitment(&signature, self.client.commitment())?
                .value;
            if confirmed {
                break;
            }
        }

        Ok(signature)
    }

    async fn get_balance(&mut self, pubkey: &Pubkey) -> Result<u64, RpcError> {
        self.client.get_balance(pubkey).map_err(RpcError::from)
    }

    fn warp_to_slot(&mut self, _slot: Slot) -> Result<(), RpcError> {
        todo!()
    }
}
