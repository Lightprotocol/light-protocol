use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PhotonProofJson {
    pub context: Context,
    pub value: Value,
}

#[derive(Serialize, Deserialize)]
pub struct Value {
    pub hash: String,
    #[serde(rename = "leafIndex")]
    pub leaf_index: i64,
    #[serde(rename = "merkleTree")]
    pub merkle_tree: String,
    pub proof: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Context {
    pub slot: i64,
}