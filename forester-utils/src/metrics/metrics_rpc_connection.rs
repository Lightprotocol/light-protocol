use std::time::Instant;
use std::fmt::Debug;
use async_trait::async_trait;
use borsh::BorshDeserialize;
use light_client::indexer::AddressMerkleTreeBundle;
use light_client::indexer::Indexer;
use light_client::indexer::IndexerError;
use light_client::indexer::LeafIndexInfo;
use light_client::indexer::MerkleProof;
use light_client::indexer::NewAddressProofWithContext;
use light_client::indexer::ProofOfLeaf;
use light_client::rpc::RpcConnection;
use light_client::rpc::RpcError;
use light_sdk::proof::ProofRpcResult;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::account::Account;
use solana_sdk::account::AccountSharedData;
use solana_sdk::clock::Slot;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::epoch_info::EpochInfo;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::hash::Hash;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::Transaction;
use solana_transaction_status::TransactionStatus;
use light_client::transaction_params::TransactionParams;

use crate::metrics::RPC_REQUESTS_TOTAL;
use crate::metrics::RPC_REQUEST_DURATION;
use crate::metrics::RPC_REQUEST_ERRORS;

pub trait MetricsWrapper<T> {
    fn inner(&self) -> &T;
    fn inner_mut(&mut self) -> &mut T;
}

#[derive(Debug)] 
pub struct MetricsRpcConnection<T: RpcConnection> {
    pub(crate) inner: T,
}


impl<T: RpcConnection> MetricsWrapper<T> for MetricsRpcConnection<T> {
    fn inner(&self) -> &T {
        &self.inner
    }

    fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

#[async_trait]
#[async_trait]
impl<T, R> Indexer<R> for MetricsRpcConnection<T>
where
    T: Indexer<R> + RpcConnection,
    R: RpcConnection,
{
    async fn get_queue_elements(
        &self,
        pubkey: [u8; 32],
        batch: u64,
        start_offset: u64,
        end_offset: u64,
    ) -> Result<Vec<[u8; 32]>, IndexerError> {
        let start = Instant::now();
        let result = self.inner.get_queue_elements(pubkey, batch, start_offset, end_offset).await;
        let duration = start.elapsed().as_secs_f64();
        match &result {
            Ok(_) => {
                RPC_REQUESTS_TOTAL.with_label_values(&["update_tree", "success"]).inc();
            }
            Err(e) => {
                RPC_REQUESTS_TOTAL.with_label_values(&["update_tree", "error"]).inc();
                RPC_REQUEST_ERRORS
                    .with_label_values(&["update_tree", &e.to_string()])
                    .inc();
            }
        }
        RPC_REQUEST_DURATION.with_label_values(&["update_tree"]).observe(duration);
        
        result
    }

    fn get_subtrees(&self, merkle_tree_pubkey: [u8; 32]) -> Result<Vec<[u8; 32]>, IndexerError> {
        self.inner.get_subtrees(merkle_tree_pubkey)
    }

    async fn create_proof_for_compressed_accounts(
        &mut self,
        compressed_accounts: Option<Vec<[u8; 32]>>,
        state_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        new_addresses: Option<&[[u8; 32]]>,
        address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        rpc: &mut R,
    ) -> ProofRpcResult {
        self.inner_mut().create_proof_for_compressed_accounts(
            compressed_accounts,
            state_merkle_tree_pubkeys,
            new_addresses,
            address_merkle_tree_pubkeys,
            rpc,
        ).await
    }
  
    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<MerkleProof>, IndexerError> {
        self.inner.get_multiple_compressed_account_proofs(hashes).await
    }

    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<String>, IndexerError> {
        self.inner.get_compressed_accounts_by_owner(owner).await
    }

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<16>>, IndexerError> {
        self.inner.get_multiple_new_address_proofs(merkle_tree_pubkey, addresses).await
    }

    async fn get_multiple_new_address_proofs_h40(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> Result<Vec<NewAddressProofWithContext<40>>, IndexerError> {
        self.inner.get_multiple_new_address_proofs_h40(merkle_tree_pubkey, addresses).await
    }

    fn get_proofs_by_indices(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        indices: &[u64],
    ) -> Vec<ProofOfLeaf> {
        self.inner.get_proofs_by_indices(merkle_tree_pubkey, indices)
    }

    fn get_leaf_indices_tx_hashes(
        &mut self,
        merkle_tree_pubkey: Pubkey,
        zkp_batch_size: usize,
    ) -> Vec<LeafIndexInfo> {
        self.inner.get_leaf_indices_tx_hashes(merkle_tree_pubkey, zkp_batch_size)
    }

    fn get_address_merkle_trees(
        &self,
    ) -> &Vec<AddressMerkleTreeBundle> {
        self.inner.get_address_merkle_trees()
    }
}



impl<T: RpcConnection> MetricsRpcConnection<T> {
    pub fn new<U: ToString>(url: U, commitment_config: Option<CommitmentConfig>) -> Self {
        Self {
            inner: T::new(url, commitment_config),
        }
    }

    async fn measure_request_mut<'a, F, Fut, R>(&'a mut self, method: &str, f: F) -> Result<R, RpcError>
    where
        F: FnOnce(&'a mut T) -> Fut + Send + 'a,
        Fut: std::future::Future<Output = Result<R, RpcError>> + Send + 'a,
        R: Send + 'static,
    {
        let method = method.to_string(); // Clone the method string
        let start = Instant::now();
        
        // Execute RPC call
        let result = f(&mut self.inner).await;
        
        // Record metrics after the borrow is done
        let duration = start.elapsed().as_secs_f64();
        match &result {
            Ok(_) => {
                RPC_REQUESTS_TOTAL.with_label_values(&[&method, "success"]).inc();
            }
            Err(e) => {
                RPC_REQUESTS_TOTAL.with_label_values(&[&method, "error"]).inc();
                RPC_REQUEST_ERRORS
                    .with_label_values(&[&method, &e.to_string()])
                    .inc();
            }
        }
        RPC_REQUEST_DURATION.with_label_values(&[&method]).observe(duration);
        
        result
    }

    async fn measure_request<'a, F, Fut, R>(&'a self, method: &str, f: F) -> Result<R, RpcError>
    where
        F: FnOnce(&'a T) -> Fut + Send + 'a,
        Fut: std::future::Future<Output = Result<R, RpcError>> + Send + 'a,
        R: Send + 'static,
    {
        let method = method.to_string(); // Clone the method string
        let start = Instant::now();
        
        // Execute RPC call
        let result = f(&self.inner).await;
        
        // Record metrics after the borrow is done
        let duration = start.elapsed().as_secs_f64();
        match &result {
            Ok(_) => {
                RPC_REQUESTS_TOTAL.with_label_values(&[&method, "success"]).inc();
            }
            Err(e) => {
                RPC_REQUESTS_TOTAL.with_label_values(&[&method, "error"]).inc();
                RPC_REQUEST_ERRORS
                    .with_label_values(&[&method, &e.to_string()])
                    .inc();
            }
        }
        RPC_REQUEST_DURATION.with_label_values(&[&method]).observe(duration);
        
        result
    }
}


#[async_trait]
impl<T: RpcConnection> RpcConnection for MetricsRpcConnection<T> {
    fn new<U: ToString>(url: U, commitment_config: Option<CommitmentConfig>) -> Self {
        Self::new(url, commitment_config)
    }

    fn get_payer(&self) -> &Keypair {
        self.inner.get_payer()
    }

    fn get_url(&self) -> String {
        self.inner.get_url()
    }

    async fn health(&self) -> Result<(), RpcError> {
        self.measure_request("health", |inner| async move {
            inner.health().await
        })
        .await
    }

    async fn get_block_time(&self, slot: u64) -> Result<i64, RpcError> {
        self.measure_request("get_block_time", |inner| async move {
            inner.get_block_time(slot).await
        })
        .await
    }

    async fn get_epoch_info(&self) -> Result<EpochInfo, RpcError> {
        self.measure_request("get_epoch_info", |inner| async move {
            inner.get_epoch_info().await
        })
        .await
    }

    async fn get_program_accounts(&self, program_id: &Pubkey) -> Result<Vec<(Pubkey, Account)>, RpcError> {
        self.measure_request("get_program_accounts", |inner| async move {
            inner.get_program_accounts(program_id).await
        })
        .await
    }

    async fn process_transaction(&mut self, transaction: Transaction) -> Result<Signature, RpcError> {
        self.measure_request_mut("process_transaction", |inner| async move {
            inner.process_transaction(transaction).await
        })
        .await
    }

    async fn process_transaction_with_context(&mut self, transaction: Transaction) -> Result<(Signature, Slot), RpcError> {
        self.measure_request_mut("process_transaction_with_context", |inner| async move {
            inner.process_transaction_with_context(transaction).await
        })
        .await
    }
    // async fn create_and_send_transaction_with_event<E>(
    //     &mut self,
    //     instructions: &[Instruction],
    //     authority: &Pubkey,
    //     signers: &[&Keypair],
    //     transaction_params: Option<TransactionParams>,
    // ) -> Result<Option<(E, Signature, Slot)>, RpcError>
    // where
    //     E: BorshDeserialize + Send + Debug,
    // {
    //     self.measure_request_mut("create_and_send_transaction_with_event", |inner| async move {
    //         inner.create_and_send_transaction_with_event(instructions, authority, signers, transaction_params).await
    //     })
    //     .await
    // }

    async fn create_and_send_transaction_with_event<E>(
        &mut self,
        instructions: &[Instruction],
        authority: &Pubkey,
        signers: &[&Keypair],
        transaction_params: Option<TransactionParams>,
    ) -> Result<Option<(E, Signature, Slot)>, RpcError>
    where
        E: BorshDeserialize + Send + Debug + 'static,
    {
        // Clone all inputs to avoid lifetime issues
        let instructions = instructions.to_vec();
        let authority = *authority;
        let signers = signers.iter().map(|k| (*k).insecure_clone()).collect::<Vec<_>>();
        let transaction_params = transaction_params.clone();

        // Create struct to hold cloned data to avoid closure lifetime issues
        struct Params {
            instructions: Vec<Instruction>,
            authority: Pubkey,
            signers: Vec<Keypair>,
            transaction_params: Option<TransactionParams>,
        }

        let params = Params {
            instructions,
            authority,
            signers,
            transaction_params,
        };

        self.measure_request_mut(
            "create_and_send_transaction_with_event",
            move |inner| async move {
                let signers_refs: Vec<&Keypair> = params.signers.iter().collect();
                inner.create_and_send_transaction_with_event::<E>(
                    &params.instructions,
                    &params.authority,
                    &signers_refs,
                    params.transaction_params,
                )
                .await
            },
        )
        .await
    }

    async fn confirm_transaction(&self, signature: Signature) -> Result<bool, RpcError> {
        self.measure_request("confirm_transaction", |inner| async move {
            inner.confirm_transaction(signature).await
        })
        .await
    }
    async fn get_account(&mut self, address: Pubkey) -> Result<Option<Account>, RpcError> {
        self.measure_request_mut("get_account", |inner| async move {
            inner.get_account(address).await
        })
        .await
    }

    fn set_account(&mut self, address: &Pubkey, account: &AccountSharedData) {
        self.inner.set_account(address, account);
    }

    async fn get_minimum_balance_for_rent_exemption(&mut self, data_len: usize) -> Result<u64, RpcError> {
        self.measure_request_mut("get_minimum_balance_for_rent_exemption", |inner| async move {
            inner.get_minimum_balance_for_rent_exemption(data_len).await
        })
        .await
    }

    async fn airdrop_lamports(&mut self, to: &Pubkey, lamports: u64) -> Result<Signature, RpcError> {
        self.measure_request_mut("airdrop_lamports", |inner| async move {
            inner.airdrop_lamports(to, lamports).await
        })
        .await
    }

    async fn get_balance(&mut self, pubkey: &Pubkey) -> Result<u64, RpcError> {
        let pubkey = *pubkey;
        self.measure_request_mut("get_balance", |inner| async move {
            inner.get_balance(&pubkey).await
        })
        .await
    }

    async fn get_latest_blockhash(&mut self) -> Result<Hash, RpcError> {
        self.measure_request_mut("get_latest_blockhash", |inner| async move {
            inner.get_latest_blockhash().await
        })
        .await
    }

    async fn get_slot(&mut self) -> Result<u64, RpcError> {
        self.measure_request_mut("get_slot", |inner| async move {
            inner.get_slot().await
        })
        .await
    }

    async fn warp_to_slot(&mut self, slot: Slot) -> Result<(), RpcError> {
        self.measure_request_mut("warp_to_slot", |inner| async move {
            inner.warp_to_slot(slot).await
        })
        .await
    }

    async fn send_transaction(&self, transaction: &Transaction) -> Result<Signature, RpcError> {
        self.measure_request("send_transaction", |inner| async move {
            inner.send_transaction(transaction).await
        })
        .await
    }

    async fn send_transaction_with_config(
        &self,
        transaction: &Transaction,
        config: RpcSendTransactionConfig,
    ) -> Result<Signature, RpcError> {
        self.measure_request("send_transaction_with_config", |inner| async move {
            inner.send_transaction_with_config(transaction, config).await
        })
        .await
    }

    async fn get_transaction_slot(&mut self, signature: &Signature) -> Result<u64, RpcError> {
        self.measure_request_mut("get_transaction_slot", |inner| async move {
            inner.get_transaction_slot(signature).await
        })
        .await
    }

    async fn get_signature_statuses(&self, signatures: &[Signature]) -> Result<Vec<Option<TransactionStatus>>, RpcError> {
        self.measure_request("get_signature_statuses", |inner| async move {
            inner.get_signature_statuses(signatures).await
        })
        .await
    }

    async fn get_block_height(&mut self) -> Result<u64, RpcError> {
        self.measure_request_mut("get_block_height", |inner| async move {
            inner.get_block_height().await
        })
        .await
    }

    async fn get_anchor_account<D>(
        &mut self,
        pubkey: &Pubkey,
    ) -> Result<Option<D>, RpcError>
    where
        D: BorshDeserialize + Send + 'static,
    {
        let pubkey = *pubkey;
        
        self.measure_request_mut(
            "get_anchor_account",
            move |inner| async move { 
                inner.get_anchor_account::<D>(&pubkey).await 
            },
        )
        .await
    }
}