use std::fmt::Debug;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::clock::Slot;
use anchor_lang::solana_program::hash::Hash;
use anchor_lang::AnchorDeserialize;
use log::info;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_program_test::BanksClientError;
use solana_sdk::account::{Account, AccountSharedData};
use solana_sdk::bs58;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::{Instruction, InstructionError};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::{Keypair, Signature, Signer};
use solana_sdk::transaction::{Transaction, TransactionError};
use solana_transaction_status::option_serializer::OptionSerializer;
use solana_transaction_status::{UiInstruction, UiTransactionEncoding};

use crate::rpc::errors::RpcError;
use crate::rpc::rpc_connection::RpcConnection;
use crate::test_env::PAYER_KEYPAIR;
use crate::transaction_params::TransactionParams;

pub const SERVER_URL: &str = "http://127.0.0.1:8899";

#[allow(dead_code)]
pub struct SolanaRpcConnection {
    pub client: RpcClient,
    payer: Keypair,
}

impl SolanaRpcConnection {
    pub async fn new(commitment_config: Option<CommitmentConfig>) -> Self {
        let payer = Keypair::from_bytes(&PAYER_KEYPAIR).unwrap();
        let commitment_config = commitment_config.unwrap_or(CommitmentConfig::confirmed());
        let client = RpcClient::new_with_commitment(SERVER_URL, commitment_config);
        client
            .request_airdrop(&payer.pubkey(), LAMPORTS_PER_SOL * 1000)
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(16)).await;
        Self { client, payer }
    }

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

impl RpcConnection for SolanaRpcConnection {
    async fn create_and_send_transaction_with_event<T>(
        &mut self,
        instructions: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(T, Signature)>, RpcError>
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
        info!("Transaction signature: {:?}", signature);
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

        // if let Some(transaction_params) = transaction_params {
            // let mut deduped_signers = signers.to_vec();
            // deduped_signers.dedup();
            // let post_balance = self.get_account(*payer).await?.unwrap().lamports;

            // a network_fee is charged if there are input compressed accounts or new addresses
            // let mut network_fee: i64 = 0;
            // if transaction_params.num_input_compressed_accounts != 0 {
            //     network_fee += transaction_params.fee_config.network_fee as i64;
            // }
            // if transaction_params.num_new_addresses != 0 {
            //     network_fee += transaction_params.fee_config.address_network_fee as i64;
            // }

            // let expected_post_balance = pre_balance as i64
            //     - i64::from(transaction_params.num_new_addresses)
            //         * transaction_params.fee_config.address_queue_rollover as i64
            //     - i64::from(transaction_params.num_output_compressed_accounts)
            //         * transaction_params.fee_config.state_merkle_tree_rollover as i64
            //     - transaction_params.compress
            //     - 5000 * deduped_signers.len() as i64
            //     - network_fee;
            // if post_balance as i64 != expected_post_balance {
            //     println!("transaction_params: {:?}", transaction_params);
            //     println!("pre_balance: {}", pre_balance);
            //     println!("post_balance: {}", post_balance);
            //     println!("expected post_balance: {}", expected_post_balance);
            //     println!(
            //         "diff post_balance: {}",
            //         post_balance as i64 - expected_post_balance
            //     );
            //     println!("network_fee: {}", network_fee);
            //     return Err(RpcError::from(BanksClientError::TransactionError(
            //         TransactionError::InstructionError(0, InstructionError::Custom(11111)),
            //     )));
            // }
        // }

        let result = event.map(|event| (event, signature));
        Ok(result)
    }

    async fn create_and_send_transaction(
        &mut self,
        instruction: &[Instruction],
        payer: &Pubkey,
        signers: &[&Keypair],
    ) -> Result<Signature, RpcError> {
        let transaction = Transaction::new_signed_with_payer(
            instruction,
            Some(payer),
            &signers.to_vec(),
            self.get_latest_blockhash().await.unwrap(),
        );
        let signature = transaction.signatures[0];
        self.process_transaction(transaction).await?;
        Ok(signature)
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
        match self.client.send_and_confirm_transaction(&transaction) {
            Ok(signature) => Ok(signature),
            Err(e) => Err(RpcError::ClientError(e)),
        }
    }

    async fn get_slot(&mut self) -> Result<u64, RpcError> {
        todo!()
    }

    async fn airdrop_lamports(
        &mut self,
        to: &Pubkey,
        lamports: u64,
    ) -> Result<Signature, RpcError> {
        self.client
            .request_airdrop(to, lamports)
            .map_err(RpcError::from)
    }

    async fn get_anchor_account<T: AnchorDeserialize>(&mut self, pubkey: &Pubkey) -> T {
        let account = self.get_account(*pubkey).await.unwrap().unwrap();
        T::deserialize(&mut &account.data[8..]).unwrap()
    }

    async fn get_balance(&mut self, pubkey: &Pubkey) -> Result<u64, RpcError> {
        self.client.get_balance(pubkey).map_err(RpcError::from)
    }

    fn warp_to_slot(&mut self, _slot: Slot) -> Result<(), RpcError> {
        todo!()
    }
}
