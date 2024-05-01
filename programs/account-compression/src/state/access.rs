use anchor_lang::prelude::*;

#[account(zero_copy)]
#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq)]
pub struct AccessMetadata {
    /// Owner of the Merkle tree.
    pub owner: Pubkey,
    /// Delegate of the Merkle tree. This will be used for program owned Merkle trees.
    pub delegate: Pubkey,
}

impl AccessMetadata {
    pub fn new(owner: Pubkey, delegate: Option<Pubkey>) -> Self {
        Self {
            owner,
            delegate: delegate.unwrap_or_default(),
        }
    }
}
