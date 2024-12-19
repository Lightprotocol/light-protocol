use env_logger::Builder;
use light_concurrent_merkle_tree::changelog::ChangelogEntry;
use light_hasher::{Hasher, Poseidon};
use log::LevelFilter;
use num_bigint::BigInt;

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

pub fn init_logger() {
    let _ = Builder::new()
        .filter_module("light_prover_client", LevelFilter::Info)
        .try_init();
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
        if current_index % 2 == 0 {
            current_hash = Poseidon::hashv(&[&current_hash, path_element]).unwrap();
        } else {
            current_hash = Poseidon::hashv(&[path_element, &current_hash]).unwrap();
        }
        current_index /= 2;
    }

    (current_hash, changelog_entry)
}
