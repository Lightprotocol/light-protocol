use crate::{instruction::tree_info::PackedStateTreeInfo, BorshDeserialize, BorshSerialize};

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

#[derive(Default, Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
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

#[derive(Default, Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
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

#[derive(Default, Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
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

#[derive(Default, Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
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

#[derive(Default, Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct CompressedAccountMetaClose {
    /// State Merkle tree context.
    pub tree_info: PackedStateTreeInfo,
    /// Address.
    pub address: [u8; 32],
}

impl CompressedAccountMetaTrait for CompressedAccountMetaClose {
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