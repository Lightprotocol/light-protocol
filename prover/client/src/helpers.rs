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
    compute_root_from_merkle_proof_with_cache(leaf, path_elements, path_index, None)
}

/// Compute Merkle root from proof with optional caching.
///
/// **Performance optimization:** When `cache` is provided, parent node computations
/// are cached and reused across multiple calls. This is useful when computing roots
/// for multiple leaves sequentially, as common parent nodes (especially at higher
/// levels) can be computed once and reused.
///
/// The cache key includes both the tree position and the child hashes to handle
/// cases where the same position is computed with different values due to tree updates.
pub fn compute_root_from_merkle_proof_with_cache<const HEIGHT: usize>(
    leaf: [u8; 32],
    path_elements: &[[u8; 32]; HEIGHT],
    path_index: u32,
    mut cache: Option<&mut std::collections::HashMap<(usize, u32, [u8; 32], [u8; 32]), [u8; 32]>>,
) -> ([u8; 32], ChangelogEntry<HEIGHT>) {
    let mut changelog_entry = ChangelogEntry::default_with_index(path_index as usize);

    let mut current_hash = leaf;
    let mut current_index = path_index;

    for (level, path_element) in path_elements.iter().enumerate() {
        changelog_entry.path[level] = Some(current_hash);

        let parent_position = current_index / 2;
        let (left_hash, right_hash) = if current_index.is_multiple_of(2) {
            (current_hash, *path_element)
        } else {
            (*path_element, current_hash)
        };

        // Use cache if provided, otherwise compute directly
        current_hash = if let Some(ref mut cache_map) = cache {
            let cache_key = (level, parent_position, left_hash, right_hash);
            *cache_map
                .entry(cache_key)
                .or_insert_with(|| Poseidon::hashv(&[&left_hash, &right_hash]).unwrap())
        } else {
            Poseidon::hashv(&[&left_hash, &right_hash]).unwrap()
        };

        current_index = parent_position;
    }

    (current_hash, changelog_entry)
}

/// Batch compute roots from merkle proofs for multiple leaves.
///
/// **Performance optimization:** Computes roots for all leaves in a single pass,
/// reusing common parent node computations when leaves are close together.
///
/// For a batch of N leaves, this reduces redundant hash operations from O(N*HEIGHT)
/// to approximately O((max_index - min_index) * HEIGHT) when leaves are sequential.
///
/// **Implementation note:** Due to sequential proof adjustments in the batch append
/// process, caching benefits are limited. Each leaf's proof is adjusted using changelogs
/// from previous leaves, so parent nodes computed for one leaf may not match those needed
/// for subsequent leaves. However, we still cache to capture any shared computation,
/// particularly at higher tree levels where leaves are more likely to share ancestors.
pub fn compute_roots_from_merkle_proofs_batch<const HEIGHT: usize>(
    leaves: &[[u8; 32]],
    path_elements: &[Vec<[u8; 32]>],
    path_indices: &[u32],
) -> Vec<([u8; 32], ChangelogEntry<HEIGHT>)> {
    use std::collections::HashMap;

    assert_eq!(leaves.len(), path_elements.len());
    assert_eq!(leaves.len(), path_indices.len());

    if leaves.is_empty() {
        return Vec::new();
    }

    // For very small batches, use the simple approach to avoid HashMap overhead
    if leaves.len() <= 2 {
        return leaves
            .iter()
            .zip(path_elements.iter())
            .zip(path_indices.iter())
            .map(|((leaf, path), &index)| {
                let path_array: [[u8; 32]; HEIGHT] = path.as_slice().try_into().unwrap();
                compute_root_from_merkle_proof(*leaf, &path_array, index)
            })
            .collect();
    }

    // Cache for computed parent nodes: (level, parent_position, left_hash, right_hash) -> parent_hash
    // We include child hashes in the key because proof adjustments may cause the same
    // parent position to be computed with different children
    let mut node_cache: HashMap<(usize, u32, [u8; 32], [u8; 32]), [u8; 32]> = HashMap::new();
    let mut results = Vec::with_capacity(leaves.len());
    let mut cache_hits = 0usize;
    let mut cache_misses = 0usize;

    // Process each leaf to compute its root
    for i in 0..leaves.len() {
        let mut changelog_entry = ChangelogEntry::default_with_index(path_indices[i] as usize);
        let mut current_hash = leaves[i];
        let mut current_index = path_indices[i];

        for level in 0..HEIGHT {
            changelog_entry.path[level] = Some(current_hash);
            let path_element = path_elements[i][level];

            let parent_position = current_index / 2;

            // Determine left and right children for this parent
            let (left_hash, right_hash) = if current_index.is_multiple_of(2) {
                (current_hash, path_element)
            } else {
                (path_element, current_hash)
            };

            let cache_key = (level, parent_position, left_hash, right_hash);

            // Check cache
            let parent_hash = if let Some(&cached) = node_cache.get(&cache_key) {
                cache_hits += 1;
                cached
            } else {
                cache_misses += 1;
                let computed = Poseidon::hashv(&[&left_hash, &right_hash]).unwrap();
                node_cache.insert(cache_key, computed);
                computed
            };

            current_hash = parent_hash;
            current_index = parent_position;
        }

        results.push((current_hash, changelog_entry));
    }

    // Log cache effectiveness for performance analysis
    if cache_hits > 0 {
        let total = cache_hits + cache_misses;
        let hit_rate = (cache_hits as f64 / total as f64) * 100.0;
        tracing::trace!(
            "Batch root computation: {} leaves, {} hashes computed, {} cached ({:.1}% hit rate)",
            leaves.len(),
            cache_misses,
            cache_hits,
            hit_rate
        );
    }

    results
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
