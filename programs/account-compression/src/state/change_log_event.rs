use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
};
use light_concurrent_merkle_tree::changelog::ChangelogEntry;
use light_macros::pubkey;

use crate::errors::AccountCompressionErrorCode;

pub const NOOP_PROGRAM_ID: Pubkey = pubkey!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
pub struct Changelogs {
    pub changelogs: Vec<ChangelogEvent>,
}

/// Event containing the Merkle path of the given
/// [`StateMerkleTree`](account_compression::state::StateMerkleTree)
/// change. Indexers can use this type of events to re-build a non-sparse
/// version of state Merkle tree.
#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
#[repr(C)]
pub enum ChangelogEvent {
    V1(ChangelogEventV1),
}

/// Node of the Merkle path with an index representing the position in a
/// non-sparse Merkle tree.
#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
pub struct PathNode {
    pub node: [u8; 32],
    pub index: u32,
}

/// Version 1 of the [`ChangelogEvent`](account_compression::state::ChangelogEvent).
#[derive(AnchorDeserialize, AnchorSerialize, Debug)]
pub struct ChangelogEventV1 {
    /// Public key of the tree.
    pub id: Pubkey,
    // Merkle paths.
    pub paths: Vec<Vec<PathNode>>,
    /// Number of successful operations on the on-chain tree.
    pub seq: u64,
    /// Changelog event index.
    pub index: u32,
}

impl ChangelogEventV1 {
    pub fn new<const HEIGHT: usize>(
        merkle_tree_account_pubkey: Pubkey,
        changelog_entries: &[ChangelogEntry<HEIGHT>],
        seq: u64,
    ) -> Result<Self> {
        let mut paths = Vec::with_capacity(changelog_entries.len());
        for changelog_entry in changelog_entries.iter() {
            let path_len = changelog_entry.path.len();
            let mut path = Vec::with_capacity(path_len);
            let path_len = u32::try_from(path_len)
                .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?;

            // Add all nodes from the changelog path.
            for (level, node) in changelog_entry.path.iter().enumerate() {
                let level = u32::try_from(level)
                    .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?;
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
            .ok_or(AccountCompressionErrorCode::EventNoChangelogEntry)?
            .index
            .try_into()
            .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?;
        Ok(Self {
            id: merkle_tree_account_pubkey,
            paths,
            seq,
            index,
        })
    }
}

#[inline(never)]
pub fn emit_indexer_event<'info>(
    data: Vec<u8>,
    noop_program: &AccountInfo<'info>,
    signer: &AccountInfo<'info>,
) -> Result<()> {
    if noop_program.key() != NOOP_PROGRAM_ID {
        return err!(AccountCompressionErrorCode::InvalidNoopPubkey);
    }
    let instruction = Instruction {
        program_id: noop_program.key(),
        accounts: vec![],
        data,
    };
    invoke(
        &instruction,
        &[noop_program.to_account_info(), signer.to_account_info()],
    )?;
    Ok(())
}

#[cfg(test)]
mod test {
    use light_concurrent_merkle_tree::ConcurrentMerkleTree;
    use light_hasher::Keccak;
    use spl_account_compression::{events::ChangeLogEventV1, ChangeLogEvent};

    use super::*;

    /// Tests the compatibility of node indexing between our event
    /// implementation and the one from spl-account-compression;
    #[test]
    fn test_changelog_event_v1() {
        const HEIGHT: usize = 2;
        const MAX_CHANGELOG: usize = 8;
        const MAX_ROOTS: usize = 8;
        const CANOPY: usize = 0;

        let pubkey = Pubkey::new_from_array([0u8; 32]);

        // Fill up the Merkle tree with random leaves.
        let mut merkle_tree =
            ConcurrentMerkleTree::<Keccak, HEIGHT>::new(HEIGHT, MAX_CHANGELOG, MAX_ROOTS, CANOPY);
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
            let changelog_entry = merkle_tree.changelog[i].clone();
            let changelog_event =
                ChangelogEventV1::new(pubkey, &[changelog_entry], i as u64).unwrap();

            let spl_changelog_entry = Box::new(spl_merkle_tree.change_logs[i]);
            let spl_changelog_event: Box<ChangeLogEvent> =
                Box::<ChangeLogEvent>::from((spl_changelog_entry, pubkey, i as u64));

            match *spl_changelog_event {
                ChangeLogEvent::V1(ChangeLogEventV1 {
                    id,
                    path,
                    seq,
                    index,
                }) => {
                    assert_eq!(id, changelog_event.id);
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
