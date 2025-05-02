use std::fmt::{Debug, Formatter};

use async_trait::async_trait;
use borsh::BorshDeserialize;
#[cfg(feature = "devenv")]
use light_client::fee::{assert_transaction_params, TransactionParams};
use light_client::{
    indexer::{AddressWithTree, Indexer, ProofRpcResult, ProofRpcResultV2},
    rpc::{merkle_tree::MerkleTreeExt, RpcConnection, RpcError, SolanaRpcConnection},
};
use light_compressed_account::indexer_event::{
    event::{BatchPublicTransactionEvent, PublicTransactionEvent},
    parse::event_from_light_transaction,
};
use light_prover_client::gnark::helpers::{ProverConfig, ProverMode};
use solana_banks_client::BanksClientError;
use solana_program_test::ProgramTestContext;
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_sdk::{
    account::{Account, AccountSharedData},
    clock::Slot,
    commitment_config::CommitmentConfig,
    hash::Hash,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    system_instruction,
    transaction::{Transaction, TransactionError},
};
use solana_transaction_status::TransactionStatus;

use crate::{
    accounts::env_accounts::EnvAccounts,
    indexer::{TestIndexer, TestIndexerExtensions},
};

pub struct ProgramTestRpcConnection {
    pub context: ProgramTestContext,
    pub indexer: Option<TestIndexer>,
    pub env_accounts: EnvAccounts,
}

pub trait TestRpcConnection {
    fn set_account(&mut self, address: &Pubkey, account: &AccountSharedData);
    fn warp_to_slot(
        &mut self,
        slot: Slot,
    ) -> impl std::future::Future<Output = Result<(), RpcError>> + Send;
}

impl TestRpcConnection for SolanaRpcConnection {
    fn set_account(&mut self, _address: &Pubkey, _account: &AccountSharedData) {
        unimplemented!()
    }

    async fn warp_to_slot(&mut self, _slot: Slot) -> Result<(), RpcError> {
        unimplemented!()
    }
}

impl ProgramTestRpcConnection {
    pub fn new(context: ProgramTestContext) -> Self {
        Self {
            context,
            indexer: None,
            env_accounts: EnvAccounts::get_local_test_validator_accounts(),
        }
    }

    pub fn indexer(&self) -> Result<&TestIndexer, RpcError> {
        self.indexer
            .as_ref()
            .ok_or(RpcError::CustomError("Indexer not Initialized".to_string()))
    }

    pub fn indexer_mut(&mut self) -> Result<&mut TestIndexer, RpcError> {
        self.indexer
            .as_mut()
            .ok_or(RpcError::CustomError("Indexer not Initialized".to_string()))
    }

    pub fn env_accounts(&self) -> &EnvAccounts {
        &self.env_accounts
    }

    pub async fn add_indexer(
        &mut self,
        env_accounts: &EnvAccounts,
        with_prover: bool,
        batch_size: Option<usize>,
    ) -> Result<(), RpcError> {
        let prover_config = if with_prover {
            Some(ProverConfig {
                circuits: vec![],
                run_mode: Some(ProverMode::Rpc),
            })
        } else {
            None
        };
        let indexer = TestIndexer::init_from_env(
            &self.context.payer,
            env_accounts,
            prover_config,
            batch_size.unwrap_or_default(),
        )
        .await;
        self.indexer = Some(indexer);
        Ok(())
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
}

impl TestRpcConnection for ProgramTestRpcConnection {
    fn set_account(&mut self, address: &Pubkey, account: &AccountSharedData) {
        self.context.set_account(address, account);
    }

    async fn warp_to_slot(&mut self, slot: Slot) -> Result<(), RpcError> {
        self.context
            .warp_to_slot(slot)
            .map_err(|_| RpcError::InvalidWarpSlot)
    }
}

impl Debug for ProgramTestRpcConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProgramTestRpcConnection")
    }
}

#[async_trait]
impl RpcConnection for ProgramTestRpcConnection {
    fn new<U: ToString>(
        _url: U,
        _commitment_config: Option<CommitmentConfig>,
        _skip_indexer: bool,
    ) -> Self
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn get_payer(&self) -> &Keypair {
        &self.context.payer
    }

    fn get_url(&self) -> String {
        "get_url doesn't make sense for ProgramTestRpcConnection".to_string()
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

    async fn get_account(&mut self, address: Pubkey) -> Result<Option<Account>, RpcError> {
        self.context
            .banks_client
            .get_account(address)
            .await
            .map_err(RpcError::from)
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

    async fn get_latest_blockhash(&mut self) -> Result<Hash, RpcError> {
        Ok(self
            .context
            .get_new_latest_blockhash()
            .await
            .map_err(|e| RpcError::from(BanksClientError::from(e)))?)
    }

    async fn get_slot(&mut self) -> Result<u64, RpcError> {
        self.context
            .banks_client
            .get_root_slot()
            .await
            .map_err(RpcError::from)
    }

    async fn get_transaction_slot(&mut self, signature: &Signature) -> Result<u64, RpcError> {
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
            "get_signature_statuses is unimplemented for ProgramTestRpcConnection".to_string(),
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

    #[cfg(not(feature = "devenv"))]
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

    #[cfg(not(feature = "devenv"))]
    async fn create_and_send_transaction_with_batched_event(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Option<(Vec<BatchPublicTransactionEvent>, Signature, Slot)>, RpcError> {
        self._create_and_send_transaction_with_batched_event(instructions, payer, signers)
            .await
    }

    #[cfg(not(feature = "devenv"))]
    async fn create_and_send_transaction_with_public_event(
        &mut self,
        instruction: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Option<(PublicTransactionEvent, Signature, Slot)>, RpcError> {
        let res = self
            .create_and_send_transaction_with_batched_event(instruction, payer, signers)
            .await?;
        let event = res.map(|e| (e.0[0].event.clone(), e.1, e.2));

        Ok(event)
    }

    #[cfg(feature = "devenv")]
    async fn create_and_send_transaction_with_public_event(
        &mut self,
        instruction: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(PublicTransactionEvent, Signature, Slot)>, RpcError> {
        let res = self
            .create_and_send_transaction_with_batched_event(
                instruction,
                payer,
                signers,
                transaction_params,
            )
            .await?;
        let event = res.map(|e| (e.0[0].event.clone(), e.1, e.2));

        Ok(event)
    }

    #[cfg(feature = "devenv")]
    async fn create_and_send_transaction_with_batched_event(
        &mut self,
        instruction: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(Vec<BatchPublicTransactionEvent>, Signature, Slot)>, RpcError> {
        let pre_balance = self
            .context
            .banks_client
            .get_account(*payer)
            .await?
            .unwrap()
            .lamports;
        let event = self
            ._create_and_send_transaction_with_batched_event(instruction, payer, signers)
            .await?;

        assert_transaction_params(self, payer, signers, pre_balance, transaction_params).await?;

        Ok(event)
    }

    #[cfg(feature = "devenv")]
    async fn create_and_send_transaction_with_event<T>(
        &mut self,
        instruction: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(T, Signature, Slot)>, RpcError>
    where
        T: BorshDeserialize + Send + Debug,
    {
        let pre_balance = self
            .context
            .banks_client
            .get_account(*payer)
            .await?
            .unwrap()
            .lamports;

        let result = self
            ._create_and_send_transaction_with_event::<T>(instruction, payer, signers)
            .await?;

        assert_transaction_params(self, payer, signers, pre_balance, transaction_params).await?;

        Ok(result)
    }

    fn indexer(&self) -> Result<&impl Indexer, RpcError> {
        self.indexer.as_ref().ok_or(RpcError::IndexerNotInitialized)
    }

    /// TODO:
    /// - issue: forester tests don't work because we have a mix of solana rpc and test indexer
    /// - Solution 1: reenable proving from test indexer with testindexer traits.
    /// - Solution 2: create SolanaRpcConnection that works with testindexer under the hood
    ///     -> this still needs test indexer proof generation but could be cleaner
    ///    -> difficult because I would require test indexer traits to make proof gen work
    /// - Solution 3: SolanaRpcTestIndexer as a separate implementation
    /// - Solution 4: enable get validity proof in test indexer by extracting the rpc call
    ///     and replacing it with next_index % 2400 (root history len) (I need v2 and v1 root history lens as parameters)
    async fn get_validity_proof(
        &mut self,
        hashes: Vec<[u8; 32]>,
        new_addresses_with_trees: Vec<AddressWithTree>,
    ) -> Result<ProofRpcResult, RpcError> {
        Ok(self
            .indexer()?
            .get_validity_proof(hashes, new_addresses_with_trees)
            .await?)
        // let mut state_merkle_tree_pubkeys = Vec::new();

        // for hash in hashes.iter() {
        //     state_merkle_tree_pubkeys.push(Pubkey::from_str_const(
        //         self.indexer()?
        //             .get_compressed_account(None, Some(*hash))
        //             .await?
        //             .tree
        //             .as_str(),
        //     ));
        // }
        // println!(
        //     "get_validity_proof state_merkle_tree_pubkeys {:?}",
        //     state_merkle_tree_pubkeys
        // );
        // let state_merkle_tree_pubkeys = if state_merkle_tree_pubkeys.is_empty() {
        //     None
        // } else {
        //     Some(state_merkle_tree_pubkeys)
        // };
        // let hashes = if hashes.is_empty() {
        //     None
        // } else {
        //     Some(hashes)
        // };
        // let new_addresses = if new_addresses_with_trees.is_empty() {
        //     None
        // } else {
        //     Some(
        //         new_addresses_with_trees
        //             .iter()
        //             .map(|x| x.address)
        //             .collect::<Vec<[u8; 32]>>(),
        //     )
        // };
        // let address_merkle_tree_pubkeys = if new_addresses_with_trees.is_empty() {
        //     None
        // } else {
        //     Some(
        //         new_addresses_with_trees
        //             .iter()
        //             .map(|x| x.tree)
        //             .collect::<Vec<Pubkey>>(),
        //     )
        // };

        // {
        //     let compressed_accounts = hashes;
        //     if compressed_accounts.is_some()
        //         && ![1usize, 2usize, 3usize, 4usize, 8usize]
        //             .contains(&compressed_accounts.as_ref().unwrap().len())
        //     {
        //         return Err(RpcError::CustomError(format!(
        //             "compressed_accounts must be of length 1, 2, 3, 4 or 8 != {}",
        //             compressed_accounts.unwrap().len()
        //         )));
        //     }
        //     if new_addresses.is_some()
        //         && ![1usize, 2usize, 3usize, 4usize, 8usize]
        //             .contains(&new_addresses.as_ref().unwrap().len())
        //     {
        //         return Err(RpcError::CustomError(format!(
        //             "new_addresses must be of length 1, 2, 3, 4 or 8 != {}",
        //             new_addresses.unwrap().len()
        //         )));
        //     }
        //     let client = Client::new();
        //     let (root_indices, address_root_indices, json_payload) =
        //         match (compressed_accounts, new_addresses) {
        //             (Some(accounts), None) => {
        //                 let (payload, payload_legacy, indices) = self
        //                     .process_inclusion_proofs(
        //                         &state_merkle_tree_pubkeys.unwrap(),
        //                         &accounts,
        //                     )
        //                     .await?;
        //                 if let Some(payload) = payload {
        //                     (indices, Vec::new(), payload.to_string())
        //                 } else {
        //                     (indices, Vec::new(), payload_legacy.unwrap().to_string())
        //                 }
        //             }
        //             (None, Some(addresses)) => {
        //                 let (payload, payload_legacy, indices) = self
        //                     .process_non_inclusion_proofs(
        //                         address_merkle_tree_pubkeys.unwrap().as_slice(),
        //                         addresses,
        //                     )
        //                     .await?;
        //                 let payload_string = if let Some(payload) = payload {
        //                     payload.to_string()
        //                 } else {
        //                     payload_legacy.unwrap().to_string()
        //                 };
        //                 (Vec::<u16>::new(), indices, payload_string)
        //             }
        //             (Some(accounts), Some(addresses)) => {
        //                 let (inclusion_payload, inclusion_payload_legacy, inclusion_indices) = self
        //                     .process_inclusion_proofs(
        //                         &state_merkle_tree_pubkeys.unwrap(),
        //                         &accounts,
        //                     )
        //                     .await?;

        //                 let (
        //                     non_inclusion_payload,
        //                     non_inclusion_payload_legacy,
        //                     non_inclusion_indices,
        //                 ) = self
        //                     .process_non_inclusion_proofs(
        //                         address_merkle_tree_pubkeys.unwrap().as_slice(),
        //                         addresses,
        //                     )
        //                     .await?;
        //                 let json_payload = if let Some(non_inclusion_payload) =
        //                     non_inclusion_payload
        //                 {
        //                     let public_input_hash = BigInt::from_bytes_be(
        //                         num_bigint::Sign::Plus,
        //                         &create_hash_chain_from_slice(&[
        //                             bigint_to_u8_32(
        //                                 &string_to_big_int(
        //                                     &inclusion_payload.as_ref().unwrap().public_input_hash,
        //                                 )
        //                                 .unwrap(),
        //                             )
        //                             .unwrap(),
        //                             bigint_to_u8_32(
        //                                 &string_to_big_int(
        //                                     &non_inclusion_payload.public_input_hash,
        //                                 )
        //                                 .unwrap(),
        //                             )
        //                             .unwrap(),
        //                         ])
        //                         .unwrap(),
        //                     );

        //                     CombinedJsonStruct {
        //                         circuit_type: ProofType::Combined.to_string(),
        //                         state_tree_height: DEFAULT_BATCH_STATE_TREE_HEIGHT,
        //                         address_tree_height: DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
        //                         public_input_hash: big_int_to_string(&public_input_hash),
        //                         inclusion: inclusion_payload.unwrap().inputs,
        //                         non_inclusion: non_inclusion_payload.inputs,
        //                     }
        //                     .to_string()
        //                 } else if let Some(non_inclusion_payload) = non_inclusion_payload_legacy {
        //                     CombinedJsonStructLegacy {
        //                         circuit_type: ProofType::Combined.to_string(),
        //                         state_tree_height: 26,
        //                         address_tree_height: 26,
        //                         inclusion: inclusion_payload_legacy.unwrap().inputs,
        //                         non_inclusion: non_inclusion_payload.inputs,
        //                     }
        //                     .to_string()
        //                 } else {
        //                     panic!("Unsupported tree height")
        //                 };
        //                 (inclusion_indices, non_inclusion_indices, json_payload)
        //             }
        //             _ => {
        //                 panic!(
        //                     "At least one of compressed_accounts or new_addresses must be provided"
        //                 )
        //             }
        //         };

        //     let mut retries = 1000;
        //     while retries > 0 {
        //         let response_result = client
        //             .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
        //             .header("Content-Type", "text/plain; charset=utf-8")
        //             .body(json_payload.clone())
        //             .send()
        //             .await;
        //         if let Ok(response_result) = response_result {
        //             if response_result.status().is_success() {
        //                 let body = response_result.text().await.unwrap();
        //                 let proof_json = deserialize_gnark_proof_json(&body).unwrap();
        //                 let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
        //                 let (proof_a, proof_b, proof_c) =
        //                     compress_proof(&proof_a, &proof_b, &proof_c);
        //                 return Ok(ProofRpcResult {
        //                     root_indices,
        //                     address_root_indices: address_root_indices.clone(),
        //                     proof: CompressedProof {
        //                         a: proof_a,
        //                         b: proof_b,
        //                         c: proof_c,
        //                     },
        //                 });
        //             }
        //         } else {
        //             println!("Error: {:#?}", response_result);
        //             tokio::time::sleep(Duration::from_secs(5)).await;
        //             retries -= 1;
        //         }
        //     }
        //     return Err(RpcError::CustomError(
        //         "Failed to get proof from server".to_string(),
        //     ));
        // }
    }

    async fn get_validity_proof_v2(
        &mut self,
        hashes: Vec<[u8; 32]>,
        new_addresses_with_trees: Vec<AddressWithTree>,
    ) -> Result<ProofRpcResultV2, RpcError> {
        Ok(self
            .indexer()?
            .get_validity_proof_v2(hashes, new_addresses_with_trees)
            .await?)
        // #[cfg(not(feature = "v2"))]
        // unimplemented!();
        // #[cfg(feature = "v2")]
        // {
        //     use light_batched_merkle_tree::queue::{BatchedQueueAccount, BatchedQueueMetadata};
        //     use light_client::indexer::ProofRpcResultV2;

        //     let hashes = _hashes;
        //     let new_addresses_with_trees = _new_addresses_with_trees;

        //     let mut state_merkle_tree_pubkeys = Vec::new();
        //     println!("get_validity_proof_v2 ");

        //     for hash in hashes.iter() {
        //         println!("hash {:?}", hash);
        //         let account = self
        //             .indexer()?
        //             .get_compressed_account(None, Some(*hash))
        //             .await?;
        //         println!("account {:?}", account);

        //         state_merkle_tree_pubkeys.push(Pubkey::from_str_const(account.tree.as_str()));
        //     }
        //     println!("state_merkle_tree_pubkeys {:?}", state_merkle_tree_pubkeys);

        //     let mut indices_to_remove = Vec::new();
        //     // for all accounts in batched trees, check whether values are in tree or queue
        //     let compressed_accounts = if !hashes.is_empty() && !state_merkle_tree_pubkeys.is_empty()
        //     {
        //         let zipped_accounts = hashes.iter().zip(state_merkle_tree_pubkeys.iter());

        //         for (i, (compressed_account, state_merkle_tree_pubkey)) in
        //             zipped_accounts.enumerate()
        //         {
        //             let accounts = self
        //                 .indexer
        //                 .as_ref()
        //                 .ok_or(RpcError::IndexerNotInitialized)?
        //                 .state_merkle_trees
        //                 .iter()
        //                 .find(|x| {
        //                     x.accounts.merkle_tree == *state_merkle_tree_pubkey && x.version == 2
        //                 });

        //             if let Some(accounts) = accounts {
        //                 let leaf_index = accounts.merkle_tree.get_leaf_index(compressed_account);
        //                 if leaf_index.is_none() {
        //                     let output_queue_pubkey = accounts.accounts.nullifier_queue;
        //                     let mut queue =
        //                         forester_utils::account_zero_copy::AccountZeroCopy::<
        //                             BatchedQueueMetadata,
        //                         >::new(self, output_queue_pubkey)
        //                         .await;
        //                     let queue_zero_copy = BatchedQueueAccount::output_from_bytes(
        //                         queue.account.data.as_mut_slice(),
        //                     )
        //                     .unwrap();
        //                     for value_array in queue_zero_copy.value_vecs.iter() {
        //                         let index =
        //                             value_array.iter().position(|x| *x == *compressed_account);
        //                         if index.is_some() {
        //                             indices_to_remove.push(i);
        //                         }
        //                     }
        //                 }
        //             }
        //         }

        //         let compress_accounts = hashes
        //             .iter()
        //             .enumerate()
        //             .filter(|(i, _)| !indices_to_remove.contains(i))
        //             .map(|(_, x)| *x)
        //             .collect::<Vec<[u8; 32]>>();

        //         if compress_accounts.is_empty() {
        //             None
        //         } else {
        //             Some(compress_accounts)
        //         }
        //     } else {
        //         None
        //     };
        //     let rpc_result: Option<ProofRpcResult> = if (compressed_accounts.is_some()
        //         && !compressed_accounts.as_ref().unwrap().is_empty())
        //         || !new_addresses_with_trees.is_empty()
        //     {
        //         Some(
        //             self.get_validity_proof(
        //                 compressed_accounts.unwrap_or_default(),
        //                 new_addresses_with_trees,
        //             )
        //             .await?,
        //         )
        //     } else {
        //         None
        //     };
        //     let address_root_indices = if let Some(rpc_result) = rpc_result.as_ref() {
        //         rpc_result.address_root_indices.clone()
        //     } else {
        //         Vec::new()
        //     };
        //     let root_indices = {
        //         let mut root_indices = if let Some(rpc_result) = rpc_result.as_ref() {
        //             rpc_result
        //                 .root_indices
        //                 .iter()
        //                 .map(|x| Some(*x))
        //                 .collect::<Vec<_>>()
        //         } else {
        //             Vec::new()
        //         };
        //         for index in indices_to_remove {
        //             root_indices.insert(index, None);
        //         }
        //         root_indices
        //     };
        //     Ok(ProofRpcResultV2 {
        //         proof: rpc_result.map(|x| x.proof),
        //         root_indices,
        //         address_root_indices,
        //     })
        // }
    }
}

impl MerkleTreeExt for ProgramTestRpcConnection {}
