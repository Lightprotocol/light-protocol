//! Instruction builder for Transfer2WithAta (decompress ATA-owned compressed tokens).
//!
//! This transforms a base Transfer2 instruction into a Transfer2WithAta instruction,
//! which enables decompression of tokens where owner = ATA pubkey.

use light_compressed_token_types::constants::TRANSFER2_WITH_ATA;
use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use super::derive_ctoken_ata;
use crate::error::{Result, TokenSdkError};

/// Input for transforming a Transfer2 instruction into Transfer2WithAta
#[derive(Debug, Clone)]
pub struct DecompressAtaParams {
    /// The wallet that owns the ATA (used for ATA derivation)
    pub owner_wallet: Pubkey,
    /// The mint of the tokens
    pub mint: Pubkey,
    /// If true, use delegate mode (delegate must sign).
    /// If false, use owner mode (owner_wallet must sign).
    pub use_delegate: bool,
    /// The delegate pubkey (only required if use_delegate=true)
    pub delegate: Option<Pubkey>,
}

/// Transforms a Transfer2 instruction into a Transfer2WithAta instruction.
///
/// This wraps an existing Transfer2 decompress instruction to work with
/// ATA-owned compressed tokens (created with compress_to_pubkey=true).
///
/// The on-chain program derives the ATA from [owner_wallet, program_id, mint],
/// validates all inputs have that ATA as owner, and performs a self-CPI
/// to Transfer2 with the ATA signed.
///
/// # Arguments
/// * `transfer2_ix` - A base Transfer2 instruction for decompression
/// * `params` - Parameters for the transformation
///
/// # Returns
/// A Transfer2WithAta instruction ready to be sent
pub fn create_decompress_ata_instruction(
    mut transfer2_ix: Instruction,
    params: DecompressAtaParams,
) -> Result<Instruction> {
    let (derived_ata, ata_bump) = derive_ctoken_ata(&params.owner_wallet, &params.mint);

    // Find or add wallet to accounts
    let wallet_index = match transfer2_ix
        .accounts
        .iter()
        .position(|m| m.pubkey == params.owner_wallet)
    {
        Some(idx) => {
            if !params.use_delegate {
                transfer2_ix.accounts[idx].is_signer = true;
            }
            idx as u8
        }
        None => {
            let idx = transfer2_ix.accounts.len() as u8;
            transfer2_ix.accounts.push(AccountMeta::new_readonly(
                params.owner_wallet,
                !params.use_delegate, // is_signer only if owner mode
            ));
            idx
        }
    };

    // In delegate mode, mark delegate as signer
    if params.use_delegate {
        let delegate = params.delegate.ok_or(TokenSdkError::InvalidAccountData)?;
        if let Some(idx) = transfer2_ix
            .accounts
            .iter()
            .position(|m| m.pubkey == delegate)
        {
            transfer2_ix.accounts[idx].is_signer = true;
        }
    }

    // Find mint index (must exist in accounts)
    let mint_index = transfer2_ix
        .accounts
        .iter()
        .position(|m| m.pubkey == params.mint)
        .ok_or(TokenSdkError::InvalidAccountData)? as u8;

    // Find ATA index (must exist in accounts as decompress destination)
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
    transfer2_ix.data.push(params.use_delegate as u8);

    Ok(Instruction {
        program_id: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: transfer2_ix.accounts,
        data: transfer2_ix.data,
    })
}
