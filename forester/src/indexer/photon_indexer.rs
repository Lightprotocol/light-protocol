use crate::utils::decode_hash;
use account_compression::initialize_address_merkle_tree::Pubkey;
use light_test_utils::indexer::{
    Indexer, IndexerError, MerkleProof, MerkleProofWithAddressContext,
};
use solana_sdk::bs58;

use photon_api::apis::configuration::Configuration;
use photon_api::models::GetCompressedAccountsByOwnerPostRequestParams;

pub struct PhotonIndexer {
    configuration: Configuration,
}

impl PhotonIndexer {
    pub fn new(path: String) -> Self {
        let configuration = Configuration {
            base_path: path,
            ..Default::default()
        };

        PhotonIndexer { configuration }
    }
}

impl Clone for PhotonIndexer {
    fn clone(&self) -> Self {
        PhotonIndexer {
            configuration: self.configuration.clone(),
        }
    }
}

impl Indexer for PhotonIndexer {
    async fn get_multiple_compressed_account_proofs(
        &self,
        hashes: Vec<String>,
    ) -> Result<Vec<MerkleProof>, IndexerError> {
        let request = photon_api::models::GetMultipleCompressedAccountProofsPostRequest {
            params: hashes,
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_multiple_compressed_account_proofs_post(
            &self.configuration,
            request,
        )
        .await;

        match result {
            Ok(response) => {
                match response.result {
                    Some(result) => {
                        let proofs = result
                            .value
                            .iter()
                            .map(|x| {
                                let mut proof_result_value = x.proof.clone();
                                proof_result_value.truncate(proof_result_value.len() - 10); // Remove canopy
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
                    None => Err(IndexerError::Custom("No result".to_string())),
                }
            }
            Err(e) => Err(IndexerError::Custom(e.to_string())),
        }
    }

    async fn get_rpc_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<String>, IndexerError> {
        let request = photon_api::models::GetCompressedAccountsByOwnerPostRequest {
            params: Box::from(GetCompressedAccountsByOwnerPostRequestParams {
                cursor: None,
                limit: None,
                owner: owner.to_string(),
            }),
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_compressed_accounts_by_owner_post(
            &self.configuration,
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

    // TODO: implement conversion
    async fn get_address_tree_proof(
        &self,
        _merkle_tree_pubkey: [u8; 32],
        address: [u8; 32],
    ) -> Result<MerkleProofWithAddressContext, IndexerError> {
        let request = photon_api::models::GetMultipleNewAddressProofsPostRequest {
            params: vec![bs58::encode(address).into_string()],
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_multiple_new_address_proofs_post(
            &self.configuration,
            request,
        )
        .await
        .unwrap();

        let _proofs = result.result.unwrap().value;

        // let merkle_proof_with_address_context: MerkleProofWithAddressContext = {
        //     merkle_tree: proofs[0].merkle_tree,
        //     proof: proofs[0].proof,
        //     address: address,
        // };

        // Ok(proofs[0])
        todo!()
    }

    fn account_nullified(&mut self, _merkle_tree_pubkey: Pubkey, _account_hash: &str) {
        unimplemented!("only needed for testing")
    }

    fn address_tree_updated(
        &mut self,
        _merkle_tree_pubkey: [u8; 32],
        _context: MerkleProofWithAddressContext,
    ) {
        unimplemented!("only needed for testing")
    }
}
