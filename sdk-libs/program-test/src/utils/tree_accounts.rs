use light_compressed_account::TreeType;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeAccounts {
    pub merkle_tree: Pubkey,
    pub queue: Pubkey,
    pub is_rolledover: bool,
    pub tree_type: TreeType,
}

impl TreeAccounts {
    pub fn new(
        merkle_tree: Pubkey,
        queue: Pubkey,
        tree_type: TreeType,
        is_rolledover: bool,
    ) -> Self {
        Self {
            merkle_tree,
            queue,
            tree_type,
            is_rolledover,
        }
    }
}
