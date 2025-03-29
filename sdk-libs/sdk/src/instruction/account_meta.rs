use light_compressed_account::compressed_account::{
    CompressedAccountWithMerkleContext, PackedMerkleContext,
};

use crate::{
    error::LightSdkError,
    instruction::{merkle_context::pack_merkle_context, pack_accounts::PackedAccounts},
    BorshDeserialize, BorshSerialize,
};

/// CompressedAccountMeta (context, address, root_index, output_merkle_tree_index)
/// CompressedAccountMetaNoLamportsNoAddress (context, root_index, output_merkle_tree_index)
/// CompressedAccountMetaWithLamportsNoAddress (context, root_index, output_merkle_tree_index)
/// CompressedAccountMetaWithLamports (context, lamports, address, root_index, output_merkle_tree_index)
pub trait CompressedAccountMetaTrait {
    fn get_merkle_context(&self) -> &PackedMerkleContext;
    fn get_lamports(&self) -> Option<u64>;
    fn get_root_index(&self) -> Option<u16>;
    fn get_address(&self) -> Option<[u8; 32]>;
    fn get_output_merkle_tree_index(&self) -> u8;
}

#[derive(Default, Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct CompressedAccountMetaNoLamportsNoAddress {
    pub merkle_context: PackedMerkleContext,
    pub output_merkle_tree_index: u8,
    pub root_index: Option<u16>,
}

impl CompressedAccountMetaTrait for CompressedAccountMetaNoLamportsNoAddress {
    fn get_merkle_context(&self) -> &PackedMerkleContext {
        &self.merkle_context
    }

    fn get_lamports(&self) -> Option<u64> {
        None
    }

    fn get_root_index(&self) -> Option<u16> {
        self.root_index
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        None
    }

    fn get_output_merkle_tree_index(&self) -> u8 {
        self.output_merkle_tree_index
    }
}

impl CompressedAccountMetaNoLamportsNoAddress {
    pub fn from_compressed_account(
        compressed_account: &CompressedAccountWithMerkleContext,
        cpi_accounts: &mut PackedAccounts,
        root_index: Option<u16>,
        output_merkle_tree: &crate::Pubkey,
    ) -> Self {
        let mut merkle_context =
            pack_merkle_context(&compressed_account.merkle_context, cpi_accounts);
        let output_merkle_tree_index = cpi_accounts.insert_or_get(*output_merkle_tree);
        if root_index.is_none() {
            merkle_context.prove_by_index = true;
        }
        CompressedAccountMetaNoLamportsNoAddress {
            merkle_context,
            root_index,
            output_merkle_tree_index,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct CompressedAccountMetaNoAddress {
    pub merkle_context: PackedMerkleContext,
    pub output_merkle_tree_index: u8,
    pub lamports: u64,
    pub root_index: Option<u16>,
}

impl CompressedAccountMetaTrait for CompressedAccountMetaNoAddress {
    fn get_merkle_context(&self) -> &PackedMerkleContext {
        &self.merkle_context
    }

    fn get_lamports(&self) -> Option<u64> {
        Some(self.lamports)
    }

    fn get_root_index(&self) -> Option<u16> {
        self.root_index
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        None
    }

    fn get_output_merkle_tree_index(&self) -> u8 {
        self.output_merkle_tree_index
    }
}

impl CompressedAccountMetaNoAddress {
    pub fn from_compressed_account(
        compressed_account: &CompressedAccountWithMerkleContext,
        cpi_accounts: &mut PackedAccounts,
        root_index: Option<u16>,
        output_merkle_tree: &crate::Pubkey,
    ) -> Self {
        let mut merkle_context =
            pack_merkle_context(&compressed_account.merkle_context, cpi_accounts);

        let output_merkle_tree_index = cpi_accounts.insert_or_get(*output_merkle_tree);
        if root_index.is_none() {
            merkle_context.prove_by_index = true;
        }
        CompressedAccountMetaNoAddress {
            merkle_context,
            root_index,
            output_merkle_tree_index,
            lamports: compressed_account.compressed_account.lamports,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct CompressedAccountMeta {
    /// Merkle tree context.
    pub merkle_context: PackedMerkleContext,
    /// Address.
    pub address: [u8; 32],
    /// Root index.
    pub root_index: Option<u16>,
    pub output_merkle_tree_index: u8,
}

impl CompressedAccountMetaTrait for CompressedAccountMeta {
    fn get_merkle_context(&self) -> &PackedMerkleContext {
        &self.merkle_context
    }

    fn get_lamports(&self) -> Option<u64> {
        None
    }

    fn get_root_index(&self) -> Option<u16> {
        self.root_index
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        Some(self.address)
    }

    fn get_output_merkle_tree_index(&self) -> u8 {
        self.output_merkle_tree_index
    }
}

impl CompressedAccountMeta {
    pub fn from_compressed_account(
        compressed_account: &CompressedAccountWithMerkleContext,
        cpi_accounts: &mut PackedAccounts,
        root_index: Option<u16>,
        output_merkle_tree: &crate::Pubkey,
    ) -> Result<Self, LightSdkError> {
        let mut merkle_context =
            pack_merkle_context(&compressed_account.merkle_context, cpi_accounts);

        let address = compressed_account
            .compressed_account
            .address
            .ok_or(LightSdkError::MissingField("address".to_string()))?;

        let output_merkle_tree_index = cpi_accounts.insert_or_get(*output_merkle_tree);

        if root_index.is_none() {
            merkle_context.prove_by_index = true;
        }
        Ok(CompressedAccountMeta {
            merkle_context,
            address,
            root_index,
            output_merkle_tree_index,
        })
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct CompressedAccountMetaWithLamports {
    /// Merkle tree context.
    pub merkle_context: PackedMerkleContext,
    /// Lamports.
    pub lamports: u64,
    /// Address.
    pub address: [u8; 32],
    /// Root index.
    pub output_merkle_tree_index: u8,
    pub root_index: Option<u16>,
}

impl CompressedAccountMetaTrait for CompressedAccountMetaWithLamports {
    fn get_merkle_context(&self) -> &PackedMerkleContext {
        &self.merkle_context
    }

    fn get_lamports(&self) -> Option<u64> {
        Some(self.lamports)
    }

    fn get_root_index(&self) -> Option<u16> {
        self.root_index
    }

    fn get_address(&self) -> Option<[u8; 32]> {
        Some(self.address)
    }

    fn get_output_merkle_tree_index(&self) -> u8 {
        self.output_merkle_tree_index
    }
}

impl CompressedAccountMetaWithLamports {
    pub fn from_compressed_account(
        compressed_account: &CompressedAccountWithMerkleContext,
        cpi_accounts: &mut PackedAccounts,
        root_index: Option<u16>,
        output_merkle_tree: &crate::Pubkey,
    ) -> Result<Self, LightSdkError> {
        let mut merkle_context =
            pack_merkle_context(&compressed_account.merkle_context, cpi_accounts);

        // Use the address if available, otherwise default
        let address = compressed_account
            .compressed_account
            .address
            .ok_or(LightSdkError::MissingField("address".to_string()))?;
        let output_merkle_tree_index = cpi_accounts.insert_or_get(*output_merkle_tree);
        if root_index.is_none() {
            merkle_context.prove_by_index = true;
        }
        Ok(CompressedAccountMetaWithLamports {
            merkle_context,
            lamports: compressed_account.compressed_account.lamports,
            address,
            root_index,
            output_merkle_tree_index,
        })
    }
}
