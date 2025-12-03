//! SDK helper for building Transfer2WithAta instructions.
//!
//! Transfer2WithAta enables decompress/transfer operations on compressed tokens
//! where ALL inputs have owner = ATA pubkey (compress_to_pubkey mode).
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

use super::transfer2::{create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType};

/// Input for decompressing ATA-owned compressed tokens
#[derive(Debug, Clone)]
pub struct DecompressAtaInput {
    /// Compressed token accounts to decompress (ALL must have owner = ATA)
    pub compressed_token_accounts: Vec<CompressedTokenAccount>,
    /// The user's wallet (must sign the transaction)
    pub owner_wallet: Pubkey,
    /// The mint of the tokens
    pub mint: Pubkey,
    /// The destination CToken ATA to decompress into
    pub destination_ata: Pubkey,
    /// Amount to decompress (if None, decompress full balance)
    pub decompress_amount: Option<u64>,
}

/// Creates a Transfer2WithAta instruction for decompressing ATA-owned compressed tokens.
///
/// This is used when compressed tokens have owner = ATA pubkey (created with compress_to_pubkey=true).
/// The instruction derives the ATA from [owner_wallet, program_id, mint], validates all inputs
/// have that ATA as owner, and performs a self-CPI to Transfer2 with the ATA signed.
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

    // Calculate total balance and decompress amount
    let total_balance: u64 = input
        .compressed_token_accounts
        .iter()
        .map(|acc| acc.token.amount)
        .sum();
    let decompress_amount = input.decompress_amount.unwrap_or(total_balance);

    // Use the EXISTING working decompress instruction builder
    // This handles all the packed account index tracking correctly
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

    // Now transform this into a Transfer2WithAta instruction:
    // 1. Add wallet to accounts if not present (needed for on-chain ATA derivation)
    // 2. Find the indices we need
    // 3. Change the discriminator
    // 4. Append the extra bytes
    // 5. Mark wallet as signer

    // Add wallet to accounts if not already there (it might not be in decompress flow)
    let wallet_index = transfer2_ix
        .accounts
        .iter()
        .position(|m| m.pubkey == input.owner_wallet);
    
    let wallet_index = match wallet_index {
        Some(idx) => {
            // Wallet already in accounts, just mark as signer
            transfer2_ix.accounts[idx].is_signer = true;
            idx as u8
        }
        None => {
            // Add wallet as signer
            let idx = transfer2_ix.accounts.len() as u8;
            transfer2_ix.accounts.push(solana_instruction::AccountMeta::new_readonly(
                input.owner_wallet,
                true, // is_signer
            ));
            idx
        }
    };

    // Find mint index (should already be in accounts for decompress)
    let mint_index = transfer2_ix
        .accounts
        .iter()
        .position(|m| m.pubkey == input.mint)
        .ok_or(TokenSdkError::InvalidAccountData)? as u8;

    // Find ATA index (should already be in accounts as the token owner/recipient)
    let ata_index = transfer2_ix
        .accounts
        .iter()
        .position(|m| m.pubkey == derived_ata)
        .ok_or(TokenSdkError::InvalidAccountData)? as u8;

    // IMPORTANT: Un-mark ATA as signer in the outer instruction!
    // The ATA is a PDA - we can't sign with it directly.
    // The on-chain Transfer2WithAta will sign for it via CPI.
    if let Some(ata_meta) = transfer2_ix.accounts.get_mut(ata_index as usize) {
        ata_meta.is_signer = false;
    }

    // Modify instruction data:
    // - Change discriminator from 101 (Transfer2) to 108 (Transfer2WithAta)
    // - Append: wallet_index, mint_index, ata_index, ata_bump
    transfer2_ix.data[0] = TRANSFER2_WITH_ATA;
    transfer2_ix.data.push(wallet_index);
    transfer2_ix.data.push(mint_index);
    transfer2_ix.data.push(ata_index);
    transfer2_ix.data.push(ata_bump);

    Ok(Instruction {
        program_id: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: transfer2_ix.accounts,
        data: transfer2_ix.data,
    })
}
