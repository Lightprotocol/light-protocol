use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::Pubkey;
use light_ctoken_types::{
    instructions::mint_action::ZMintToDecompressedAction, state::ZCompressedMintMut,
};
use light_profiler::profile;
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use crate::{
    mint_action::{accounts::MintActionAccounts, check_authority},
    shared::mint_to_token_pool,
    transfer2::compression::{compress_ctokens, NativeCompressionInputs},
};

#[allow(clippy::too_many_arguments)]
#[profile]
pub fn process_mint_to_decompressed_action(
    action: &ZMintToDecompressedAction,
    current_supply: u64,
    compressed_mint: &ZCompressedMintMut<'_>,
    validated_accounts: &MintActionAccounts,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    mint: Pubkey,
    instruction_mint_authority: Option<Pubkey>,
) -> Result<u64, ProgramError> {
    check_authority(
        compressed_mint.base.mint_authority(),
        instruction_mint_authority,
        validated_accounts.authority.key(),
        "mint authority",
    )?;

    let amount = u64::from(action.recipient.amount);
    let updated_supply = current_supply
        .checked_add(amount)
        .ok_or(ErrorCode::MintActionAmountTooLarge)?;

    handle_spl_mint_initialized_token_pool(
        validated_accounts,
        compressed_mint.metadata.spl_mint_initialized(),
        amount,
        mint,
    )?;

    // Get the recipient token account from packed accounts using the index
    let token_account_info = packed_accounts.get_u8(
        action.recipient.account_index,
        "decompressed mint to recipient",
    )?;

    // Authority check now performed above - safe to proceed with decompression
    // Use the decompress_only constructor for simple decompression operations
    let inputs = NativeCompressionInputs::decompress_only(
        amount,
        mint.to_bytes(),
        token_account_info,
        packed_accounts,
    );
    // For mint_to_decompressed, we don't need to handle lamport transfers
    // as there's no compressible extension on the target account
    compress_ctokens(inputs)?;
    Ok(updated_supply)
}

#[profile]
pub fn handle_spl_mint_initialized_token_pool(
    validated_accounts: &MintActionAccounts,
    spl_mint_initialized: bool,
    amount: u64,
    mint: Pubkey,
) -> Result<(), ProgramError> {
    if let Some(system_accounts) = validated_accounts.executing.as_ref() {
        // If SPL mint is initialized, mint tokens to the token pool to maintain SPL mint supply consistency
        if spl_mint_initialized {
            let mint_account = system_accounts
                .mint
                .ok_or(ErrorCode::MintActionMissingMintAccount)?;
            if mint.to_bytes() != *mint_account.key() {
                msg!("Mint account mismatch");
                return Err(ErrorCode::MintAccountMismatch.into());
            }
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
    } else if spl_mint_initialized {
        msg!("if SPL mint is initialized, executing accounts must be present");
        return Err(ErrorCode::Transfer2CpiContextWriteInvalidAccess.into());
    }
    Ok(())
}
