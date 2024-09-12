use std::str::FromStr;

use light_sdk::{
    address::AddressWithMerkleContext,
    compressed_account::{
        CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext,
    },
    merkle_context::MerkleContext,
    proof::{CompressedProof, MerkleProof, NewAddressProofWithContext, ProofRpcResult},
};
use photon_api::{
    apis::configuration::{ApiKey, Configuration},
    models::{
        AddressWithTree, GetCompressedAccountsByOwnerPostRequestParams,
        GetMultipleCompressedAccountProofsPostRequest, GetMultipleNewAddressProofsV2PostRequest,
        GetValidityProofPostRequest, GetValidityProofPostRequestParams,
    },
};
use solana_sdk::pubkey::Pubkey;

use crate::utils::decode_hash;

use super::{Hashes, Indexer, IndexerError};

#[derive(Debug)]
pub struct PhotonIndexer {
    configuration: Configuration,
}

impl PhotonIndexer {
    pub fn new(base_path: String, api_key: Option<String>) -> Self {
        let configuration = Configuration {
            base_path,
            api_key: api_key.map(|key| ApiKey {
                prefix: Some("api-key".to_string()),
                key,
            }),
            ..Default::default()
        };
        Self { configuration }
    }
}

impl Indexer for PhotonIndexer {
    async fn get_compressed_accounts_by_owner(
        &self,
        owner: &solana_sdk::pubkey::Pubkey,
    ) -> Result<Vec<CompressedAccountWithMerkleContext>, IndexerError> {
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
            &self.configuration,
            request,
        )
        .await
        .unwrap();
        let items = result.result.ok_or(IndexerError::EmptyResult)?.value.items;

        // PANICS: We assume correctness of data returned by Photon.
        let compressed_accounts = items
            .iter()
            .map(|account| CompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::from_str(&account.owner).unwrap(),
                    lamports: account.lamports as u64,
                    address: account
                        .address
                        .as_ref()
                        .map(|address| decode_hash(&address)),
                    data: account.data.as_ref().map(|data| CompressedAccountData {
                        discriminator: (data.discriminator as u64).to_le_bytes(),
                        data: bs58::decode(&data.data).into_vec().unwrap(),
                        data_hash: decode_hash(&data.data_hash),
                    }),
                },
                merkle_context: MerkleContext {
                    merkle_tree_pubkey: Pubkey::from_str(&account.tree).unwrap(),
                    nullifier_queue_pubkey: Pubkey::new_unique(),
                    leaf_index: account.leaf_index as u32,
                    queue_index: None,
                },
            })
            .collect::<Vec<_>>();
        Ok(compressed_accounts)
    }

    async fn get_multiple_compressed_account_proofs<'a>(
        &self,
        hashes: Hashes<'a>,
    ) -> Result<Vec<MerkleProof>, IndexerError> {
        let hashes = match hashes {
            Hashes::Array(hashes) => hashes
                .iter()
                .map(|hash| bs58::encode(hash).into_string())
                .collect::<Vec<_>>(),
            Hashes::String(hashes) => hashes.to_vec(),
        };

        let request = GetMultipleCompressedAccountProofsPostRequest {
            params: hashes,
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_multiple_compressed_account_proofs_post(
            &self.configuration,
            request,
        )
        .await
        .unwrap();
        let items = result.result.ok_or(IndexerError::EmptyResult)?.value;

        let proofs = items
            .iter()
            .map(|proof| MerkleProof {
                hash: proof.hash.clone(),
                leaf_index: proof.leaf_index,
                merkle_tree: proof.merkle_tree.clone(),
                proof: Vec::new(),
                root_seq: proof.root_seq,
            })
            .collect::<Vec<_>>();

        Ok(proofs)
    }

    async fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: &Pubkey,
        addresses: &[[u8; 32]],
    ) -> Result<Vec<light_sdk::proof::NewAddressProofWithContext>, IndexerError> {
        let params: Vec<AddressWithTree> = addresses
            .iter()
            .map(|x| AddressWithTree {
                address: bs58::encode(x).into_string(),
                tree: bs58::encode(&merkle_tree_pubkey).into_string(),
            })
            .collect();
        let request = GetMultipleNewAddressProofsV2PostRequest {
            params,
            ..Default::default()
        };

        let result = photon_api::apis::default_api::get_multiple_new_address_proofs_v2_post(
            &self.configuration,
            request,
        )
        .await
        .unwrap();
        let items = result.result.ok_or(IndexerError::EmptyResult)?.value;

        let proofs = items
            .iter()
            .map(|proof| NewAddressProofWithContext {
                merkle_tree: Pubkey::from_str(&proof.merkle_tree).unwrap(),
                root: decode_hash(&proof.root),
                root_seq: proof.root_seq,
                low_address_index: proof.low_element_leaf_index as u64,
                low_address_value: decode_hash(&proof.lower_range_address),
                low_address_next_index: proof.next_index as u64,
                low_address_next_value: decode_hash(&proof.higher_range_address),
                low_address_proof: proof
                    .proof
                    .iter()
                    .take(
                        // Proof nodes without canopy
                        16,
                    )
                    .map(|proof| decode_hash(proof))
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
                new_low_element: None,
                new_element: None,
                new_element_next_value: None,
            })
            .collect::<Vec<_>>();

        Ok(proofs)
    }

    async fn get_validity_proof(
        &self,
        compressed_accounts: &[CompressedAccountWithMerkleContext],
        new_addresses: &[AddressWithMerkleContext],
    ) -> Result<ProofRpcResult, IndexerError> {
        let hashes = if compressed_accounts.is_empty() {
            None
        } else {
            let mut hashes = Vec::with_capacity(compressed_accounts.len());
            for account in compressed_accounts.iter() {
                let hash = account.hash().map_err(|_| IndexerError::AccountHash)?;
                let hash = bs58::encode(hash).into_string();
                hashes.push(hash);
            }
            Some(hashes)
        };
        let new_addresses_with_trees = if new_addresses.is_empty() {
            None
        } else {
            Some(
                new_addresses
                    .iter()
                    .map(|address| AddressWithTree {
                        address: bs58::encode(address.address).into_string(),
                        tree: bs58::encode(
                            address
                                .address_merkle_context
                                .address_merkle_tree_pubkey
                                .to_bytes(),
                        )
                        .into_string(),
                    })
                    .collect::<Vec<_>>(),
            )
        };

        let request = GetValidityProofPostRequest {
            params: Box::new(GetValidityProofPostRequestParams {
                hashes,
                new_addresses: None,
                new_addresses_with_trees,
            }),
            ..Default::default()
        };

        let result =
            photon_api::apis::default_api::get_validity_proof_post(&self.configuration, request)
                .await
                .unwrap();
        let value = result.result.ok_or(IndexerError::EmptyResult)?.value;

        println!("VALUE: {value:?}");

        let proof = ProofRpcResult {
            // FIXME
            // proof: CompressedProof {
            //     a: value.compressed_proof.a,
            //     b: value.compressed_proof.b,
            //     c: value.compressed_proof.c,
            // },
            proof: CompressedProof {
                a: [0; 32],
                b: [0; 64],
                c: [0; 32],
            },
            root_indices: value
                .root_indices
                .iter()
                .map(|index| *index as u16)
                .collect::<Vec<_>>(),
            // FIXME
            address_root_indices: Vec::new(),
        };

        Ok(proof)
    }
}
