use crate::{Hasher, HasherError, Poseidon};

/// Creates a hash chain from an array of [u8;32] arrays.
///
/// # Parameters
/// - `inputs`: An array of [u8;32] arrays to be hashed.
///
/// # Returns
/// - `Result<[u8; 32], HasherError>`: The resulting hash chain or an error.
pub fn create_hash_chain_from_array<const T: usize>(
    inputs: [[u8; 32]; T],
) -> Result<[u8; 32], HasherError> {
    create_hash_chain_from_slice(&inputs)
}

/// Creates a hash chain from a slice of [u8;32] arrays.
///
/// # Parameters
/// - `inputs`: A slice of [u8;32] array to be hashed.
///
/// # Returns
/// - `Result<[u8; 32], HasherError>`: The resulting hash chain or an error.
pub fn create_hash_chain_from_slice(inputs: &[[u8; 32]]) -> Result<[u8; 32], HasherError> {
    if inputs.is_empty() {
        return Ok([0u8; 32]);
    }
    let mut hash_chain = inputs[0];
    for input in inputs.iter().skip(1) {
        hash_chain = Poseidon::hashv(&[&hash_chain, input])?;
    }
    Ok(hash_chain)
}

/// Creates a two inputs hash chain from two slices of [u8;32] arrays.
/// The two slices must have the same length.
/// Hashes are hashed in pairs, with the first hash from
/// the first slice and the second hash from the second slice.
/// H(i) = H(H(i-1), hashes_first[i], hashes_second[i])
///
/// # Parameters
/// - `hashes_first`: A slice of [u8;32] arrays to be hashed first.
/// - `hashes_second`: A slice of [u8;32] arrays to be hashed second.
///
/// # Returns
/// - `Result<[u8; 32], HasherError>`: The resulting hash chain or an error.
pub fn create_two_inputs_hash_chain(
    hashes_first: &[[u8; 32]],
    hashes_second: &[[u8; 32]],
) -> Result<[u8; 32], HasherError> {
    let first_len = hashes_first.len();
    if first_len != hashes_second.len() {
        return Err(HasherError::InvalidInputLength(
            first_len,
            hashes_second.len(),
        ));
    }
    if hashes_first.is_empty() {
        return Ok([0u8; 32]);
    }
    let mut hash_chain = Poseidon::hashv(&[&hashes_first[0], &hashes_second[0]])?;

    if first_len == 1 {
        return Ok(hash_chain);
    }

    for i in 1..first_len {
        hash_chain = Poseidon::hashv(&[&hash_chain, &hashes_first[i], &hashes_second[i]])?;
    }
    Ok(hash_chain)
}

#[cfg(test)]
mod hash_chain_tests {
    use ark_ff::PrimeField;
    use light_poseidon::PoseidonError;
    use num_bigint::BigUint;

    use super::*;
    use crate::{bigint::bigint_to_be_bytes_array, Hasher, HasherError, Poseidon};

    /// Tests for `create_hash_chain_from_slice` function:
    /// Functional tests:
    /// 1. Functional - with hardcoded values.
    /// 2. Functional - with manual hash comparison.
    /// 3. Functional - for determinism (hashing the same input twice).
    /// 4. Functional - empty input case returns zero hash.
    ///
    /// Failing tests:
    /// 5. Failing - input larger than modulus
    #[test]
    fn test_create_hash_chain_from_slice() {
        // 1. Functional test with hardcoded values.
        {
            // Define hardcoded inputs.
            let inputs: [[u8; 32]; 3] = [[1u8; 32], [2u8; 32], [3u8; 32]];

            // Manually compute the expected hash chain using Poseidon.
            // Note: The expected hash values should be precomputed using the same Poseidon parameters.
            // For demonstration purposes, we'll assume hypothetical hash outputs.
            // In a real scenario, replace these with actual expected values.
            let intermediate_hash_1 = Poseidon::hashv(&[&inputs[0], &inputs[1]]).unwrap();
            let expected_hash = Poseidon::hashv(&[&intermediate_hash_1, &inputs[2]]).unwrap();

            // Call the function under test.
            let result = create_hash_chain_from_slice(&inputs).unwrap();

            // Assert that the result matches the expected hash.
            assert_eq!(
                result, expected_hash,
                "Functional test with hardcoded values failed."
            );
        }

        // 2. Functional test with manual hash comparison.
        {
            let inputs: [[u8; 32]; 2] = [[4u8; 32], [5u8; 32]];

            // Manually compute the expected hash.
            let expected_hash = Poseidon::hashv(&[&inputs[0], &inputs[1]]).unwrap();
            let hard_coded_expected_hash = [
                13, 250, 206, 124, 182, 159, 160, 87, 57, 23, 80, 155, 25, 43, 40, 136, 228, 255,
                201, 1, 22, 168, 211, 220, 176, 187, 23, 176, 46, 198, 140, 211,
            ];

            let result = create_hash_chain_from_slice(&inputs).unwrap();

            assert_eq!(
                result, expected_hash,
                "Functional test with manual hash comparison failed."
            );
            assert_eq!(result, hard_coded_expected_hash);
        }

        // 2. Functional test with manual hash comparison.
        {
            let inputs = [[4u8; 32], [5u8; 32], [6u8; 32]];

            let expected_hash = Poseidon::hashv(&[&inputs[0], &inputs[1]]).unwrap();
            let expected_hash = Poseidon::hashv(&[&expected_hash, &inputs[2]]).unwrap();
            let hard_coded_expected_hash = [
                12, 74, 32, 81, 132, 82, 10, 115, 75, 248, 169, 125, 228, 230, 140, 167, 149, 181,
                244, 194, 63, 201, 26, 150, 142, 4, 60, 16, 77, 145, 194, 152,
            ];

            let result = create_hash_chain_from_slice(&inputs).unwrap();

            assert_eq!(
                result, expected_hash,
                "Functional test with manual hash comparison failed."
            );
            assert_eq!(result, hard_coded_expected_hash);
        }

        // 3. Functional test for determinism (hashing the same input twice).
        {
            // Define inputs.
            let inputs: [[u8; 32]; 2] = [[6u8; 32], [7u8; 32]];

            // Compute hash chain the first time.
            let first_hash = create_hash_chain_from_slice(&inputs).unwrap();

            // Compute hash chain the second time.
            let second_hash = create_hash_chain_from_slice(&inputs).unwrap();

            // Assert that both hashes are identical.
            assert_eq!(
                first_hash, second_hash,
                "Determinism test failed: Hashes do not match."
            );
        }

        // 4. Test empty input case
        {
            let inputs: [[u8; 32]; 0] = [];
            let result = create_hash_chain_from_slice(&inputs).unwrap();
            assert_eq!(result, [0u8; 32], "Empty input should return zero hash");
        }

        // 5. Failing - input larger than modulus
        {
            let modulus: BigUint = ark_bn254::Fr::MODULUS.into();
            let modulus_bytes: [u8; 32] = bigint_to_be_bytes_array(&modulus).unwrap();
            let huge_input = vec![modulus_bytes, modulus_bytes];
            let result = create_hash_chain_from_slice(&huge_input);
            assert!(
                matches!(result, Err(HasherError::Poseidon(error)) if error  == PoseidonError::InputLargerThanModulus),
            );
        }
    }

    /// Tests for `create_two_inputs_hash_chain` function:
    /// 1. Functional - empty inputs.
    /// 2. Functional - 1 input each.
    /// 3. Functional - 2 inputs each.
    /// 4. Failing - invalid input length for hashes_first.
    /// 5. Failing - invalid input length for hashes_second.
    #[test]
    fn test_create_two_inputs_hash_chain() {
        // 1. Functional test with empty inputs.
        {
            let hashes_first: &[[u8; 32]] = &[];
            let hashes_second: &[[u8; 32]] = &[];
            let result = create_two_inputs_hash_chain(hashes_first, hashes_second).unwrap();
            assert_eq!(result, [0u8; 32], "Empty input should return zero hash");
        }

        // 2. Functional test with 1 input each.
        {
            let hashes_first: &[[u8; 32]] = &[[1u8; 32]];
            let hashes_second: &[[u8; 32]] = &[[2u8; 32]];
            let expected_hash = Poseidon::hashv(&[&hashes_first[0], &hashes_second[0]]).unwrap();
            let result = create_two_inputs_hash_chain(hashes_first, hashes_second).unwrap();
            assert_eq!(result, expected_hash, "Single input each test failed");
        }

        // 3. Functional test with 2 inputs each.
        {
            let hashes_first: &[[u8; 32]] = &[[1u8; 32], [2u8; 32]];
            let hashes_second: &[[u8; 32]] = &[[3u8; 32], [4u8; 32]];
            let intermediate_hash =
                Poseidon::hashv(&[&hashes_first[0], &hashes_second[0]]).unwrap();
            let expected_hash =
                Poseidon::hashv(&[&intermediate_hash, &hashes_first[1], &hashes_second[1]])
                    .unwrap();
            let result = create_two_inputs_hash_chain(hashes_first, hashes_second).unwrap();
            assert_eq!(result, expected_hash, "Two inputs each test failed");
        }

        // 4. Failing test with invalid input length for hashes_first.
        {
            let hashes_first: &[[u8; 32]] = &[[1u8; 32]];
            let hashes_second: &[[u8; 32]] = &[[2u8; 32], [3u8; 32]];
            let result = create_two_inputs_hash_chain(hashes_first, hashes_second);
            assert!(
                matches!(result, Err(HasherError::InvalidInputLength(1, 2))),
                "Invalid input length for hashes_first test failed"
            );
        }

        // 5. Failing test with invalid input length for hashes_second.
        {
            let hashes_first: &[[u8; 32]] = &[[1u8; 32], [2u8; 32]];
            let hashes_second: &[[u8; 32]] = &[[3u8; 32]];
            let result = create_two_inputs_hash_chain(hashes_first, hashes_second);
            assert!(
                matches!(result, Err(HasherError::InvalidInputLength(2, 1))),
                "Invalid input length for hashes_second test failed"
            );
        }
    }
}
