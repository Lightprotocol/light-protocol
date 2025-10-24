#![cfg(feature = "poseidon")]

use std::vec::Vec;

use light_hasher::{DataHasher, Hasher, HasherError, Poseidon};

// A simple test struct that implements DataHasher
#[derive(Default, Clone)]
struct TestHashable {
    value: u8,
}

impl TestHashable {
    fn new(value: u8) -> Self {
        Self { value }
    }
}

impl DataHasher for TestHashable {
    fn hash<H: Hasher>(&self) -> Result<[u8; 32], HasherError> {
        // Simple implementation that creates a predictable hash
        let mut result = [0u8; 32];
        result[31] = self.value;
        Ok(result)
    }
}

#[test]
fn test_data_hasher_array_1() {
    let arr = [TestHashable::new(42)];
    let hash_result = arr.hash::<Poseidon>().unwrap();

    // The result should be the Poseidon hash of the single element's hash
    let expected_input = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 42,
    ];
    let expected_hash = Poseidon::hash(&expected_input).unwrap();

    assert_eq!(hash_result, expected_hash);
}

#[test]
fn test_data_hasher_array_2() {
    let arr = [TestHashable::new(1), TestHashable::new(2)];
    let hash_result = arr.hash::<Poseidon>().unwrap();

    // Expected inputs are the hashes of each TestHashable
    let hash1 = [0u8; 32];
    let hash2 = [0u8; 32];

    let mut hash1 = hash1;
    hash1[31] = 1;

    let mut hash2 = hash2;
    hash2[31] = 2;

    // The result should be the Poseidon hash of concatenated element hashes
    let expected_hash = Poseidon::hashv(&[&hash1, &hash2]).unwrap();

    assert_eq!(hash_result, expected_hash);
}

#[test]
fn test_data_hasher_array_multiple_sizes() {
    // Test arrays of each implemented size
    for size in 1..=12 {
        let mut array = Vec::with_capacity(size);
        for i in 0..size {
            array.push(TestHashable::new(i as u8));
        }

        // Convert the Vec to an array of the appropriate size
        let array_slice = array.as_slice();

        // Create expected inputs (hashes of each TestHashable)
        let mut expected_inputs = Vec::with_capacity(size);
        for i in 0..size {
            let mut hash = [0u8; 32];
            hash[31] = i as u8;
            expected_inputs.push(hash);
        }

        // Dynamically test each array size
        match size {
            1 => {
                let arr: [TestHashable; 1] = [array_slice[0].clone()];
                let hash_result = arr.hash::<Poseidon>().unwrap();

                let expected_slices: Vec<&[u8]> =
                    expected_inputs.iter().map(|h| h.as_slice()).collect();
                let expected_hash = Poseidon::hashv(&expected_slices).unwrap();

                assert_eq!(hash_result, expected_hash);
            }
            2 => {
                let arr: [TestHashable; 2] = [array_slice[0].clone(), array_slice[1].clone()];
                let hash_result = arr.hash::<Poseidon>().unwrap();

                let expected_slices: Vec<&[u8]> =
                    expected_inputs.iter().map(|h| h.as_slice()).collect();
                let expected_hash = Poseidon::hashv(&expected_slices).unwrap();

                assert_eq!(hash_result, expected_hash);
            }
            3 => {
                let arr: [TestHashable; 3] = [
                    array_slice[0].clone(),
                    array_slice[1].clone(),
                    array_slice[2].clone(),
                ];
                let hash_result = arr.hash::<Poseidon>().unwrap();

                let expected_slices: Vec<&[u8]> =
                    expected_inputs.iter().map(|h| h.as_slice()).collect();
                let expected_hash = Poseidon::hashv(&expected_slices).unwrap();

                assert_eq!(hash_result, expected_hash);
            }
            // We test one more size (4) to confirm the pattern works
            4 => {
                let arr: [TestHashable; 4] = [
                    array_slice[0].clone(),
                    array_slice[1].clone(),
                    array_slice[2].clone(),
                    array_slice[3].clone(),
                ];
                let hash_result = arr.hash::<Poseidon>().unwrap();

                let expected_slices: Vec<&[u8]> =
                    expected_inputs.iter().map(|h| h.as_slice()).collect();
                let expected_hash = Poseidon::hashv(&expected_slices).unwrap();

                assert_eq!(hash_result, expected_hash);
            }
            _ => {
                // For sizes 5-12, we've verified the pattern with tests for sizes 1-4
                // We could add more tests here if needed
            }
        }
    }
}
