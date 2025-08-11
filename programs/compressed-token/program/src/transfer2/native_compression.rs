use anchor_compressed_token::{
    check_spl_token_pool_derivation, ErrorCode,
};
use anchor_lang::prelude::ProgramError;
use light_account_checks::{checks::check_owner, packed_accounts::ProgramPackedAccounts};
use light_ctoken_types::{
    instructions::transfer2::{
        CompressionMode, ZCompressedTokenInstructionDataTransfer2, ZCompression,
    },
    state::CompressedToken,
};
use light_zero_copy::borsh_mut::DeserializeMut;
use pinocchio::account_info::AccountInfo;
use solana_pubkey::Pubkey;
use spl_pod::solana_msg::msg;

use crate::{
    constants::BUMP_CPI_AUTHORITY,
    shared::owner_validation::verify_and_update_token_account_authority_with_compressed_token,
    LIGHT_CPI_SIGNER,
};
use light_sdk_types::CPI_AUTHORITY_PDA_SEED;

const SPL_TOKEN_ID: &[u8; 32] = &spl_token::ID.to_bytes();
const SPL_TOKEN_2022_ID: &[u8; 32] = &spl_token_2022::ID.to_bytes();
const ID: &[u8; 32] = &LIGHT_CPI_SIGNER.program_id;
/// Process native compressions/decompressions with token accounts
pub fn process_token_compression(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
) -> Result<(), ProgramError> {
    if let Some(compressions) = inputs.compressions.as_ref() {
        for compression in compressions {
            let source_or_recipient = packed_accounts.get_u8(
                compression.source_or_recipient,
                "compression source or recipient",
            )?;

            match unsafe { source_or_recipient.owner() } {
                ID => {
                    process_native_compressions(compression, source_or_recipient, packed_accounts)?;
                }
                SPL_TOKEN_ID | SPL_TOKEN_2022_ID => {
                    process_spl_compressions(compression, source_or_recipient, packed_accounts)?;
                }
                _ => return Err(ProgramError::InvalidInstructionData),
            }
        }
    }
    Ok(())
}

/// Validate compression fields based on compression mode
fn validate_compression_mode_fields(compression: &ZCompression) -> Result<(), ProgramError> {
    let mode = compression.mode;

    match mode {
        CompressionMode::Decompress => {
            // Decompress must have authority = 0
            if compression.authority != 0 {
                msg!("authority must be 0 for Decompress mode");
                return Err(ProgramError::InvalidInstructionData);
            }
        }
        CompressionMode::Compress => {
            // No additional validation needed for regular compress
        }
    }

    Ok(())
}

/// Process compression/decompression for token accounts using zero-copy PodAccount
fn process_native_compressions(
    compression: &ZCompression,
    token_account_info: &AccountInfo,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
) -> Result<(), ProgramError> {
    let mode = compression.mode;

    // Validate compression fields for the given mode
    validate_compression_mode_fields(compression)?;
    // Get authority account and effective compression amount
    let authority_account = packed_accounts.get_u8(compression.authority, "process_native_compression: authority")?;

    let mint_account = *packed_accounts
        .get_u8(compression.mint, "process_native_compression: token mint")?
        .key();
    native_compression(
        Some(authority_account),
        (*compression.amount).into(),
        mint_account.into(),
        token_account_info,
        mode,
    )?;

    Ok(())
}

/// Perform native compression/decompression on a token account
pub fn native_compression(
    authority: Option<&AccountInfo>,
    amount: u64,
    mint: Pubkey,
    token_account_info: &AccountInfo,
    mode: CompressionMode,
) -> Result<(), ProgramError> {
    msg!(
        "token_account_info {:?}",
        solana_pubkey::Pubkey::new_from_array(*token_account_info.key())
    );
    check_owner(&crate::LIGHT_CPI_SIGNER.program_id, token_account_info)?;
    // Access token account data as mutable bytes
    let mut token_account_data = token_account_info
        .try_borrow_mut_data()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;

    // Use zero-copy deserialization to access the compressed token account
    let (mut compressed_token, _) = CompressedToken::zero_copy_at_mut(&mut token_account_data)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if compressed_token.mint.to_bytes() != mint.to_bytes() {
        msg!(
            "mint mismatch account: compressed_token.mint {:?}, mint {:?}",
            solana_pubkey::Pubkey::new_from_array(compressed_token.mint.to_bytes()),
            solana_pubkey::Pubkey::new_from_array(mint.to_bytes())
        );
        return Err(ProgramError::InvalidAccountData);
    }

    // Get current balance
    let current_balance: u64 = u64::from(*compressed_token.amount);

    // Calculate new balance using effective amount
    let new_balance = match mode {
        CompressionMode::Compress => {
            // Verify authority for compression operations and update delegated amount if needed
            let authority_account = authority.ok_or(ErrorCode::InvalidCompressAuthority)?;
            verify_and_update_token_account_authority_with_compressed_token(
                &mut compressed_token,
                authority_account,
                amount,
            )?;

            // Compress: subtract from solana account
            current_balance
                .checked_sub(amount)
                .ok_or(ProgramError::ArithmeticOverflow)?
        }
        CompressionMode::Decompress => {
            // Decompress: add to solana account
            current_balance
                .checked_add(amount)
                .ok_or(ProgramError::ArithmeticOverflow)?
        }
    };

    // Update the balance in the compressed token account
    *compressed_token.amount = new_balance.into();

    compressed_token
        .update_compressible_last_written_slot()
        .map_err(|_| ProgramError::InvalidAccountData)?;
    Ok(())
}

/// Process compression/decompression for SPL token accounts 
fn process_spl_compressions(
    compression: &ZCompression,
    token_account_info: &AccountInfo,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
) -> Result<(), ProgramError> {
    let mode = compression.mode;

    // Validate compression fields for the given mode
    validate_compression_mode_fields(compression)?;
    
    // Get authority account and effective compression amount
    let authority_account = packed_accounts.get_u8(compression.authority, "process_spl_compression: authority")?;

    let mint_account = *packed_accounts
        .get_u8(compression.mint, "process_spl_compression: token mint")?
        .key();

    spl_compression(
        Some(authority_account),
        (*compression.amount).into(),
        mint_account.into(),
        token_account_info,
        mode,
        packed_accounts,
    )?;

    Ok(())
}

/// SPL token compression/decompression
fn spl_compression(
    authority: Option<&AccountInfo>,
    amount: u64,
    mint: Pubkey,
    token_account_info: &AccountInfo,
    mode: CompressionMode,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
) -> Result<(), ProgramError> {
    match mode {
        CompressionMode::Compress => {
            let authority_account = authority.ok_or(ErrorCode::InvalidCompressAuthority)?;
            
            // Find token pool account in packed_accounts
            let token_pool_account = find_token_pool_account(packed_accounts, &mint)?;
            
            // Validate token pool derivation
            check_spl_token_pool_derivation(
                &solana_pubkey::Pubkey::new_from_array(*token_pool_account.key()),
                &solana_pubkey::Pubkey::new_from_array(mint.to_bytes())
            ).map_err(|_| ProgramError::InvalidAccountData)?;
            
            // Find token program account
            let token_program_account = find_token_program_account(packed_accounts)?;
            
            // Transfer from user account to token pool
            spl_token_transfer(
                token_account_info,
                token_pool_account,
                authority_account,
                token_program_account,
                amount,
            )
        }
        CompressionMode::Decompress => {
            // Find token pool accounts and handle multi-pool transfers
            invoke_token_program_with_multiple_token_pool_accounts(
                packed_accounts,
                &mint.to_bytes(),
                token_account_info,
                amount,
            )
        }
    }
}

/// Find token pool account in packed accounts for the given mint
fn find_token_pool_account<'a>(
    packed_accounts: &'a ProgramPackedAccounts<'a, AccountInfo>,
    mint: &Pubkey,
) -> Result<&'a AccountInfo, ProgramError> {
    // Iterate through packed accounts to find valid token pool
    for i in 0..packed_accounts.accounts.len() {
        if let Ok(account) = packed_accounts.get_u8(i as u8, "find_token_pool_account: account") {
            // Check if this account is a valid token pool for the mint
            if check_spl_token_pool_derivation(
                &solana_pubkey::Pubkey::new_from_array(*account.key()),
                &solana_pubkey::Pubkey::new_from_array(mint.to_bytes())
            ).is_ok() {
                return Ok(account);
            }
        }
    }
    Err(ProgramError::InvalidAccountData)
}

/// Find token program account (SPL Token or Token-2022) in packed accounts
fn find_token_program_account<'a>(
    packed_accounts: &'a ProgramPackedAccounts<'a, AccountInfo>,
) -> Result<&'a AccountInfo, ProgramError> {
    for i in 0..packed_accounts.accounts.len() {
        if let Ok(account) = packed_accounts.get_u8(i as u8, "find_token_program_account: account") {
            let owner = unsafe { account.owner() };
            if owner == SPL_TOKEN_ID || owner == SPL_TOKEN_2022_ID {
                return Ok(account);
            }
        }
    }
    Err(ProgramError::InvalidAccountData)
}


/// SPL token transfer instruction using pinocchio CPI
fn spl_token_transfer(
    from: &AccountInfo,
    to: &AccountInfo,
    authority: &AccountInfo,
    token_program: &AccountInfo,
    amount: u64,
) -> Result<(), ProgramError> {
    let token_program_owner = unsafe { token_program.owner() };
    
    // Construct SPL token transfer instruction data manually using stack allocation
    let instruction_data = match token_program_owner {
        SPL_TOKEN_2022_ID | SPL_TOKEN_ID => {
            // SPL Token Transfer instruction: [3, amount (8 bytes)]
            let mut data = [0u8; 9]; // Transfer instruction discriminator + amount
            data[0] = 3u8; // Transfer instruction discriminator
            data[1..9].copy_from_slice(&amount.to_le_bytes());
            data
        }
        _ => return Err(ProgramError::InvalidInstructionData),
    };

    // Build account metas for SPL token transfer
    let account_metas = [
        pinocchio::instruction::AccountMeta::new(from.key(), false, false),  // source
        pinocchio::instruction::AccountMeta::new(to.key(), true, false),    // destination  
        pinocchio::instruction::AccountMeta::new(authority.key(), false, true), // authority (signer)
    ];

    let instruction = pinocchio::instruction::Instruction {
        program_id: token_program.key(),
        accounts: &account_metas,
        data: &instruction_data,
    };

    // Create pinocchio signer for CPI authority
    let bump_seed = [BUMP_CPI_AUTHORITY];
    let seed_array = [
        pinocchio::instruction::Seed::from(CPI_AUTHORITY_PDA_SEED),
        pinocchio::instruction::Seed::from(bump_seed.as_slice()),
    ];
    let signer = pinocchio::instruction::Signer::from(&seed_array);

    // Execute CPI using pinocchio native function with references
    let account_infos = &[from, to, authority];
    
    pinocchio::cpi::slice_invoke_signed(&instruction, account_infos, &[signer])
        .map_err(|_| ProgramError::InvalidArgument)?;
    
    Ok(())
}

/// Handle multiple token pool accounts for large decompression amounts
/// TODO: Implement proper multi-pool decompression logic
fn invoke_token_program_with_multiple_token_pool_accounts(
    _packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    _mint_bytes: &[u8; 32],
    _recipient: &AccountInfo,
    _amount: u64,
) -> Result<(), ProgramError> {
    // Multi-pool decompression not yet implemented
    Err(ProgramError::InvalidInstructionData)
}



