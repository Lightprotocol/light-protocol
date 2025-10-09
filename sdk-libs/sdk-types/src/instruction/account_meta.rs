use super::tree_info::PackedStateTreeInfo;
use crate::{AnchorDeserialize, AnchorSerialize};

/// CompressedAccountMeta (context, address, root_index, output_state_tree_index)
/// CompressedAccountMetaNoLamportsNoAddress (context, root_index, output_state_tree_index)
/// CompressedAccountMetaWithLamportsNoAddress (context, root_index, output_state_tree_index)
/// CompressedAccountMetaWithLamports (context, lamports, address, root_index, output_state_tree_index)
pub trait CompressedAccountMetaTrait {
    fn get_tree_info(&self) -> &PackedStateTreeInfo;
    fn get_lamports(&self) -> Option<u64>;
    fn get_root_index(&self) -> Option<u16>;
    fn get_address(&self) -> Option<[u8; 32]>;
    fn get_output_state_tree_index(&self) -> Option<u8>;
}

#[derive(Default, Debug, Clone, Copy, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccountMetaNoLamportsNoAddress {
    pub tree_info: PackedStateTreeInfo,
    pub output_state_tree_index: u8,
}

impl CompressedAccountMetaTrait for CompressedAccountMetaNoLamportsNoAddress {
    fn get_tree_info(&self) -> &PackedStateTreeInfo {
        &self.tree_info
    }

    fn get_lamports(&self) -> Option<u64> {
        None
    }

    fn get_root_index(&self) -> Option<u16> {
        if self.tree_info.prove_by_index {
            None
        } else {
            Some(self.tree_info.root_index)
        }
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        None
    }

    fn get_output_state_tree_index(&self) -> Option<u8> {
        Some(self.output_state_tree_index)
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccountMetaNoAddress {
    pub tree_info: PackedStateTreeInfo,
    pub output_state_tree_index: u8,
    pub lamports: u64,
}

impl CompressedAccountMetaTrait for CompressedAccountMetaNoAddress {
    fn get_tree_info(&self) -> &PackedStateTreeInfo {
        &self.tree_info
    }

    fn get_lamports(&self) -> Option<u64> {
        Some(self.lamports)
    }

    fn get_root_index(&self) -> Option<u16> {
        if self.tree_info.prove_by_index {
            None
        } else {
            Some(self.tree_info.root_index)
        }
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        None
    }

    fn get_output_state_tree_index(&self) -> Option<u8> {
        Some(self.output_state_tree_index)
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccountMeta {
    /// Merkle tree context.
    pub tree_info: PackedStateTreeInfo,
    /// Address.
    pub address: [u8; 32],
    /// Output merkle tree index.
    pub output_state_tree_index: u8,
}

impl CompressedAccountMetaTrait for CompressedAccountMeta {
    fn get_tree_info(&self) -> &PackedStateTreeInfo {
        &self.tree_info
    }

    fn get_lamports(&self) -> Option<u64> {
        None
    }

    fn get_root_index(&self) -> Option<u16> {
        if self.tree_info.prove_by_index {
            None
        } else {
            Some(self.tree_info.root_index)
        }
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        Some(self.address)
    }

    fn get_output_state_tree_index(&self) -> Option<u8> {
        Some(self.output_state_tree_index)
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccountMetaInitIfNeeded {
    /// Initialize account.
    /// False if account is currently initialized.
    pub init: bool,
    /// Account is initialized and the address is created in the same intruction.
    /// False if account is currently initialized.
    pub with_new_adress: bool,
    /// Output merkle tree index.
    pub output_state_tree_index: u8,
    /// Address.
    pub address: [u8; 32],
    /// Merkle tree context.
    pub tree_info: Option<PackedStateTreeInfo>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccountMetaWithLamports {
    /// Merkle tree context.
    pub tree_info: PackedStateTreeInfo,
    /// Lamports.
    pub lamports: u64,
    /// Address.
    pub address: [u8; 32],
    /// Root index.
    pub output_state_tree_index: u8,
}

impl CompressedAccountMetaTrait for CompressedAccountMetaWithLamports {
    fn get_tree_info(&self) -> &PackedStateTreeInfo {
        &self.tree_info
    }

    fn get_lamports(&self) -> Option<u64> {
        Some(self.lamports)
    }

    fn get_root_index(&self) -> Option<u16> {
        if self.tree_info.prove_by_index {
            None
        } else {
            Some(self.tree_info.root_index)
        }
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        Some(self.address)
    }

    fn get_output_state_tree_index(&self) -> Option<u8> {
        Some(self.output_state_tree_index)
    }
}
pub type CompressedAccountMetaBurn = CompressedAccountMetaReadOnly;

#[derive(Default, Debug, Clone, Copy, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedAccountMetaReadOnly {
    /// State Merkle tree context.
    pub tree_info: PackedStateTreeInfo,
    /// Address.
    pub address: [u8; 32],
}

impl CompressedAccountMetaTrait for CompressedAccountMetaReadOnly {
    fn get_tree_info(&self) -> &PackedStateTreeInfo {
        &self.tree_info
    }

    fn get_lamports(&self) -> Option<u64> {
        None
    }

    fn get_root_index(&self) -> Option<u16> {
        if self.tree_info.prove_by_index {
            None
        } else {
            Some(self.tree_info.root_index)
        }
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        Some(self.address)
    }

    fn get_output_state_tree_index(&self) -> Option<u8> {
        None
    }
}
