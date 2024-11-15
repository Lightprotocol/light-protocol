use crate::indexer::{MerkleProof, NewAddressProofWithContext};
use photon_api::{
    apis::{
        configuration::{ApiKey, Configuration},
        default_api::{
            GetMultipleCompressedAccountProofsPostError, GetMultipleNewAddressProofsV2PostError,
        },
        Error as PhotonError,
    },
    models::{GetCompressedAccountPost429Response, GetCompressedAccountsByOwnerPostRequestParams},
};
use solana_sdk::{bs58, pubkey::Pubkey};

pub fn decode_base58(account: &str) -> [u8; 32] {
    let bytes = bs58::decode(account).into_vec().unwrap();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}

#[derive(Debug)]
pub struct PhotonClient {
    config: Configuration,
}

#[derive(Debug, thiserror::Error)]
pub enum PhotonClientError {
    #[error(transparent)]
    GetMultipleCompressedAccountProofsError(
        #[from] PhotonError<GetMultipleCompressedAccountProofsPostError>,
    ),
    #[error(transparent)]
    GetCompressedAccountsByOwnerError(#[from] PhotonError<GetCompressedAccountPost429Response>),
    #[error(transparent)]
    GetMultipleNewAddressProofsError(#[from] PhotonError<GetMultipleNewAddressProofsV2PostError>),
    #[error("Decode error: {0}")]
    DecodeError(String),
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
        hashes: Vec<String>,
    ) -> Result<Vec<MerkleProof>, PhotonClientError> {
        let request = photon_api::models::GetMultipleCompressedAccountProofsPostRequest {
            params: hashes,
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
                            .map(|x| decode_base58(x))
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
    ) -> Result<Vec<String>, PhotonClientError> {
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

        Ok(hashes)
    }

    pub async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: [u8; 32],
        addresses: Vec<[u8; 32]>,
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
            let tree_pubkey = decode_base58(&photon_proof.merkle_tree);
            let low_address_value = decode_base58(&photon_proof.lower_range_address);
            let next_address_value = decode_base58(&photon_proof.higher_range_address);
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
                        .map(|x: &String| decode_base58(x))
                        .collect();
                    proof_vec.truncate(proof_vec.len() - 10); // Remove canopy
                    let mut proof_arr = [[0u8; 32]; 16];
                    proof_arr.copy_from_slice(&proof_vec);
                    proof_arr
                },
                root: decode_base58(&photon_proof.root),
                root_seq: photon_proof.root_seq,
                new_low_element: None,
                new_element: None,
                new_element_next_value: None,
            };
            proofs.push(proof);
        }

        Ok(proofs)
    }
}
