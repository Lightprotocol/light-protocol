use crate::{hash_chain::create_hash_chain_from_slice, CompressedAccountError};

/// Creates a transaction hash from the given input and output compressed account hashes and the current slot.
///
/// # Parameters
/// - `input_compressed_account_hashes`: A slice of input compressed account hashes.
/// - `output_compressed_account_hashes`: A slice of output compressed account hashes.
/// - `current_slot`: The current slot as a 64-bit unsigned integer.
pub fn create_tx_hash(
    input_compressed_account_hashes: &[[u8; 32]],
    output_compressed_account_hashes: &[[u8; 32]],
    current_slot: u64,
) -> Result<[u8; 32], CompressedAccountError> {
    let version = [0u8; 32];
    let mut current_slot_bytes = [0u8; 32];
    current_slot_bytes[24..].copy_from_slice(&current_slot.to_be_bytes());
    let inputs_hash_chain = create_hash_chain_from_slice(input_compressed_account_hashes)?;
    let outputs_hash_chain = create_hash_chain_from_slice(output_compressed_account_hashes)?;
    let hash_chain = create_hash_chain_from_slice(&[
        version,
        inputs_hash_chain,
        outputs_hash_chain,
        current_slot_bytes,
    ])?;
    Ok(hash_chain)
}

pub fn create_tx_hash_from_hash_chains(
    inputs_hash_chain: &[u8; 32],
    outputs_hash_chain: &[u8; 32],
    current_slot: u64,
) -> Result<[u8; 32], CompressedAccountError> {
    let version = [0u8; 32];
    let mut current_slot_bytes = [0u8; 32];
    current_slot_bytes[24..].copy_from_slice(&current_slot.to_be_bytes());
    let hash_chain = create_hash_chain_from_slice(&[
        version,
        *inputs_hash_chain,
        *outputs_hash_chain,
        current_slot_bytes,
    ])?;
    Ok(hash_chain)
}

#[cfg(test)]
mod hash_chain_tests {
    use light_hasher::{Hasher, Poseidon};

    use super::*;

    /// Tests for `create_tx_hash` function:
    /// Functional tests:
    /// 1. Functional - empty input slices.
    /// 2. Functional - single element slices.
    /// 3. Functional - multiple elements.
    #[test]
    fn test_create_tx_hash() {
        // 1. Functional test with empty input slices.
        {
            let input_hashes: &[[u8; 32]] = &[];
            let output_hashes: &[[u8; 32]] = &[];
            let current_slot: u64 = 0;
            let result = create_tx_hash(input_hashes, output_hashes, current_slot).unwrap();
            let result_2 = create_tx_hash(input_hashes, output_hashes, current_slot).unwrap();
            assert_eq!(result, result_2);
            let expected_hash =
                create_hash_chain_from_slice(&[[0u8; 32], [0u8; 32], [0u8; 32], [0u8; 32]])
                    .unwrap();
            assert_eq!(
                result,
                [
                    34, 200, 164, 208, 164, 254, 32, 90, 32, 86, 179, 30, 247, 124, 171, 192, 209,
                    57, 53, 27, 29, 19, 51, 1, 231, 57, 166, 115, 49, 208, 244, 143
                ],
            );
            assert_eq!(result, expected_hash);
        }

        // 2. Functional test with single element slices.
        {
            let input_hashes: &[[u8; 32]] = &[[1u8; 32]];
            let output_hashes: &[[u8; 32]] = &[[2u8; 32]];
            let current_slot: u64 = 1;
            let result = create_tx_hash(input_hashes, output_hashes, current_slot).unwrap();
            let result_2 = create_tx_hash(input_hashes, output_hashes, current_slot).unwrap();
            assert_eq!(result, result_2);
            let mut slot_bytes = [0u8; 32];
            slot_bytes[24..].copy_from_slice(&current_slot.to_be_bytes());
            let expected_hash = create_hash_chain_from_slice(&[
                [0u8; 32],
                input_hashes[0],
                output_hashes[0],
                slot_bytes,
            ]);
            let hard_coded_expected_hash = [
                26, 2, 125, 115, 183, 17, 182, 221, 81, 35, 84, 46, 94, 86, 98, 1, 32, 169, 75, 74,
                163, 148, 202, 22, 163, 60, 43, 57, 188, 253, 203, 81,
            ];
            assert_eq!(result, expected_hash.unwrap());
            assert_eq!(result, hard_coded_expected_hash);
        }

        // 3. Functional test with multiple elements.
        {
            let input_hashes: &[[u8; 32]] = &[[1u8; 32], [2u8; 32]];
            let output_hashes: &[[u8; 32]] = &[[3u8; 32], [4u8; 32]];
            let current_slot: u64 = 2;
            let result = create_tx_hash(input_hashes, output_hashes, current_slot).unwrap();
            let result_2 = create_tx_hash(input_hashes, output_hashes, current_slot).unwrap();
            assert_eq!(result, result_2);
            let inclusion = Poseidon::hashv(&[&input_hashes[0], &input_hashes[1]]).unwrap();
            let outputs = Poseidon::hashv(&[&output_hashes[0], &output_hashes[1]]).unwrap();
            let mut slot_bytes = [0u8; 32];
            slot_bytes[24..].copy_from_slice(&current_slot.to_be_bytes());
            let expected_hash =
                create_hash_chain_from_slice(&[[0u8; 32], inclusion, outputs, slot_bytes]);
            let hard_coded_expected_hash = [
                16, 202, 249, 127, 226, 111, 123, 166, 255, 16, 162, 227, 122, 43, 247, 141, 81,
                139, 79, 182, 213, 250, 225, 201, 235, 241, 26, 77, 117, 4, 91, 150,
            ];
            assert_eq!(result, expected_hash.unwrap());
            assert_eq!(result, hard_coded_expected_hash);
        }
    }
}
