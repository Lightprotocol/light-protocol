/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 * Generated by: https://openapi-generator.tech
 */

use crate::{
    models,
    models::{AccountProofInputs, AddressProofInputs},
};

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedProofWithContextV2 {
    #[serde(rename = "compressedProof", skip_serializing_if = "Option::is_none")]
    pub compressed_proof: Option<Box<models::CompressedProof>>,

    #[serde(rename = "accounts", skip_serializing_if = "Vec::is_empty")]
    pub accounts: Vec<AccountProofInputs>,

    #[serde(rename = "addresses", skip_serializing_if = "Vec::is_empty")]
    pub addresses: Vec<AddressProofInputs>,
}

impl CompressedProofWithContextV2 {
    pub fn new(
        accounts: Vec<AccountProofInputs>,
        addresses: Vec<AddressProofInputs>,
    ) -> CompressedProofWithContextV2 {
        CompressedProofWithContextV2 {
            accounts,
            addresses,
            compressed_proof: None,
        }
    }
}
