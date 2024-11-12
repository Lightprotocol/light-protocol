use crate::indexer::{Indexer, IndexerError, MerkleProof, NewAddressProofWithContext};
use light_sdk::{
    compressed_account::CompressedAccountWithMerkleContext, event::PublicTransactionEvent,
    proof::ProofRpcResult, token::TokenDataWithMerkleContext,
};
use photon_api::apis::configuration::{ApiKey, Configuration};
use photon_api::models::{
    AddressWithTree, GetCompressedAccountsByOwnerPostRequestParams,
    GetLatestCompressionSignaturesPostRequestParams,
};
use solana_sdk::bs58;
use solana_sdk::pubkey::Pubkey;
use std::fmt::Debug;
use tracing::debug;

use super::RpcRequirements;

/// PhotonIndexer implements the Indexer trait to interact with compressed accounts via the ZK Compression API.
///
/// # Example
/// ```rust, no_run
/// # use solana_sdk::pubkey::Pubkey;
/// # use light_client::{indexer::PhotonIndexer, rpc::SolanaRpcConnection, RpcConnection, Indexer};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
///     let rpc = SolanaRpcConnection::new("https://api.devnet.solana.com".to_string(), None);
///     let indexer = PhotonIndexer::new(
///         "https://devnet.helius-rpc.com".to_string(),
///         Some("api-key".to_string()),
///         rpc
///     );
///     let owner = Pubkey::new_unique();
///     let accounts = indexer.get_rpc_compressed_accounts_by_owner(&owner).await?;
///     Ok(())
/// # }
/// ```
pub struct PhotonIndexer<R>
where
    R: RpcRequirements,
{
    configuration: Configuration,
    #[allow(dead_code)]
    rpc: R,
}

pub fn decode_hash(account: &str) -> [u8; 32] {
    let bytes = bs58::decode(account).into_vec().unwrap();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}

impl<R> PhotonIndexer<R>
where
    R: RpcRequirements,
{
    pub fn new(path: String, api_key: Option<String>, rpc: R) -> Self {
        let configuration = Configuration {
            base_path: path,
            api_key: api_key.map(|key| ApiKey {
                prefix: Some("api-key".to_string()),
                key,
            }),
            ..Default::default()
        };

        PhotonIndexer { configuration, rpc }
    }
}

impl<R> Debug for PhotonIndexer<R>
where
    R: RpcRequirements,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhotonIndexer")
            .field("configuration", &self.configuration)
            .finish()
    }
}
#[allow(clippy::manual_async_fn)]
impl<R: RpcRequirements> Indexer for PhotonIndexer<R> {
    type Rpc = R;

    /// Gets all compressed accounts owned by a public key
    ///
    /// # Arguments
    /// * `owner` - Public key of the account owner
    fn get_compressed_accounts_by_owner(
        &self,
        _owner: &Pubkey,
    ) -> Vec<CompressedAccountWithMerkleContext> {
        unimplemented!()
    }

    /// Gets a single compressed account by its hash
    ///
    /// # Arguments
    /// * `hash` - Base58 encoded hash of the compressed account
    ///
    /// # Returns
    /// Account data with its merkle context if found
    fn get_compressed_account(
        &self,
        _hash: String,
    ) -> impl std::future::Future<Output = Result<CompressedAccountWithMerkleContext, IndexerError>> + Send
    {
        async move { unimplemented!() }
    }

    /// Gets compressed account hashes owned by a public key
    ///
    /// # Arguments
    /// * `owner` - Public key of the account owner
    ///
    /// # Returns
    /// List of base58 encoded account hashes
    fn get_rpc_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> impl std::future::Future<Output = Result<Vec<String>, IndexerError>> + Send {
        let configuration = self.configuration.clone();
        async move {
            let request = photon_api::models::GetCompressedAccountsByOwnerPostRequest {
                params: Box::from(GetCompressedAccountsByOwnerPostRequestParams {
                    cursor: None,
                    data_slice: None,
                    filters: None,
                    limit: None,
                    owner: owner.to_string(),
                }),
                ..Default::default()
            };

            let result = photon_api::apis::default_api::get_compressed_accounts_by_owner_post(
                &configuration,
                request,
            )
            .await?;

            let accs = result
                .result
                .ok_or_else(|| IndexerError::Custom("No result returned".to_string()))?
                .value;

            Ok(accs.items.into_iter().map(|acc| acc.hash).collect())
        }
    }

    /// Gets multiple compressed accounts by their hashes
    ///
    /// # Arguments
    /// * `hashes` - List of base58 encoded account hashes
    ///
    /// # Returns
    /// List of accounts with their merkle contexts
    fn get_multiple_compressed_accounts(
        &self,
        _hashes: Vec<String>,
    ) -> impl std::future::Future<
        Output = Result<Vec<CompressedAccountWithMerkleContext>, IndexerError>,
    > + Send {
        async move { unimplemented!() }
    }

    /// Gets the SOL balance of a compressed account
    ///
    /// # Arguments
    /// * `hash` - Base58 encoded hash of the compressed account
    ///
    /// # Returns
    /// Balance in lamports
    fn get_compressed_account_balance(
        &self,
        _hash: String,
    ) -> impl std::future::Future<Output = Result<u64, IndexerError>> + Send {
        async move { unimplemented!() }
    }

    /// Gets total SOL balance of all compressed accounts owned by a public key
    ///
    /// # Arguments
    /// * `owner` - Public key of the account owner
    ///
    /// # Returns
    /// Total balance in lamports
    fn get_compressed_balance_by_owner(
        &self,
        _owner: &Pubkey,
    ) -> impl std::future::Future<Output = Result<u64, IndexerError>> + Send {
        async move { unimplemented!() }
    }

    /// Gets merkle proof for a single compressed account
    ///
    /// # Arguments
    /// * `hash` - Base58 encoded hash of the compressed account
    fn get_compressed_account_proof(
        &self,
        _hash: String,
    ) -> impl std::future::Future<Output = Result<MerkleProof, IndexerError>> + Send {
        async move { unimplemented!() }
    }

    /// Gets merkle proofs for multiple compressed accounts
    ///
    /// # Arguments
    /// * `hashes` - List of base58 encoded account hashes
    ///
    /// # Returns
    /// List of merkle proofs with leaf indices and tree context
    fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> impl std::future::Future<Output = Result<Vec<MerkleProof>, IndexerError>> + Send {
        let configuration = self.configuration.clone();
        async move {
            debug!("Getting proofs for {:?}", hashes);
            let request = photon_api::models::GetMultipleCompressedAccountProofsPostRequest {
                params: hashes,
                ..Default::default()
            };

            let result =
                photon_api::apis::default_api::get_multiple_compressed_account_proofs_post(
                    &configuration,
                    request,
                )
                .await?;

            match result.result {
                Some(result) => {
                    let proofs = result
                        .value
                        .iter()
                        .map(|x| {
                            let mut proof_result_value = x.proof.clone();
                            proof_result_value.truncate(proof_result_value.len() - 10);
                            let proof: Vec<[u8; 32]> =
                                proof_result_value.iter().map(|x| decode_hash(x)).collect();
                            MerkleProof {
                                hash: x.hash.clone(),
                                leaf_index: x.leaf_index,
                                merkle_tree: x.merkle_tree.clone(),
                                proof,
                                root_seq: x.root_seq,
                            }
                        })
                        .collect();

                    Ok(proofs)
                }
                None => {
                    let error = result.error.unwrap();
                    Err(IndexerError::Custom(error.message.unwrap()))
                }
            }
        }
    }

    /// Gets non-inclusion proofs for new addresses in a merkle tree
    ///
    /// # Arguments
    /// * `merkle_tree_pubkey` - Public key of the merkle tree
    /// * `addresses` - List of 32-byte addresses to prove non-inclusion for
    ///
    /// # Returns
    /// List of non-inclusion proofs with merkle context
    fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
    ) -> impl std::future::Future<Output = Result<Vec<NewAddressProofWithContext>, IndexerError>> + Send
    {
        let configuration = self.configuration.clone();
        async move {
            let params: Vec<AddressWithTree> = addresses
                .iter()
                .map(|x| AddressWithTree {
                    address: bs58::encode(x).into_string(),
                    tree: bs58::encode(&merkle_tree_pubkey).into_string(),
                })
                .collect();

            let request = photon_api::models::GetMultipleNewAddressProofsV2PostRequest {
                params,
                ..Default::default()
            };

            debug!("Request: {:?}", request);

            let result = photon_api::apis::default_api::get_multiple_new_address_proofs_v2_post(
                &configuration,
                request,
            )
            .await?;

            let photon_proofs = result
                .result
                .ok_or_else(|| IndexerError::Custom("No result returned".to_string()))?
                .value;

            let proofs = photon_proofs
                .into_iter()
                .map(|photon_proof| {
                    let tree_pubkey = decode_hash(&photon_proof.merkle_tree);
                    let low_address_value = decode_hash(&photon_proof.lower_range_address);
                    let next_address_value = decode_hash(&photon_proof.higher_range_address);

                    let mut proof_vec: Vec<[u8; 32]> =
                        photon_proof.proof.iter().map(|x| decode_hash(x)).collect();
                    proof_vec.truncate(proof_vec.len() - 10);

                    let mut proof_arr = [[0u8; 32]; 16];
                    proof_arr.copy_from_slice(&proof_vec);

                    NewAddressProofWithContext {
                        merkle_tree: tree_pubkey,
                        low_address_index: photon_proof.low_element_leaf_index as u64,
                        low_address_value,
                        low_address_next_index: photon_proof.next_index as u64,
                        low_address_next_value: next_address_value,
                        low_address_proof: proof_arr,
                        root: decode_hash(&photon_proof.root),
                        root_seq: photon_proof.root_seq,
                        new_low_element: None,
                        new_element: None,
                        new_element_next_value: None,
                    }
                })
                .collect();

            Ok(proofs)
        }
    }

    /// Gets validity proof for compressed accounts and/or new addresses
    ///
    /// # Arguments
    /// * `compressed_accounts` - Optional list of compressed account hashes
    /// * `state_merkle_tree_pubkeys` - Optional list of state merkle tree public keys
    /// * `new_addresses` - Optional list of new addresses
    /// * `address_merkle_tree_pubkeys` - Optional list of address merkle tree public keys
    /// * `rpc` - RPC client for fetching on-chain data
    fn get_validity_proof(
        &mut self,
        _compressed_accounts: Option<&[[u8; 32]]>,
        _state_merkle_tree_pubkeys: Option<&[Pubkey]>,
        _new_addresses: Option<&[[u8; 32]]>,
        _address_merkle_tree_pubkeys: Option<Vec<Pubkey>>,
        _rpc: &mut R,
    ) -> impl std::future::Future<Output = ProofRpcResult> + Send {
        async move { unimplemented!() }
    }

    /// Gets transaction information including compression details
    ///
    /// # Arguments
    /// * `signature` - Base58 encoded transaction signature
    fn get_transaction_with_compression_info(
        &self,
        _signature: String,
    ) -> impl std::future::Future<Output = Result<super::TransactionInfo, IndexerError>> + Send
    {
        async move { unimplemented!() }
    }

    /// Gets latest compression-related transaction signatures
    ///
    /// # Arguments
    /// * `params` - Parameters for filtering and pagination
    fn get_latest_compression_signatures(
        &self,
        _params: GetLatestCompressionSignaturesPostRequestParams,
    ) -> impl std::future::Future<Output = Result<Vec<String>, IndexerError>> + Send {
        async move { unimplemented!() }
    }

    /// Gets latest non-voting transaction signatures
    fn get_latest_non_voting_signatures(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<String>, IndexerError>> + Send {
        async move { unimplemented!() }
    }

    /// Checks if the indexer service is healthy
    ///
    /// # Returns
    /// `true` if the service is healthy, `false` otherwise
    fn get_indexer_health(
        &self,
    ) -> impl std::future::Future<Output = Result<bool, IndexerError>> + Send {
        async move { unimplemented!() }
    }

    /// Gets the current slot number of the indexer
    ///
    /// # Returns
    /// Current slot number
    fn get_indexer_slot(
        &self,
    ) -> impl std::future::Future<Output = Result<u64, IndexerError>> + Send {
        async move { unimplemented!() }
    }

    /// Processes a transaction event and updates compressed account state
    ///
    /// # Arguments
    /// * `event` - Transaction event containing input/output compressed accounts
    ///
    /// # Returns
    /// Tuple of (new compressed accounts, new token accounts) created by this event
    fn add_event_and_compressed_accounts(
        &mut self,
        _event: &PublicTransactionEvent,
    ) -> (
        Vec<CompressedAccountWithMerkleContext>,
        Vec<TokenDataWithMerkleContext>,
    ) {
        unimplemented!()
    }
}
