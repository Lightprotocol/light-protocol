/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 * Generated by: https://openapi-generator.tech
 */

use crate::models;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct DataSlice {
    #[serde(rename = "length")]
    pub length: u32,
    #[serde(rename = "offset")]
    pub offset: u32,
}

impl DataSlice {
    pub fn new(length: u32, offset: u32) -> DataSlice {
        DataSlice { length, offset }
    }
}
