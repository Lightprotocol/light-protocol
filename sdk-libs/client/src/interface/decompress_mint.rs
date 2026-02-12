//! Mint decompression for hot/cold handling.

use borsh::BorshDeserialize;
use light_compressed_account::{
    address::derive_address, instruction_data::compressed_proof::ValidityProof,
};
use light_token::instruction::DecompressMint;
use light_token_interface::{
    instructions::mint_action::{MintInstructionData, MintWithContext},
    state::Mint,
    MINT_ADDRESS_TREE,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use thiserror::Error;

use super::AccountInterface;
use crate::indexer::{Indexer, ValidityProofWithContext};

/// Error type for mint load operations.
#[derive(Debug, Error)]
pub enum DecompressMintError {
    #[error("Mint not found for address {address:?}")]
    MintNotFound { address: Pubkey },

    #[error("Missing mint data in cold account")]
    MissingMintData,

    #[error("Invalid mint account owner: expected Light Token Program")]
    InvalidMintOwner,

    #[error("Program error: {0}")]
    ProgramError(#[from] solana_program_error::ProgramError),

    #[error("Mint already hot")]
    AlreadyDecompressed,

    #[error("Validity proof required for cold mint")]
    ProofRequired,

    #[error("Indexer error: {0}")]
    IndexerError(#[from] crate::indexer::IndexerError),
}

pub const DEFAULT_RENT_PAYMENT: u8 = 2;
pub const DEFAULT_WRITE_TOP_UP: u32 = 0;

/// Builds load instruction for a cold mint. Returns empty vec if already hot.
pub fn build_decompress_mint(
    mint: &AccountInterface,
    fee_payer: Pubkey,
    validity_proof: Option<ValidityProofWithContext>,
    rent_payment: Option<u8>,
    write_top_up: Option<u32>,
) -> Result<Vec<Instruction>, DecompressMintError> {
    if mint.is_hot() {
        return Ok(vec![]);
    }
    let mint_data = mint.as_mint().ok_or(DecompressMintError::ProofRequired)?;
    if mint_data.metadata.mint_decompressed {
        return Ok(vec![]);
    }
    let compressed_address = mint
        .mint_compressed_address()
        .ok_or(DecompressMintError::ProofRequired)?;

    let proof_result = validity_proof.ok_or(DecompressMintError::ProofRequired)?;
    let account_info = &proof_result.accounts[0];
    let state_tree = account_info.tree_info.tree;
    let input_queue = account_info.tree_info.queue;
    let output_queue = account_info
        .tree_info
        .next_tree_info
        .as_ref()
        .map(|next| next.queue)
        .unwrap_or(input_queue);

    let mint_instruction_data = MintInstructionData::try_from(mint_data.clone())
        .map_err(|_| DecompressMintError::MissingMintData)?;

    let compressed_mint_with_context = MintWithContext {
        leaf_index: account_info.leaf_index as u32,
        prove_by_index: account_info.root_index.proof_by_index(),
        root_index: account_info.root_index.root_index().unwrap_or_default(),
        address: compressed_address,
        mint: Some(mint_instruction_data),
    };

    let decompress = DecompressMint {
        payer: fee_payer,
        authority: fee_payer,
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

/// Load (decompress) a pre-fetched mint. Returns empty vec if already hot.
pub async fn decompress_mint<I: Indexer>(
    mint: &AccountInterface,
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressMintError> {
    let hash = match mint.hash() {
        Some(h) => h,
        None => return Ok(vec![]),
    };
    if let Some(mint_data) = mint.as_mint() {
        if mint_data.metadata.mint_decompressed {
            return Ok(vec![]);
        }
    }

    let proof = indexer
        .get_validity_proof(vec![hash], vec![], None)
        .await?
        .value;

    build_decompress_mint(mint, fee_payer, Some(proof), None, None)
}

/// Request to load (decompress) a cold mint.
#[derive(Debug, Clone)]
pub struct DecompressMintRequest {
    pub mint_seed_pubkey: Pubkey,
    pub address_tree: Option<Pubkey>,
    pub rent_payment: Option<u8>,
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

/// Loads (decompresses) a cold mint to on-chain. Idempotent.
pub async fn decompress_mint_idempotent<I: Indexer>(
    request: DecompressMintRequest,
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressMintError> {
    // 1. Derive addresses
    let address_tree = request
        .address_tree
        .unwrap_or(Pubkey::new_from_array(MINT_ADDRESS_TREE));
    let compressed_address = derive_address(
        &request.mint_seed_pubkey.to_bytes(),
        &address_tree.to_bytes(),
        &light_token_interface::LIGHT_TOKEN_PROGRAM_ID,
    );

    // 2. Fetch cold mint from indexer
    let compressed_account = indexer
        .get_compressed_account(compressed_address, None)
        .await?
        .value
        .ok_or(DecompressMintError::MintNotFound {
            address: request.mint_seed_pubkey,
        })?;

    // 3. Check if data is empty (already hot)
    let data = match compressed_account.data.as_ref() {
        Some(d) if !d.data.is_empty() => d,
        _ => return Ok(vec![]), // Empty data = already decompressed (idempotent)
    };

    // 4. Parse mint data from cold account
    let mint_data =
        Mint::try_from_slice(&data.data).map_err(|_| DecompressMintError::MissingMintData)?;

    // 5. Check if already decompressed flag is set - return empty vec (idempotent)
    if mint_data.metadata.mint_decompressed {
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

    // 7. Build MintWithContext
    let mint_instruction_data = MintInstructionData::try_from(mint_data)
        .map_err(|_| DecompressMintError::MissingMintData)?;

    let compressed_mint_with_context = MintWithContext {
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
