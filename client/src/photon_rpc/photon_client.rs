use crate::indexer::{MerkleProof, NewAddressProofWithContext};
use photon_api::{
    apis::configuration::{ApiKey, Configuration},
    models::GetCompressedAccountsByOwnerPostRequestParams,
};
use solana_sdk::{bs58, pubkey::Pubkey};

use super::types::AddressWithTree;
use super::{
    models::{AccountBalanceResponse, CompressedAccountsResponse},
    Address, Base58Conversions, CompressedAccountResponse, Hash, PhotonClientError,
    TokenAccountBalanceResponse,
};

#[derive(Debug)]
pub struct PhotonClient {
    config: Configuration,
}

impl PhotonClient {
    pub fn new(url: String, api_key: String) -> Self {
        let mut config = Configuration::new();
        config.base_path = url;
        config.api_key = Some(ApiKey {
            key: api_key,
            prefix: None,
        });
        PhotonClient { config }
    }

    pub async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<Hash>,
    ) -> Result<Vec<MerkleProof>, PhotonClientError> {
        let request = photon_api::models::GetMultipleCompressedAccountProofsPostRequest {
            params: hashes.iter().map(|h| h.to_base58()).collect(),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_multiple_compressed_account_proofs_post(
            &self.config,
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
                        let proof = proof_result_value
                            .iter()
                            .map(|x| Hash::from_base58(x).unwrap())
                            .collect();
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
            None => Err(PhotonClientError::DecodeError("Missing result".to_string())),
        }
    }

    pub async fn get_rpc_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<Hash>, PhotonClientError> {
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
            &self.config,
            request,
        )
        .await
        .unwrap();

        let accs = result.result.unwrap().value;
        let mut hashes = Vec::new();
        for acc in accs.items {
            hashes.push(acc.hash);
        }

        Ok(hashes
            .iter()
            .map(|x| Hash::from_base58(x).unwrap())
            .collect())
    }

    pub async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: Pubkey,
        addresses: Vec<Address>,
    ) -> Result<Vec<NewAddressProofWithContext>, PhotonClientError> {
        let params: Vec<photon_api::models::AddressWithTree> = addresses
            .iter()
            .map(|x| photon_api::models::AddressWithTree {
                address: bs58::encode(x).into_string(),
                tree: bs58::encode(&merkle_tree_pubkey).into_string(),
            })
            .collect();

        let request = photon_api::models::GetMultipleNewAddressProofsV2PostRequest {
            params,
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_multiple_new_address_proofs_v2_post(
            &self.config,
            request,
        )
        .await;

        if result.is_err() {
            return Err(PhotonClientError::GetMultipleNewAddressProofsError(
                result.err().unwrap(),
            ));
        }

        let photon_proofs = result.unwrap().result.unwrap().value;
        let mut proofs: Vec<NewAddressProofWithContext> = Vec::new();
        for photon_proof in photon_proofs {
            let tree_pubkey = Hash::from_base58(&photon_proof.merkle_tree).unwrap();
            let low_address_value = Hash::from_base58(&photon_proof.lower_range_address).unwrap();
            let next_address_value = Hash::from_base58(&photon_proof.higher_range_address).unwrap();
            let proof = NewAddressProofWithContext {
                merkle_tree: tree_pubkey,
                low_address_index: photon_proof.low_element_leaf_index as u64,
                low_address_value,
                low_address_next_index: photon_proof.next_index as u64,
                low_address_next_value: next_address_value,
                low_address_proof: {
                    let mut proof_vec: Vec<[u8; 32]> = photon_proof
                        .proof
                        .iter()
                        .map(|x: &String| Hash::from_base58(x).unwrap())
                        .collect();
                    proof_vec.truncate(proof_vec.len() - 10); // Remove canopy
                    let mut proof_arr = [[0u8; 32]; 16];
                    proof_arr.copy_from_slice(&proof_vec);
                    proof_arr
                },
                root: Hash::from_base58(&photon_proof.root).unwrap(),
                root_seq: photon_proof.root_seq,
                new_low_element: None,
                new_element: None,
                new_element_next_value: None,
            };
            proofs.push(proof);
        }

        Ok(proofs)
    }

    pub async fn get_validity_proof(
        &self,
        hashes: Vec<Hash>,
        new_addresses_with_trees: Vec<AddressWithTree>,
    ) -> Result<photon_api::models::GetValidityProofPost200ResponseResult, PhotonClientError> {
        let request = photon_api::models::GetValidityProofPostRequest {
            params: Box::new(photon_api::models::GetValidityProofPostRequestParams {
                hashes: Some(hashes.iter().map(|x| x.to_base58()).collect()),
                new_addresses: None,
                new_addresses_with_trees: Some(
                    new_addresses_with_trees
                        .iter()
                        .map(|x| photon_api::models::AddressWithTree {
                            address: x.address.to_base58(),
                            tree: x.tree.to_string(),
                        })
                        .collect(),
                ),
            }),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_validity_proof_post(&self.config, request)
            .await
            .map_err(|e| PhotonClientError::DecodeError(e.to_string()))?;

        match result.result {
            Some(result) => Ok(*result),
            None => Err(PhotonClientError::DecodeError("Missing result".to_string())),
        }
    }

    pub async fn get_compressed_account(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<CompressedAccountResponse, PhotonClientError> {
        let params = self.build_account_params(address, hash)?;
        let request = photon_api::models::GetCompressedAccountPostRequest {
            params: Box::new(params),
            ..Default::default()
        };

        let result =
            photon_api::apis::default_api::get_compressed_account_post(&self.config, request)
                .await
                .map_err(|e| PhotonClientError::DecodeError(e.to_string()))?;

        Self::handle_result(result.result).map(|r| CompressedAccountResponse::from(*r))
    }

    pub async fn get_compressed_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<
        photon_api::models::GetCompressedTokenAccountsByDelegatePost200ResponseResult,
        PhotonClientError,
    > {
        let request = photon_api::models::GetCompressedTokenAccountsByOwnerPostRequest {
            params: Box::new(
                photon_api::models::GetCompressedTokenAccountsByOwnerPostRequestParams {
                    owner: owner.to_string(),
                    mint: mint.map(|x| Some(x.to_string())),
                    cursor: None,
                    limit: None,
                },
            ),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_compressed_token_accounts_by_owner_post(
            &self.config,
            request,
        )
        .await
        .map_err(|e| PhotonClientError::DecodeError(e.to_string()))?;

        Self::handle_result(result.result).map(|r| *r)
    }

    pub async fn get_compressed_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<AccountBalanceResponse, PhotonClientError> {
        let params = self.build_account_params(address, hash)?;
        let request = photon_api::models::GetCompressedAccountBalancePostRequest {
            params: Box::new(params),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_compressed_account_balance_post(
            &self.config,
            request,
        )
        .await
        .map_err(|e| PhotonClientError::DecodeError(e.to_string()))?;

        Self::handle_result(result.result).map(|r| AccountBalanceResponse::from(*r))
    }

    pub async fn get_compressed_token_account_balance(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<TokenAccountBalanceResponse, PhotonClientError> {
        let request = photon_api::models::GetCompressedTokenAccountBalancePostRequest {
            params: Box::new(photon_api::models::GetCompressedAccountPostRequestParams {
                address: address.map(|x| Some(x.to_base58())),
                hash: hash.map(|x| Some(x.to_base58())),
            }),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_compressed_token_account_balance_post(
            &self.config,
            request,
        )
        .await
        .map_err(|e| PhotonClientError::DecodeError(e.to_string()))?;

        Self::handle_result(result.result).map(|r| TokenAccountBalanceResponse::from(*r))
    }

    pub async fn get_compressed_token_balances_by_owner(
        &self,
        owner: &Pubkey,
        mint: Option<Pubkey>,
    ) -> Result<
        photon_api::models::GetCompressedTokenBalancesByOwnerPost200ResponseResult,
        PhotonClientError,
    > {
        let request = photon_api::models::GetCompressedTokenBalancesByOwnerPostRequest {
            params: Box::new(
                photon_api::models::GetCompressedTokenAccountsByOwnerPostRequestParams {
                    owner: owner.to_string(),
                    mint: mint.map(|x| Some(x.to_string())),
                    cursor: None,
                    limit: None,
                },
            ),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_compressed_token_balances_by_owner_post(
            &self.config,
            request,
        )
        .await
        .map_err(|e| PhotonClientError::DecodeError(e.to_string()))?;

        Self::handle_result(result.result).map(|r| *r)
    }

    pub async fn get_compression_signatures_for_account(
        &self,
        hash: Hash,
    ) -> Result<
        photon_api::models::GetCompressionSignaturesForAccountPost200ResponseResult,
        PhotonClientError,
    > {
        let request = photon_api::models::GetCompressionSignaturesForAccountPostRequest {
            params: Box::new(
                photon_api::models::GetCompressedAccountProofPostRequestParams {
                    hash: hash.to_base58(),
                },
            ),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_compression_signatures_for_account_post(
            &self.config,
            request,
        )
        .await
        .map_err(|e| PhotonClientError::DecodeError(e.to_string()))?;

        Self::handle_result(result.result).map(|r| *r)
    }

    pub async fn get_multiple_compressed_accounts(
        &self,
        addresses: Vec<Address>,
        hashes: Vec<Hash>,
    ) -> Result<CompressedAccountsResponse, PhotonClientError> {
        let request = photon_api::models::GetMultipleCompressedAccountsPostRequest {
            params: Box::new(
                photon_api::models::GetMultipleCompressedAccountsPostRequestParams {
                    addresses: Some(addresses.iter().map(|x| Some(x.to_base58())).collect()),
                    hashes: Some(hashes.iter().map(|x| Some(x.to_base58())).collect()),
                },
            ),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_multiple_compressed_accounts_post(
            &self.config,
            request,
        )
        .await
        .map_err(|e| PhotonClientError::DecodeError(e.to_string()))?;

        Self::handle_result(result.result).map(|r| CompressedAccountsResponse::from(*r))
    }

    fn handle_result<T>(result: Option<T>) -> Result<T, PhotonClientError> {
        match result {
            Some(result) => Ok(result),
            None => Err(PhotonClientError::DecodeError("Missing result".to_string())),
        }
    }

    fn build_account_params(
        &self,
        address: Option<Address>,
        hash: Option<Hash>,
    ) -> Result<photon_api::models::GetCompressedAccountPostRequestParams, PhotonClientError> {
        if address.is_none() && hash.is_none() {
            return Err(PhotonClientError::DecodeError(
                "Either address or hash must be provided".to_string(),
            ));
        }

        if address.is_some() && hash.is_some() {
            return Err(PhotonClientError::DecodeError(
                "Only one of address or hash must be provided".to_string(),
            ));
        }

        Ok(photon_api::models::GetCompressedAccountPostRequestParams {
            address: address.map(|x| Some(x.to_base58())),
            hash: hash.map(|x| Some(x.to_base58())),
        })
    }
}
