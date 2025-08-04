use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::{
    instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly, Pubkey,
};
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::{
        mint_actions::{
            MintActionCompressedInstructionData, ZAction, ZMintActionCompressedInstructionData,
        },
        mint_to_compressed::ZMintToAction,
    },
};

use light_sdk::instruction::PackedMerkleContext;
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;
use spl_token::solana_program::log::sol_log_compute_units;

use crate::{
    mint_action::{
        accounts::{AccountsConfig, MintActionAccounts},
        create_mint::process_create_mint_action,
        create_spl_mint::process_create_spl_mint_action,
        mint_input::create_input_compressed_mint_account,
        mint_output::create_output_compressed_mint_account,
        update_authority::update_authority,
        zero_copy_config::get_zero_copy_configs,
    },
    shared::{
        cpi::execute_cpi_invoke, mint_to_token_pool, token_output::set_output_compressed_account,
    },
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
    let (parsed_instruction_data, _) =
        MintActionCompressedInstructionData::zero_copy_at(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
    // msg!(" parsed_instruction_data  {:?}", parsed_instruction_data);

    sol_log_compute_units();
    // 112 CU write to cpi contex
    let accounts_config = AccountsConfig::new(&parsed_instruction_data);
    msg!("accounts_config {:?}", accounts_config);
    // Validate and parse
    let validated_accounts = MintActionAccounts::validate_and_parse(accounts, &accounts_config)?;
    sol_log_compute_units();

    let (config, mut cpi_bytes, mint_size_config) =
        get_zero_copy_configs(&parsed_instruction_data)?;
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
        return Err(ProgramError::InvalidInstructionData);
    }

    sol_log_compute_units();
    let mut hash_cache = HashCache::new();
    let queue_indices = get_queue_indices(&parsed_instruction_data, &validated_accounts)?;

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
    let (freeze_authority, mint_authority, supply) = process_actions(
        &parsed_instruction_data,
        &validated_accounts,
        &accounts_config,
        &mut cpi_instruction_struct,
        &mut hash_cache,
        &queue_indices,
    )?;

    create_output_compressed_mint_account(
        &mut cpi_instruction_struct.output_compressed_accounts[0],
        parsed_instruction_data.mint.spl_mint,
        parsed_instruction_data.mint.decimals,
        freeze_authority,
        mint_authority,
        supply.into(),
        mint_size_config,
        parsed_instruction_data.compressed_address,
        queue_indices.output_queue_index,
        parsed_instruction_data.mint.version,
        accounts_config.is_decompressed,
        parsed_instruction_data.mint.extensions.as_deref(),
        &mut hash_cache,
    )?;
    sol_log_compute_units();
    msg!("cpi_instruction_struct {:?}", cpi_instruction_struct);
    let cpi_accounts_offset = validated_accounts.cpi_accounts_offset();

    if let Some(executing) = validated_accounts.executing.as_ref() {
        // Execute CPI to light-system-program
        execute_cpi_invoke(
            &accounts[cpi_accounts_offset..],
            cpi_bytes,
            validated_accounts.tree_pubkeys().as_slice(),
            accounts_config.with_lamports,
            None,
            executing.system.cpi_context.map(|x| *x.key()),
            false, // write to cpi context account
        )
    } else {
        execute_cpi_invoke(
            &accounts[cpi_accounts_offset..],
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
    }
}

#[derive(Debug)]
pub struct QueueIndices {
    pub in_tree_index: u8,
    pub in_queue_index: u8,
    pub out_token_queue_index: u8,
    pub output_queue_index: u8,
}

fn get_queue_indices(
    parsed_instruction_data: &ZMintActionCompressedInstructionData<'_>,
    validated_accounts: &MintActionAccounts,
) -> Result<QueueIndices, ProgramError> {
    let in_tree_index = parsed_instruction_data
        .cpi_context
        .as_ref()
        .map(|cpi_context| cpi_context.in_tree_index)
        .unwrap_or(1);
    let in_queue_index = parsed_instruction_data
        .cpi_context
        .as_ref()
        .map(|cpi_context| cpi_context.in_queue_index)
        .unwrap_or(2);
    let out_token_queue_index =
        if let Some(cpi_context) = parsed_instruction_data.cpi_context.as_ref() {
            cpi_context.token_out_queue_index
        } else if let Some(system_accounts) = validated_accounts.executing.as_ref() {
            if let Some(tokens_out_queue) = system_accounts.tokens_out_queue {
                if system_accounts.out_output_queue.key() == tokens_out_queue.key() {
                    0
                } else {
                    3
                }
            } else {
                0
            }
        } else {
            msg!("No system accounts provided for queue index");
            return Err(ProgramError::InvalidAccountData);
        };
    let output_queue_index = if let Some(cpi_context) = parsed_instruction_data.cpi_context.as_ref()
    {
        cpi_context.out_queue_index
    } else {
        0
    };

    Ok(QueueIndices {
        in_tree_index,
        in_queue_index,
        out_token_queue_index,
        output_queue_index,
    })
}

fn process_actions(
    parsed_instruction_data: &ZMintActionCompressedInstructionData,
    validated_accounts: &MintActionAccounts,
    accounts_config: &crate::mint_action::accounts::AccountsConfig,
    cpi_instruction_struct: &mut light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut,
    hash_cache: &mut HashCache,
    queue_indices: &QueueIndices,
) -> Result<(Option<Pubkey>, Option<Pubkey>, u64), ProgramError> {
    let mut freeze_authority = parsed_instruction_data.mint.freeze_authority.map(|fa| *fa);
    let mut mint_authority = parsed_instruction_data.mint.mint_authority.map(|fa| *fa);
    let mut supply: u64 = parsed_instruction_data.mint.supply.into();

    for action in parsed_instruction_data.actions.iter() {
        match action {
            ZAction::MintTo(action) => {
                let sum_amounts = action
                    .recipients
                    .iter()
                    .map(|x| u64::from(x.amount))
                    .sum::<u64>();
                supply = supply
                    .checked_add(sum_amounts)
                    .ok_or(ProgramError::ArithmeticOverflow)?;
                if let Some(system_accounts) = validated_accounts.executing.as_ref() {
                    // If mint is decompressed, mint tokens to the token pool to maintain SPL mint supply consistency
                    if accounts_config.is_decompressed {
                        let sum_amounts: u64 =
                            action.recipients.iter().map(|x| u64::from(x.amount)).sum();
                        let mint_account = system_accounts
                            .mint
                            .ok_or(ProgramError::InvalidAccountData)?;
                        let token_pool_account = system_accounts
                            .token_pool_pda
                            .ok_or(ProgramError::InvalidAccountData)?;
                        let token_program = system_accounts
                            .token_program
                            .ok_or(ProgramError::InvalidAccountData)?;
                        msg!("minting {}", sum_amounts);
                        mint_to_token_pool(
                            mint_account,
                            token_pool_account,
                            token_program,
                            validated_accounts.cpi_authority()?,
                            sum_amounts,
                        )?;
                    }
                    // Create output token accounts
                    create_output_compressed_token_accounts(
                        action,
                        cpi_instruction_struct,
                        hash_cache,
                        parsed_instruction_data.mint.spl_mint,
                        queue_indices.out_token_queue_index,
                    )?;
                }
            }
            ZAction::UpdateMintAuthority(update_action) => {
                mint_authority = update_authority(
                    update_action,
                    validated_accounts.authority.key(),
                    mint_authority,
                    "mint authority",
                )?;
            }
            ZAction::UpdateFreezeAuthority(update_action) => {
                freeze_authority = update_authority(
                    update_action,
                    validated_accounts.authority.key(),
                    freeze_authority,
                    "freeze authority",
                )?;
            }
            ZAction::CreateSplMint(create_spl_action) => {
                process_create_spl_mint_action(
                    create_spl_action,
                    &validated_accounts,
                    &parsed_instruction_data.mint,
                )?;
            }
            _ => {
                msg!("Unsupported action type");
                return Err(ProgramError::InvalidInstructionData);
            }
        }
    }

    Ok((freeze_authority, mint_authority, supply))
}

fn create_output_compressed_token_accounts(
    parsed_instruction_data: &ZMintToAction<'_>,
    cpi_instruction_struct: &mut light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut<'_>,
    hash_cache: &mut HashCache,
    mint: Pubkey,
    queue_pubkey_index: u8,
) -> Result<(), ProgramError> {
    let hashed_mint = hash_cache.get_or_hash_mint(&mint.to_bytes())?;

    let lamports = parsed_instruction_data
        .lamports
        .map(|lamports| u64::from(*lamports));
    for (recipient, output_account) in parsed_instruction_data.recipients.iter().zip(
        cpi_instruction_struct
            .output_compressed_accounts
            .iter_mut()
            .skip(1), // Skip the first account which is the mint account.
    ) {
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
