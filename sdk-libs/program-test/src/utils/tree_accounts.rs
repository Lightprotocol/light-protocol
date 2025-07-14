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

    pub fn to_forester_utils(&self) -> forester_utils::forester_epoch::TreeAccounts {
        forester_utils::forester_epoch::TreeAccounts::new(
            self.merkle_tree,
            self.queue,
            self.tree_type,
            self.is_rolledover,
        )
    }
}