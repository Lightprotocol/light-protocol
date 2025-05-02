use std::{
    fmt::{Debug, Display, Formatter},
    time::Duration,
};

use async_trait::async_trait;
use borsh::BorshDeserialize;
use bs58;
use light_compressed_account::indexer_event::{
    event::{BatchPublicTransactionEvent, PublicTransactionEvent},
    parse::event_from_light_transaction,
};
use solana_account::Account;
use solana_clock::Slot;
use solana_commitment_config::CommitmentConfig;
use solana_hash::Hash;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_api::config::{RpcSendTransactionConfig, RpcTransactionConfig};
use solana_signature::Signature;
use solana_transaction::Transaction;
use solana_transaction_status::{
    option_serializer::OptionSerializer, TransactionStatus, UiInstruction, UiTransactionEncoding,
};
use tokio::time::{sleep, Instant};
use tracing::warn;

use super::rpc_connection::RpcConnectionConfig;
use crate::{
    indexer::{photon_indexer::PhotonIndexer, Indexer},
    rpc::{errors::RpcError, merkle_tree::MerkleTreeExt, rpc_connection::RpcConnection},
};

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

#[derive(Clone, Debug, Copy)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub retry_delay: Duration,
    /// Max Light slot timeout in time based on solana slot length and light
    /// slot length.
    pub timeout: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        RetryConfig {
            max_retries: 30,
            retry_delay: Duration::from_secs(1),
            timeout: Duration::from_secs(60),
        }
    }
}

#[allow(dead_code)]
pub struct SolanaRpcConnection {
    pub client: RpcClient,
    pub payer: Keypair,
    pub retry_config: RetryConfig,
    pub indexer: Option<PhotonIndexer>,
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
    pub fn new_with_retry(config: RpcConnectionConfig, retry_config: Option<RetryConfig>) -> Self {
        let payer = Keypair::new();
        let commitment_config = config
            .commitment_config
            .unwrap_or(CommitmentConfig::confirmed());
        let client = RpcClient::new_with_commitment(config.url.to_string(), commitment_config);
        let retry_config = retry_config.unwrap_or_default();

        let indexer = if config.with_indexer {
            Some(PhotonIndexer::new(
                "http://127.0.0.1:8784".to_string(),
                None,
            ))
        } else {
            None
        };
        Self {
            client,
            payer,
            retry_config,
            indexer,
        }
    }

    pub fn add_indexer(&mut self, path: String, api_key: Option<String>) {
        self.indexer = Some(PhotonIndexer::new(path, api_key));
    }

    async fn retry<F, Fut, T>(&self, operation: F) -> Result<T, RpcError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, RpcError>>,
    {
        let mut attempts = 0;
        let start_time = Instant::now();
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    let retry = self.should_retry(&e);
                    if retry {
                        attempts += 1;
                        if attempts >= self.retry_config.max_retries
                            || start_time.elapsed() >= self.retry_config.timeout
                        {
                            return Err(e);
                        }
                        warn!(
                            "Operation failed, retrying in {:?} (attempt {}/{}): {:?}",
                            self.retry_config.retry_delay,
                            attempts,
                            self.retry_config.max_retries,
                            e
                        );
                        tokio::task::yield_now().await;
                        sleep(self.retry_config.retry_delay).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }

    async fn _create_and_send_transaction_with_batched_event(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Option<(Vec<BatchPublicTransactionEvent>, Signature, Slot)>, RpcError> {
        let latest_blockhash = self.client.get_latest_blockhash()?;

        let mut instructions_vec = vec![
            solana_compute_budget_interface::ComputeBudgetInstruction::set_compute_unit_limit(
                1_000_000,
            ),
        ];
        instructions_vec.extend_from_slice(instructions);

        let transaction = Transaction::new_signed_with_payer(
            instructions_vec.as_slice(),
            Some(payer),
            signers,
            latest_blockhash,
        );

        let (signature, slot) = self
            .process_transaction_with_context(transaction.clone())
            .await?;

        let mut vec = Vec::new();
        let mut vec_accounts = Vec::new();
        let mut program_ids = Vec::new();
        instructions_vec.iter().for_each(|x| {
            program_ids.push(x.program_id);
            vec.push(x.data.clone());
            vec_accounts.push(x.accounts.iter().map(|x| x.pubkey).collect());
        });
        {
            let rpc_transaction_config = RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Base64),
                commitment: Some(self.client.commitment()),
                ..Default::default()
            };
            let transaction = self
                .client
                .get_transaction_with_config(&signature, rpc_transaction_config)
                .map_err(|e| RpcError::CustomError(e.to_string()))?;
            let decoded_transaction = transaction
                .transaction
                .transaction
                .decode()
                .clone()
                .unwrap();
            let account_keys = decoded_transaction.message.static_account_keys();
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
                            let accounts = &ui_compiled_instruction.accounts;
                            let data = bs58::decode(&ui_compiled_instruction.data)
                                .into_vec()
                                .map_err(|_| {
                                    RpcError::CustomError(
                                        "Failed to decode instruction data".to_string(),
                                    )
                                })?;
                            vec.push(data);
                            program_ids.push(
                                account_keys[ui_compiled_instruction.program_id_index as usize],
                            );
                            vec_accounts.push(
                                accounts
                                    .iter()
                                    .map(|x| account_keys[(*x) as usize])
                                    .collect(),
                            );
                        }
                        UiInstruction::Parsed(_) => {
                            println!("Parsed instructions are not implemented yet");
                        }
                    }
                }
            }
        }
        let parsed_event =
            event_from_light_transaction(program_ids.as_slice(), vec.as_slice(), vec_accounts)
                .unwrap();
        let event = parsed_event.map(|e| (e, signature, slot));
        Ok(event)
    }

    async fn _create_and_send_transaction_with_event<T>(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Option<(T, Signature, u64)>, RpcError>
    where
        T: BorshDeserialize + Send + Debug,
    {
        let latest_blockhash = self.client.get_latest_blockhash()?;

        let mut instructions_vec = vec![
            solana_compute_budget_interface::ComputeBudgetInstruction::set_compute_unit_limit(
                1_000_000,
            ),
        ];
        instructions_vec.extend_from_slice(instructions);

        let transaction = Transaction::new_signed_with_payer(
            instructions_vec.as_slice(),
            Some(payer),
            signers,
            latest_blockhash,
        );

        let (signature, slot) = self
            .process_transaction_with_context(transaction.clone())
            .await?;

        let mut parsed_event = None;
        for instruction in &transaction.message.instructions {
            let ix_data = instruction.data.clone();
            match T::deserialize(&mut &instruction.data[..]) {
                Ok(e) => {
                    parsed_event = Some(e);
                    break;
                }
                Err(e) => {
                    warn!(
                        "Failed to parse event: {:?}, type: {:?}, ix data: {:?}",
                        e,
                        std::any::type_name::<T>(),
                        ix_data
                    );
                }
            }
        }

        if parsed_event.is_none() {
            parsed_event = self.parse_inner_instructions::<T>(signature).ok();
        }

        let result = parsed_event.map(|e| (e, signature, slot));
        Ok(result)
    }
}

impl SolanaRpcConnection {
    #[allow(clippy::result_large_err)]
    fn parse_inner_instructions<T: BorshDeserialize>(
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

                        match T::try_from_slice(data.as_slice()) {
                            Ok(parsed_data) => return Ok(parsed_data),
                            Err(e) => {
                                warn!("Failed to parse inner instruction: {:?}", e);
                            }
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

#[async_trait]
impl RpcConnection for SolanaRpcConnection {
    fn new(config: RpcConnectionConfig) -> Self
    where
        Self: Sized,
    {
        Self::new_with_retry(config, None)
    }

    fn get_payer(&self) -> &Keypair {
        &self.payer
    }

    fn get_url(&self) -> String {
        self.client.url()
    }

    async fn health(&self) -> Result<(), RpcError> {
        self.retry(|| async { self.client.get_health().map_err(RpcError::from) })
            .await
    }

    async fn get_program_accounts(
        &self,
        program_id: &Pubkey,
    ) -> Result<Vec<(Pubkey, Account)>, RpcError> {
        self.retry(|| async {
            self.client
                .get_program_accounts(program_id)
                .map_err(RpcError::from)
        })
        .await
    }

    async fn process_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<Signature, RpcError> {
        self.retry(|| async {
            self.client
                .send_and_confirm_transaction(&transaction)
                .map_err(RpcError::from)
        })
        .await
    }

    async fn process_transaction_with_context(
        &mut self,
        transaction: Transaction,
    ) -> Result<(Signature, Slot), RpcError> {
        self.retry(|| async {
            let signature = self.client.send_and_confirm_transaction(&transaction)?;
            let sig_info = self.client.get_signature_statuses(&[signature])?;
            let slot = sig_info
                .value
                .first()
                .and_then(|s| s.as_ref())
                .map(|s| s.slot)
                .ok_or_else(|| RpcError::CustomError("Failed to get slot".into()))?;
            Ok((signature, slot))
        })
        .await
    }

    async fn confirm_transaction(&self, signature: Signature) -> Result<bool, RpcError> {
        self.retry(|| async {
            self.client
                .confirm_transaction(&signature)
                .map_err(RpcError::from)
        })
        .await
    }

    async fn get_account(&self, address: Pubkey) -> Result<Option<Account>, RpcError> {
        self.retry(|| async {
            self.client
                .get_account_with_commitment(&address, self.client.commitment())
                .map(|response| response.value)
                .map_err(RpcError::from)
        })
        .await
    }

    async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> Result<u64, RpcError> {
        self.retry(|| async {
            self.client
                .get_minimum_balance_for_rent_exemption(data_len)
                .map_err(RpcError::from)
        })
        .await
    }

    async fn airdrop_lamports(
        &mut self,
        to: &Pubkey,
        lamports: u64,
    ) -> Result<Signature, RpcError> {
        self.retry(|| async {
            let signature = self
                .client
                .request_airdrop(to, lamports)
                .map_err(RpcError::ClientError)?;
            self.retry(|| async {
                if self
                    .client
                    .confirm_transaction_with_commitment(&signature, self.client.commitment())?
                    .value
                {
                    Ok(())
                } else {
                    Err(RpcError::CustomError("Airdrop not confirmed".into()))
                }
            })
            .await?;

            Ok(signature)
        })
        .await
    }

    async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64, RpcError> {
        self.retry(|| async { self.client.get_balance(pubkey).map_err(RpcError::from) })
            .await
    }

    async fn get_latest_blockhash(&mut self) -> Result<(Hash, u64), RpcError> {
        self.retry(|| async {
            self.client
                // Confirmed commitments land more reliably than finalized
                // https://www.helius.dev/blog/how-to-deal-with-blockhash-errors-on-solana#how-to-deal-with-blockhash-errors
                .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
                .map_err(RpcError::from)
        })
        .await
    }

    async fn get_slot(&self) -> Result<u64, RpcError> {
        self.retry(|| async { self.client.get_slot().map_err(RpcError::from) })
            .await
    }

    async fn send_transaction(&self, transaction: &Transaction) -> Result<Signature, RpcError> {
        self.retry(|| async {
            self.client
                .send_transaction_with_config(
                    transaction,
                    RpcSendTransactionConfig {
                        skip_preflight: true,
                        max_retries: Some(self.retry_config.max_retries as usize),
                        ..Default::default()
                    },
                )
                .map_err(RpcError::from)
        })
        .await
    }

    async fn send_transaction_with_config(
        &self,
        transaction: &Transaction,
        config: RpcSendTransactionConfig,
    ) -> Result<Signature, RpcError> {
        self.retry(|| async {
            self.client
                .send_transaction_with_config(transaction, config)
                .map_err(RpcError::from)
        })
        .await
    }

    async fn get_transaction_slot(&self, signature: &Signature) -> Result<u64, RpcError> {
        self.retry(|| async {
            Ok(self
                .client
                .get_transaction_with_config(
                    signature,
                    RpcTransactionConfig {
                        encoding: Some(UiTransactionEncoding::Base64),
                        commitment: Some(self.client.commitment()),
                        ..Default::default()
                    },
                )
                .map_err(RpcError::from)?
                .slot)
        })
        .await
    }

    async fn get_signature_statuses(
        &self,
        signatures: &[Signature],
    ) -> Result<Vec<Option<TransactionStatus>>, RpcError> {
        self.client
            .get_signature_statuses(signatures)
            .map(|response| response.value)
            .map_err(RpcError::from)
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

    async fn create_and_send_transaction_with_public_event(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Option<(PublicTransactionEvent, Signature, Slot)>, RpcError> {
        let parsed_event = self
            ._create_and_send_transaction_with_batched_event(instructions, payer, signers)
            .await?;

        let event = parsed_event.map(|(e, signature, slot)| (e[0].event.clone(), signature, slot));
        Ok(event)
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

    fn indexer(&self) -> Result<&impl Indexer, RpcError> {
        self.indexer.as_ref().ok_or(RpcError::IndexerNotInitialized)
    }

    fn indexer_mut(&mut self) -> Result<&mut impl Indexer, RpcError> {
        self.indexer.as_mut().ok_or(RpcError::IndexerNotInitialized)
    }
}

impl MerkleTreeExt for SolanaRpcConnection {}
