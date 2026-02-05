use light_account::PackedAccounts;
use light_compressed_account::TreeType;
use solana_pubkey::Pubkey;

use super::super::{
    base58::{decode_base58_option_to_pubkey, decode_base58_to_fixed_array},
    IndexerError,
};

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct NextTreeInfo {
    pub cpi_context: Option<Pubkey>,
    pub queue: Pubkey,
    pub tree: Pubkey,
    pub tree_type: TreeType,
}

impl NextTreeInfo {
    /// Get the index of the output tree in the packed accounts.
    /// For StateV1, it returns the index of the tree account.
    /// For StateV2, it returns the index of the queue account.
    /// (For V2 trees new state is inserted into the output queue.
    /// The forester updates the tree from the queue asynchronously.)
    pub fn pack_output_tree_index(
        &self,
        packed_accounts: &mut PackedAccounts,
    ) -> Result<u8, IndexerError> {
        match self.tree_type {
            TreeType::StateV1 => Ok(packed_accounts.insert_or_get(self.tree)),
            TreeType::StateV2 => Ok(packed_accounts.insert_or_get(self.queue)),
            _ => Err(IndexerError::InvalidPackTreeType),
        }
    }
    pub fn from_api_model(
        value: &photon_api::models::TreeContextInfo,
    ) -> Result<Self, IndexerError> {
        Ok(Self {
            tree_type: TreeType::from(value.tree_type as u64),
            tree: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.tree)?),
            queue: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.queue)?),
            cpi_context: decode_base58_option_to_pubkey(&value.cpi_context)?,
        })
    }
}

impl TryFrom<&photon_api::models::TreeContextInfo> for NextTreeInfo {
    type Error = IndexerError;

    fn try_from(value: &photon_api::models::TreeContextInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            tree_type: TreeType::from(value.tree_type as u64),
            tree: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.tree)?),
            queue: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.queue)?),
            cpi_context: decode_base58_option_to_pubkey(&value.cpi_context)?,
        })
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct TreeInfo {
    pub cpi_context: Option<Pubkey>,
    pub next_tree_info: Option<NextTreeInfo>,
    pub queue: Pubkey,
    pub tree: Pubkey,
    pub tree_type: TreeType,
}

impl TreeInfo {
    /// Get the index of the output tree in the packed accounts.
    /// For StateV1, it returns the index of the tree account.
    /// For StateV2, it returns the index of the queue account.
    /// (For V2 trees new state is inserted into the output queue.
    /// The forester updates the tree from the queue asynchronously.)
    pub fn pack_output_tree_index(
        &self,
        packed_accounts: &mut PackedAccounts,
    ) -> Result<u8, IndexerError> {
        match self.tree_type {
            TreeType::StateV1 => Ok(packed_accounts.insert_or_get(self.tree)),
            TreeType::StateV2 => Ok(packed_accounts.insert_or_get(self.queue)),
            _ => Err(IndexerError::InvalidPackTreeType),
        }
    }

    pub fn get_output_pubkey(&self) -> Result<Pubkey, IndexerError> {
        match self.tree_type {
            TreeType::StateV1 => Ok(self.tree),
            TreeType::StateV2 => Ok(self.queue),
            _ => Err(IndexerError::InvalidPackTreeType),
        }
    }

    pub fn from_api_model(
        value: &photon_api::models::MerkleContextV2,
    ) -> Result<Self, IndexerError> {
        Ok(Self {
            tree_type: TreeType::from(value.tree_type as u64),
            tree: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.tree)?),
            queue: Pubkey::new_from_array(decode_base58_to_fixed_array(&value.queue)?),
            cpi_context: decode_base58_option_to_pubkey(&value.cpi_context)?,
            next_tree_info: value
                .next_tree_context
                .as_ref()
                .map(|tree_info| NextTreeInfo::from_api_model(tree_info.as_ref()))
                .transpose()?,
        })
    }

    pub fn to_light_merkle_context(
        &self,
        leaf_index: u32,
        prove_by_index: bool,
    ) -> light_compressed_account::compressed_account::MerkleContext {
        use light_compressed_account::Pubkey;
        light_compressed_account::compressed_account::MerkleContext {
            merkle_tree_pubkey: Pubkey::new_from_array(self.tree.to_bytes()),
            queue_pubkey: Pubkey::new_from_array(self.queue.to_bytes()),
            leaf_index,
            tree_type: self.tree_type,
            prove_by_index,
        }
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct StateMerkleTreeAccounts {
    pub merkle_tree: Pubkey,
    pub nullifier_queue: Pubkey,
    pub cpi_context: Pubkey,
    pub tree_type: TreeType,
}

#[allow(clippy::from_over_into)]
impl Into<TreeInfo> for StateMerkleTreeAccounts {
    fn into(self) -> TreeInfo {
        TreeInfo {
            tree: self.merkle_tree,
            queue: self.nullifier_queue,
            cpi_context: Some(self.cpi_context),
            tree_type: self.tree_type,
            next_tree_info: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AddressMerkleTreeAccounts {
    pub merkle_tree: Pubkey,
    pub queue: Pubkey,
}
