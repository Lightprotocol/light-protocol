use std::{
    fmt::{Debug, Display, Formatter},
    time::Duration,
};

use async_trait::async_trait;
use borsh::BorshDeserialize;
use bs58;
use light_compressed_account::TreeType;
use light_event::{
    event::{BatchPublicTransactionEvent, PublicTransactionEvent},
    parse::event_from_light_transaction,
};
use solana_account::Account;
use solana_clock::Slot;
use solana_commitment_config::CommitmentConfig;
use solana_hash::Hash;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::{pubkey, Pubkey};
use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_api::config::{RpcSendTransactionConfig, RpcTransactionConfig};
use solana_signature::Signature;
use solana_transaction::Transaction;
use solana_transaction_status_client_types::{
    option_serializer::OptionSerializer, TransactionStatus, UiInstruction, UiTransactionEncoding,
};
use tokio::time::{sleep, Instant};
use tracing::warn;

use super::LightClientConfig;
use crate::{
    indexer::{photon_indexer::PhotonIndexer, Indexer, TreeInfo},
    rpc::{
        errors::RpcError,
        get_light_state_tree_infos::{
            default_state_tree_lookup_tables, get_light_state_tree_infos,
        },
        merkle_tree::MerkleTreeExt,
        Rpc,
    },
};

pub enum RpcUrl {
    Testnet,
    Devnet,
    Localnet,
    ZKTestnet,
    Custom(String),
}

impl Display for RpcUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            RpcUrl::Testnet => "https://api.testnet.solana.com".to_string(),
            RpcUrl::Devnet => "https://api.devnet.solana.com".to_string(),
            RpcUrl::Localnet => "http://localhost:8899".to_string(),
            RpcUrl::ZKTestnet => "https://zk-testnet.helius.dev:8899".to_string(),
            RpcUrl::Custom(url) => url.clone(),
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
            max_retries: 10,
            retry_delay: Duration::from_secs(1),
            timeout: Duration::from_secs(60),
        }
    }
}

#[allow(dead_code)]
pub struct LightClient {
    pub client: RpcClient,
    pub payer: Keypair,
    pub retry_config: RetryConfig,
    pub indexer: Option<PhotonIndexer>,
    pub state_merkle_trees: Vec<TreeInfo>,
}

impl Debug for LightClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "LightClient {{ client: {:?} }}", self.client.url())
    }
}

impl LightClient {
    pub async fn new_with_retry(
        config: LightClientConfig,
        retry_config: Option<RetryConfig>,
    ) -> Result<Self, RpcError> {
        let payer = Keypair::new();
        let commitment_config = config
            .commitment_config
            .unwrap_or(CommitmentConfig::confirmed());
        let client = RpcClient::new_with_commitment(config.url.to_string(), commitment_config);
        let retry_config = retry_config.unwrap_or_default();

        let indexer = config
            .photon_url
            .map(|path| PhotonIndexer::new(path, config.api_key));

        let mut new = Self {
            client,
            payer,
            retry_config,
            indexer,
            state_merkle_trees: Vec::new(),
        };
        if config.fetch_active_tree {
            new.get_latest_active_state_trees().await?;
        }
        Ok(new)
    }

    pub fn add_indexer(&mut self, path: String, api_key: Option<String>) {
        self.indexer = Some(PhotonIndexer::new(path, api_key));
    }

    /// Detects the network type based on the RPC URL
    fn detect_network(&self) -> RpcUrl {
        let url = self.client.url();

        if url.contains("devnet") {
            RpcUrl::Devnet
        } else if url.contains("testnet") {
            RpcUrl::Testnet
        } else if url.contains("localhost") || url.contains("127.0.0.1") {
            RpcUrl::Localnet
        } else if url.contains("zk-testnet") {
            RpcUrl::ZKTestnet
        } else {
            // Default to mainnet for production URLs and custom URLs
            RpcUrl::Custom(url.to_string())
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
            program_ids.push(light_compressed_account::Pubkey::new_from_array(
                x.program_id.to_bytes(),
            ));
            vec.push(x.data.clone());
            vec_accounts.push(
                x.accounts
                    .iter()
                    .map(|x| light_compressed_account::Pubkey::new_from_array(x.pubkey.to_bytes()))
                    .collect(),
            );
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
                            program_ids.push(light_compressed_account::Pubkey::new_from_array(
                                account_keys[ui_compiled_instruction.program_id_index as usize]
                                    .to_bytes(),
                            ));
                            vec_accounts.push(
                                accounts
                                    .iter()
                                    .map(|x| {
                                        light_compressed_account::Pubkey::new_from_array(
                                            account_keys[(*x) as usize].to_bytes(),
                                        )
                                    })
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
                .map_err(|e| RpcError::CustomError(format!("Failed to parse event: {e:?}")))?;
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

impl LightClient {
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
impl Rpc for LightClient {
    async fn new(config: LightClientConfig) -> Result<Self, RpcError>
    where
        Self: Sized,
    {
        Self::new_with_retry(config, None).await
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

    /// Fetch the latest state tree addresses from the cluster.
    async fn get_latest_active_state_trees(&mut self) -> Result<Vec<TreeInfo>, RpcError> {
        let network = self.detect_network();

        // Return default test values for localnet
        if matches!(network, RpcUrl::Localnet) {
            use light_compressed_account::TreeType;
            use solana_pubkey::pubkey;

            use crate::indexer::TreeInfo;

            #[cfg(feature = "v2")]
            let default_trees = vec![
                TreeInfo {
                    tree: pubkey!("bmt1LryLZUMmF7ZtqESaw7wifBXLfXHQYoE4GAmrahU"),
                    queue: pubkey!("oq1na8gojfdUhsfCpyjNt6h4JaDWtHf1yQj4koBWfto"),
                    cpi_context: Some(pubkey!("cpi15BoVPKgEPw5o8wc2T816GE7b378nMXnhH3Xbq4y")),
                    next_tree_info: None,
                    tree_type: TreeType::StateV2,
                },
                TreeInfo {
                    tree: pubkey!("bmt2UxoBxB9xWev4BkLvkGdapsz6sZGkzViPNph7VFi"),
                    queue: pubkey!("oq2UkeMsJLfXt2QHzim242SUi3nvjJs8Pn7Eac9H9vg"),
                    cpi_context: Some(pubkey!("cpi2yGapXUR3As5SjnHBAVvmApNiLsbeZpF3euWnW6B")),
                    next_tree_info: None,
                    tree_type: TreeType::StateV2,
                },
                TreeInfo {
                    tree: pubkey!("bmt3ccLd4bqSVZVeCJnH1F6C8jNygAhaDfxDwePyyGb"),
                    queue: pubkey!("oq3AxjekBWgo64gpauB6QtuZNesuv19xrhaC1ZM1THQ"),
                    cpi_context: Some(pubkey!("cpi3mbwMpSX8FAGMZVP85AwxqCaQMfEk9Em1v8QK9Rf")),
                    next_tree_info: None,
                    tree_type: TreeType::StateV2,
                },
                TreeInfo {
                    tree: pubkey!("bmt4d3p1a4YQgk9PeZv5s4DBUmbF5NxqYpk9HGjQsd8"),
                    queue: pubkey!("oq4ypwvVGzCUMoiKKHWh4S1SgZJ9vCvKpcz6RT6A8dq"),
                    cpi_context: Some(pubkey!("cpi4yyPDc4bCgHAnsenunGA8Y77j3XEDyjgfyCKgcoc")),
                    next_tree_info: None,
                    tree_type: TreeType::StateV2,
                },
                TreeInfo {
                    tree: pubkey!("bmt5yU97jC88YXTuSukYHa8Z5Bi2ZDUtmzfkDTA2mG2"),
                    queue: pubkey!("oq5oh5ZR3yGomuQgFduNDzjtGvVWfDRGLuDVjv9a96P"),
                    cpi_context: Some(pubkey!("cpi5ZTjdgYpZ1Xr7B1cMLLUE81oTtJbNNAyKary2nV6")),
                    next_tree_info: None,
                    tree_type: TreeType::StateV2,
                },
            ];

            #[cfg(not(feature = "v2"))]
            let default_trees = vec![TreeInfo {
                tree: pubkey!("smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT"),
                queue: pubkey!("nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148"),
                cpi_context: Some(pubkey!("cpi1uHzrEhBG733DoEJNgHCyRS3XmmyVNZx5fonubE4")),
                next_tree_info: None,
                tree_type: TreeType::StateV1,
            }];

            self.state_merkle_trees = default_trees.clone();
            return Ok(default_trees);
        }

        let (mainnet_tables, devnet_tables) = default_state_tree_lookup_tables();

        let lookup_tables = match network {
            RpcUrl::Devnet | RpcUrl::Testnet | RpcUrl::ZKTestnet => &devnet_tables,
            _ => &mainnet_tables, // Default to mainnet for production and custom URLs
        };

        let res = get_light_state_tree_infos(
            self,
            &lookup_tables[0].state_tree_lookup_table,
            &lookup_tables[0].nullify_table,
        )
        .await?;
        self.state_merkle_trees = res.clone();
        Ok(res)
    }

    /// Fetch the latest state tree addresses from the cluster.
    fn get_state_tree_infos(&self) -> Vec<TreeInfo> {
        self.state_merkle_trees.to_vec()
    }

    /// Gets a random active state tree.
    /// State trees are cached and have to be fetched or set.
    /// Returns v1 state trees by default, v2 state trees when v2 feature is enabled.
    fn get_random_state_tree_info(&self) -> Result<TreeInfo, RpcError> {
        let mut rng = rand::thread_rng();

        #[cfg(feature = "v2")]
        let filtered_trees: Vec<TreeInfo> = self
            .state_merkle_trees
            .iter()
            .filter(|tree| tree.tree_type == TreeType::StateV2)
            .copied()
            .collect();

        #[cfg(not(feature = "v2"))]
        let filtered_trees: Vec<TreeInfo> = self
            .state_merkle_trees
            .iter()
            .filter(|tree| tree.tree_type == TreeType::StateV1)
            .copied()
            .collect();

        select_state_tree_info(&mut rng, &filtered_trees)
    }

    /// Gets a random v1 state tree.
    /// State trees are cached and have to be fetched or set.
    fn get_random_state_tree_info_v1(&self) -> Result<TreeInfo, RpcError> {
        let mut rng = rand::thread_rng();
        let v1_trees: Vec<TreeInfo> = self
            .state_merkle_trees
            .iter()
            .filter(|tree| tree.tree_type == TreeType::StateV1)
            .copied()
            .collect();
        select_state_tree_info(&mut rng, &v1_trees)
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

    fn get_address_tree_v2(&self) -> TreeInfo {
        TreeInfo {
            tree: pubkey!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx"),
            queue: pubkey!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx"),
            cpi_context: None,
            next_tree_info: None,
            tree_type: TreeType::AddressV2,
        }
    }
}

impl MerkleTreeExt for LightClient {}

/// Selects a random state tree from the provided list.
///
/// This function should be used together with `get_state_tree_infos()` to first
/// retrieve the list of state trees, then select one randomly.
///
/// # Arguments
/// * `rng` - A mutable reference to a random number generator
/// * `state_trees` - A slice of `TreeInfo` representing state trees
///
/// # Returns
/// A randomly selected `TreeInfo` from the provided list, or an error if the list is empty
///
/// # Errors
/// Returns `RpcError::NoStateTreesAvailable` if the provided slice is empty
///
/// # Example
/// ```ignore
/// use rand::thread_rng;
/// let tree_infos = client.get_state_tree_infos();
/// let mut rng = thread_rng();
/// let selected_tree = select_state_tree_info(&mut rng, &tree_infos)?;
/// ```
pub fn select_state_tree_info<R: rand::Rng>(
    rng: &mut R,
    state_trees: &[TreeInfo],
) -> Result<TreeInfo, RpcError> {
    if state_trees.is_empty() {
        return Err(RpcError::NoStateTreesAvailable);
    }

    Ok(state_trees[rng.gen_range(0..state_trees.len())])
}
