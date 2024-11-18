#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use solana_program::pubkey::Pubkey;

#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq, Default)]
pub struct MerkleTreeMetadata {
    pub access_metadata: AccessMetadata,
    pub rollover_metadata: RolloverMetadata,
    // Queue associated with this Merkle tree.
    pub associated_queue: Pubkey,
    // Next Merkle tree to be used after rollover.
    pub next_merkle_tree: Pubkey,
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq, Default)]
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

#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq, Default)]
pub struct RolloverMetadata {
    /// Unique index.
    pub index: u64,
    /// This fee is used for rent for the next account.
    /// It accumulates in the account so that once the corresponding Merkle tree account is full it can be rolled over
    pub rollover_fee: u64,
    /// The threshold in percentage points when the account should be rolled over (95 corresponds to 95% filled).
    pub rollover_threshold: u64,
    /// Tip for maintaining the account.
    pub network_fee: u64,
    /// The slot when the account was rolled over, a rolled over account should not be written to.
    pub rolledover_slot: u64,
    /// If current slot is greater than rolledover_slot + close_threshold and
    /// the account is empty it can be closed. No 'close' functionality has been
    /// implemented yet.
    pub close_threshold: u64,
    /// Placeholder for bytes of additional accounts which are tied to the
    /// Merkle trees operation and need to be rolled over as well.
    pub additional_bytes: u64,
}
