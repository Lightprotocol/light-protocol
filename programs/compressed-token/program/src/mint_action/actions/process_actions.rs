use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::instruction_data::data::ZOutputCompressedAccountWithPackedContextMut;
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::mint_action::{ZAction, ZMintActionCompressedInstructionData},
    state::CompressedMint,
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;

use crate::mint_action::{
    accounts::MintActionAccounts,
    check_authority,
    mint_to::process_mint_to_compressed_action,
    mint_to_ctoken::process_mint_to_ctoken_action,
    queue_indices::QueueIndices,
    update_metadata::{
        process_remove_metadata_key_action, process_update_metadata_authority_action,
        process_update_metadata_field_action,
    },
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
                    parsed_instruction_data.mint.metadata.mint,
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
                // compressed_mint.metadata.spl_mint_initialized = true;
            }
            ZAction::MintToCToken(mint_to_ctoken_action) => {
                process_mint_to_ctoken_action(
                    mint_to_ctoken_action,
                    compressed_mint,
                    validated_accounts,
                    packed_accounts,
                    parsed_instruction_data.mint.metadata.mint,
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
        }
    }

    Ok(())
}
