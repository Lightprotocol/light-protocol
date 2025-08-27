use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_types::{
    hash_cache::HashCache, instructions::mint_action::ZMintToAction, state::ZCompressedMintMut,
};
use light_sdk_pinocchio::ZOutputCompressedAccountWithPackedContextMut;
use spl_pod::solana_msg::msg;

use crate::{
    mint_action::{
        accounts::{AccountsConfig, MintActionAccounts},
        mint_to_decompressed::handle_decompressed_mint_to_token_pool,
    },
    shared::token_output::set_output_compressed_account,
};

#[inline(always)]
pub fn mint_authority_check(
    compressed_mint: &ZCompressedMintMut<'_>,
    validated_accounts: &MintActionAccounts,
    instruction_fallback: Option<Pubkey>,
) -> Result<(), ErrorCode> {
    // Get current authority (from field or instruction fallback)
    let mint_authority = compressed_mint
        .base
        .mint_authority
        .as_ref()
        .map(|a| **a)
        .or(instruction_fallback)
        .ok_or(ErrorCode::InvalidAuthorityMint)?;

    if *validated_accounts.authority.key() != mint_authority.to_bytes() {
        use anchor_lang::prelude::msg;
        msg!(
            "authority.key()  {:?} != mint {:?}",
            solana_pubkey::Pubkey::new_from_array(*validated_accounts.authority.key()),
            solana_pubkey::Pubkey::new_from_array(mint_authority.to_bytes())
        );
        Err(ErrorCode::InvalidAuthorityMint)
    } else {
        Ok(())
    }
}

/// Processes a mint-to action by validating authority, calculating amounts, and creating compressed token accounts.
///
/// ## Process Steps
/// 1. **Authority Validation**: Verify signer matches current mint authority from compressed mint state
/// 2. **Amount Calculation**: Sum recipient amounts with overflow protection
/// 3. **Lamports Calculation**: Calculate total lamports for compressed accounts (if specified)
/// 4. **Supply Update**: Calculate new total supply with overflow protection
/// 5. **SPL Mint Synchronization**: For decompressed mints, validate accounts and mint equivalent tokens to token pool via CPI
/// 6. **Compressed Account Creation**: Create new compressed token account for each recipient
///
/// ## Decompressed Mint Handling
/// Decompressed mint means that an spl mint exists for this compressed mint.
/// When `accounts_config.is_decompressed` is true, the function maintains consistency between the compressed
/// token supply and the underlying SPL mint supply by minting equivalent tokens to a program-controlled
/// token pool account via CPI to SPL Token 2022.
#[allow(clippy::too_many_arguments)]
pub fn process_mint_to_action(
    action: &ZMintToAction,
    compressed_mint: &ZCompressedMintMut<'_>,
    validated_accounts: &MintActionAccounts,
    accounts_config: &AccountsConfig,
    cpi_instruction_struct: &mut [ZOutputCompressedAccountWithPackedContextMut<'_>],
    hash_cache: &mut HashCache,
    mint: Pubkey,
    out_token_queue_index: u8,
    instruction_mint_authority: Option<Pubkey>,
) -> Result<u64, ProgramError> {
    mint_authority_check(
        compressed_mint,
        validated_accounts,
        instruction_mint_authority,
    )?;

    let mut sum_amounts: u64 = 0;
    for recipient in &action.recipients {
        sum_amounts = sum_amounts
            .checked_add(u64::from(recipient.amount))
            .ok_or(ErrorCode::MintActionAmountTooLarge)?;
    }

    let updated_supply = sum_amounts
        .checked_add(compressed_mint.base.supply.into())
        .ok_or(ErrorCode::MintActionAmountTooLarge)?;

    handle_decompressed_mint_to_token_pool(validated_accounts, accounts_config, sum_amounts, mint)?;

    // Create output token accounts
    create_output_compressed_token_accounts(
        action,
        cpi_instruction_struct,
        hash_cache,
        mint,
        out_token_queue_index,
    )?;
    Ok(updated_supply)
}

fn create_output_compressed_token_accounts(
    parsed_instruction_data: &ZMintToAction<'_>,
    output_compressed_accounts: &mut [ZOutputCompressedAccountWithPackedContextMut<'_>],
    hash_cache: &mut HashCache,
    mint: Pubkey,
    queue_pubkey_index: u8,
) -> Result<(), ProgramError> {
    let hashed_mint = hash_cache.get_or_hash_mint(&mint.to_bytes())?;

    let lamports = parsed_instruction_data
        .lamports
        .map(|lamports| u64::from(*lamports));
    for (recipient, output_account) in parsed_instruction_data
        .recipients
        .iter()
        .zip(output_compressed_accounts.iter_mut())
    {
        let output_delegate = None;
        set_output_compressed_account::<false>(
            output_account,
            hash_cache,
            recipient.recipient,
            output_delegate,
            recipient.amount,
            lamports,
            mint,
            &hashed_mint,
            queue_pubkey_index,
            parsed_instruction_data.token_account_version,
        )?;
    }
    Ok(())
}
