//! SDK helper for building Transfer2WithAta instructions.
//!
//! Transfer2WithAta enables decompress/transfer operations on compressed tokens
//! where ALL inputs have owner = ATA pubkey (compress_to_pubkey mode).
//!
//! Supports two modes:
//! 1. Owner mode: owner_wallet signs (for tokens without delegate)
//! 2. Delegate mode: delegate signs (for tokens with delegate set)
//!
//! This leverages the existing decompress instruction builder and wraps it.

use light_client::{
    indexer::{CompressedTokenAccount, Indexer},
    rpc::Rpc,
};
use light_compressed_token_sdk::{ctoken::derive_ctoken_ata, error::TokenSdkError};
use light_compressed_token_types::constants::TRANSFER2_WITH_ATA;
use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
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
/// This is used when compressed tokens have owner = ATA pubkey (created with compress_to_pubkey=true).
/// The instruction derives the ATA from [owner_wallet, program_id, mint], validates all inputs
/// have that ATA as owner, and performs a self-CPI to Transfer2 with the ATA signed.
///
/// Supports two modes:
/// - Owner mode (use_delegate = false): owner_wallet must sign
/// - Delegate mode (use_delegate = true): delegate (from inputs) must sign
pub async fn create_decompress_ata_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    input: DecompressAtaInput,
    payer: Pubkey,
) -> Result<Instruction, TokenSdkError> {
    if input.compressed_token_accounts.is_empty() {
        return Err(TokenSdkError::InvalidAccountData);
    }

    // Derive ATA and validate
    let (derived_ata, ata_bump) = derive_ctoken_ata(&input.owner_wallet, &input.mint);
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

    // Calculate total balance and decompress amount
    let total_balance: u64 = input
        .compressed_token_accounts
        .iter()
        .map(|acc| acc.token.amount)
        .sum();
    let decompress_amount = input.decompress_amount.unwrap_or(total_balance);

    // Use the EXISTING working decompress instruction builder
    let mut transfer2_ix = create_generic_transfer2_instruction(
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

    // Add wallet to accounts if not already there (always needed for ATA derivation)
    let wallet_index = match transfer2_ix
        .accounts
        .iter()
        .position(|m| m.pubkey == input.owner_wallet)
    {
        Some(idx) => {
            if !input.use_delegate {
                transfer2_ix.accounts[idx].is_signer = true;
            }
            idx as u8
        }
        None => {
            let idx = transfer2_ix.accounts.len() as u8;
            transfer2_ix
                .accounts
                .push(solana_instruction::AccountMeta::new_readonly(
                    input.owner_wallet,
                    !input.use_delegate, // is_signer only if owner mode
                ));
            idx
        }
    };

    // In delegate mode, mark delegate as signer
    if let Some(delegate_pubkey) = delegate {
        if let Some(idx) = transfer2_ix
            .accounts
            .iter()
            .position(|m| m.pubkey == delegate_pubkey)
        {
            transfer2_ix.accounts[idx].is_signer = true;
        }
        // Note: delegate should already be in accounts from the decompress instruction
        // since it's referenced in the compressed token inputs
    }

    // Find mint and ATA indices
    let mint_index = transfer2_ix
        .accounts
        .iter()
        .position(|m| m.pubkey == input.mint)
        .ok_or(TokenSdkError::InvalidAccountData)? as u8;

    let ata_index = transfer2_ix
        .accounts
        .iter()
        .position(|m| m.pubkey == derived_ata)
        .ok_or(TokenSdkError::InvalidAccountData)? as u8;

    // Un-mark ATA as signer - it's a PDA, on-chain will sign via CPI
    if let Some(ata_meta) = transfer2_ix.accounts.get_mut(ata_index as usize) {
        ata_meta.is_signer = false;
    }

    // Modify instruction data:
    // - Change discriminator to Transfer2WithAta
    // - Append: [wallet_idx, mint_idx, ata_idx, bump, use_delegate]
    transfer2_ix.data[0] = TRANSFER2_WITH_ATA;
    transfer2_ix.data.push(wallet_index);
    transfer2_ix.data.push(mint_index);
    transfer2_ix.data.push(ata_index);
    transfer2_ix.data.push(ata_bump);
    transfer2_ix.data.push(input.use_delegate as u8);

    Ok(Instruction {
        program_id: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: transfer2_ix.accounts,
        data: transfer2_ix.data,
    })
}
