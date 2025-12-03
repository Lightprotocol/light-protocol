//! SDK helper for building Transfer2WithAta instructions via RPC.
//!
//! This module provides the high-level interface for decompressing ATA-owned
//! compressed tokens. It fetches required data via RPC and delegates the
//! instruction building to the compressed-token-sdk.

use light_client::{
    indexer::{CompressedTokenAccount, Indexer},
    rpc::Rpc,
};
use light_compressed_token_sdk::{
    ctoken::{create_decompress_ata_instruction, derive_ctoken_ata, DecompressAtaParams},
    error::TokenSdkError,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use super::transfer2::{
    create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
};

/// Input for decompressing ATA-owned compressed tokens
#[derive(Debug, Clone)]
pub struct DecompressAtaInput {
    /// Compressed token accounts to decompress (ALL must have owner = ATA)
    pub compressed_token_accounts: Vec<CompressedTokenAccount>,
    /// The wallet that owns the ATA (used for ATA derivation, must sign if no delegate)
    pub owner_wallet: Pubkey,
    /// The mint of the tokens
    pub mint: Pubkey,
    /// The destination CToken ATA to decompress into
    pub destination_ata: Pubkey,
    /// Amount to decompress (if None, decompress full balance)
    pub decompress_amount: Option<u64>,
    /// If true, use delegate mode (delegate from inputs must sign).
    /// If false, use owner mode (owner_wallet must sign).
    pub use_delegate: bool,
}

/// Creates a Transfer2WithAta instruction for decompressing ATA-owned compressed tokens.
///
/// This fetches required account data via RPC, builds a base Transfer2 decompress
/// instruction, and then transforms it into a Transfer2WithAta instruction.
pub async fn create_decompress_ata_instruction_rpc<R: Rpc + Indexer>(
    rpc: &mut R,
    input: DecompressAtaInput,
    payer: Pubkey,
) -> Result<Instruction, TokenSdkError> {
    if input.compressed_token_accounts.is_empty() {
        return Err(TokenSdkError::InvalidAccountData);
    }

    // Derive ATA and validate
    let (derived_ata, _) = derive_ctoken_ata(&input.owner_wallet, &input.mint);
    if input.destination_ata != derived_ata {
        return Err(TokenSdkError::InvalidAccountData);
    }

    // Validate all inputs have owner = ATA
    for account in &input.compressed_token_accounts {
        if account.token.owner != derived_ata {
            return Err(TokenSdkError::InvalidAccountData);
        }
    }

    // If delegate mode, validate all inputs have same delegate set
    let delegate = if input.use_delegate {
        let first_delegate = input.compressed_token_accounts[0]
            .token
            .delegate
            .ok_or(TokenSdkError::InvalidAccountData)?;
        for account in &input.compressed_token_accounts {
            if account.token.delegate != Some(first_delegate) {
                return Err(TokenSdkError::InvalidAccountData);
            }
        }
        Some(first_delegate)
    } else {
        None
    };

    // Calculate decompress amount
    let total_balance: u64 = input
        .compressed_token_accounts
        .iter()
        .map(|acc| acc.token.amount)
        .sum();
    let decompress_amount = input.decompress_amount.unwrap_or(total_balance);

    // Build base Transfer2 decompress instruction via RPC
    let transfer2_ix = create_generic_transfer2_instruction(
        rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: input.compressed_token_accounts,
            decompress_amount,
            solana_token_account: derived_ata,
            amount: decompress_amount,
            pool_index: None,
        })],
        payer,
        true, // filter zero outputs
    )
    .await?;

    // Transform to Transfer2WithAta using SDK
    create_decompress_ata_instruction(
        transfer2_ix,
        DecompressAtaParams {
            owner_wallet: input.owner_wallet,
            mint: input.mint,
            use_delegate: input.use_delegate,
            delegate,
        },
    )
}
