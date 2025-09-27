use std::process::Command;

use light_hasher::{Hasher, Poseidon};
use light_sparse_merkle_tree::changelog::ChangelogEntry;
use num_bigint::{BigInt, BigUint};
use num_traits::{Num, ToPrimitive};
use serde::Serialize;
use serde_json::json;

pub fn get_project_root() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout).ok()
    } else {
        None
    }
}

pub fn change_endianness(bytes: &[u8]) -> Vec<u8> {
    let mut vec = Vec::new();
    for b in bytes.chunks(32) {
        for byte in b.iter().rev() {
            vec.push(*byte);
        }
    }
    vec
}
pub fn convert_endianness_128(bytes: &[u8]) -> Vec<u8> {
    bytes
        .chunks(64)
        .flat_map(|b| b.iter().copied().rev().collect::<Vec<u8>>())
        .collect::<Vec<u8>>()
}

pub fn bigint_to_u8_32(n: &BigInt) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let (_, bytes_be) = n.to_bytes_be();
    if bytes_be.len() > 32 {
        Err("Number too large to fit in [u8; 32]")?;
    }
    let mut array = [0; 32];
    let bytes = &bytes_be[..bytes_be.len()];
    array[(32 - bytes.len())..].copy_from_slice(bytes);
    Ok(array)
}

pub fn compute_root_from_merkle_proof<const HEIGHT: usize>(
    leaf: [u8; 32],
    path_elements: &[[u8; 32]; HEIGHT],
    path_index: u32,
) -> ([u8; 32], ChangelogEntry<HEIGHT>) {
    let mut changelog_entry = ChangelogEntry::default_with_index(path_index as usize);

    let mut current_hash = leaf;
    let mut current_index = path_index;
    for (level, path_element) in path_elements.iter().enumerate() {
        changelog_entry.path[level] = Some(current_hash);
        if current_index.is_multiple_of(2) {
            current_hash = Poseidon::hashv(&[&current_hash, path_element]).unwrap();
        } else {
            current_hash = Poseidon::hashv(&[path_element, &current_hash]).unwrap();
        }
        current_index /= 2;
    }

    (current_hash, changelog_entry)
}

pub fn big_uint_to_string(big_uint: &BigUint) -> String {
    format!("0x{}", big_uint.to_str_radix(16))
}

pub fn big_int_to_string(big_int: &BigInt) -> String {
    format!("0x{}", big_int.to_str_radix(16))
}
pub fn string_to_big_int(hex_str: &str) -> Option<BigInt> {
    if hex_str.starts_with("0x") || hex_str.starts_with("0X") {
        BigInt::from_str_radix(&hex_str[2..], 16).ok()
    } else {
        None
    }
}

pub fn create_vec_of_string(number_of_utxos: usize, element: &BigInt) -> Vec<String> {
    vec![big_int_to_string(element); number_of_utxos]
}

pub fn create_vec_of_u32(number_of_utxos: usize, element: &BigInt) -> Vec<u32> {
    vec![element.to_u32().unwrap(); number_of_utxos]
}

pub fn create_vec_of_vec_of_string(
    number_of_utxos: usize,
    elements: &[BigInt],
) -> Vec<Vec<String>> {
    let vec: Vec<String> = elements
        .iter()
        .map(|e| format!("0x{}", e.to_str_radix(16)))
        .collect();
    vec![vec; number_of_utxos]
}

pub fn create_json_from_struct<T>(json_struct: &T) -> String
where
    T: Serialize,
{
    let json = json!(json_struct);
    match serde_json::to_string_pretty(&json) {
        Ok(json) => json,
        Err(_) => panic!("Merkle tree data invalid"),
    }
}
