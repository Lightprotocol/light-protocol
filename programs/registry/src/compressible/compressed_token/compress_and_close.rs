use anchor_lang::{prelude::ProgramError, pubkey, AnchorDeserialize, AnchorSerialize, Result};
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_ctoken_types::{
    instructions::transfer2::{
        CompressedTokenInstructionDataTransfer2, Compression, CompressionMode,
        MultiTokenTransferOutputData,
    },
    state::CToken,
};
use light_program_profiler::profile;
use solana_account_info::AccountInfo;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use spl_pod::solana_msg::msg;

use crate::errors::RegistryError;

const TRANSFER2_DISCRIMINATOR: u8 = 101;
use super::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, COMPRESSED_TOKEN_PROGRAM_ID,
    LIGHT_SYSTEM_PROGRAM_ID, REGISTERED_PROGRAM_PDA,
};

pub const CPI_AUTHORITY_PDA: Pubkey = pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");

/// Struct to hold all the indices needed for CompressAndClose operation
#[derive(Debug, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressAndCloseIndices {
    pub source_index: u8,
    pub mint_index: u8,
    pub owner_index: u8,
    pub rent_sponsor_index: u8, // Can vary with custom rent sponsors
}

/// Compress and close compressed token accounts with pre-computed indices
///
/// This function is designed for on-chain use (e.g., from registry program).
/// It reads account data, builds Compression structs manually, and constructs
/// the Transfer2 instruction with all necessary accounts.
///
/// # Arguments
/// * `fee_payer` - The fee payer pubkey
/// * `cpi_context_pubkey` - Optional CPI context account for optimized multi-program transactions
/// * `authority_index` - Index of compression authority in packed_accounts
/// * `destination_index` - Index of compression incentive destination in packed_accounts
/// * `indices` - Slice of per-account indices (source, mint, owner, rent_sponsor)
/// * `packed_accounts` - Slice of all accounts (AccountInfo) that will be used in the instruction
///
/// # Returns
/// An instruction that compresses and closes all provided token accounts
#[profile]
pub fn compress_and_close_ctoken_accounts_with_indices<'info>(
    fee_payer: Pubkey,
    authority_index: u8,
    destination_index: u8,
    indices: &[CompressAndCloseIndices],
    packed_accounts: &ProgramPackedAccounts<'info, AccountInfo<'info>>,
) -> Result<Instruction> {
    if indices.is_empty() {
        msg!("indices empty");
        return Err(ProgramError::NotEnoughAccountKeys.into());
    }

    // Convert packed_accounts to AccountMetas
    let mut packed_account_metas = Vec::with_capacity(packed_accounts.accounts.len());
    for info in packed_accounts.accounts.iter() {
        packed_account_metas.push(AccountMeta {
            pubkey: *info.key,
            is_signer: info.is_signer,
            is_writable: info.is_writable,
        });
    }

    // Create one output per compression (no deduplication)
    let mut output_accounts = Vec::with_capacity(indices.len());
    let mut compressions = Vec::with_capacity(indices.len());

    // Process each set of indices
    for (i, idx) in indices.iter().enumerate() {
        // Get the amount from the source token account
        let source_account = packed_accounts
            .get_u8(idx.source_index, "source_account")
            .map_err(ProgramError::from)?;

        let account_data = source_account
            .try_borrow_data()
            .map_err(|_| RegistryError::InvalidSigner)?;

        let amount = CToken::amount_from_slice(&account_data).map_err(|e| {
            anchor_lang::prelude::msg!("Failed to read amount from CToken: {:?}", e);
            RegistryError::InvalidSigner
        })?;

        // Create one output account per compression operation
        output_accounts.push(MultiTokenTransferOutputData {
            owner: idx.owner_index,
            amount,
            delegate: 0,
            mint: idx.mint_index,
            version: 3, // Shaflat
            has_delegate: false,
        });

        let compression = Compression {
            mode: CompressionMode::CompressAndClose,
            amount,
            mint: idx.mint_index,
            source_or_recipient: idx.source_index,
            authority: authority_index,
            pool_account_index: idx.rent_sponsor_index,
            pool_index: i as u8,
            bump: destination_index,
        };

        compressions.push(compression);
    }

    packed_account_metas
        .get_mut(authority_index as usize)
        .ok_or(ProgramError::NotEnoughAccountKeys)?
        .is_signer = true;

    // Build instruction data inline
    let instruction_data = CompressedTokenInstructionDataTransfer2 {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        output_queue: 0, // Output queue is at index 0 in packed_accounts
        proof: None,
        in_token_data: vec![], // No inputs for compress_and_close
        out_token_data: output_accounts,
        in_lamports: None,
        out_lamports: None,
        in_tlv: None,
        out_tlv: None,
        compressions: Some(compressions),
        cpi_context: None,
    };

    // Serialize instruction data
    let serialized = instruction_data
        .try_to_vec()
        .map_err(|_| RegistryError::InvalidSigner)?;

    // Build instruction data with discriminator
    let mut data = Vec::with_capacity(1 + serialized.len());
    data.push(TRANSFER2_DISCRIMINATOR);
    data.extend(serialized);

    // Build account metas following Transfer2 accounts layout
    let mut account_metas = Vec::with_capacity(10 + packed_account_metas.len());

    // Core system accounts
    account_metas.push(AccountMeta::new_readonly(LIGHT_SYSTEM_PROGRAM_ID, false));
    account_metas.push(AccountMeta::new(fee_payer, true)); // fee_payer (signer)
    account_metas.push(AccountMeta::new_readonly(CPI_AUTHORITY_PDA, false));
    account_metas.push(AccountMeta::new_readonly(REGISTERED_PROGRAM_PDA, false));
    account_metas.push(AccountMeta::new_readonly(
        ACCOUNT_COMPRESSION_AUTHORITY_PDA,
        false,
    ));
    account_metas.push(AccountMeta::new_readonly(
        ACCOUNT_COMPRESSION_PROGRAM_ID,
        false,
    ));
    account_metas.push(AccountMeta::new_readonly(
        Pubkey::from([0u8; 32]), // system_program
        false,
    ));
    // Packed accounts (trees, queues, mints, owners, etc.)
    account_metas.extend(packed_account_metas);

    Ok(Instruction {
        program_id: COMPRESSED_TOKEN_PROGRAM_ID,
        accounts: account_metas,
        data,
    })
}
