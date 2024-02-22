use borsh::{BorshDeserialize, BorshSerialize};
use light_concurrent_merkle_tree::changelog::ChangelogEntry;

pub mod errors;

use errors::EventError;

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Changelogs {
    pub changelogs: Vec<ChangelogEvent>,
}

/// Event containing the Merkle path of the given
/// [`StateMerkleTree`](light_merkle_tree_program::state::StateMerkleTree)
/// change. Indexers can use this type of events to re-build a non-sparse
/// version of state Merkle tree.
#[derive(BorshDeserialize, BorshSerialize, Debug)]
#[repr(C)]
pub enum ChangelogEvent {
    V1(ChangelogEventV1),
}

/// Node of the Merkle path with an index representing the position in a
/// non-sparse Merkle tree.
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct PathNode {
    pub node: [u8; 32],
    pub index: u32,
}

/// Version 1 of the [`ChangelogEvent`](light_merkle_tree_program::state::ChangelogEvent).
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct ChangelogEventV1 {
    /// Public key of the tree.
    pub id: [u8; 32],
    // Merkle paths.
    pub paths: Vec<Vec<PathNode>>,
    /// Number of successful operations on the on-chain tree.
    pub seq: u64,
    /// Changelog event index.
    pub index: u32,
}

impl ChangelogEventV1 {
    pub fn new<const HEIGHT: usize>(
        merkle_tree_account_pubkey: [u8; 32],
        changelog_entries: Vec<ChangelogEntry<HEIGHT>>,
        seq: u64,
    ) -> Result<Self, EventError> {
        let mut paths = Vec::with_capacity(changelog_entries.len());
        for changelog_entry in changelog_entries.iter() {
            let path_len = changelog_entry.path.len();
            let mut path = Vec::with_capacity(path_len);
            let path_len = u32::try_from(path_len).map_err(|_| EventError::IntegerOverflow)?;

            // Add all nodes from the changelog path.
            for (level, node) in changelog_entry.path.iter().enumerate() {
                let level = u32::try_from(level).map_err(|_| EventError::IntegerOverflow)?;
                let index = (1 << (path_len - level)) + (changelog_entry.index as u32 >> level);
                path.push(PathNode {
                    node: node.to_owned(),
                    index,
                });
            }

            // Add root.
            path.push(PathNode {
                node: changelog_entry.root,
                index: 1,
            });

            paths.push(path);
        }

        // NOTE(vadorovsky): So far we are using the index of the first changelog
        // entry as the index of the event. This makes the most sense with
        // regards to keeping compatibility with spl-account-compression.
        // However, we might need to change that if if some other way of infering
        // indexes makes more sense from indexer's point of view.
        //
        // Currently, indexers can use the `seq` value to ensure that no events
        // were lost.
        let index: u32 = changelog_entries
            .first()
            .ok_or(EventError::EventNoChangelogEntry)?
            .index
            .try_into()
            .map_err(|_| EventError::IntegerOverflow)?;
        Ok(Self {
            id: merkle_tree_account_pubkey,
            paths,
            seq,
            index,
        })
    }
}

#[cfg(test)]
mod test {
    use light_concurrent_merkle_tree::{light_hasher::Keccak, ConcurrentMerkleTree};
    use solana_program::pubkey::Pubkey;
    use spl_account_compression::{events::ChangeLogEventV1, ChangeLogEvent};

    use super::*;

    /// Tests the compatibility of node indexing between our event
    /// implementation and the one from spl-account-compression;
    #[test]
    fn test_changelog_event_v1() {
        const HEIGHT: usize = 2;
        const MAX_CHANGELOG: usize = 8;
        const MAX_ROOTS: usize = 8;

        let pubkey = [0u8; 32];

        // Fill up the Merkle tree with random leaves.
        // let mut merkle_tree = MerkleTree::<Poseidon, HEIGHT, ROOTS>::new().unwrap();
        let mut merkle_tree =
            ConcurrentMerkleTree::<Keccak, HEIGHT, MAX_CHANGELOG, MAX_ROOTS>::default();
        merkle_tree.init().unwrap();
        let mut spl_merkle_tree =
            spl_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree::<
                HEIGHT,
                MAX_CHANGELOG,
            >::new();
        spl_merkle_tree.initialize().unwrap();

        let leaves = 1 << HEIGHT;

        for i in 0..leaves {
            merkle_tree.append(&[(i + 1) as u8; 32]).unwrap();
            spl_merkle_tree.append([(i + 1) as u8; 32]).unwrap();
        }

        for i in 0..leaves {
            let changelog_entry = merkle_tree.changelog[i];
            let changelog_event =
                ChangelogEventV1::new(pubkey, vec![changelog_entry], i as u64).unwrap();

            let spl_changelog_entry = Box::new(spl_merkle_tree.change_logs[i]);
            let spl_changelog_event: Box<ChangeLogEvent> = Box::<ChangeLogEvent>::from((
                spl_changelog_entry,
                Pubkey::new_from_array(pubkey),
                i as u64,
            ));

            match *spl_changelog_event {
                ChangeLogEvent::V1(ChangeLogEventV1 {
                    id,
                    path,
                    seq,
                    index,
                }) => {
                    assert_eq!(id.to_bytes(), changelog_event.id);
                    assert_eq!(path.len(), changelog_event.paths[0].len());
                    for j in 0..HEIGHT {
                        assert_eq!(path[j].node, changelog_event.paths[0][j].node);
                        assert_eq!(path[j].index, changelog_event.paths[0][j].index);
                    }
                    assert_eq!(seq, changelog_event.seq);
                    assert_eq!(index, changelog_event.index);
                }
            }
        }
    }
}
