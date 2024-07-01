use anchor_lang::prelude::*;

#[account(zero_copy)]
#[derive(AnchorDeserialize, Debug, PartialEq, Default)]
pub struct AccessMetadata {
    /// Owner of the Merkle tree.
    pub owner: Pubkey,
    /// Program owner of the Merkle tree. This will be used for program owned Merkle trees.
    pub program_owner: Pubkey,
}

impl AccessMetadata {
    pub fn new(owner: Pubkey, program_owner: Option<Pubkey>) -> Self {
        Self {
            owner,
            program_owner: program_owner.unwrap_or_default(),
        }
    }
}
