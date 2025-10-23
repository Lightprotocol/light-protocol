use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_types::{
    hash_cache::HashCache, instructions::mint_action::ZMintToCompressedAction,
    state::CompressedMint,
};
use light_program_profiler::profile;
use light_sdk_pinocchio::instruction::ZOutputCompressedAccountWithPackedContextMut;

use crate::{
    mint_action::{
        accounts::MintActionAccounts, check_authority,
        mint_to_ctoken::handle_spl_mint_initialized_token_pool,
    },
    shared::token_output::set_output_compressed_account,
};

/// Processes a mint-to action by validating authority, calculating amounts, and creating compressed token accounts.
///
/// ## Process Steps
/// 1. **Authority Validation**: Verify signer matches current mint authority from compressed mint state
/// 2. **Amount Calculation**: Sum recipient amounts with overflow protection
/// 3. **Lamports Calculation**: Calculate total lamports for compressed accounts (if specified)
/// 4. **Supply Update**: Calculate new total supply with overflow protection
/// 5. **SPL Mint Synchronization**: For initialized SPL mints, validate accounts and mint equivalent tokens to token pool via CPI
/// 6. **Compressed Account Creation**: Create new compressed token account for each recipient
///
/// ## SPL Mint Synchronization
/// When `accounts_config.spl_mint_initialized` is true, an SPL mint exists for this compressed mint.
/// The function maintains consistency between the compressed token supply and the underlying SPL mint supply
/// by minting equivalent tokens to a program-controlled token pool account via CPI to SPL Token 2022.
#[allow(clippy::too_many_arguments)]
#[profile]
pub fn process_mint_to_compressed_action<'a>(
    action: &ZMintToCompressedAction,
    compressed_mint: &mut CompressedMint,
    validated_accounts: &MintActionAccounts,
    output_accounts_iter: &mut impl Iterator<
        Item = &'a mut ZOutputCompressedAccountWithPackedContextMut<'a>,
    >,
    hash_cache: &mut HashCache,
    mint: Pubkey,
    out_token_queue_index: u8,
) -> Result<(), ProgramError> {
    check_authority(
        compressed_mint.base.mint_authority,
        validated_accounts.authority.key(),
        "mint_to_compressed: mint authority",
    )?;

    let mut sum_amounts: u64 = 0;
    for recipient in &action.recipients {
        sum_amounts = sum_amounts
            .checked_add(u64::from(recipient.amount))
            .ok_or(ErrorCode::MintActionAmountTooLarge)?;
    }

    compressed_mint.base.supply = sum_amounts
        .checked_add(compressed_mint.base.supply)
        .ok_or(ErrorCode::MintActionAmountTooLarge)?;

    // Check SPL mint initialization from compressed mint state, not config
    handle_spl_mint_initialized_token_pool(
        validated_accounts,
        compressed_mint.metadata.spl_mint_initialized,
        sum_amounts,
        mint,
    )?;

    // Create output token accounts
    create_output_compressed_token_accounts(
        action,
        output_accounts_iter,
        hash_cache,
        mint,
        out_token_queue_index,
    )?;
    Ok(())
}

#[profile]
fn create_output_compressed_token_accounts<'a>(
    parsed_instruction_data: &ZMintToCompressedAction<'_>,
    output_accounts_iter: &mut impl Iterator<
        Item = &'a mut ZOutputCompressedAccountWithPackedContextMut<'a>,
    >,
    hash_cache: &mut HashCache,
    mint: Pubkey,
    queue_pubkey_index: u8,
) -> Result<(), ProgramError> {
    let expected_recipients = parsed_instruction_data.recipients.len();
    let mut processed_count = 0;

    for (recipient, output_account) in parsed_instruction_data
        .recipients
        .iter()
        .zip(output_accounts_iter)
    {
        let output_delegate = None;
        set_output_compressed_account(
            output_account,
            hash_cache,
            recipient.recipient,
            output_delegate,
            recipient.amount,
            None::<u64>,
            mint,
            queue_pubkey_index,
            parsed_instruction_data.token_account_version,
        )?;
        processed_count += 1;
    }

    // Validate that we processed all expected recipients
    if processed_count != expected_recipients {
        return Err(ErrorCode::MintActionOutputSerializationFailed.into());
    }

    Ok(())
}
