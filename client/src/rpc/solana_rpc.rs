use crate::rpc::errors::RpcError;
use crate::rpc::rpc_connection::RpcConnection;
use crate::transaction_params::TransactionParams;
use async_trait::async_trait;
use borsh::BorshDeserialize;
use log::warn;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcSendTransactionConfig, RpcTransactionConfig};
use solana_program::clock::Slot;
use solana_program::hash::Hash;
use solana_program::pubkey::Pubkey;
use solana_sdk::account::{Account, AccountSharedData};
use solana_sdk::bs58;
use solana_sdk::clock::UnixTimestamp;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::epoch_info::EpochInfo;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::transaction::Transaction;
use solana_transaction_status::option_serializer::OptionSerializer;
use solana_transaction_status::{UiInstruction, UiTransactionEncoding};
use std::fmt::{Debug, Display, Formatter};
use std::time::Duration;
use tokio::time::{sleep, Instant};

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
    pub timeout: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        RetryConfig {
            max_retries: 10,
            retry_delay: Duration::from_millis(100),
            timeout: Duration::from_secs(60),
        }
    }
}

#[allow(dead_code)]
pub struct SolanaRpcConnection {
    pub client: RpcClient,
    pub payer: Keypair,
    retry_config: RetryConfig,
}

impl SolanaRpcConnection {
    pub fn new_with_retry<U: ToString>(
        url: U,
        commitment_config: Option<CommitmentConfig>,
        retry_config: Option<RetryConfig>,
        payer: Option<Keypair>,
    ) -> Self {
        let payer = payer.unwrap_or(Keypair::new());
        let commitment_config = commitment_config.unwrap_or(CommitmentConfig::confirmed());
        let client = RpcClient::new_with_commitment(url.to_string(), commitment_config);
        let retry_config = retry_config.unwrap_or_default();
        Self {
            client,
            payer,
            retry_config,
        }
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
                    attempts += 1;
                    if attempts >= self.retry_config.max_retries
                        || start_time.elapsed() >= self.retry_config.timeout
                    {
                        return Err(e);
                    }
                    warn!(
                        "Operation failed, retrying in {:?} (attempt {}/{}): {:?}",
                        self.retry_config.retry_delay, attempts, self.retry_config.max_retries, e
                    );
                    sleep(self.retry_config.retry_delay).await;
                }
            }
        }
    }
}

impl SolanaRpcConnection {
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

#[async_trait]
impl RpcConnection for SolanaRpcConnection {
    fn new<U: ToString>(url: U, commitment_config: Option<CommitmentConfig>) -> Self
    where
        Self: Sized,
    {
        Self::new_with_retry(url, commitment_config, None, None)
    }

    async fn get_payer(&self) -> Keypair {
        self.payer.insecure_clone()
    }

    fn get_url(&self) -> String {
        self.client.url()
    }

    async fn health(&self) -> Result<(), RpcError> {
        self.retry(|| async { self.client.get_health().map_err(RpcError::from) })
            .await
    }

    async fn get_block_time(&self, slot: u64) -> Result<UnixTimestamp, RpcError> {
        self.retry(|| async { self.client.get_block_time(slot).map_err(RpcError::from) })
            .await
    }

    async fn get_epoch_info(&self) -> Result<EpochInfo, RpcError> {
        self.retry(|| async { self.client.get_epoch_info().map_err(RpcError::from) })
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

    async fn process_transaction(&self, transaction: Transaction) -> Result<Signature, RpcError> {
        self.retry(|| async {
            self.client
                .send_and_confirm_transaction(&transaction)
                .map_err(RpcError::from)
        })
        .await
    }

    async fn process_transaction_with_context(
        &self,
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

    async fn create_and_send_transaction_with_event<T>(
        &self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(T, Signature, u64)>, RpcError>
    where
        T: BorshDeserialize + Send + Debug,
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

        let (signature, slot) = self
            .process_transaction_with_context(transaction.clone())
            .await?;

        let mut parsed_event = None;
        for instruction in &transaction.message.instructions {
            if let Ok(e) = T::deserialize(&mut &instruction.data[..]) {
                parsed_event = Some(e);
                break;
            }
        }

        if parsed_event.is_none() {
            parsed_event = self.parse_inner_instructions::<T>(signature).ok();
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
                return Err(RpcError::AssertRpcError(format!("unexpected balance after transaction: expected {expected_post_balance}, got {post_balance}")));
            }
        }

        let result = parsed_event.map(|e| (e, signature, slot));
        Ok(result)
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

    async fn set_account(&self, _address: &Pubkey, _account: &AccountSharedData) {
        unimplemented!()
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

    async fn airdrop_lamports(&self, to: &Pubkey, lamports: u64) -> Result<Signature, RpcError> {
        self.retry(|| async {
            let signature = self
                .client
                .request_airdrop(to, lamports)
                .map_err(RpcError::ClientError)?;
            println!("Airdrop signature: {:?}", signature);
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

    async fn get_latest_blockhash(&self) -> Result<Hash, RpcError> {
        self.retry(|| async { self.client.get_latest_blockhash().map_err(RpcError::from) })
            .await
    }

    async fn get_slot(&self) -> Result<u64, RpcError> {
        self.retry(|| async { self.client.get_slot().map_err(RpcError::from) })
            .await
    }

    async fn warp_to_slot(&self, _slot: Slot) -> Result<(), RpcError> {
        Err(RpcError::CustomError(
            "Warp to slot is not supported in SolanaRpcConnection".to_string(),
        ))
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
}
