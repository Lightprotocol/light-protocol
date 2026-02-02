//! Helper for packing validity proofs into remaining accounts.

pub use light_sdk::instruction::{PackedAddressTreeInfo, PackedStateTreeInfo};
use light_sdk::{
    instruction::{PackedAccounts, SystemAccountMetaConfig},
    PackedAccountsExt,
};
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;
use thiserror::Error;

use crate::indexer::{TreeInfo, ValidityProofWithContext};

#[derive(Debug, Error)]
pub enum PackError {
    #[error("Failed to add system accounts: {0}")]
    SystemAccounts(#[from] light_sdk::error::LightSdkError),
}

/// Packed state tree infos from validity proof.
#[derive(Clone, Default, Debug)]
pub struct PackedStateTreeInfos {
    pub packed_tree_infos: Vec<PackedStateTreeInfo>,
    pub output_tree_index: u8,
}

/// Packed tree infos from validity proof.
#[derive(Clone, Default, Debug)]
pub struct PackedTreeInfos {
    pub state_trees: Option<PackedStateTreeInfos>,
    pub address_trees: Vec<PackedAddressTreeInfo>,
}

/// Result of packing a validity proof into remaining accounts.
pub struct PackedProofResult {
    /// Remaining accounts to append to your instruction's accounts.
    pub remaining_accounts: Vec<AccountMeta>,
    /// Packed tree infos from the proof. Use `.address_trees` or `.state_trees` as needed.
    pub packed_tree_infos: PackedTreeInfos,
    /// Index of output tree in remaining accounts. Pass to instruction data.
    pub output_tree_index: u8,
    /// Index of state merkle tree in remaining accounts (when included for mint creation).
    pub state_tree_index: Option<u8>,
    /// Offset where system accounts start. Pass to instruction data if needed.
    pub system_accounts_offset: u8,
}

/// Packs a validity proof into remaining accounts for instruction building.
pub fn pack_proof(
    program_id: &Pubkey,
    proof: ValidityProofWithContext,
    output_tree: &TreeInfo,
    cpi_context: Option<Pubkey>,
) -> Result<PackedProofResult, PackError> {
    pack_proof_internal(program_id, proof, output_tree, cpi_context, false)
}

/// Same as `pack_proof` but also includes state merkle tree for mint creation.
pub fn pack_proof_for_mints(
    program_id: &Pubkey,
    proof: ValidityProofWithContext,
    output_tree: &TreeInfo,
    cpi_context: Option<Pubkey>,
) -> Result<PackedProofResult, PackError> {
    pack_proof_internal(program_id, proof, output_tree, cpi_context, true)
}

fn pack_proof_internal(
    program_id: &Pubkey,
    proof: ValidityProofWithContext,
    output_tree: &TreeInfo,
    cpi_context: Option<Pubkey>,
    include_state_tree: bool,
) -> Result<PackedProofResult, PackError> {
    let mut packed = PackedAccounts::default();

    let system_config = match cpi_context {
        Some(ctx) => SystemAccountMetaConfig::new_with_cpi_context(*program_id, ctx),
        None => SystemAccountMetaConfig::new(*program_id),
    };
    packed.add_system_accounts_v2(system_config)?;

    let output_queue = output_tree
        .next_tree_info
        .as_ref()
        .map(|n| n.queue)
        .unwrap_or(output_tree.queue);
    let output_tree_index = packed.insert_or_get(output_queue);

    // For mint creation: pack address tree first (index 1), then state tree.
    let (client_packed_tree_infos, state_tree_index) = if include_state_tree {
        // Pack tree infos first to ensure address tree is at index 1
        let tree_infos = proof.pack_tree_infos(&mut packed);

        // Then add state tree (will be after address tree)
        let state_tree = output_tree
            .next_tree_info
            .as_ref()
            .map(|n| n.tree)
            .unwrap_or(output_tree.tree);
        let state_idx = packed.insert_or_get(state_tree);

        (tree_infos, Some(state_idx))
    } else {
        let tree_infos = proof.pack_tree_infos(&mut packed);
        (tree_infos, None)
    };
    let (remaining_accounts, system_offset, _) = packed.to_account_metas();

    // Convert from light_client's types to our local types
    let packed_tree_infos = PackedTreeInfos {
        state_trees: client_packed_tree_infos
            .state_trees
            .map(|st| PackedStateTreeInfos {
                packed_tree_infos: st.packed_tree_infos,
                output_tree_index: st.output_tree_index,
            }),
        address_trees: client_packed_tree_infos.address_trees,
    };

    Ok(PackedProofResult {
        remaining_accounts,
        packed_tree_infos,
        output_tree_index,
        state_tree_index,
        system_accounts_offset: system_offset as u8,
    })
}
