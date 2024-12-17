use bytemuck::{Pod, Zeroable};
use solana_program::pubkey::Pubkey;

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};

#[repr(C)]
#[derive(
    AnchorDeserialize, AnchorSerialize, Debug, PartialEq, Default, Pod, Zeroable, Clone, Copy,
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
