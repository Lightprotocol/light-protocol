use anchor_lang::solana_program::program_error::ProgramError;
use arrayvec::ArrayVec;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use light_compressed_account::{
    instruction_data::with_readonly::{
        InstructionDataInvokeCpiWithReadOnly, InstructionDataInvokeCpiWithReadOnlyConfig,
    },
    Pubkey,
};
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::{
        mint_actions::{
            MintActionCompressedInstructionData, ZAction, ZMintActionCompressedInstructionData,
        },
        mint_to_compressed::ZMintToAction,
    },
    state::{CompressedMint, CompressedMintConfig},
    CTokenError, COMPRESSED_MINT_SEED,
};
use light_sdk::instruction::PackedMerkleContext;
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;
use spl_token::solana_program::log::sol_log_compute_units;

use light_hasher::{Hasher, Poseidon, Sha256};

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    create_spl_mint::processor::{
        create_mint_account, create_token_pool_account_manual, initialize_mint_account_for_action,
        initialize_token_pool_account_for_action,
    },
    extensions::processor::create_extension_hash_chain,
    mint::mint_output::create_output_compressed_mint_account,
    mint_action::accounts::MintActionAccounts,
    shared::{
        cpi::execute_cpi_invoke,
        cpi_bytes_size::{
            allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
        },
        mint_to_token_pool,
        token_output::set_output_compressed_account,
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
    // TODO: refactor cpi hash_cache struct we don't need the index in the struct.
    let with_cpi_context = parsed_instruction_data.cpi_context.is_some();
    let write_to_cpi_context = parsed_instruction_data
        .cpi_context
        .as_ref()
        .map(|x| x.first_set_context() || x.set_context())
        .unwrap_or_default();
    let with_lamports = parsed_instruction_data
        .actions
        .iter()
        .any(|action| matches!(action, ZAction::MintTo(mint_to_action) if mint_to_action.lamports.is_some()));
    // TODO: differentiate between will be compressed or is compressed.
    let is_decompressed = parsed_instruction_data.mint.is_decompressed()
        | parsed_instruction_data
            .actions
            .iter()
            .any(|action| matches!(action, ZAction::CreateSplMint(_)));
    // We need mint signer if create mint, and create spl mint.
    let with_mint_signer = parsed_instruction_data.create_mint()
        | parsed_instruction_data
            .actions
            .iter()
            .any(|action| matches!(action, ZAction::CreateSplMint(_)));
    msg!("is decompressed {}", is_decompressed);
    msg!("with_mint_signer {}", with_mint_signer);
    // Validate and parse
    let validated_accounts = MintActionAccounts::validate_and_parse(
        accounts,
        with_lamports,
        is_decompressed,
        with_mint_signer,
        with_cpi_context,
        write_to_cpi_context,
    )?;
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

    if !write_to_cpi_context
        && !parsed_instruction_data.prove_by_index()
        && parsed_instruction_data.proof.is_none()
    {
        msg!("Proof missing");
        return Err(ProgramError::InvalidInstructionData);
    }

    sol_log_compute_units();
    let mut hash_cache = HashCache::new();
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
    // If create mint
    // 1. derive spl mint pda
    // 2. set create address
    // else
    // 1. set input compressed mint account
    if parsed_instruction_data.create_mint() {
        // 1. Create spl mint PDA using provided bump
        // - The compressed address is derived from the spl_mint_pda.
        // - The spl mint pda is used as mint in compressed token accounts.
        // Note: we cant use pinocchio_pubkey::derive_address because don't use the mint_pda in this ix.
        //  The pda would be unvalidated and an invalid bump could be used.
        let mint_signer = validated_accounts
            .mint_signer
            .ok_or(CTokenError::ExpectedMintSignerAccount)?;
        let spl_mint_pda: Pubkey = solana_pubkey::Pubkey::create_program_address(
            &[
                COMPRESSED_MINT_SEED,
                mint_signer.key().as_slice(),
                &[parsed_instruction_data.mint_bump],
            ],
            &crate::ID,
        )?
        .into();
        msg!("post mint_size_config {:?}", mint_size_config);
        if spl_mint_pda.to_bytes() != parsed_instruction_data.mint.spl_mint.to_bytes() {
            msg!("Invalid mint PDA derivation");
            return Err(ProgramError::InvalidAccountData);
        }
        // 2. Create NewAddressParams
        let address_merkle_tree_account_index =
            if let Some(cpi_context) = parsed_instruction_data.cpi_context.as_ref() {
                cpi_context.in_tree_index
            } else {
                1 // Address tree is at index 1 after out_output_queue
            };
        cpi_instruction_struct.new_address_params[0].set(
            spl_mint_pda.to_bytes(),
            parsed_instruction_data.root_index.into(),
            Some(0),
            address_merkle_tree_account_index,
        );
        // Validate mint parameters
        if u64::from(parsed_instruction_data.mint.supply) != 0 {
            msg!("Initial supply must be 0 for new mint creation");
            return Err(ProgramError::InvalidInstructionData);
        }

        // Validate version is supported
        if parsed_instruction_data.mint.version > 1 {
            msg!("Unsupported mint version");
            return Err(ProgramError::InvalidInstructionData);
        }

        // Validate is_decompressed is false for new mint creation
        if parsed_instruction_data.mint.is_decompressed() {
            msg!("New mint must start as compressed (is_decompressed=false)");
            return Err(ProgramError::InvalidInstructionData);
        }
    } else {
        // Process input compressed mint account
        create_input_compressed_mint_account(
            &mut cpi_instruction_struct.input_compressed_accounts[0],
            &mut hash_cache,
            &parsed_instruction_data,
            PackedMerkleContext {
                merkle_tree_pubkey_index: in_tree_index,
                queue_pubkey_index: in_queue_index,
                leaf_index: parsed_instruction_data.leaf_index.into(),
                prove_by_index: parsed_instruction_data.prove_by_index(),
            },
        )?;
    }
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
                    if is_decompressed {
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
                        &mut cpi_instruction_struct,
                        &mut hash_cache,
                        parsed_instruction_data.mint.spl_mint,
                        out_token_queue_index,
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

    // 3. Create compressed mint account data
    // TODO: bench performance input struct vs direct inputs.
    let output_queue_index = if let Some(cpi_context) = parsed_instruction_data.cpi_context.as_ref()
    {
        cpi_context.out_queue_index
    } else {
        0
    };

    let mut token_context = HashCache::new();

    create_output_compressed_mint_account(
        &mut cpi_instruction_struct.output_compressed_accounts[0],
        parsed_instruction_data.mint.spl_mint,
        parsed_instruction_data.mint.decimals,
        freeze_authority,
        mint_authority,
        supply.into(),
        mint_size_config,
        parsed_instruction_data.compressed_address,
        output_queue_index,
        parsed_instruction_data.mint.version,
        is_decompressed,
        parsed_instruction_data.mint.extensions.as_deref(),
        &mut token_context,
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
            with_lamports,
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

fn get_zero_copy_configs(
    parsed_instruction_data: &ZMintActionCompressedInstructionData<'_>,
) -> Result<
    (
        InstructionDataInvokeCpiWithReadOnlyConfig,
        Vec<u8>,
        CompressedMintConfig,
    ),
    ProgramError,
> {
    use light_ctoken_types::state::CompressedMintConfig;
    msg!("get_zero_copy_configs");
    // Process extensions to get the proper config for CPI bytes allocation
    let (_, extensions_config, _) = crate::extensions::process_extensions_config(
        parsed_instruction_data.mint.extensions.as_ref(),
    )?;
    msg!("get_zero_copy_configs1");

    // Calculate final authority states after processing all actions
    let mut final_mint_authority = parsed_instruction_data.mint.mint_authority.is_some();
    let mut final_freeze_authority = parsed_instruction_data.mint.freeze_authority.is_some();

    // Process actions in order to determine final authority states
    for action in parsed_instruction_data.actions.iter() {
        match action {
            ZAction::UpdateMintAuthority(update_action) => {
                // None = revoke authority, Some(key) = set new authority
                final_mint_authority = update_action.new_authority.is_some();
            }
            ZAction::UpdateFreezeAuthority(update_action) => {
                // None = revoke authority, Some(key) = set new authority
                final_freeze_authority = update_action.new_authority.is_some();
            }
            ZAction::UpdateMetadata => {
                // TODO: When UpdateMetadata is implemented, process extension modifications here
                // and recalculate final extensions_config for correct output mint size calculation
            }
            _ => {} // Other actions don't affect authority or extension states
        }
    }
    msg!("get_zero_copy_configs2");

    // Output mint config (always present) with final authority states
    let output_mint_config = CompressedMintConfig {
        mint_authority: (final_mint_authority, ()),
        freeze_authority: (final_freeze_authority, ()),
        extensions: (!extensions_config.is_empty(), extensions_config),
    };

    // Count recipients from MintTo actions
    let num_recipients = parsed_instruction_data
        .actions
        .iter()
        .map(|action| match action {
            ZAction::MintTo(mint_to_action) => mint_to_action.recipients.len(),
            _ => 0,
        })
        .sum();
    msg!("get_zero_copy_configs2");

    let input = CpiConfigInput {
        input_accounts: {
            let mut inputs = ArrayVec::new();
            // Add input mint if not creating mint
            if !parsed_instruction_data.create_mint() {
                inputs.push(true); // Input mint has address
            }
            inputs
        },
        output_accounts: {
            let mut outputs = ArrayVec::new();
            // First output is always the mint account
            outputs.push((
                true,
                crate::shared::cpi_bytes_size::mint_data_len(&output_mint_config),
            ));

            // Add token accounts for recipients
            for _ in 0..num_recipients {
                outputs.push((false, crate::shared::cpi_bytes_size::token_data_len(false)));
                // No delegates for simple mint
            }
            outputs
        },
        has_proof: parsed_instruction_data.proof.is_some(),
        // Add new address params if creating a mint
        new_address_params: if parsed_instruction_data.create_mint() {
            1
        } else {
            0
        },
    };
    msg!("get_zero_copy_configs5");

    let config = cpi_bytes_config(input);
    msg!("get_zero_copy_configs6");
    let cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);
    msg!("get_zero_copy_configs7");

    Ok((config, cpi_bytes, output_mint_config))
}

/// Creates and validates an input compressed mint account.
/// This function follows the same pattern as create_output_compressed_mint_account
/// but processes existing compressed mint accounts as inputs.
///
/// Steps:
/// 1. Set InAccount fields (discriminator, merkle hash_cache, address)
/// 2. Validate the compressed mint data matches expected values
/// 3. Compute data hash using HashCache for caching
/// 4. Return validated CompressedMint data for output processing
pub fn create_input_compressed_mint_account(
    input_compressed_account: &mut ZInAccountMut,
    hash_cache: &mut HashCache,
    mint_instruction_data: &ZMintActionCompressedInstructionData,
    merkle_context: PackedMerkleContext,
) -> Result<(), ProgramError> {
    let mint = &mint_instruction_data.mint;
    // 1. Compute data hash using HashCache for caching
    let data_hash = {
        let hashed_spl_mint = hash_cache
            .get_or_hash_mint(&mint.spl_mint.into())
            .map_err(ProgramError::from)?;
        let mut supply_bytes = [0u8; 32];
        supply_bytes[24..].copy_from_slice(mint.supply.get().to_be_bytes().as_slice());

        let hashed_mint_authority = mint
            .mint_authority
            .map(|pubkey| hash_cache.get_or_hash_pubkey(&pubkey.to_bytes()));
        let hashed_freeze_authority = mint
            .freeze_authority
            .map(|pubkey| hash_cache.get_or_hash_pubkey(&pubkey.to_bytes()));

        // Compute the data hash using the CompressedMint hash function
        let data_hash = CompressedMint::hash_with_hashed_values(
            &hashed_spl_mint,
            &supply_bytes,
            mint.decimals,
            mint.is_decompressed(),
            &hashed_mint_authority.as_ref(),
            &hashed_freeze_authority.as_ref(),
            mint.version,
        )?;

        let extension_hashchain =
            mint_instruction_data
                .mint
                .extensions
                .as_ref()
                .map(|extensions| {
                    create_extension_hash_chain(
                        extensions,
                        &hashed_spl_mint,
                        hash_cache,
                        mint.version,
                    )
                });
        if let Some(extension_hashchain) = extension_hashchain {
            if mint.version == 0 {
                Poseidon::hashv(&[data_hash.as_slice(), extension_hashchain?.as_slice()])?
            } else if mint.version == 1 {
                let mut hash =
                    Sha256::hashv(&[data_hash.as_slice(), extension_hashchain?.as_slice()])?;
                hash[0] = 0;
                hash
            } else {
                return Err(ProgramError::from(CTokenError::InvalidTokenDataVersion));
            }
        } else if mint.version == 0 {
            data_hash
        } else if mint.version == 1 {
            let mut hash = data_hash;
            hash[0] = 0;
            hash
        } else {
            return Err(ProgramError::from(CTokenError::InvalidTokenDataVersion));
        }
    };

    // 2. Set InAccount fields
    input_compressed_account.set(
        COMPRESSED_MINT_DISCRIMINATOR,
        data_hash,
        &merkle_context,
        mint_instruction_data.root_index,
        0,
        Some(mint_instruction_data.compressed_address.as_ref()),
    )?;

    Ok(())
}

/// Helper function for processing authority update actions
fn update_authority(
    update_action: &light_ctoken_types::instructions::mint_actions::ZUpdateAuthority<'_>,
    signer_key: &pinocchio::pubkey::Pubkey,
    current_authority: Option<Pubkey>,
    authority_name: &str,
) -> Result<Option<Pubkey>, ProgramError> {
    // Verify that the signer is the current authority
    let current_authority_pubkey = current_authority.ok_or(ProgramError::InvalidArgument)?;
    if *signer_key != current_authority_pubkey.to_bytes() {
        msg!(
            "Invalid authority: signer does not match current {}",
            authority_name
        );
        return Err(ProgramError::InvalidArgument);
    }

    // Update the authority (None = revoke, Some(key) = set new authority)
    Ok(update_action.new_authority.as_ref().map(|auth| **auth))
}

/// Helper function for processing CreateSplMint action
fn process_create_spl_mint_action(
    create_spl_action: &light_ctoken_types::instructions::mint_actions::ZCreateSplMintAction<'_>,
    validated_accounts: &MintActionAccounts,
    mint_data: &light_ctoken_types::instructions::create_compressed_mint::ZCompressedMintInstructionData<'_>,
) -> Result<(), ProgramError> {
    let executing_accounts = validated_accounts
        .executing
        .as_ref()
        .ok_or(ProgramError::InvalidAccountData)?;

    // Check mint authority if it exists
    if let Some(ix_data_mint_authority) = mint_data.mint_authority {
        if *validated_accounts.authority.key() != ix_data_mint_authority.to_bytes() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // Verify mint PDA matches the spl_mint field in compressed mint inputs
    let expected_mint: [u8; 32] = mint_data.spl_mint.to_bytes();
    if executing_accounts
        .mint
        .ok_or(ProgramError::InvalidAccountData)?
        .key()
        != &expected_mint
    {
        return Err(ProgramError::InvalidAccountData);
    }

    // 1. Create the mint account manually (PDA derived from our program, owned by token program)
    let mint_signer = validated_accounts
        .mint_signer
        .ok_or(CTokenError::ExpectedMintSignerAccount)?;
    create_mint_account(
        executing_accounts,
        &crate::LIGHT_CPI_SIGNER.program_id,
        create_spl_action.mint_bump,
        mint_signer,
    )?;

    // 2. Initialize the mint account using Token-2022's initialize_mint2 instruction
    initialize_mint_account_for_action(executing_accounts, mint_data)?;

    // 3. Create the token pool account manually (PDA derived from our program, owned by token program)
    create_token_pool_account_manual(executing_accounts, &crate::LIGHT_CPI_SIGNER.program_id)?;

    // 4. Initialize the token pool account
    initialize_token_pool_account_for_action(executing_accounts)?;

    // 5. Mint the existing supply to the token pool if there's any supply
    if mint_data.supply > 0 {
        crate::shared::mint_to_token_pool(
            executing_accounts
                .mint
                .ok_or(ProgramError::InvalidAccountData)?,
            executing_accounts
                .token_pool_pda
                .ok_or(ProgramError::InvalidAccountData)?,
            executing_accounts
                .token_program
                .ok_or(ProgramError::InvalidAccountData)?,
            executing_accounts.system.cpi_authority_pda,
            mint_data.supply.into(),
        )?;
    }

    Ok(())
}
