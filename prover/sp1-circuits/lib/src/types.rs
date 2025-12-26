//! Input/output types for SP1 batch circuits.
//!
//! These types match the JSON format used by the Go prover server.

use serde::{Deserialize, Serialize};

use crate::poseidon::Hash;

/// Batch Append circuit inputs.
///
/// Matches the JSON format from `BatchAppendInputsJSON` in Go.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchAppendInputs {
    /// Circuit type identifier (should be "append")
    pub circuit_type: String,

    /// Height of the state tree (e.g., 32)
    #[serde(default)]
    pub state_tree_height: u32,

    /// Public input hash (hex string with 0x prefix)
    pub public_input_hash: String,

    /// Old Merkle root before batch append (hex string)
    pub old_root: String,

    /// New Merkle root after batch append (hex string)
    pub new_root: String,

    /// Hash chain of all new leaves (hex string)
    pub leaves_hashchain_hash: String,

    /// Starting index for appending leaves
    pub start_index: u64,

    /// Existing leaves at positions (may be zero if empty)
    pub old_leaves: Vec<String>,

    /// New leaves to append
    pub leaves: Vec<String>,

    /// Merkle proofs for each leaf position
    pub merkle_proofs: Vec<Vec<String>>,

    /// Tree height
    pub height: u32,

    /// Batch size
    pub batch_size: u32,
}

/// Batch Update circuit inputs.
///
/// Matches the JSON format from `BatchUpdateInputsJSON` in Go.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateInputs {
    /// Circuit type identifier (should be "update")
    pub circuit_type: String,

    /// Public input hash (hex string with 0x prefix)
    pub public_input_hash: String,

    /// Old Merkle root before batch update (hex string)
    pub old_root: String,

    /// New Merkle root after batch update (hex string)
    pub new_root: String,

    /// Hash chain of nullifier hashes (hex string)
    pub leaves_hashchain_hash: String,

    /// Transaction hashes for each nullification
    pub tx_hashes: Vec<String>,

    /// New leaf values (to be nullified)
    pub leaves: Vec<String>,

    /// Current leaf values at positions
    pub old_leaves: Vec<String>,

    /// Merkle proofs for each position
    pub merkle_proofs: Vec<Vec<String>>,

    /// Path indices (positions in tree)
    pub path_indices: Vec<u64>,

    /// Tree height
    pub height: u32,

    /// Batch size
    pub batch_size: u32,
}

/// Batch Address Append circuit inputs.
///
/// Matches the JSON format from `BatchAddressAppendParametersJSON` in Go.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchAddressAppendInputs {
    /// Circuit type identifier (should be "address-append")
    pub circuit_type: String,

    /// Public input hash (hex string with 0x prefix)
    pub public_input_hash: String,

    /// Old Merkle root (hex string)
    pub old_root: String,

    /// New Merkle root (hex string)
    pub new_root: String,

    /// Hash chain of new element values (hex string)
    pub hashchain_hash: String,

    /// Starting index for new elements
    pub start_index: u64,

    /// Low element values (lower bounds)
    pub low_element_values: Vec<String>,

    /// Low element indices in tree
    pub low_element_indices: Vec<String>,

    /// Low element next values (upper bounds)
    pub low_element_next_values: Vec<String>,

    /// New element values to insert
    pub new_element_values: Vec<String>,

    /// Merkle proofs for low elements
    pub low_element_proofs: Vec<Vec<String>>,

    /// Merkle proofs for new element positions
    pub new_element_proofs: Vec<Vec<String>>,

    /// Tree height
    pub tree_height: u32,

    /// Batch size
    pub batch_size: u32,
}

/// Proof output format matching Gnark's Groth16 proof structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOutput {
    /// A point (G1) - 2 coordinates
    pub ar: Vec<String>,

    /// B point (G2) - 2x2 matrix
    pub bs: Vec<Vec<String>>,

    /// C point (G1) - 2 coordinates
    pub krs: Vec<String>,
}

/// Parse a hex string (with or without 0x prefix) to 32 bytes.
pub fn hex_to_bytes32(hex: &str) -> Result<Hash, String> {
    let hex = hex.strip_prefix("0x").unwrap_or(hex);

    // Pad with leading zeros if needed
    let padded = format!("{:0>64}", hex);

    let bytes = hex::decode(&padded).map_err(|e| format!("Invalid hex: {}", e))?;

    if bytes.len() != 32 {
        return Err(format!("Expected 32 bytes, got {}", bytes.len()));
    }

    let mut result = [0u8; 32];
    result.copy_from_slice(&bytes);
    Ok(result)
}

/// Convert 32 bytes to hex string with 0x prefix.
pub fn bytes32_to_hex(bytes: &Hash) -> String {
    format!("0x{}", hex::encode(bytes))
}

impl BatchAppendInputs {
    /// Parse all hex fields into byte arrays.
    pub fn parse(&self) -> Result<ParsedBatchAppendInputs, String> {
        let public_input_hash = hex_to_bytes32(&self.public_input_hash)?;
        let old_root = hex_to_bytes32(&self.old_root)?;
        let new_root = hex_to_bytes32(&self.new_root)?;
        let leaves_hashchain_hash = hex_to_bytes32(&self.leaves_hashchain_hash)?;

        let old_leaves: Result<Vec<Hash>, String> =
            self.old_leaves.iter().map(|h| hex_to_bytes32(h)).collect();
        let leaves: Result<Vec<Hash>, String> =
            self.leaves.iter().map(|h| hex_to_bytes32(h)).collect();

        let merkle_proofs: Result<Vec<Vec<Hash>>, String> = self
            .merkle_proofs
            .iter()
            .map(|proof| proof.iter().map(|h| hex_to_bytes32(h)).collect())
            .collect();

        Ok(ParsedBatchAppendInputs {
            public_input_hash,
            old_root,
            new_root,
            leaves_hashchain_hash,
            start_index: self.start_index as u32,
            old_leaves: old_leaves?,
            leaves: leaves?,
            merkle_proofs: merkle_proofs?,
            height: self.height,
            batch_size: self.batch_size,
        })
    }
}

/// Parsed BatchAppend inputs with byte arrays instead of hex strings.
#[derive(Debug, Clone)]
pub struct ParsedBatchAppendInputs {
    pub public_input_hash: Hash,
    pub old_root: Hash,
    pub new_root: Hash,
    pub leaves_hashchain_hash: Hash,
    pub start_index: u32,
    pub old_leaves: Vec<Hash>,
    pub leaves: Vec<Hash>,
    pub merkle_proofs: Vec<Vec<Hash>>,
    pub height: u32,
    pub batch_size: u32,
}

impl BatchUpdateInputs {
    /// Parse all hex fields into byte arrays.
    pub fn parse(&self) -> Result<ParsedBatchUpdateInputs, String> {
        let public_input_hash = hex_to_bytes32(&self.public_input_hash)?;
        let old_root = hex_to_bytes32(&self.old_root)?;
        let new_root = hex_to_bytes32(&self.new_root)?;
        let leaves_hashchain_hash = hex_to_bytes32(&self.leaves_hashchain_hash)?;

        let tx_hashes: Result<Vec<Hash>, String> =
            self.tx_hashes.iter().map(|h| hex_to_bytes32(h)).collect();
        let leaves: Result<Vec<Hash>, String> =
            self.leaves.iter().map(|h| hex_to_bytes32(h)).collect();
        let old_leaves: Result<Vec<Hash>, String> =
            self.old_leaves.iter().map(|h| hex_to_bytes32(h)).collect();

        let merkle_proofs: Result<Vec<Vec<Hash>>, String> = self
            .merkle_proofs
            .iter()
            .map(|proof| proof.iter().map(|h| hex_to_bytes32(h)).collect())
            .collect();

        // Convert path_indices from u64 to u32
        let path_indices: Vec<u32> = self.path_indices.iter().map(|&i| i as u32).collect();

        Ok(ParsedBatchUpdateInputs {
            public_input_hash,
            old_root,
            new_root,
            leaves_hashchain_hash,
            tx_hashes: tx_hashes?,
            leaves: leaves?,
            old_leaves: old_leaves?,
            merkle_proofs: merkle_proofs?,
            path_indices,
            height: self.height,
            batch_size: self.batch_size,
        })
    }
}

/// Parsed BatchUpdate inputs with byte arrays instead of hex strings.
#[derive(Debug, Clone)]
pub struct ParsedBatchUpdateInputs {
    pub public_input_hash: Hash,
    pub old_root: Hash,
    pub new_root: Hash,
    pub leaves_hashchain_hash: Hash,
    pub tx_hashes: Vec<Hash>,
    pub leaves: Vec<Hash>,
    pub old_leaves: Vec<Hash>,
    pub merkle_proofs: Vec<Vec<Hash>>,
    pub path_indices: Vec<u32>,
    pub height: u32,
    pub batch_size: u32,
}

impl BatchAddressAppendInputs {
    /// Parse all hex fields into byte arrays.
    pub fn parse(&self) -> Result<ParsedBatchAddressAppendInputs, String> {
        let public_input_hash = hex_to_bytes32(&self.public_input_hash)?;
        let old_root = hex_to_bytes32(&self.old_root)?;
        let new_root = hex_to_bytes32(&self.new_root)?;
        let hashchain_hash = hex_to_bytes32(&self.hashchain_hash)?;

        let low_element_values: Result<Vec<Hash>, String> = self
            .low_element_values
            .iter()
            .map(|h| hex_to_bytes32(h))
            .collect();
        let low_element_indices: Result<Vec<u32>, String> = self
            .low_element_indices
            .iter()
            .map(|h| {
                let bytes = hex_to_bytes32(h)?;
                // Extract last 4 bytes as u32
                Ok(u32::from_be_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]))
            })
            .collect();
        let low_element_next_values: Result<Vec<Hash>, String> = self
            .low_element_next_values
            .iter()
            .map(|h| hex_to_bytes32(h))
            .collect();
        let new_element_values: Result<Vec<Hash>, String> = self
            .new_element_values
            .iter()
            .map(|h| hex_to_bytes32(h))
            .collect();

        let low_element_proofs: Result<Vec<Vec<Hash>>, String> = self
            .low_element_proofs
            .iter()
            .map(|proof| proof.iter().map(|h| hex_to_bytes32(h)).collect())
            .collect();
        let new_element_proofs: Result<Vec<Vec<Hash>>, String> = self
            .new_element_proofs
            .iter()
            .map(|proof| proof.iter().map(|h| hex_to_bytes32(h)).collect())
            .collect();

        Ok(ParsedBatchAddressAppendInputs {
            public_input_hash,
            old_root,
            new_root,
            hashchain_hash,
            start_index: self.start_index as u32,
            low_element_values: low_element_values?,
            low_element_indices: low_element_indices?,
            low_element_next_values: low_element_next_values?,
            new_element_values: new_element_values?,
            low_element_proofs: low_element_proofs?,
            new_element_proofs: new_element_proofs?,
            tree_height: self.tree_height,
            batch_size: self.batch_size,
        })
    }
}

/// Parsed BatchAddressAppend inputs with byte arrays instead of hex strings.
#[derive(Debug, Clone)]
pub struct ParsedBatchAddressAppendInputs {
    pub public_input_hash: Hash,
    pub old_root: Hash,
    pub new_root: Hash,
    pub hashchain_hash: Hash,
    pub start_index: u32,
    pub low_element_values: Vec<Hash>,
    pub low_element_indices: Vec<u32>,
    pub low_element_next_values: Vec<Hash>,
    pub new_element_values: Vec<Hash>,
    pub low_element_proofs: Vec<Vec<Hash>>,
    pub new_element_proofs: Vec<Vec<Hash>>,
    pub tree_height: u32,
    pub batch_size: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_bytes32() {
        let hex = "0x0000000000000000000000000000000000000000000000000000000000000001";
        let bytes = hex_to_bytes32(hex).unwrap();
        assert_eq!(bytes[31], 1);
        assert_eq!(bytes[0..31], [0u8; 31]);

        // Without prefix
        let hex2 = "0000000000000000000000000000000000000000000000000000000000000002";
        let bytes2 = hex_to_bytes32(hex2).unwrap();
        assert_eq!(bytes2[31], 2);

        // Short hex (should be padded)
        let hex3 = "0x1";
        let bytes3 = hex_to_bytes32(hex3).unwrap();
        assert_eq!(bytes3[31], 1);
    }

    #[test]
    fn test_bytes32_to_hex() {
        let mut bytes = [0u8; 32];
        bytes[31] = 1;
        let hex = bytes32_to_hex(&bytes);
        assert_eq!(
            hex,
            "0x0000000000000000000000000000000000000000000000000000000000000001"
        );
    }

    #[test]
    fn test_batch_append_deserialize() {
        let json = r#"{
            "circuitType": "append",
            "stateTreeHeight": 32,
            "publicInputHash": "0x1234",
            "oldRoot": "0x5678",
            "newRoot": "0xabcd",
            "leavesHashchainHash": "0xef01",
            "startIndex": 100,
            "oldLeaves": ["0x0", "0x0"],
            "leaves": ["0x1", "0x2"],
            "merkleProofs": [["0xa", "0xb"], ["0xc", "0xd"]],
            "height": 32,
            "batchSize": 2
        }"#;

        let inputs: BatchAppendInputs = serde_json::from_str(json).unwrap();
        assert_eq!(inputs.circuit_type, "append");
        assert_eq!(inputs.start_index, 100);
        assert_eq!(inputs.batch_size, 2);
        assert_eq!(inputs.leaves.len(), 2);
    }
}
