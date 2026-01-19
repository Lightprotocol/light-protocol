//! Helper for getting validity proofs for creating new rent free accounts.
//! Programs must pass this to light accounts that they initialize.

use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
use light_sdk::instruction::PackedAddressTreeInfo;
use light_token_interface::MINT_ADDRESS_TREE;
use light_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;
use thiserror::Error;

use super::pack::{pack_proof, pack_proof_for_mints, PackError};
use crate::{
    indexer::{AddressWithTree, Indexer, IndexerError, ValidityProofWithContext},
    rpc::{Rpc, RpcError},
};

/// Error type for create accounts proof operations.
#[derive(Debug, Error)]
pub enum CreateAccountsProofError {
    #[error("Inputs cannot be empty")]
    EmptyInputs,

    #[error("Indexer error: {0}")]
    Indexer(#[from] IndexerError),

    #[error("RPC error: {0}")]
    Rpc(RpcError),

    #[error("Pack error: {0}")]
    Pack(#[from] PackError),
}

/// Input for creating new accounts.
/// `program_id` from main fn is used as default owner for `Pda` variant.
#[derive(Clone, Debug)]
pub enum CreateAccountsProofInput {
    /// PDA owned by the calling program (uses program_id from main fn)
    Pda(Pubkey),
    /// PDA with explicit owner (for cross-program accounts)
    PdaWithOwner { pda: Pubkey, owner: Pubkey },
    /// Mint (always uses LIGHT_TOKEN_PROGRAM_ID internally)
    Mint(Pubkey),
}

impl CreateAccountsProofInput {
    /// Standard PDA owned by calling program.
    /// Address derived: `derive_address(&pda, &tree, &program_id)`
    pub fn pda(pda: Pubkey) -> Self {
        Self::Pda(pda)
    }

    /// PDA with explicit owner (rare: cross-program accounts).
    /// Address derived: `derive_address(&pda, &tree, &owner)`
    pub fn pda_with_owner(pda: Pubkey, owner: Pubkey) -> Self {
        Self::PdaWithOwner { pda, owner }
    }

    /// Compressed mint (Mint).
    /// Address derived: `derive_mint_compressed_address(&mint_signer, &tree)`
    pub fn mint(mint_signer: Pubkey) -> Self {
        Self::Mint(mint_signer)
    }

    /// Derive the cold address (mints always use MINT_ADDRESS_TREE).
    fn derive_address(&self, address_tree: &Pubkey, program_id: &Pubkey) -> [u8; 32] {
        match self {
            Self::Pda(pda) => light_compressed_account::address::derive_address(
                &pda.to_bytes(),
                &address_tree.to_bytes(),
                &program_id.to_bytes(),
            ),
            Self::PdaWithOwner { pda, owner } => light_compressed_account::address::derive_address(
                &pda.to_bytes(),
                &address_tree.to_bytes(),
                &owner.to_bytes(),
            ),
            // Mints always use MINT_ADDRESS_TREE regardless of passed tree
            Self::Mint(signer) => {
                derive_mint_compressed_address(signer, &Pubkey::new_from_array(MINT_ADDRESS_TREE))
            }
        }
    }

    /// Get the address tree for this input type.
    fn address_tree(&self, default_tree: &Pubkey) -> Pubkey {
        match self {
            Self::Pda(_) | Self::PdaWithOwner { .. } => *default_tree,
            // Mints always use MINT_ADDRESS_TREE
            Self::Mint(_) => Pubkey::new_from_array(MINT_ADDRESS_TREE),
        }
    }
}

pub use light_compressible::CreateAccountsProof;

/// Result of `get_create_accounts_proof`.
pub struct CreateAccountsProofResult {
    /// Proof data to include in instruction data.
    pub create_accounts_proof: CreateAccountsProof,
    /// Remaining accounts to append to instruction accounts.
    pub remaining_accounts: Vec<AccountMeta>,
}

/// Gets validity proof for creating new cold accounts (INIT flow).
pub async fn get_create_accounts_proof<R: Rpc + Indexer>(
    rpc: &R,
    program_id: &Pubkey,
    inputs: Vec<CreateAccountsProofInput>,
) -> Result<CreateAccountsProofResult, CreateAccountsProofError> {
    if inputs.is_empty() {
        // Token-only instructions: no addresses to derive, but still need tree info
        let state_tree_info = rpc
            .get_random_state_tree_info()
            .map_err(CreateAccountsProofError::Rpc)?;

        // Pack system accounts with empty proof
        let packed = pack_proof(
            program_id,
            ValidityProofWithContext::default(),
            &state_tree_info,
            None, // No CPI context needed for token-only
        )?;

        return Ok(CreateAccountsProofResult {
            create_accounts_proof: CreateAccountsProof {
                proof: ValidityProof::default(),
                address_tree_info: PackedAddressTreeInfo::default(),
                output_state_tree_index: packed.output_tree_index,
                state_tree_index: None,
            },
            remaining_accounts: packed.remaining_accounts,
        });
    }

    // 1. Get address tree (opinionated: always V2)
    let address_tree = rpc.get_address_tree_v2();
    let address_tree_pubkey = address_tree.tree;

    // 2. Derive all cold addresses
    let derived_addresses: Vec<[u8; 32]> = inputs
        .iter()
        .map(|input| input.derive_address(&address_tree_pubkey, program_id))
        .collect();

    // 3. Build AddressWithTree for each
    let addresses_with_trees: Vec<AddressWithTree> = inputs
        .iter()
        .zip(derived_addresses.iter())
        .map(|(input, &address)| AddressWithTree {
            address,
            tree: input.address_tree(&address_tree_pubkey),
        })
        .collect();

    // 4. Get validity proof (empty hashes = INIT flow)
    let validity_proof = rpc
        .get_validity_proof(vec![], addresses_with_trees, None)
        .await?
        .value;

    // 5. Get output state tree
    let state_tree_info = rpc
        .get_random_state_tree_info()
        .map_err(CreateAccountsProofError::Rpc)?;

    // 6. Determine CPI context (needed for mints)
    let has_mints = inputs
        .iter()
        .any(|i| matches!(i, CreateAccountsProofInput::Mint(_)));
    let cpi_context = if has_mints {
        state_tree_info.cpi_context
    } else {
        None
    };

    // 7. Pack proof (use mint-aware packing if mints are present)
    let packed = if has_mints {
        pack_proof_for_mints(
            program_id,
            validity_proof.clone(),
            &state_tree_info,
            cpi_context,
        )?
    } else {
        pack_proof(
            program_id,
            validity_proof.clone(),
            &state_tree_info,
            cpi_context,
        )?
    };

    // All addresses use the same tree, so just take the first packed info
    let address_tree_info = packed
        .packed_tree_infos
        .address_trees
        .first()
        .copied()
        .ok_or(CreateAccountsProofError::EmptyInputs)?;

    Ok(CreateAccountsProofResult {
        create_accounts_proof: CreateAccountsProof {
            proof: validity_proof.proof,
            address_tree_info,
            output_state_tree_index: packed.output_tree_index,
            state_tree_index: packed.state_tree_index,
        },
        remaining_accounts: packed.remaining_accounts,
    })
}
