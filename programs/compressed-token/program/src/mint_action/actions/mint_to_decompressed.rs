use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::Pubkey;
use light_ctoken_types::{
    instructions::{mint_action::ZMintToDecompressedAction, transfer2::ZCompressionMode},
    state::ZCompressedMintMut,
};
use light_profiler::profile;
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use crate::{
    mint_action::{
        accounts::{AccountsConfig, MintActionAccounts},
        mint_to::mint_authority_check,
    },
    shared::mint_to_token_pool,
    transfer2::native_compression::native_compression,
};

#[allow(clippy::too_many_arguments)]
#[profile]
pub fn process_mint_to_decompressed_action(
    action: &ZMintToDecompressedAction,
    current_supply: u64,
    compressed_mint: &ZCompressedMintMut<'_>,
    validated_accounts: &MintActionAccounts,
    accounts_config: &AccountsConfig,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    mint: Pubkey,
    instruction_mint_authority: Option<Pubkey>,
) -> Result<u64, ProgramError> {
    mint_authority_check(
        compressed_mint,
        validated_accounts,
        instruction_mint_authority,
    )?;

    let amount = u64::from(action.recipient.amount);
    let updated_supply = current_supply
        .checked_add(amount)
        .ok_or(ErrorCode::MintActionAmountTooLarge)?;

    handle_decompressed_mint_to_token_pool(validated_accounts, accounts_config, amount, mint)?;

    // Get the recipient token account from packed accounts using the index
    let token_account_info = packed_accounts.get_u8(
        action.recipient.account_index,
        "decompressed mint to recipient",
    )?;

    // Authority check now performed above - safe to proceed with decompression
    native_compression(
        None, // No authority needed for decompression
        None,
        amount,
        &mint.into(),
        token_account_info,
        None,
        &ZCompressionMode::Decompress,
        packed_accounts,
    )?;
    Ok(updated_supply)
}

#[profile]
pub fn handle_decompressed_mint_to_token_pool(
    validated_accounts: &MintActionAccounts,
    accounts_config: &crate::mint_action::accounts::AccountsConfig,
    amount: u64,
    mint: Pubkey,
) -> Result<(), ProgramError> {
    if let Some(system_accounts) = validated_accounts.executing.as_ref() {
        // If mint is decompressed, mint tokens to the token pool to maintain SPL mint supply consistency
        if accounts_config.is_decompressed {
            let mint_account = system_accounts
                .mint
                .ok_or(ErrorCode::MintActionMissingMintAccount)?;
            if mint.to_bytes() != *mint_account.key() {
                msg!("Mint account mismatch");
                return Err(ErrorCode::MintAccountMismatch.into());
            }
            // TODO: check derivation. with bump.
            let token_pool_account = system_accounts
                .token_pool_pda
                .ok_or(ErrorCode::MintActionMissingTokenPoolAccount)?;
            let token_program = system_accounts
                .token_program
                .ok_or(ErrorCode::MintActionMissingTokenProgram)?;

            mint_to_token_pool(
                mint_account,
                token_pool_account,
                token_program,
                validated_accounts.cpi_authority()?,
                amount,
            )?;
        }
    } else if accounts_config.is_decompressed {
        msg!("if mint is decompressed executing accounts must be present");
        return Err(ErrorCode::Transfer2CpiContextWriteInvalidAccess.into());
    }
    Ok(())
}
