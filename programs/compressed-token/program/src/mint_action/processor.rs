use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_compressed_account::{
    instruction_data::{
        data::ZOutputCompressedAccountWithPackedContextMut,
        with_readonly::InstructionDataInvokeCpiWithReadOnly,
    },
    Pubkey,
};
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::mint_actions::{
        MintActionCompressedInstructionData, ZAction, ZMintActionCompressedInstructionData,
    },
    state::{CompressedMint, ZCompressedMintMut},
};
use light_sdk::instruction::PackedMerkleContext;
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;
use spl_token::solana_program::log::sol_log_compute_units;

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    extensions::processor::extensions_state_in_output_compressed_account,
    mint_action::{
        accounts::{AccountsConfig, MintActionAccounts},
        create_mint::process_create_mint_action,
        create_spl_mint::process_create_spl_mint_action,
        mint_input::create_input_compressed_mint_account,
        mint_to::process_mint_to_action,
        mint_to_decompressed::process_mint_to_decompressed_action,
        queue_indices::QueueIndices,
        update_authority::update_authority,
        update_metadata::{
            process_remove_metadata_key_action, process_update_metadata_authority_action,
            process_update_metadata_field_action,
        },
        zero_copy_config::get_zero_copy_configs,
    },
    shared::cpi::execute_cpi_invoke,
    transfer2::accounts::ProgramPackedAccounts,
};

// Create mint - no input
// Mint to - mint input, mint output with increased supply, if spl mint exists
// Update mint - mint input, mint output, update mint or freeze authority

/// Checks:
/// 1. check mint_signer (compressed mint randomness) is signer
/// 2.
pub fn process_mint_action(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();
    // 677 CU
    let (mut parsed_instruction_data, _) =
        MintActionCompressedInstructionData::zero_copy_at(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
    msg!("parsed_instruction_data  {:?}", parsed_instruction_data);

    sol_log_compute_units();
    // 112 CU write to cpi contex
    let accounts_config = AccountsConfig::new(&parsed_instruction_data);
    msg!("accounts_config {:?}", accounts_config);
    // Validate and parse
    let validated_accounts = MintActionAccounts::validate_and_parse(accounts, &accounts_config)?;
    sol_log_compute_units();

    let (config, mut cpi_bytes, mint_size_config, idempotent) =
        get_zero_copy_configs(&mut parsed_instruction_data)?;
    msg!("post get_zero_copy_configs config {:?}", config);
    msg!("post mint_size_config {:?}", mint_size_config);
    sol_log_compute_units();
    let (mut cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;
    cpi_instruction_struct.initialize(
        crate::LIGHT_CPI_SIGNER.bump,
        &crate::LIGHT_CPI_SIGNER.program_id.into(),
        parsed_instruction_data.proof,
        &parsed_instruction_data.cpi_context,
    )?;
    if !accounts_config.write_to_cpi_context
        && !parsed_instruction_data.prove_by_index()
        && parsed_instruction_data.proof.is_none()
    {
        msg!("Proof missing");
        return Err(ErrorCode::MintActionProofMissing.into());
    }

    sol_log_compute_units();
    let mut hash_cache = HashCache::new();
    let queue_indices = QueueIndices::new(&parsed_instruction_data, &validated_accounts)?;
    let compressed_lamports = parsed_instruction_data
        .actions
        .iter()
        .map(|action| {
            if let ZAction::MintTo(action) = action {
                if let Some(lamports) = action.lamports {
                    u64::from(*lamports)
                } else {
                    0
                }
            } else {
                0
            }
        })
        .sum::<u64>();
    cpi_instruction_struct.compress_or_decompress_lamports = compressed_lamports.into();
    cpi_instruction_struct.is_compress = if compressed_lamports > 0 { 1 } else { 0 };
    // If create mint
    // 1. derive spl mint pda
    // 2. set create address
    // else
    // 1. set input compressed mint account
    if parsed_instruction_data.create_mint() {
        process_create_mint_action(
            &parsed_instruction_data,
            &validated_accounts,
            &mut cpi_instruction_struct,
            &mint_size_config,
        )?;
    } else {
        // Process input compressed mint account
        create_input_compressed_mint_account(
            &mut cpi_instruction_struct.input_compressed_accounts[0],
            &mut hash_cache,
            &parsed_instruction_data,
            PackedMerkleContext {
                merkle_tree_pubkey_index: queue_indices.in_tree_index,
                queue_pubkey_index: queue_indices.in_queue_index,
                leaf_index: parsed_instruction_data.leaf_index.into(),
                prove_by_index: parsed_instruction_data.prove_by_index(),
            },
        )?;
    }
    {
        let freeze_authority = parsed_instruction_data.mint.freeze_authority.map(|fa| *fa);
        let mint_authority = parsed_instruction_data.mint.mint_authority.map(|fa| *fa);

        let (mint_account, token_accounts): (
            &mut ZOutputCompressedAccountWithPackedContextMut<'_>,
            &mut [ZOutputCompressedAccountWithPackedContextMut<'_>],
        ) = if cpi_instruction_struct.output_compressed_accounts.len() == 1 {
            (
                &mut cpi_instruction_struct.output_compressed_accounts[0],
                &mut [],
            )
        } else {
            let (mint_account, token_accounts) = cpi_instruction_struct
                .output_compressed_accounts
                .split_at_mut(1);
            (&mut mint_account[0], token_accounts)
        };
        let mint_pda = parsed_instruction_data.mint.spl_mint;

        // 2. Set output compressed account
        mint_account.set(
            crate::LIGHT_CPI_SIGNER.program_id.into(),
            0,
            Some(parsed_instruction_data.compressed_address),
            queue_indices.output_queue_index,
            COMPRESSED_MINT_DISCRIMINATOR,
            [0u8; 32],
        )?;

        let compressed_account_data = mint_account
            .compressed_account
            .data
            .as_mut()
            .ok_or(ErrorCode::MintActionOutputSerializationFailed)?;

        let (mut compressed_mint, _) =
            CompressedMint::new_zero_copy(compressed_account_data.data, mint_size_config)
                .map_err(|_| ErrorCode::MintActionOutputSerializationFailed)?;
        compressed_mint.set(
            parsed_instruction_data.mint.version,
            mint_pda,
            parsed_instruction_data.mint.supply,
            parsed_instruction_data.mint.decimals,
            accounts_config.is_decompressed,
            mint_authority,
            freeze_authority,
        )?;
        if let Some(extensions) = parsed_instruction_data.mint.extensions.as_deref() {
            let z_extensions = compressed_mint
                .extensions
                .as_mut()
                .ok_or(ProgramError::AccountAlreadyInitialized)?;

            extensions_state_in_output_compressed_account(
                extensions,
                z_extensions.as_mut_slice(),
                mint_pda,
            )?;
        }

        process_actions(
            &parsed_instruction_data,
            &validated_accounts,
            &accounts_config,
            token_accounts,
            &mut hash_cache,
            &queue_indices,
            &validated_accounts.packed_accounts,
            &mut compressed_mint,
        )?;
        msg!("pre data hash");
        *compressed_account_data.data_hash = compressed_mint.hash(&mut hash_cache)?;
        msg!("post data hash");
    };
    sol_log_compute_units();

    msg!("queue_indices {:?}", queue_indices);
    let cpi_accounts_offset = validated_accounts.cpi_accounts_start_offset();
    let end_offset = validated_accounts.cpi_accounts_end_offset(queue_indices.deduplicated);
    msg!("cpi accounts start offset: {}", cpi_accounts_offset);
    msg!("cpi accounts end offset: {}", end_offset);
    msg!(
        "account info pubkeys {:?}",
        accounts[cpi_accounts_offset..end_offset]
            .iter()
            .map(|info| solana_pubkey::Pubkey::new_from_array(*info.key()))
            .collect::<Vec<_>>()
    );
    // TODO: implement a more robust end offset calculation than - num_decompressed_recipients as usize
    let res = if let Some(executing) = validated_accounts.executing.as_ref() {
        // Execute CPI to light-system-program
        execute_cpi_invoke(
            &accounts[cpi_accounts_offset..end_offset],
            cpi_bytes,
            validated_accounts
                .tree_pubkeys(queue_indices.deduplicated)
                .as_slice(),
            accounts_config.with_lamports,
            None,
            executing.system.cpi_context.map(|x| *x.key()),
            false, // write to cpi context account
        )
    } else {
        execute_cpi_invoke(
            &accounts[cpi_accounts_offset..cpi_accounts_offset + 3],
            cpi_bytes,
            &[],
            false, // no sol_pool_pda for create_compressed_mint
            None,
            validated_accounts
                .write_to_cpi_context_system
                .as_ref()
                .map(|x| *x.cpi_context.key()),
            true,
        )
    };
    // idempotent can be passed with key removal
    // TODO: consider limiting use to sole key removal.
    if idempotent {
        Ok(())
    } else {
        res
    }
}

fn process_actions<'a>(
    parsed_instruction_data: &ZMintActionCompressedInstructionData,
    validated_accounts: &MintActionAccounts,
    accounts_config: &AccountsConfig,
    cpi_instruction_struct: &'a mut [ZOutputCompressedAccountWithPackedContextMut<'a>],
    hash_cache: &mut HashCache,
    queue_indices: &QueueIndices,
    packed_accounts: &ProgramPackedAccounts,
    compressed_mint: &mut ZCompressedMintMut<'a>,
) -> Result<(), ProgramError> {
    for (index, action) in parsed_instruction_data.actions.iter().enumerate() {
        msg!("Processing action at index {}", index);
        match action {
            ZAction::MintTo(action) => {
                msg!("Processing MintTo action");
                let (new_supply, _lamports) = process_mint_to_action(
                    action,
                    u64::from(compressed_mint.supply),
                    validated_accounts,
                    accounts_config,
                    cpi_instruction_struct,
                    hash_cache,
                    parsed_instruction_data.mint.spl_mint,
                    queue_indices.out_token_queue_index,
                )?;
                compressed_mint.supply = new_supply.into();
            }
            ZAction::UpdateMintAuthority(update_action) => {
                msg!("Processing UpdateMintAuthority action");
                let current_mint_authority = parsed_instruction_data
                    .mint
                    .mint_authority
                    .as_ref()
                    .map(|auth| **auth);

                let new_mint_authority = update_authority(
                    update_action,
                    validated_accounts.authority.key(),
                    current_mint_authority,
                    "mint authority",
                )?;
                if let Some(mint_auth_ref) = compressed_mint.mint_authority.as_mut() {
                    if let Some(new_auth) = new_mint_authority {
                        **mint_auth_ref = new_auth;
                    }
                } else if new_mint_authority.is_some() {
                    msg!("Cannot set mint authority when none was allocated");
                    return Err(ErrorCode::MintActionUnsupportedOperation.into());
                }
            }
            ZAction::UpdateFreezeAuthority(update_action) => {
                msg!("Processing UpdateFreezeAuthority action");
                let current_freeze_authority =
                    compressed_mint.freeze_authority.as_ref().map(|auth| **auth);
                let new_freeze_authority = update_authority(
                    update_action,
                    validated_accounts.authority.key(),
                    current_freeze_authority,
                    "freeze authority",
                )?;
                if let Some(freeze_auth_ref) = compressed_mint.freeze_authority.as_mut() {
                    if let Some(new_auth) = new_freeze_authority {
                        **freeze_auth_ref = new_auth;
                    }
                } else if new_freeze_authority.is_some() {
                    msg!("Cannot set freeze authority when none was allocated");
                    return Err(ErrorCode::MintActionUnsupportedOperation.into());
                }
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
                    u64::from(compressed_mint.supply),
                    validated_accounts,
                    accounts_config,
                    packed_accounts,
                    parsed_instruction_data.mint.spl_mint,
                )?;
                compressed_mint.supply = new_supply.into();
            }
            ZAction::UpdateMetadataField(update_metadata_action) => {
                msg!("Processing UpdateMetadataField action");
                process_update_metadata_field_action(
                    update_metadata_action,
                    compressed_mint,
                    &Pubkey::from(*validated_accounts.authority.key()),
                )?;
            }
            ZAction::UpdateMetadataAuthority(update_metadata_authority_action) => {
                msg!("Processing UpdateMetadataAuthority action");
                process_update_metadata_authority_action(
                    update_metadata_authority_action,
                    compressed_mint,
                    &Pubkey::from(*validated_accounts.authority.key()),
                )?;
            }
            ZAction::RemoveMetadataKey(remove_metadata_key_action) => {
                msg!("Processing RemoveMetadataKey action");
                process_remove_metadata_key_action(
                    remove_metadata_key_action,
                    compressed_mint,
                    &Pubkey::from(*validated_accounts.authority.key()),
                )?;
            }
        }
    }

    Ok(())
}
