use std::{
    fmt::{Debug, Display, Formatter},
    time::Duration,
};

use async_trait::async_trait;
use borsh::BorshDeserialize;
use light_compressed_account::event::{event_from_light_transaction, PublicTransactionEvent};
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcSendTransactionConfig, RpcTransactionConfig},
};
use solana_program::{clock::Slot, hash::Hash, pubkey::Pubkey};
use solana_sdk::{
    account::{Account, AccountSharedData},
    bs58,
    clock::UnixTimestamp,
    commitment_config::CommitmentConfig,
    epoch_info::EpochInfo,
    instruction::Instruction,
    signature::{Keypair, Signature},
    transaction::Transaction,
};
use solana_transaction_status::{
    option_serializer::OptionSerializer, TransactionStatus, UiInstruction, UiTransactionEncoding,
};
use tokio::time::{sleep, Instant};
use tracing::warn;

use crate::{
    rate_limiter::RateLimiter,
    rpc::{errors::RpcError, merkle_tree::MerkleTreeExt, rpc_connection::RpcConnection},
    transaction_params::TransactionParams,
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
    retry_config: RetryConfig,
    rpc_rate_limiter: Option<RateLimiter>,
    send_tx_rate_limiter: Option<RateLimiter>,
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
    pub fn new_with_retry<U: ToString>(
        url: U,
        commitment_config: Option<CommitmentConfig>,
        retry_config: Option<RetryConfig>,
        rpc_rps: Option<u32>,
        send_tx_rps: Option<u32>,
    ) -> Self {
        let payer = Keypair::new();
        let commitment_config = commitment_config.unwrap_or(CommitmentConfig::confirmed());
        let client = RpcClient::new_with_commitment(url.to_string(), commitment_config);
        let retry_config = retry_config.unwrap_or_default();

        let mut rpc_rate_limiter = None;
        if let Some(rps) = rpc_rps {
            rpc_rate_limiter = Some(RateLimiter::new(rps));
        }

        let mut send_tx_rate_limiter = None;
        if let Some(rps) = send_tx_rps {
            send_tx_rate_limiter = Some(RateLimiter::new(rps));
        }

        Self {
            client,
            payer,
            retry_config,
            rpc_rate_limiter,
            send_tx_rate_limiter,
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
            if let Some(limiter) = &self.rpc_rate_limiter {
                limiter.acquire_with_wait().await;
            }

            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !self.should_retry(&e) {
                        return Err(e);
                    }

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

    async fn retry_with_tx_rate_limit<F, Fut, T>(&self, operation: F) -> Result<T, RpcError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, RpcError>>,
    {
        let mut attempts = 0;
        let start_time = Instant::now();
        loop {
            if let Some(limiter) = &self.send_tx_rate_limiter {
                limiter.acquire_with_wait().await;
            }

            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !self.should_retry(&e) {
                        return Err(e);
                    }

                    attempts += 1;
                    if attempts >= self.retry_config.max_retries
                        || start_time.elapsed() >= self.retry_config.timeout
                    {
                        return Err(e);
                    }
                    warn!(
                        "Transaction operation failed, retrying in {:?} (attempt {}/{}): {:?}",
                        self.retry_config.retry_delay, attempts, self.retry_config.max_retries, e
                    );
                    sleep(self.retry_config.retry_delay).await;
                }
            }
        }
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
    fn new<U: ToString>(url: U, commitment_config: Option<CommitmentConfig>) -> Self
    where
        Self: Sized,
    {
        Self::new_with_retry(url, commitment_config, None, None, None)
    }

    fn set_rpc_rate_limiter(&mut self, rate_limiter: RateLimiter) {
        self.rpc_rate_limiter = Some(rate_limiter);
    }

    fn set_send_tx_rate_limiter(&mut self, rate_limiter: RateLimiter) {
        self.send_tx_rate_limiter = Some(rate_limiter);
    }

    fn rpc_rate_limiter(&self) -> Option<&RateLimiter> {
        self.rpc_rate_limiter.as_ref()
    }

    fn send_tx_rate_limiter(&self) -> Option<&RateLimiter> {
        self.send_tx_rate_limiter.as_ref()
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

    async fn process_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<Signature, RpcError> {
        self.retry_with_tx_rate_limit(|| async {
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
        self.retry_with_tx_rate_limit(|| async {
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

    async fn process_transaction_with_config(
        &mut self,
        transaction: Transaction,
        config: RpcSendTransactionConfig,
    ) -> Result<Signature, RpcError> {
        self.send_transaction_with_config(&transaction, RpcSendTransactionConfig { ..config })
            .await
    }

    async fn create_and_send_transaction_with_event<T>(
        &mut self,
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
            let ix_data = instruction.data.clone();
            match T::deserialize(&mut &instruction.data[..]) {
                Ok(e) => {
                    parsed_event = Some(e);
                    break;
                }
                Err(e) => {
                    warn!("Failed to parse event: {:?}, type: {:?}, ix data: {:?}", e, std::any::type_name::<T>(), ix_data);
                }
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
            if transaction_params.num_input_compressed_accounts != 0
                || transaction_params.num_output_compressed_accounts != 0
            {
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

    async fn get_account(&mut self, address: Pubkey) -> Result<Option<Account>, RpcError> {
        self.retry(|| async {
            self.client
                .get_account_with_commitment(&address, self.client.commitment())
                .map(|response| response.value)
                .map_err(RpcError::from)
        })
        .await
    }

    fn set_account(&mut self, _address: &Pubkey, _account: &AccountSharedData) {
        unimplemented!()
    }

    async fn get_minimum_balance_for_rent_exemption(
        &mut self,
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

    async fn get_balance(&mut self, pubkey: &Pubkey) -> Result<u64, RpcError> {
        self.retry(|| async { self.client.get_balance(pubkey).map_err(RpcError::from) })
            .await
    }

    async fn get_latest_blockhash(&mut self) -> Result<Hash, RpcError> {
        self.retry(|| async {
            self.client
                // Confirmed commitments land more reliably than finalized
                // https://www.helius.dev/blog/how-to-deal-with-blockhash-errors-on-solana#how-to-deal-with-blockhash-errors
                .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
                .map(|response| response.0)
                .map_err(RpcError::from)
        })
        .await
    }

    async fn get_slot(&mut self) -> Result<u64, RpcError> {
        self.retry(|| async { self.client.get_slot().map_err(RpcError::from) })
            .await
    }

    async fn warp_to_slot(&mut self, _slot: Slot) -> Result<(), RpcError> {
        Err(RpcError::CustomError(
            "Warp to slot is not supported in SolanaRpcConnection".to_string(),
        ))
    }

    async fn send_transaction(&self, transaction: &Transaction) -> Result<Signature, RpcError> {
        self.retry_with_tx_rate_limit(|| async {
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
        self.retry_with_tx_rate_limit(|| async {
            self.client
                .send_transaction_with_config(transaction, config)
                .map_err(RpcError::from)
        })
        .await
    }

    async fn get_transaction_slot(&mut self, signature: &Signature) -> Result<u64, RpcError> {
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

    async fn get_block_height(&mut self) -> Result<u64, RpcError> {
        self.retry(|| async { self.client.get_block_height().map_err(RpcError::from) })
            .await
    }
    async fn create_and_send_transaction_with_public_event(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(PublicTransactionEvent, Signature, Slot)>, RpcError> {
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

        let mut vec = Vec::new();
        let mut vec_accounts = Vec::new();
        instructions_vec.iter().for_each(|x| {
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
        let parsed_event = event_from_light_transaction(vec.as_slice(), vec_accounts).unwrap();
        if let Some(transaction_params) = transaction_params {
            let mut deduped_signers = signers.to_vec();
            deduped_signers.dedup();
            let post_balance = self.get_account(*payer).await?.unwrap().lamports;
            // a network_fee is charged if there are input compressed accounts or new addresses
            let mut network_fee: i64 = 0;
            if transaction_params.num_input_compressed_accounts != 0
                || transaction_params.num_output_compressed_accounts != 0
            {
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
                return Err(RpcError::AssertRpcError(format!("unexpected balance after transaction: expected {expected_post_balance}, got {post_balance}")));
            }
        }
        let event = parsed_event.map(|e| (e.event, signature, slot));
        Ok(event)
    }
}

impl MerkleTreeExt for SolanaRpcConnection {}
