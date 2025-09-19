use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_types::{
    hash_cache::HashCache, instructions::mint_action::ZMintToAction, state::ZCompressedMintMut,
};
use light_profiler::profile;
use light_sdk_pinocchio::ZOutputCompressedAccountWithPackedContextMut;

use crate::{
    mint_action::{
        accounts::MintActionAccounts, check_authority,
        mint_to_decompressed::handle_spl_mint_initialized_token_pool,
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
pub fn process_mint_to_action(
    action: &ZMintToAction,
    compressed_mint: &ZCompressedMintMut<'_>,
    validated_accounts: &MintActionAccounts,
    cpi_instruction_struct: &mut [ZOutputCompressedAccountWithPackedContextMut<'_>],
    hash_cache: &mut HashCache,
    mint: Pubkey,
    out_token_queue_index: u8,
    instruction_mint_authority: Option<Pubkey>,
) -> Result<u64, ProgramError> {
    check_authority(
        compressed_mint.base.mint_authority(),
        instruction_mint_authority,
        validated_accounts.authority.key(),
        "mint authority",
    )?;

    let mut sum_amounts: u64 = 0;
    for recipient in &action.recipients {
        sum_amounts = sum_amounts
            .checked_add(u64::from(recipient.amount))
            .ok_or(ErrorCode::MintActionAmountTooLarge)?;
    }

    let updated_supply = sum_amounts
        .checked_add((*compressed_mint.base.supply).into())
        .ok_or(ErrorCode::MintActionAmountTooLarge)?;

    // Check SPL mint initialization from compressed mint state, not config
    handle_spl_mint_initialized_token_pool(
        validated_accounts,
        compressed_mint.metadata.spl_mint_initialized(),
        sum_amounts,
        mint,
    )?;

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

#[profile]
fn create_output_compressed_token_accounts(
    parsed_instruction_data: &ZMintToAction<'_>,
    output_compressed_accounts: &mut [ZOutputCompressedAccountWithPackedContextMut<'_>],
    hash_cache: &mut HashCache,
    mint: Pubkey,
    queue_pubkey_index: u8,
) -> Result<(), ProgramError> {
    for (recipient, output_account) in parsed_instruction_data
        .recipients
        .iter()
        .zip(output_compressed_accounts.iter_mut())
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
    }
    Ok(())
}
