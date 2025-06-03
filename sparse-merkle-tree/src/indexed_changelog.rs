use light_indexed_array::{array::IndexedElement, changelog::RawIndexedElement};
use num_bigint::BigUint;

use crate::error::SparseMerkleTreeError;

/// NET_HEIGHT = HEIGHT -  CANOPY_DEPTH
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexedChangelogEntry<I, const NET_HEIGHT: usize>
where
    I: Clone,
{
    /// Element that was a subject to the change.
    pub element: RawIndexedElement<I>,
    /// Merkle proof of that operation.
    pub proof: [[u8; 32]; NET_HEIGHT],
    /// Index of a changelog entry in `ConcurrentMerkleTree` corresponding to
    /// the same operation.
    pub changelog_index: usize,
}

/// Patch the indexed changelogs.
/// 1. find changelog entries of the same index
/// 2. iterate over entries
///    2.1 if next_value < new_element.value patch element
pub fn patch_indexed_changelogs<const HEIGHT: usize>(
    indexed_changelog_index: usize,
    changelog_index: &mut usize,
    indexed_changelogs: &mut Vec<IndexedChangelogEntry<usize, HEIGHT>>,
    low_element: &mut IndexedElement<usize>,
    new_element: &mut IndexedElement<usize>,
    low_element_next_value: &mut BigUint,
    low_leaf_proof: &mut Vec<[u8; 32]>,
) -> Result<(), SparseMerkleTreeError> {
    // Tests are in program-tests/merkle-tree/tests/indexed_changelog.rs
    let next_indexed_changelog_indices: Vec<usize> = (*indexed_changelogs)
        [indexed_changelog_index..]
        .iter()
        .enumerate()
        .filter_map(|(index, changelog_entry)| {
            if changelog_entry.element.index == low_element.index {
                Some(indexed_changelog_index + index)
            } else {
                None
            }
        })
        .collect();

    let mut new_low_element = None;
    for next_indexed_changelog_index in next_indexed_changelog_indices {
        let changelog_entry = &mut indexed_changelogs[next_indexed_changelog_index];

        let next_element_value = BigUint::from_bytes_be(&changelog_entry.element.next_value);
        if next_element_value < new_element.value {
            // If the next element is lower than the current element, it means
            // that it should become the low element.
            //
            // Save it and break the loop.
            new_low_element = Some(((next_indexed_changelog_index + 1), next_element_value));
            break;
        }

        // Patch the changelog index.
        *changelog_index = changelog_entry.changelog_index + 1;

        // Patch the `next_index` of `new_element`.
        new_element.next_index = changelog_entry.element.next_index;
        // Patch the element.
        low_element.update_from_raw_element(&changelog_entry.element);
        // Patch the next value.
        *low_element_next_value = BigUint::from_bytes_be(&changelog_entry.element.next_value);
        // Patch the proof.
        *low_leaf_proof = changelog_entry.proof.to_vec();
    }

    // If we found a new low element.
    if let Some((new_low_element_changelog_index, new_low_element)) = new_low_element {
        let new_low_element_changelog_entry = &indexed_changelogs[new_low_element_changelog_index];
        *changelog_index = new_low_element_changelog_entry.changelog_index + 1;
        *low_element = IndexedElement {
            index: new_low_element_changelog_entry.element.index,
            value: new_low_element.clone(),
            next_index: new_low_element_changelog_entry.element.next_index,
        };

        *low_leaf_proof = new_low_element_changelog_entry.proof.to_vec();
        new_element.next_index = low_element.next_index;
        if new_low_element_changelog_index == indexed_changelogs.len() - 1 {
            return Ok(());
        }
        // Start the patching process from scratch for the new low element.
        patch_indexed_changelogs(
            new_low_element_changelog_index,
            changelog_index,
            indexed_changelogs,
            low_element,
            new_element,
            low_element_next_value,
            low_leaf_proof,
        )?
    }

    Ok(())
}
