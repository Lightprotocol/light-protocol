use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_types::instructions::{
    mint_actions::ZMintToDecompressedAction, transfer2::CompressionMode,
};
use spl_pod::solana_msg::msg;

use crate::{
    mint_action::accounts::MintActionAccounts, shared::mint_to_token_pool,
    transfer2::native_compression::native_compression,
};

pub fn process_mint_to_decompressed_action(
    action: &ZMintToDecompressedAction,
    current_supply: u64,
    validated_accounts: &MintActionAccounts,
    accounts_config: &crate::mint_action::accounts::AccountsConfig,
    packed_accounts: &crate::transfer2::accounts::ProgramPackedAccounts,
    mint: Pubkey,
) -> Result<u64, ProgramError> {
    let amount = u64::from(action.recipient.amount);
    let updated_supply = current_supply
        .checked_add(amount)
        .ok_or(ErrorCode::MintActionAmountTooLarge)?;

    handle_decompressed_mint_to_token_pool(validated_accounts, accounts_config, amount, mint)?;

    // Get the recipient token account from packed accounts using the index
    let token_account_info = packed_accounts.get_u8(action.recipient.account_index)?;
    
    // For decompression (minting tokens into account), no authority check is needed
    // The mint authority validation happens at the mint_action level
    native_compression(
        None, // No authority needed for decompression
        amount,
        mint.into(),
        token_account_info,
        CompressionMode::Decompress,
    )?;
    Ok(updated_supply)
}

fn handle_decompressed_mint_to_token_pool(
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

            msg!(
                "Minting {} tokens to token pool for decompressed action",
                amount
            );
            mint_to_token_pool(
                mint_account,
                token_pool_account,
                token_program,
                validated_accounts.cpi_authority()?,
                amount,
            )?;
        }
    }
    Ok(())
}
