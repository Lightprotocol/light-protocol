use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::instruction_data::data::ZOutputCompressedAccountWithPackedContextMut;
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::mint_action::{ZAction, ZMintActionCompressedInstructionData},
    state::ZCompressedMintMut,
};
use light_profiler::profile;
use pinocchio::account_info::AccountInfo;

use crate::mint_action::{
    accounts::MintActionAccounts,
    check_authority,
    create_spl_mint::process_create_spl_mint_action,
    mint_to::process_mint_to_action,
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
            ZAction::MintToCompressed(action) => {
                let new_supply = process_mint_to_action(
                    action,
                    compressed_mint,
                    validated_accounts,
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
                check_authority(
                    compressed_mint.base.mint_authority(),
                    parsed_instruction_data
                        .mint
                        .mint_authority
                        .as_ref()
                        .map(|a| **a),
                    validated_accounts.authority.key(),
                    "mint authority",
                )?;
                compressed_mint
                    .base
                    .set_mint_authority(update_action.new_authority.map(|e| *e));
            }
            ZAction::UpdateFreezeAuthority(update_action) => {
                check_authority(
                    compressed_mint.base.freeze_authority(),
                    parsed_instruction_data
                        .mint
                        .freeze_authority
                        .as_ref()
                        .map(|a| **a),
                    validated_accounts.authority.key(),
                    "freeze authority",
                )?;
                compressed_mint
                    .base
                    .set_freeze_authority(update_action.new_authority.map(|e| *e));
            }
            ZAction::CreateSplMint(create_spl_action) => {
                process_create_spl_mint_action(
                    create_spl_action,
                    validated_accounts,
                    &parsed_instruction_data.mint,
                    parsed_instruction_data.token_pool_bump,
                )?;
            }
            ZAction::MintToCToken(mint_to_ctoken_action) => {
                let new_supply = process_mint_to_ctoken_action(
                    mint_to_ctoken_action,
                    compressed_mint.base.supply.get(),
                    compressed_mint,
                    validated_accounts,
                    packed_accounts,
                    parsed_instruction_data.mint.metadata.spl_mint,
                    parsed_instruction_data
                        .mint
                        .mint_authority
                        .as_ref()
                        .map(|a| **a),
                )?;
                *compressed_mint.base.supply = new_supply.into();
            }
            ZAction::UpdateMetadataField(update_metadata_action) => {
                process_update_metadata_field_action(
                    update_metadata_action,
                    compressed_mint,
                    &validated_metadata_authority,
                )?;
            }
            ZAction::UpdateMetadataAuthority(update_metadata_authority_action) => {
                process_update_metadata_authority_action(
                    update_metadata_authority_action,
                    compressed_mint,
                    &mut validated_metadata_authority,
                )?;
            }
            ZAction::RemoveMetadataKey(remove_metadata_key_action) => {
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
