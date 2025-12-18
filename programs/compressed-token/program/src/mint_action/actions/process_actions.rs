use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use arrayvec::ArrayVec;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::instruction_data::data::ZOutputCompressedAccountWithPackedContextMut;
use light_ctoken_interface::{
    hash_cache::HashCache,
    instructions::mint_action::{ZAction, ZMintActionCompressedInstructionData},
    state::CompressedMint,
    CTokenError,
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use crate::{
    mint_action::{
        accounts::MintActionAccounts,
        check_authority,
        compress_and_close_cmint::process_compress_and_close_cmint_action,
        decompress_mint::process_decompress_mint_action,
        mint_to::process_mint_to_compressed_action,
        mint_to_ctoken::process_mint_to_ctoken_action,
        queue_indices::QueueIndices,
        update_metadata::{
            process_remove_metadata_key_action, process_update_metadata_authority_action,
            process_update_metadata_field_action,
        },
    },
    shared::{
        convert_program_error,
        transfer_lamports::{multi_transfer_lamports, Transfer},
    },
    MAX_PACKED_ACCOUNTS,
};

#[allow(clippy::too_many_arguments)]
#[profile]
pub fn process_actions<'a>(
    parsed_instruction_data: &ZMintActionCompressedInstructionData,
    validated_accounts: &MintActionAccounts,
    output_accounts_iter: &mut impl Iterator<
        Item = &'a mut ZOutputCompressedAccountWithPackedContextMut<'a>,
    >,
    hash_cache: &mut HashCache,
    queue_indices: &QueueIndices,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    compressed_mint: &mut CompressedMint,
) -> Result<(), ProgramError> {
    // Array to accumulate transfer amounts by account index
    let mut transfer_map = [0u64; MAX_PACKED_ACCOUNTS];
    // Initialize budget: +1 allows exact match (total == max_top_up)
    let max_top_up: u16 = parsed_instruction_data.max_top_up.get();
    let mut lamports_budget = (max_top_up as u64).saturating_add(1);

    // Start metadata authority with same value as mint authority
    for action in parsed_instruction_data.actions.iter() {
        match action {
            ZAction::MintToCompressed(action) => {
                process_mint_to_compressed_action(
                    action,
                    compressed_mint,
                    validated_accounts,
                    output_accounts_iter,
                    hash_cache,
                    compressed_mint.metadata.mint,
                    queue_indices.out_token_queue_index,
                )?;
            }
            ZAction::UpdateMintAuthority(update_action) => {
                check_authority(
                    compressed_mint.base.mint_authority,
                    validated_accounts.authority.key(),
                    "mint authority",
                )?;
                compressed_mint.base.mint_authority =
                    update_action.new_authority.as_ref().map(|a| **a);
            }
            ZAction::UpdateFreezeAuthority(update_action) => {
                check_authority(
                    compressed_mint.base.freeze_authority,
                    validated_accounts.authority.key(),
                    "freeze authority",
                )?;

                compressed_mint.base.freeze_authority =
                    update_action.new_authority.as_ref().map(|a| **a);
            }
            ZAction::CreateSplMint(_create_spl_action) => {
                // The creation of an associated spl mint is not activated.
                return Err(ErrorCode::MintActionUnsupportedOperation.into());
                // process_create_spl_mint_action(
                //     create_spl_action,
                //     validated_accounts,
                //     &parsed_instruction_data.mint,
                //     parsed_instruction_data.token_pool_bump,
                // )?;
                // compressed_mint.metadata.cmint_decompressed = true;
            }
            ZAction::MintToCToken(mint_to_ctoken_action) => {
                let account_index = mint_to_ctoken_action.account_index as usize;
                if account_index >= MAX_PACKED_ACCOUNTS {
                    msg!(
                        "Account index {} out of bounds, max {} allowed",
                        account_index,
                        MAX_PACKED_ACCOUNTS
                    );
                    return Err(ErrorCode::TooManyCompressionTransfers.into());
                }
                process_mint_to_ctoken_action(
                    mint_to_ctoken_action,
                    compressed_mint,
                    validated_accounts,
                    packed_accounts,
                    compressed_mint.metadata.mint,
                    &mut transfer_map[account_index],
                    &mut lamports_budget,
                )?;
            }
            ZAction::UpdateMetadataField(update_metadata_action) => {
                process_update_metadata_field_action(
                    update_metadata_action,
                    compressed_mint,
                    validated_accounts.authority.key(),
                )?;
            }
            ZAction::UpdateMetadataAuthority(update_metadata_authority_action) => {
                process_update_metadata_authority_action(
                    update_metadata_authority_action,
                    compressed_mint,
                    validated_accounts.authority.key(),
                )?;
            }
            ZAction::RemoveMetadataKey(remove_metadata_key_action) => {
                process_remove_metadata_key_action(
                    remove_metadata_key_action,
                    compressed_mint,
                    validated_accounts.authority.key(),
                )?;
            }
            ZAction::DecompressMint(decompress_action) => {
                let mint_signer = validated_accounts
                    .mint_signer
                    .ok_or(ErrorCode::MintActionMissingMintSigner)?;
                let fee_payer = validated_accounts
                    .executing
                    .as_ref()
                    .map(|exec| exec.system.fee_payer)
                    .ok_or_else(|| {
                        msg!("Fee payer required for DecompressMint action");
                        ProgramError::NotEnoughAccountKeys
                    })?;
                process_decompress_mint_action(
                    decompress_action,
                    compressed_mint,
                    validated_accounts,
                    mint_signer,
                    fee_payer,
                )?;
            }
            ZAction::CompressAndCloseCMint(action) => {
                process_compress_and_close_cmint_action(
                    action,
                    compressed_mint,
                    validated_accounts,
                )?;
            }
        }
    }

    // Build transfers array from deduplicated map
    let transfers: ArrayVec<Transfer, MAX_PACKED_ACCOUNTS> = transfer_map
        .iter()
        .enumerate()
        .filter_map(|(index, &amount)| {
            if amount != 0 {
                Some((index as u8, amount))
            } else {
                None
            }
        })
        .map(|(index, amount)| {
            Ok(Transfer {
                account: packed_accounts.get_u8(index, "transfer account")?,
                amount,
            })
        })
        .collect::<Result<ArrayVec<Transfer, MAX_PACKED_ACCOUNTS>, ProgramError>>()?;

    // Execute transfers if any exist
    if !transfers.is_empty() {
        // Check budget wasn't exhausted (0 means exceeded max_top_up)
        if max_top_up != 0 && lamports_budget == 0 {
            return Err(CTokenError::MaxTopUpExceeded.into());
        }

        let fee_payer = validated_accounts
            .executing
            .as_ref()
            .map(|exec| exec.system.fee_payer)
            .ok_or_else(|| {
                msg!("Fee payer required for compressible token account top-ups");
                ProgramError::NotEnoughAccountKeys
            })?;
        multi_transfer_lamports(fee_payer, &transfers).map_err(convert_program_error)?;
    }

    Ok(())
}
