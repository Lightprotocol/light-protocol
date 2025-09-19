use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::instruction_data::data::ZOutputCompressedAccountWithPackedContextMut;
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::{
        extensions::ZExtensionInstructionData,
        mint_action::{ZAction, ZMintActionCompressedInstructionData},
    },
    state::ZCompressedMintMut,
};
use light_profiler::profile;
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use crate::mint_action::{
    accounts::{AccountsConfig, MintActionAccounts},
    create_spl_mint::process_create_spl_mint_action,
    mint_to::process_mint_to_action,
    mint_to_decompressed::process_mint_to_decompressed_action,
    queue_indices::QueueIndices,
    update_authority::validate_and_update_authority,
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
    accounts_config: &AccountsConfig,
    cpi_instruction_struct: &'a mut [ZOutputCompressedAccountWithPackedContextMut<'a>],
    hash_cache: &mut HashCache,
    queue_indices: &QueueIndices,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    compressed_mint: &mut ZCompressedMintMut<'a>,
) -> Result<(), ProgramError> {
    // Centralized authority validation - extract and validate authorities at the start
    let signer_key = *validated_accounts.authority.key();

    // Validate mint authority
    let mut _validated_mint_authority = None;
    if let Some(current_mint_auth) = parsed_instruction_data.mint.mint_authority.as_ref() {
        if current_mint_auth.to_bytes() == signer_key {
            _validated_mint_authority = Some(**current_mint_auth);
        }
    }

    // Start metadata authority with same value as mint authority
    let mut validated_metadata_authority = Some(light_compressed_account::Pubkey::from(signer_key));
    for action in parsed_instruction_data.actions.iter() {
        match action {
            ZAction::MintTo(action) => {
                let new_supply = process_mint_to_action(
                    action,
                    compressed_mint,
                    validated_accounts,
                    accounts_config,
                    cpi_instruction_struct,
                    hash_cache,
                    parsed_instruction_data.mint.metadata.spl_mint,
                    queue_indices.out_token_queue_index,
                    parsed_instruction_data
                        .mint
                        .mint_authority
                        .as_ref()
                        .map(|a| **a),
                )?;
                *compressed_mint.base.supply = new_supply.into();
            }
            ZAction::UpdateMintAuthority(update_action) => {
                msg!("Processing UpdateMintAuthority action");
                validate_and_update_authority(
                    compressed_mint.base.mint_authority(),
                    parsed_instruction_data
                        .mint
                        .mint_authority
                        .as_ref()
                        .map(|a| **a),
                    // update_action,
                    validated_accounts.authority.key(),
                    "mint authority",
                )?;
                compressed_mint
                    .base
                    .set_mint_authority(update_action.new_authority.map(|e| *e));
            }
            ZAction::UpdateFreezeAuthority(update_action) => {
                msg!("Processing UpdateFreezeAuthority action");
                validate_and_update_authority(
                    compressed_mint.base.freeze_authority(),
                    parsed_instruction_data
                        .mint
                        .freeze_authority
                        .as_ref()
                        .map(|a| **a),
                    // update_action,
                    validated_accounts.authority.key(),
                    "freeze authority",
                )?;
                compressed_mint
                    .base
                    .set_freeze_authority(update_action.new_authority.map(|e| *e));
            }
            ZAction::CreateSplMint(create_spl_action) => {
                msg!("Processing CreateSplMint action");
                process_create_spl_mint_action(
                    create_spl_action,
                    validated_accounts,
                    &parsed_instruction_data.mint,
                )?;
            }
            ZAction::MintToDecompressed(mint_to_decompressed_action) => {
                msg!("Processing MintToDecompressed action");
                let new_supply = process_mint_to_decompressed_action(
                    mint_to_decompressed_action,
                    compressed_mint.base.supply.get(),
                    compressed_mint,
                    validated_accounts,
                    accounts_config,
                    packed_accounts,
                    parsed_instruction_data.mint.metadata.spl_mint,
                    parsed_instruction_data
                        .mint
                        .mint_authority
                        .as_ref()
                        .map(|a| **a),
                )?;
                *compressed_mint.base.supply = new_supply.into();
                msg!("done Processing MintToDecompressed action");
            }
            ZAction::UpdateMetadataField(update_metadata_action) => {
                msg!("Processing UpdateMetadataField action - START");
                msg!(
                    "UpdateMetadataField: extension_index={}, field_type={}, value_len={}",
                    update_metadata_action.extension_index,
                    update_metadata_action.field_type,
                    update_metadata_action.value.len()
                );
                process_update_metadata_field_action(
                    update_metadata_action,
                    compressed_mint,
                    &validated_metadata_authority,
                )?;
                msg!("Processing UpdateMetadataField action - COMPLETE");
            }
            ZAction::UpdateMetadataAuthority(update_metadata_authority_action) => {
                msg!("Processing UpdateMetadataAuthority action");
                let old_authority = parsed_instruction_data
                    .mint
                    .extensions
                    .as_ref()
                    .and_then(|extensions| {
                        extensions.get(update_metadata_authority_action.extension_index as usize)
                    })
                    .and_then(|ext| match ext {
                        ZExtensionInstructionData::TokenMetadata(metadata_extension) => {
                            metadata_extension.update_authority
                        }
                        _ => None,
                    });
                process_update_metadata_authority_action(
                    update_metadata_authority_action,
                    compressed_mint,
                    &old_authority,
                    &mut validated_metadata_authority,
                )?;
            }
            ZAction::RemoveMetadataKey(remove_metadata_key_action) => {
                msg!("Processing RemoveMetadataKey action");
                process_remove_metadata_key_action(
                    remove_metadata_key_action,
                    compressed_mint,
                    &validated_metadata_authority,
                )?;
            }
        }
    }

    Ok(())
}
