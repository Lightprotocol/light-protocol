//! Helper for packing validity proofs into remaining accounts.
//!
//! # Usage
//!
//! ```rust,ignore
//! // 1. Derive addresses & get proof
//! let proof = rpc.get_validity_proof(hashes, addresses, None).await?.value;
//!
//! // 2. Pack into remaining accounts
//! let packed = pack_proof(&program_id, proof.clone(), &output_tree, cpi_context)?;
//!
//! // 3. Build instruction
//! let ix = Instruction {
//!     program_id,
//!     accounts: [my_accounts.to_account_metas(None), packed.remaining_accounts].concat(),
//!     data: MyInstruction {
//!         proof: proof.proof,
//!         address_tree_infos: packed.packed_tree_infos.address_trees,
//!         output_tree_index: packed.output_tree_index,
//!     }.data(),
//! };
//! ```

use light_client::indexer::{TreeInfo, ValidityProofWithContext};
use light_sdk::instruction::{PackedAccounts, SystemAccountMetaConfig};
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;
use thiserror::Error;

pub use light_sdk::instruction::{PackedAddressTreeInfo, PackedStateTreeInfo};

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
    /// Offset where system accounts start. Pass to instruction data if needed.
    pub system_accounts_offset: u8,
}

/// Packs a validity proof into remaining accounts for instruction building.
///
/// Handles all the `PackedAccounts` boilerplate:
/// - Adds system accounts (with optional CPI context)
/// - Inserts output tree queue
/// - Packs tree infos from proof
///
/// # Arguments
/// - `program_id`: Your program's ID
/// - `proof`: Validity proof from `get_validity_proof()`
/// - `output_tree`: Tree info for writing outputs (from `get_random_state_tree_info()`)
/// - `cpi_context`: CPI context pubkey. Required when mixing PDAs with tokens in same tx.
///   Get from `tree_info.cpi_context`.
///
/// # Returns
/// `PackedProofResult` containing remaining accounts and indices for instruction data.
pub fn pack_proof(
    program_id: &Pubkey,
    proof: ValidityProofWithContext,
    output_tree: &TreeInfo,
    cpi_context: Option<Pubkey>,
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

    let client_packed_tree_infos = proof.pack_tree_infos(&mut packed);
    let (remaining_accounts, system_offset, _) = packed.to_account_metas();

    // Convert from light_client's types to our local types
    let packed_tree_infos = PackedTreeInfos {
        state_trees: client_packed_tree_infos.state_trees.map(|st| PackedStateTreeInfos {
            packed_tree_infos: st.packed_tree_infos,
            output_tree_index: st.output_tree_index,
        }),
        address_trees: client_packed_tree_infos.address_trees,
    };

    Ok(PackedProofResult {
        remaining_accounts,
        packed_tree_infos,
        output_tree_index,
        system_accounts_offset: system_offset as u8,
    })
}
