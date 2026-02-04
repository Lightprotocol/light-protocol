use std::{fmt::Debug, marker::Send};

use anchor_lang::pubkey;
use async_trait::async_trait;
use borsh::BorshDeserialize;
use light_client::{
    indexer::{CompressedAccount, CompressedTokenAccount, Context, Indexer, Response, TreeInfo},
    interface::{AccountInterface, TokenAccountInterface},
    rpc::{LightClientConfig, Rpc, RpcError},
};
use light_compressed_account::TreeType;
use light_event::{
    error::ParseIndexerEventError,
    event::{BatchPublicTransactionEvent, PublicTransactionEvent},
    parse::event_from_light_transaction,
};
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_sdk::{
    account::Account,
    address_lookup_table::AddressLookupTableAccount,
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
    litesvm_extensions::LiteSvmExtensions,
    program_test::LightProgramTest,
};

#[async_trait]
impl Rpc for LightProgramTest {
    async fn new(_config: LightClientConfig) -> Result<Self, RpcError>
    where
        Self: Sized,
    {
        Err(RpcError::CustomError(
            "LightProgramTest::new is not supported in program-test context".into(),
        ))
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
        program_id: &Pubkey,
    ) -> Result<Vec<(Pubkey, Account)>, RpcError> {
        Ok(self.context.get_program_accounts(program_id))
    }

    async fn get_program_accounts_with_discriminator(
        &self,
        program_id: &Pubkey,
        discriminator: &[u8],
    ) -> Result<Vec<(Pubkey, Account)>, RpcError> {
        let all_accounts = self.context.get_program_accounts(program_id);
        Ok(all_accounts
            .into_iter()
            .filter(|(_, account)| {
                account.data.len() >= discriminator.len()
                    && &account.data[..discriminator.len()] == discriminator
            })
            .collect())
    }

    async fn confirm_transaction(&self, _transaction: Signature) -> Result<bool, RpcError> {
        Ok(true)
    }

    async fn get_account(&self, address: Pubkey) -> Result<Option<Account>, RpcError> {
        Ok(self.context.get_account(&address))
    }

    async fn get_multiple_accounts(
        &self,
        addresses: &[Pubkey],
    ) -> Result<Vec<Option<Account>>, RpcError> {
        Ok(addresses
            .iter()
            .map(|address| self.context.get_account(address))
            .collect())
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
            // Delegate to _send_transaction_with_batched_event which handles counter, logging and pre_context
            self._send_transaction_with_batched_event(transaction)
                .await?;
        } else {
            // Cache the current context before transaction execution
            let pre_context_snapshot = self.context.clone();

            // Handle transaction directly without logging (logging should be done elsewhere)
            self.transaction_counter += 1;
            let _res = self.context.send_transaction(transaction).map_err(|x| {
                if self.config.log_failed_tx {
                    println!("{}", x.meta.pretty_logs());
                }

                RpcError::TransactionError(x.err)
            })?;

            self.maybe_print_logs(_res.pretty_logs());

            // Update pre_context only after successful transaction execution
            self.pre_context = Some(pre_context_snapshot);
        }
        Ok(sig)
    }

    async fn process_transaction_with_context(
        &mut self,
        transaction: Transaction,
    ) -> Result<(Signature, Slot), RpcError> {
        let sig = *transaction.signatures.first().unwrap();

        // Cache the current context before transaction execution
        let pre_context_snapshot = self.context.clone();

        self.transaction_counter += 1;
        let _res = self.context.send_transaction(transaction).map_err(|x| {
            if self.config.log_failed_tx {
                println!("{}", x.meta.pretty_logs());
            }
            RpcError::TransactionError(x.err)
        })?;

        let slot = self.context.get_sysvar::<Clock>().slot;
        self.maybe_print_logs(_res.pretty_logs());

        // Update pre_context only after successful transaction execution
        self.pre_context = Some(pre_context_snapshot);

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
            .iter()
            .copied()
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
            .iter()
            .copied()
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

    /// Gets a random v1 state tree.
    /// State trees are cached and have to be fetched or set.
    fn get_random_state_tree_info_v1(&self) -> Result<TreeInfo, RpcError> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        if self.test_accounts.v1_state_trees.is_empty() {
            return Err(RpcError::NoStateTreesAvailable);
        }
        Ok(self.test_accounts.v1_state_trees
            [rng.gen_range(0..self.test_accounts.v1_state_trees.len())]
        .into())
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

    async fn create_and_send_versioned_transaction<'a>(
        &'a mut self,
        _instructions: &'a [Instruction],
        _payer: &'a Pubkey,
        _signers: &'a [&'a Keypair],
        _address_lookup_tables: &'a [AddressLookupTableAccount],
    ) -> Result<Signature, RpcError> {
        unimplemented!(
            "create_and_send_versioned_transaction is unimplemented for LightProgramTest"
        );
    }

    async fn get_account_interface(
        &self,
        address: &Pubkey,
        _config: Option<light_client::indexer::IndexerRpcConfig>,
    ) -> Result<Response<Option<AccountInterface>>, RpcError> {
        let slot = self.context.get_sysvar::<Clock>().slot;

        // Hot: check on-chain first
        if let Some(account) = self.context.get_account(address) {
            if account.lamports > 0 {
                return Ok(Response {
                    context: Context { slot },
                    value: Some(AccountInterface::hot(*address, account)),
                });
            }
        }

        // Cold: check TestIndexer by onchain pubkey (mirrors Photon behavior)
        if let Some(indexer) = self.indexer.as_ref() {
            // First try: lookup by onchain_pubkey (for accounts with DECOMPRESSED_PDA_DISCRIMINATOR)
            if let Some(compressed_with_ctx) =
                indexer.find_compressed_account_by_onchain_pubkey(&address.to_bytes())
            {
                let owner: Pubkey = compressed_with_ctx.compressed_account.owner.into();
                let compressed: CompressedAccount = compressed_with_ctx.clone().try_into().map_err(
                    |e| {
                        RpcError::CustomError(format!(
                            "CompressedAccountWithMerkleContext conversion failed for address {}: {:?}",
                            address, e
                        ))
                    },
                )?;

                return Ok(Response {
                    context: Context { slot },
                    value: Some(AccountInterface::cold(*address, compressed, owner)),
                });
            }

            // Second try: lookup by PDA seed (for accounts whose address was derived from this pubkey)
            if let Some(compressed_with_ctx) =
                indexer.find_compressed_account_by_pda_seed(&address.to_bytes())
            {
                let owner: Pubkey = compressed_with_ctx.compressed_account.owner.into();
                let compressed: CompressedAccount = compressed_with_ctx.clone().try_into().map_err(
                    |e| {
                        RpcError::CustomError(format!(
                            "CompressedAccountWithMerkleContext conversion failed for PDA seed {}: {:?}",
                            address, e
                        ))
                    },
                )?;

                return Ok(Response {
                    context: Context { slot },
                    value: Some(AccountInterface::cold(*address, compressed, owner)),
                });
            }
        }

        Ok(Response {
            context: Context { slot },
            value: None,
        })
    }

    async fn get_token_account_interface(
        &self,
        address: &Pubkey,
        _config: Option<light_client::indexer::IndexerRpcConfig>,
    ) -> Result<Response<Option<TokenAccountInterface>>, RpcError> {
        use light_sdk::constants::LIGHT_TOKEN_PROGRAM_ID;

        let light_token_program_id: Pubkey = LIGHT_TOKEN_PROGRAM_ID.into();
        let slot = self.context.get_sysvar::<Clock>().slot;

        // Hot: check on-chain first (must be owned by LIGHT_TOKEN_PROGRAM_ID)
        if let Some(account) = self.context.get_account(address) {
            if account.lamports > 0 && account.owner == light_token_program_id {
                match TokenAccountInterface::hot(*address, account) {
                    Ok(iface) => {
                        return Ok(Response {
                            context: Context { slot },
                            value: Some(iface),
                        });
                    }
                    Err(_) => {
                        // Fall through to cold lookup if parsing failed
                    }
                }
            }
        }

        // Cold: check TestIndexer by onchain_pubkey, PDA seed, or token_data.owner
        if let Some(indexer) = self.indexer.as_ref() {
            // First try: lookup by onchain_pubkey (for accounts with DECOMPRESSED_PDA_DISCRIMINATOR)
            let token_acc = indexer
                .find_token_account_by_onchain_pubkey(&address.to_bytes())
                .or_else(|| {
                    // Second try: lookup by PDA seed (for accounts whose address was derived from this pubkey)
                    indexer.find_token_account_by_pda_seed(&address.to_bytes())
                });

            if let Some(token_acc) = token_acc {
                // Convert to CompressedTokenAccount
                let compressed_account: CompressedAccount = token_acc
                    .compressed_account
                    .clone()
                    .try_into()
                    .map_err(|e| RpcError::CustomError(format!("conversion error: {:?}", e)))?;

                let compressed_token = CompressedTokenAccount {
                    token: token_acc.token_data.clone(),
                    account: compressed_account,
                };

                return Ok(Response {
                    context: Context { slot },
                    value: Some(TokenAccountInterface::cold(
                        *address,
                        compressed_token,
                        *address, // owner = hot address for program-owned tokens
                        light_token_program_id,
                    )),
                });
            }

            // Third try: lookup by token_data.owner (for tokens where owner == address)
            let result = indexer
                .get_compressed_token_accounts_by_owner(address, None, None)
                .await
                .map_err(|e| RpcError::CustomError(format!("indexer error: {}", e)))?;

            let items = result.value.items;
            if items.len() > 1 {
                return Err(RpcError::CustomError(format!(
                    "Ambiguous lookup: found {} compressed token accounts for address {}. \
                     Use get_compressed_token_accounts_by_owner for multiple accounts.",
                    items.len(),
                    address
                )));
            }

            if let Some(token_acc) = items.into_iter().next() {
                let key = token_acc
                    .account
                    .address
                    .map(Pubkey::new_from_array)
                    .unwrap_or(*address);
                return Ok(Response {
                    context: Context { slot },
                    value: Some(TokenAccountInterface::cold(
                        key,
                        token_acc,
                        *address, // owner = hot address for program-owned tokens
                        light_token_program_id,
                    )),
                });
            }
        }

        Ok(Response {
            context: Context { slot },
            value: None,
        })
    }

    async fn get_ata_interface(
        &self,
        owner: &Pubkey,
        mint: &Pubkey,
        _config: Option<light_client::indexer::IndexerRpcConfig>,
    ) -> Result<Response<Option<TokenAccountInterface>>, RpcError> {
        use light_client::indexer::GetCompressedTokenAccountsByOwnerOrDelegateOptions;
        use light_sdk::constants::LIGHT_TOKEN_PROGRAM_ID;
        use light_token::instruction::derive_token_ata;

        let (ata, _bump) = derive_token_ata(owner, mint);
        let light_token_program_id: Pubkey = LIGHT_TOKEN_PROGRAM_ID.into();
        let slot = self.context.get_sysvar::<Clock>().slot;

        // First try: on-chain (hot) lookup
        // We handle this directly instead of using get_token_account_interface
        // because we need to control owner_override for ata_bump() to work
        if let Some(account) = self.context.get_account(&ata) {
            if account.lamports > 0 && account.owner == light_token_program_id {
                match TokenAccountInterface::hot(ata, account) {
                    Ok(iface) => {
                        return Ok(Response {
                            context: Context { slot },
                            value: Some(iface),
                        });
                    }
                    Err(_) => {
                        // Fall through to cold lookup if parsing failed
                    }
                }
            }
        }

        // Cold: search compressed tokens by ata_pubkey + mint
        // In Light Protocol, token_data.owner is the token account pubkey (ATA), not wallet owner
        // But we need to pass the wallet owner for TokenAccountInterface::cold so ata_bump() works
        if let Some(indexer) = self.indexer.as_ref() {
            let options = Some(GetCompressedTokenAccountsByOwnerOrDelegateOptions {
                mint: Some(*mint),
                ..Default::default()
            });
            let result = indexer
                .get_compressed_token_accounts_by_owner(&ata, options, None)
                .await
                .map_err(|e| RpcError::CustomError(format!("indexer error: {}", e)))?;

            let items = result.value.items;
            if items.len() > 1 {
                return Err(RpcError::CustomError(format!(
                    "Ambiguous lookup: found {} compressed token accounts for ATA {} (owner: {}, mint: {}). \
                     Use get_compressed_token_accounts_by_owner for multiple accounts.",
                    items.len(),
                    ata,
                    owner,
                    mint
                )));
            }

            if let Some(token_acc) = items.into_iter().next() {
                return Ok(Response {
                    context: Context { slot },
                    value: Some(TokenAccountInterface::cold(
                        ata, // key = ATA pubkey (derived, so we use it directly)
                        token_acc,
                        *owner, // owner_override = wallet owner (for ata_bump() to work)
                        light_token_program_id,
                    )),
                });
            }
        }

        Ok(Response {
            context: Context { slot },
            value: None,
        })
    }

    async fn get_multiple_account_interfaces(
        &self,
        addresses: Vec<&Pubkey>,
        _config: Option<light_client::indexer::IndexerRpcConfig>,
    ) -> Result<Response<Vec<Option<AccountInterface>>>, RpcError> {
        let slot = self.context.get_sysvar::<Clock>().slot;
        let mut results: Vec<Option<AccountInterface>> = vec![None; addresses.len()];

        // Batch fetch on-chain accounts (hot path)
        let owned_addresses: Vec<Pubkey> = addresses.iter().map(|a| **a).collect();
        let on_chain_accounts: Vec<Option<Account>> = owned_addresses
            .iter()
            .map(|addr| self.context.get_account(addr))
            .collect();

        // Track which addresses still need cold lookup
        let mut cold_lookup_indices: Vec<usize> = Vec::new();
        let mut cold_lookup_pubkeys: Vec<[u8; 32]> = Vec::new();

        for (i, (address, maybe_account)) in addresses
            .iter()
            .zip(on_chain_accounts.into_iter())
            .enumerate()
        {
            if let Some(account) = maybe_account {
                if account.lamports > 0 {
                    results[i] = Some(AccountInterface::hot(**address, account));
                    continue;
                }
            }
            // Not found on-chain or has 0 lamports, need cold lookup
            cold_lookup_indices.push(i);
            cold_lookup_pubkeys.push(address.to_bytes());
        }

        // Batch lookup cold accounts from TestIndexer
        if !cold_lookup_pubkeys.is_empty() {
            if let Some(indexer) = self.indexer.as_ref() {
                let cold_results = indexer
                    .find_multiple_compressed_accounts_by_onchain_pubkeys(&cold_lookup_pubkeys);

                for (lookup_idx, maybe_compressed) in cold_results.into_iter().enumerate() {
                    let original_idx = cold_lookup_indices[lookup_idx];
                    if let Some(compressed_with_ctx) = maybe_compressed {
                        let owner: Pubkey = compressed_with_ctx.compressed_account.owner.into();
                        let compressed: CompressedAccount =
                            compressed_with_ctx.clone().try_into().map_err(|e| {
                                RpcError::CustomError(format!("conversion error: {:?}", e))
                            })?;

                        results[original_idx] = Some(AccountInterface::cold(
                            *addresses[original_idx],
                            compressed,
                            owner,
                        ));
                    }
                }
            }
        }

        Ok(Response {
            context: Context { slot },
            value: results,
        })
    }
}

impl LightProgramTest {
    fn maybe_print_logs(&self, logs: impl std::fmt::Display) {
        // Use enhanced logging if enabled and RUST_BACKTRACE is set
        if crate::logging::should_use_enhanced_logging(&self.config) {
            // Enhanced logging will be handled in the transaction processing methods
            return;
        }

        // Fallback to basic logging
        if !self.config.no_logs && cfg!(debug_assertions) && std::env::var("RUST_BACKTRACE").is_ok()
        {
            println!("{}", logs);
        }
    }

    async fn _send_transaction_with_batched_event(
        &mut self,
        transaction: Transaction,
    ) -> Result<Option<(Vec<BatchPublicTransactionEvent>, Signature, Slot)>, RpcError> {
        let mut vec = Vec::new();

        let signature = transaction.signatures[0];
        let transaction_for_logging = transaction.clone(); // Clone for logging

        // Capture lightweight pre-transaction account states for logging
        let pre_states = crate::logging::capture_account_states(&self.context, &transaction);
        // Clone context for test assertions (get_pre_transaction_account needs full account data)
        let pre_context_snapshot = self.context.clone();

        // Simulate the transaction. Currently, in banks-client/server, only
        // simulations are able to track CPIs. Therefore, simulating is the
        // only way to retrieve the event.
        let simulation_result = self.context.simulate_transaction(transaction.clone());

        // Transaction was successful, execute it.
        self.transaction_counter += 1;
        let transaction_result = self.context.send_transaction(transaction.clone());
        let slot = self.context.get_sysvar::<Clock>().slot;

        // Capture post-transaction account states for logging
        let post_states =
            crate::logging::capture_account_states(&self.context, &transaction_for_logging);

        // Always try enhanced logging for file output (both success and failure)
        if crate::logging::should_use_enhanced_logging(&self.config) {
            crate::logging::log_transaction_enhanced(
                &self.config,
                &transaction_for_logging,
                &transaction_result,
                &signature,
                slot,
                self.transaction_counter,
                Some(&pre_states),
                Some(&post_states),
            );
        }

        // Handle transaction result after logging
        let _res = transaction_result.as_ref().map_err(|x| {
            // Prevent duplicate prints for failing tx.
            if self.config.log_failed_tx {
                crate::logging::log_transaction_enhanced_with_console(
                    &self.config,
                    &transaction_for_logging,
                    &transaction_result,
                    &signature,
                    slot,
                    self.transaction_counter,
                    true, // Enable console output
                    Some(&pre_states),
                    Some(&post_states),
                );
            }
            RpcError::TransactionError(x.err.clone())
        })?;

        // Console logging - if RUST_BACKTRACE is set, print to console too
        if !self.config.no_logs && std::env::var("RUST_BACKTRACE").is_ok() {
            if crate::logging::should_use_enhanced_logging(&self.config) {
                // Print enhanced logs to console
                crate::logging::log_transaction_enhanced_with_console(
                    &self.config,
                    &transaction_for_logging,
                    &transaction_result,
                    &signature,
                    slot,
                    self.transaction_counter,
                    true, // Enable console output
                    Some(&pre_states),
                    Some(&post_states),
                );

                // if self.config.log_light_protocol_events {
                //     if let Some(ref event_data) = event {
                //         println!("event:\n {:?}", event_data);
                //     }
                // }
            } else {
                // Fallback to basic log printing
                self.maybe_print_logs(_res.pretty_logs());
            }
        }

        let simulation_result = simulation_result.unwrap();
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
        if self.config.log_light_protocol_events {
            println!("event:\n {:?}", event);
        }
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

        // Update pre_context only after successful transaction execution
        self.pre_context = Some(pre_context_snapshot);

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

        // Cache the current context before transaction execution
        let pre_context_snapshot = self.context.clone();

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
        self.transaction_counter += 1;
        let _res = self.context.send_transaction(transaction).map_err(|x| {
            if self.config.log_failed_tx {
                println!("{}", x.meta.pretty_logs());
            }
            RpcError::TransactionError(x.err)
        })?;
        self.maybe_print_logs(_res.pretty_logs());

        // Update pre_context only after successful transaction execution
        self.pre_context = Some(pre_context_snapshot);

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
