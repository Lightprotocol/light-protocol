use bytemuck::{Pod, Zeroable};
use light_compressed_account::pubkey::Pubkey;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::{AnchorDeserialize, AnchorSerialize};

#[repr(C)]
#[derive(
    AnchorDeserialize,
    AnchorSerialize,
    Debug,
    PartialEq,
    Default,
    Pod,
    Zeroable,
    Clone,
    Copy,
    FromBytes,
    IntoBytes,
    KnownLayout,
    Immutable,
)]
pub struct AccessMetadata {
    /// Owner of the Merkle tree.
    pub owner: Pubkey,
    /// Program owner of the Merkle tree. This will be used for program owned Merkle trees.
    pub program_owner: Pubkey,
    /// Optional privileged forester pubkey, can be set for custom Merkle trees
    /// without a network fee. Merkle trees without network fees are not
    /// forested by light foresters. The variable is not used in the account
    /// compression program but the registry program. The registry program
    /// implements access control to prevent contention during forester. The
    /// forester pubkey specified in this struct can bypass contention checks.
    pub forester: Pubkey,
}

impl AccessMetadata {
    pub fn new(owner: Pubkey, program_owner: Option<Pubkey>, forester: Option<Pubkey>) -> Self {
        Self {
            owner,
            program_owner: program_owner.unwrap_or_default(),
            forester: forester.unwrap_or_default(),
        }
    }
}

#[test]
fn test_new() {
    let owner = Pubkey::new_unique();
    let program_owner = Pubkey::new_unique();
    let forester = Pubkey::new_unique();
    let access_metadata = AccessMetadata::new(owner, Some(program_owner), Some(forester));
    assert_eq!(access_metadata.owner, owner);
    assert_eq!(access_metadata.program_owner, program_owner);
    assert_eq!(access_metadata.forester, forester);

    // With no program owner and forester
    let access_metadata = AccessMetadata::new(owner, None, None);
    assert_eq!(access_metadata.owner, owner);
    assert_eq!(access_metadata.program_owner, Pubkey::default());
    assert_eq!(access_metadata.forester, Pubkey::default());
}
