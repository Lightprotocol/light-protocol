//! Decompress compressed CMint accounts.
//!
//! This module provides client-side functionality to decompress compressed
//! CMint accounts (mints created via `#[compressible]` macro that have been
//! auto-compressed by forester).
//!
//! DecompressMint is permissionless - any fee_payer can decompress any
//! compressed mint. The mint_seed_pubkey is required for PDA derivation.
//!
//! Three APIs are provided:
//! - `decompress_mint`: Simple async API (fetches state + proof internally)
//! - `build_decompress_mint`: Sync, caller provides pre-fetched state + proof
//! - `decompress_cmint`: High-perf wrapper (takes MintInterface, fetches proof internally)

use borsh::BorshDeserialize;
use light_client::indexer::{CompressedAccount, Indexer, IndexerError, ValidityProofWithContext};
use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
use light_token_interface::{
    instructions::mint_action::{CompressedMintInstructionData, CompressedMintWithContext},
    state::CompressedMint,
    CMINT_ADDRESS_TREE,
};
use light_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address;
use light_token_sdk::token::{find_mint_address, DecompressMint};
use solana_account::Account;
use solana_instruction::Instruction;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use thiserror::Error;

/// Error type for decompress mint operations.
#[derive(Debug, Error)]
pub enum DecompressMintError {
    #[error("Indexer error: {0}")]
    Indexer(#[from] IndexerError),

    #[error("Compressed mint not found for signer {signer:?}")]
    MintNotFound { signer: Pubkey },

    #[error("Missing compressed mint data in account")]
    MissingMintData,

    #[error("Program error: {0}")]
    ProgramError(#[from] ProgramError),

    #[error("Proof required for cold mint")]
    ProofRequired,
}

/// State of a CMint - either on-chain (hot), compressed (cold), or non-existent.
#[derive(Debug, Clone)]
pub enum MintState {
    /// CMint exists on-chain - no decompression needed.
    Hot { account: Account },
    /// CMint is compressed - needs decompression.
    Cold {
        compressed: CompressedAccount,
        mint_data: CompressedMint,
    },
    /// CMint doesn't exist (neither on-chain nor compressed).
    None,
}

/// Interface for a CMint that provides all info needed for decompression.
///
/// This is a superset of the solana Account type, containing:
/// - CMint pubkey (derived from signer)
/// - Signer pubkey (mint authority seed)
/// - State: Hot (on-chain), Cold (compressed), or None
#[derive(Debug, Clone)]
pub struct MintInterface {
    /// The CMint PDA pubkey.
    pub cmint: Pubkey,
    /// The mint signer pubkey (used to derive CMint).
    pub signer: Pubkey,
    /// Address tree where compressed mint lives.
    pub address_tree: Pubkey,
    /// Compressed address (for proof).
    pub compressed_address: [u8; 32],
    /// Current state of the CMint.
    pub state: MintState,
}

impl MintInterface {
    /// Returns true if this CMint needs decompression (is cold).
    #[inline]
    pub fn is_cold(&self) -> bool {
        matches!(self.state, MintState::Cold { .. })
    }

    /// Returns true if this CMint exists on-chain (is hot).
    #[inline]
    pub fn is_hot(&self) -> bool {
        matches!(self.state, MintState::Hot { .. })
    }

    /// Returns the compressed account hash if cold.
    pub fn hash(&self) -> Option<[u8; 32]> {
        match &self.state {
            MintState::Cold { compressed, .. } => Some(compressed.hash),
            _ => None,
        }
    }

    /// Returns the on-chain account if hot.
    pub fn account(&self) -> Option<&Account> {
        match &self.state {
            MintState::Hot { account } => Some(account),
            _ => None,
        }
    }

    /// Returns the compressed account and mint data if cold.
    pub fn compressed(&self) -> Option<(&CompressedAccount, &CompressedMint)> {
        match &self.state {
            MintState::Cold {
                compressed,
                mint_data,
            } => Some((compressed, mint_data)),
            _ => None,
        }
    }
}

/// Default rent payment in epochs (~24 hours per epoch)
pub const DEFAULT_RENT_PAYMENT: u8 = 2;
/// Default write top-up lamports (~3 hours rent per write)
pub const DEFAULT_WRITE_TOP_UP: u32 = 766;

/// Builds decompress instruction for a CMint synchronously.
///
/// This is a high-performance API for apps that pre-fetch mint state.
/// Returns empty vec if mint is hot (on-chain) - fast exit.
///
/// # Arguments
/// * `mint` - Pre-fetched MintInterface (from `get_mint_interface`)
/// * `fee_payer` - Fee payer pubkey
/// * `validity_proof` - Proof for cold mint (required if cold, ignored if hot)
/// * `rent_payment` - Rent payment in epochs (default: 2)
/// * `write_top_up` - Lamports for future writes (default: 766)
///
/// # Returns
/// * Vec with single decompress instruction
/// * Empty vec if mint is hot
pub fn build_decompress_mint(
    mint: &MintInterface,
    fee_payer: Pubkey,
    validity_proof: Option<ValidityProofWithContext>,
    rent_payment: Option<u8>,
    write_top_up: Option<u32>,
) -> Result<Vec<Instruction>, DecompressMintError> {
    // Fast exit if hot
    let mint_data = match &mint.state {
        MintState::Hot { .. } | MintState::None => return Ok(vec![]),
        MintState::Cold { mint_data, .. } => mint_data,
    };

    // Check if already decompressed flag is set - return empty vec (idempotent)
    if mint_data.metadata.cmint_decompressed {
        return Ok(vec![]);
    }

    // Proof required for cold mint
    let proof_result = validity_proof.ok_or(DecompressMintError::ProofRequired)?;

    // Extract tree info from proof result
    let account_info = &proof_result.accounts[0];
    let state_tree = account_info.tree_info.tree;
    let input_queue = account_info.tree_info.queue;
    let output_queue = account_info
        .tree_info
        .next_tree_info
        .as_ref()
        .map(|next| next.queue)
        .unwrap_or(input_queue);

    // Build CompressedMintWithContext
    let mint_instruction_data = CompressedMintInstructionData::try_from(mint_data.clone())
        .map_err(|_| DecompressMintError::MissingMintData)?;

    let compressed_mint_with_context = CompressedMintWithContext {
        leaf_index: account_info.leaf_index as u32,
        prove_by_index: account_info.root_index.proof_by_index(),
        root_index: account_info.root_index.root_index().unwrap_or_default(),
        address: mint.compressed_address,
        mint: Some(mint_instruction_data),
    };

    // Build DecompressMint instruction
    let decompress = DecompressMint {
        payer: fee_payer,
        authority: fee_payer, // Permissionless - any signer works
        state_tree,
        input_queue,
        output_queue,
        compressed_mint_with_context,
        proof: ValidityProof(proof_result.proof.into()),
        rent_payment: rent_payment.unwrap_or(DEFAULT_RENT_PAYMENT),
        write_top_up: write_top_up.unwrap_or(DEFAULT_WRITE_TOP_UP),
    };

    let ix = decompress
        .instruction()
        .map_err(DecompressMintError::from)?;
    Ok(vec![ix])
}

/// High-performance wrapper: decompress pre-fetched mint.
///
/// Takes pre-fetched `MintInterface`, fetches proof internally, builds instruction.
/// Returns empty vec if mint is hot (on-chain) - fast exit.
///
/// # Example
/// ```ignore
/// // Pre-fetch mint state
/// let mint = rpc.get_mint_interface(&signer).await?;
///
/// // Decompress if cold (fetches proof internally)
/// let instructions = decompress_cmint(&mint, fee_payer, &rpc).await?;
/// ```
pub async fn decompress_cmint<I: Indexer>(
    mint: &MintInterface,
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressMintError> {
    // Fast exit if hot or doesn't exist
    let hash = match mint.hash() {
        Some(h) => h,
        None => return Ok(vec![]),
    };

    // Check decompressed flag before fetching proof
    if let Some((_, mint_data)) = mint.compressed() {
        if mint_data.metadata.cmint_decompressed {
            return Ok(vec![]);
        }
    }

    // Get validity proof
    let proof = indexer
        .get_validity_proof(vec![hash], vec![], None)
        .await?
        .value;

    // Build instruction (sync)
    build_decompress_mint(mint, fee_payer, Some(proof), None, None)
}

/// Request to decompress a compressed CMint.
#[derive(Debug, Clone)]
pub struct DecompressMintRequest {
    /// The seed pubkey used to derive the CMint PDA.
    /// This is the same value passed as `mint_signer` when the mint was created.
    pub mint_seed_pubkey: Pubkey,
    /// Address tree where the compressed mint was created.
    /// If None, uses the default cmint address tree.
    pub address_tree: Option<Pubkey>,
    /// Rent payment in epochs (must be 0 or >= 2). Default: 2
    pub rent_payment: Option<u8>,
    /// Lamports for future write operations. Default: 766
    pub write_top_up: Option<u32>,
}

impl DecompressMintRequest {
    pub fn new(mint_seed_pubkey: Pubkey) -> Self {
        Self {
            mint_seed_pubkey,
            address_tree: None,
            rent_payment: None,
            write_top_up: None,
        }
    }

    pub fn with_address_tree(mut self, address_tree: Pubkey) -> Self {
        self.address_tree = Some(address_tree);
        self
    }

    pub fn with_rent_payment(mut self, rent_payment: u8) -> Self {
        self.rent_payment = Some(rent_payment);
        self
    }

    pub fn with_write_top_up(mut self, write_top_up: u32) -> Self {
        self.write_top_up = Some(write_top_up);
        self
    }
}

/// Decompress a compressed mint with default parameters.
/// Returns empty vec if already decompressed (idempotent).
pub async fn decompress_mint<I: Indexer>(
    mint_seed_pubkey: Pubkey,
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressMintError> {
    decompress_mint_idempotent(
        DecompressMintRequest::new(mint_seed_pubkey),
        fee_payer,
        indexer,
    )
    .await
}

/// Decompresses a compressed CMint to an on-chain CMint Solana account.
///
/// This is permissionless - any fee_payer can decompress any compressed mint.
/// Returns empty vec if already decompressed (idempotent).
pub async fn decompress_mint_idempotent<I: Indexer>(
    request: DecompressMintRequest,
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressMintError> {
    // 1. Derive addresses
    let address_tree = request
        .address_tree
        .unwrap_or(Pubkey::new_from_array(CMINT_ADDRESS_TREE));
    let compressed_address =
        derive_mint_compressed_address(&request.mint_seed_pubkey, &address_tree);

    // 2. Fetch compressed mint account from indexer
    let compressed_account = indexer
        .get_compressed_account(compressed_address, None)
        .await?
        .value
        .ok_or(DecompressMintError::MintNotFound {
            signer: request.mint_seed_pubkey,
        })?;

    // 3. Check if data is empty (already decompressed - empty shell remains)
    // After decompression, the compressed account has empty data but the address persists.
    let data = match compressed_account.data.as_ref() {
        Some(d) if !d.data.is_empty() => d,
        _ => return Ok(vec![]), // Empty data = already decompressed (idempotent)
    };

    // 4. Parse mint data from compressed account
    let mint_data = CompressedMint::try_from_slice(&data.data)
        .map_err(|_| DecompressMintError::MissingMintData)?;

    // 5. Check if already decompressed flag is set - return empty vec (idempotent)
    if mint_data.metadata.cmint_decompressed {
        return Ok(vec![]);
    }

    // 5. Get validity proof
    let proof_result = indexer
        .get_validity_proof(vec![compressed_account.hash], vec![], None)
        .await?
        .value;

    // 6. Extract tree info from proof result
    let account_info = &proof_result.accounts[0];
    let state_tree = account_info.tree_info.tree;
    let input_queue = account_info.tree_info.queue;
    let output_queue = account_info
        .tree_info
        .next_tree_info
        .as_ref()
        .map(|next| next.queue)
        .unwrap_or(input_queue);

    // 7. Build CompressedMintWithContext
    // NOTE: prove_by_index and leaf_index come from account_info (the proof), not compressed_account
    // The query may have stale values, but the proof is authoritative.
    let mint_instruction_data = CompressedMintInstructionData::try_from(mint_data)
        .map_err(|_| DecompressMintError::MissingMintData)?;

    let compressed_mint_with_context = CompressedMintWithContext {
        leaf_index: account_info.leaf_index as u32,
        prove_by_index: account_info.root_index.proof_by_index(),
        root_index: account_info.root_index.root_index().unwrap_or_default(),
        address: compressed_address,
        mint: Some(mint_instruction_data),
    };

    // 8. Build DecompressMint instruction
    let decompress = DecompressMint {
        payer: fee_payer,
        authority: fee_payer, // Permissionless - any signer works
        state_tree,
        input_queue,
        output_queue,
        compressed_mint_with_context,
        proof: ValidityProof(proof_result.proof.into()),
        rent_payment: request.rent_payment.unwrap_or(DEFAULT_RENT_PAYMENT),
        write_top_up: request.write_top_up.unwrap_or(DEFAULT_WRITE_TOP_UP),
    };

    let ix = decompress
        .instruction()
        .map_err(DecompressMintError::from)?;
    Ok(vec![ix])
}

/// Derive MintInterface from signer pubkey and on-chain/compressed state.
/// Helper for creating MintInterface when you have the data.
pub fn create_mint_interface(
    signer: Pubkey,
    address_tree: Pubkey,
    onchain_account: Option<Account>,
    compressed: Option<(CompressedAccount, CompressedMint)>,
) -> MintInterface {
    let (cmint, _) = find_mint_address(&signer);
    let compressed_address = derive_mint_compressed_address(&signer, &address_tree);

    let state = if let Some(account) = onchain_account {
        MintState::Hot { account }
    } else if let Some((compressed, mint_data)) = compressed {
        MintState::Cold {
            compressed,
            mint_data,
        }
    } else {
        MintState::None
    };

    MintInterface {
        cmint,
        signer,
        address_tree,
        compressed_address,
        state,
    }
}
