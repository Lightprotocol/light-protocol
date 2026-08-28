use light_bounded_vec::BoundedVec;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{
    array::IndexedArray, changelog::IndexedChangelogEntry, reference,
    zero_copy::IndexedMerkleTreeZeroCopyMut, IndexedMerkleTree,
};
use num_bigint::BigUint;

const HEIGHT: usize = 26;
const NET_HEIGHT: usize = 16;
const CANOPY_DEPTH: usize = HEIGHT - NET_HEIGHT;
const CHANGELOG_CAPACITY: usize = 16;
const ROOTS_CAPACITY: usize = 16;
const INDEXED_CHANGELOG_CAPACITY: usize = 6;
const ENTRY_SIZE: usize = 600;

fn expected_entry(entry: &IndexedChangelogEntry<usize, NET_HEIGHT>) -> [u8; ENTRY_SIZE] {
    let mut bytes = [0; ENTRY_SIZE];
    for (dst, node) in bytes[..512].chunks_exact_mut(32).zip(entry.proof.iter()) {
        dst.copy_from_slice(node);
    }
    bytes[512..544].copy_from_slice(&entry.element.value);
    bytes[544..576].copy_from_slice(&entry.element.next_value);
    bytes[576..584].copy_from_slice(&entry.element.next_index.to_ne_bytes());
    bytes[584..592].copy_from_slice(&entry.element.index.to_ne_bytes());
    bytes[592..600].copy_from_slice(&entry.changelog_index.to_ne_bytes());
    bytes
}

fn run_all_push_paths(prefill: u8) -> Vec<[u8; ENTRY_SIZE]> {
    let mut bytes = vec![
        prefill;
        IndexedMerkleTree::<Poseidon, usize, HEIGHT, NET_HEIGHT>::size_in_account(
            HEIGHT,
            CHANGELOG_CAPACITY,
            ROOTS_CAPACITY,
            CANOPY_DEPTH,
            INDEXED_CHANGELOG_CAPACITY,
        )
    ];

    let mut tree = IndexedMerkleTreeZeroCopyMut::<Poseidon, usize, HEIGHT, NET_HEIGHT>::
        from_bytes_zero_copy_init(
            &mut bytes,
            HEIGHT,
            CANOPY_DEPTH,
            CHANGELOG_CAPACITY,
            ROOTS_CAPACITY,
            INDEXED_CHANGELOG_CAPACITY,
        )
        .unwrap();
    tree.init().unwrap();
    tree.add_highest_element().unwrap();

    let mut indexed_array = IndexedArray::<Poseidon, usize>::default();
    indexed_array.init().unwrap();
    let mut reference_tree =
        reference::IndexedMerkleTree::<Poseidon, usize>::new(HEIGHT, CANOPY_DEPTH).unwrap();
    reference_tree.init().unwrap();

    let address = BigUint::from(7_u8);
    let (low_element, low_element_next_value) = indexed_array
        .find_low_element_for_nonexistent(&address)
        .unwrap();
    let net_height_proof = reference_tree
        .get_proof_of_leaf(low_element.index(), false)
        .unwrap();
    let mut proof = BoundedVec::with_capacity(HEIGHT);
    for node in net_height_proof.iter() {
        proof.push(*node).unwrap();
    }
    let changelog_index = tree.changelog_index();
    let indexed_changelog_index = tree.indexed_changelog_index();
    tree.update(
        changelog_index,
        indexed_changelog_index,
        address,
        low_element,
        low_element_next_value,
        &mut proof,
    )
    .unwrap();

    assert_eq!(tree.indexed_changelog.len(), INDEXED_CHANGELOG_CAPACITY);
    tree.indexed_changelog
        .as_slice()
        .iter()
        .map(|entry| {
            // SAFETY: The layout assertions pin the entry to 600 bytes and
            // prove that its fields cover the entire value without padding.
            let actual = unsafe {
                std::slice::from_raw_parts(
                    (entry as *const IndexedChangelogEntry<usize, NET_HEIGHT>).cast::<u8>(),
                    ENTRY_SIZE,
                )
            };
            let actual: [u8; ENTRY_SIZE] = actual.try_into().unwrap();
            assert_eq!(actual, expected_entry(entry));
            actual
        })
        .collect()
}

#[test]
fn indexed_changelog_pushes_write_every_byte() {
    assert_eq!(run_all_push_paths(0xFF), run_all_push_paths(0xAA));
}
