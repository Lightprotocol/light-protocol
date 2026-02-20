//! Helper for packing validity proofs into remaining accounts.

use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
pub use light_sdk::instruction::{PackedAddressTreeInfo, PackedStateTreeInfo};
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

#[cfg(test)]
mod tests {
    use light_compressed_account::TreeType;
    use solana_pubkey::Pubkey;

    use super::{pack_proof, pack_proof_for_mints};
    use crate::indexer::{TreeInfo, ValidityProofWithContext};

    fn make_state_v1_tree_info() -> TreeInfo {
        TreeInfo {
            tree_type: TreeType::StateV1,
            tree: Pubkey::new_unique(),
            queue: Pubkey::new_unique(),
            cpi_context: None,
            next_tree_info: None,
        }
    }

    #[test]
    fn test_pack_proof_minimal_valid_proof_no_cpi_context() {
        let program_id = Pubkey::new_unique();
        let proof = ValidityProofWithContext::default();
        let output_tree = make_state_v1_tree_info();

        let result = pack_proof(&program_id, proof, &output_tree, None).unwrap();

        // v2 system accounts (with self_program, no cpi_context):
        //   light_system_program, cpi_signer, registered_program_pda,
        //   account_compression_authority, account_compression_program, system_program = 6
        // + output queue = 1
        // Total = 7
        assert_eq!(
            result.remaining_accounts.len(),
            7,
            "expected 7 remaining accounts without cpi_context"
        );
        assert_eq!(result.state_tree_index, None);
        // system_accounts_offset is 0 because system accounts are prepended
        // at the start of remaining_accounts by add_system_accounts_raw
        assert_eq!(result.system_accounts_offset, 0);
    }

    #[test]
    fn test_pack_proof_with_cpi_context_adds_extra_account() {
        let program_id = Pubkey::new_unique();
        let cpi_context_pubkey = Pubkey::new_unique();
        let proof = ValidityProofWithContext::default();
        let output_tree = make_state_v1_tree_info();

        let result_no_cpi = pack_proof(&program_id, proof.clone(), &output_tree, None).unwrap();
        let result_with_cpi =
            pack_proof(&program_id, proof, &output_tree, Some(cpi_context_pubkey)).unwrap();

        // cpi_context adds one more account
        assert_eq!(
            result_with_cpi.remaining_accounts.len(),
            result_no_cpi.remaining_accounts.len() + 1,
            "cpi_context should add exactly one account"
        );
    }

    #[test]
    fn test_pack_proof_for_mints_adds_state_tree_index() {
        let program_id = Pubkey::new_unique();
        let proof = ValidityProofWithContext::default();
        let output_tree = make_state_v1_tree_info();

        let result = pack_proof_for_mints(&program_id, proof, &output_tree, None).unwrap();

        // state_tree_index must be Some for mint creation path
        assert!(
            result.state_tree_index.is_some(),
            "pack_proof_for_mints should set state_tree_index"
        );
        let idx = result.state_tree_index.unwrap();
        assert!(
            (idx as usize) < result.remaining_accounts.len(),
            "state_tree_index must be a valid index into remaining_accounts"
        );
    }

    #[test]
    fn test_pack_proof_vs_pack_proof_for_mints_output_tree_index_consistent() {
        let program_id = Pubkey::new_unique();
        let proof = ValidityProofWithContext::default();
        let tree = Pubkey::new_unique();
        let queue = Pubkey::new_unique();
        let output_tree = TreeInfo {
            tree_type: TreeType::StateV1,
            tree,
            queue,
            cpi_context: None,
            next_tree_info: None,
        };

        let r1 = pack_proof(&program_id, proof.clone(), &output_tree, None).unwrap();
        let r2 = pack_proof_for_mints(&program_id, proof, &output_tree, None).unwrap();

        // Both should have the same output_tree_index since they use the same output_tree
        assert_eq!(r1.output_tree_index, r2.output_tree_index);

        // pack_proof_for_mints adds exactly one account (the state tree)
        assert_eq!(
            r2.remaining_accounts.len(),
            r1.remaining_accounts.len() + 1,
            "pack_proof_for_mints adds exactly one account (the state tree)"
        );
    }
}
