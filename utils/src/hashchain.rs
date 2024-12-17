use light_hasher::{Hasher, Poseidon};

use crate::UtilsError;

pub fn create_hash_chain<const T: usize>(inputs: [[u8; 32]; T]) -> Result<[u8; 32], UtilsError> {
    let mut hash_chain = inputs[0];
    for input in inputs.iter().skip(1) {
        hash_chain = Poseidon::hashv(&[&hash_chain, input])?;
    }
    Ok(hash_chain)
}

pub fn create_hash_chain_from_vec(inputs: Vec<[u8; 32]>) -> Result<[u8; 32], UtilsError> {
    let mut hash_chain = inputs[0];
    for input in inputs.iter().skip(1) {
        hash_chain = Poseidon::hashv(&[&hash_chain, input])?;
    }
    Ok(hash_chain)
}

pub fn create_hash_chain_from_slice(inputs: &[[u8; 32]]) -> Result<[u8; 32], UtilsError> {
    if inputs.is_empty() {
        return Ok([0u8; 32]);
    }
    let mut hash_chain = inputs[0];
    for input in inputs.iter().skip(1) {
        hash_chain = Poseidon::hashv(&[&hash_chain, input])?;
    }
    Ok(hash_chain)
}

// TODO: add tests
pub fn create_two_inputs_hash_chain(
    hashes_first: &[[u8; 32]],
    hashes_second: &[[u8; 32]],
) -> Result<[u8; 32], UtilsError> {
    assert_eq!(hashes_first.len(), hashes_second.len());
    if hashes_first.is_empty() {
        return Ok([0u8; 32]);
    }
    let mut hash_chain = Poseidon::hashv(&[&hashes_first[0], &hashes_second[0]])?;

    if hashes_first.len() == 1 {
        return Ok(hash_chain);
    }

    for i in 1..hashes_first.len() {
        hash_chain = Poseidon::hashv(&[&hash_chain, &hashes_first[i], &hashes_second[i]])?;
    }
    Ok(hash_chain)
}

pub fn create_tx_hash(
    input_compressed_account_hashes: &[[u8; 32]],
    output_compressed_account_hashes: &[[u8; 32]],
    current_slot: u64,
) -> Result<[u8; 32], UtilsError> {
    let version = [0u8; 32];
    let mut slot_bytes = [0u8; 32];
    slot_bytes[24..].copy_from_slice(&current_slot.to_be_bytes());
    let inputs_hash_chain = create_hash_chain_from_slice(input_compressed_account_hashes)?;
    let outputs_hash_chain = create_hash_chain_from_slice(output_compressed_account_hashes)?;
    let hash_chain = create_hash_chain_from_slice(&[
        version,
        inputs_hash_chain,
        outputs_hash_chain,
        slot_bytes,
    ])?;
    Ok(hash_chain)
}
