use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::{
    compressed_account::{CompressedAccountConfig, CompressedAccountDataConfig},
    instruction_data::with_readonly::{
        InstructionDataInvokeCpiWithReadOnly, InstructionDataInvokeCpiWithReadOnlyConfig,
    },
    Pubkey,
};
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::{
        create_compressed_mint::CreateCompressedMintInstructionData,
        mint_actions::{
            Action, MintActionCompressedInstructionData, ZAction,
            ZMintActionCompressedInstructionData,
        },
        mint_to_compressed::ZMintToAction,
    },
    state::{CompressedMint, CompressedMintConfig},
    CTokenError, COMPRESSED_MINT_SEED,
};
use light_sdk::instruction::PackedMerkleContext;
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew, U64};
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;
use spl_token::solana_program::log::sol_log_compute_units;

use crate::{
    mint::{
        accounts::CreateCompressedMintAccounts, mint_output::create_output_compressed_mint_account,
    },
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
pub fn process_create_compressed_mint(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();
    // 677 CU
    let (parsed_instruction_data, _) =
        MintActionCompressedInstructionData::zero_copy_at(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

    sol_log_compute_units();
    // 112 CU write to cpi contex
    // TODO: refactor cpi hash_cache struct we don't need the index in the struct.
    let with_cpi_context = parsed_instruction_data.cpi_context.is_some();
    let write_to_cpi_context = parsed_instruction_data
        .cpi_context
        .as_ref()
        .map(|x| x.first_set_context() || x.set_context())
        .unwrap_or_default();
    // TODO: fix if mint to requires lamports.
    let with_lamports = false;
    // TODO: differentiate between will be compressed or is compressed.
    let is_decompressed = parsed_instruction_data.mint.is_decompressed();
    // Validate and parse
    let validated_accounts = MintActionAccounts::validate_and_parse(
        accounts,
        with_lamports,
        is_decompressed,
        with_cpi_context,
        write_to_cpi_context,
    )?;
    sol_log_compute_units();

    let (config, mut cpi_bytes, mint_size_config) =
        get_zero_copy_configs(&parsed_instruction_data)?;

    // let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);

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

    if !write_to_cpi_context && parsed_instruction_data.proof.is_none() {
        msg!("Proof missing");
        return Err(ProgramError::InvalidInstructionData);
    }

    sol_log_compute_units();
    let mut hash_cache = HashCache::new();
    let in_tree_index = parsed_instruction_data
        .cpi_context
        .as_ref()
        .map(|cpi_context| cpi_context.in_tree_index)
        .unwrap_or(0);
    let in_queue_index = parsed_instruction_data
        .cpi_context
        .as_ref()
        .map(|cpi_context| cpi_context.in_queue_index)
        .unwrap_or(1);
    let out_token_queue_index = parsed_instruction_data
        .cpi_context
        .as_ref()
        .map(|cpi_context| cpi_context.token_out_queue_index)
        .unwrap_or(2);
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
        let spl_mint_pda: Pubkey = solana_pubkey::Pubkey::create_program_address(
            &[
                COMPRESSED_MINT_SEED,
                validated_accounts.mint_signer.key().as_slice(),
                &[parsed_instruction_data.mint_bump],
            ],
            &crate::ID,
        )?
        .into();
        if spl_mint_pda.to_bytes() != parsed_instruction_data.mint.spl_mint.to_bytes() {
            msg!("Invalid mint");
            panic!("Invalid mint");
            //return Err(ErrorCode::InvalidMint.into());
        }
        // 2. Create NewAddressParams
        let address_merkle_tree_account_index =
            if let Some(cpi_context) = parsed_instruction_data.cpi_context.as_ref() {
                cpi_context.in_tree_index
            } else {
                0
            };
        cpi_instruction_struct.new_address_params[0].set(
            spl_mint_pda.to_bytes(),
            parsed_instruction_data.root_index.into(),
            Some(0),
            address_merkle_tree_account_index,
        );
        if u64::from(parsed_instruction_data.mint.supply) != 0 {
            msg!("Invalid supply");
            panic!("Invalid supply");
            //return Err(ErrorCode::InvalidSupply.into());
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
    let mut freeze_authority = parsed_instruction_data.mint.freeze_authority;
    let mut mint_authority = parsed_instruction_data.mint.mint_authority;
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

                        mint_to_token_pool(
                            mint_account,
                            token_pool_account,
                            token_program,
                            validated_accounts.cpi_authority()?,
                            sum_amounts,
                        )?;
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
            }
            _ => {
                msg!("Invalid action");
                unimplemented!()
            }
        }
    }

    // 3. Create compressed mint account data
    // TODO: add input struct, try to use CompressedMintInput
    // TODO: bench performance input struct vs direct inputs.
    let output_queue_index = if let Some(cpi_context) = parsed_instruction_data.cpi_context.as_ref()
    {
        cpi_context.out_queue_index
    } else {
        1
    };
    let mut token_context = HashCache::new();

    create_output_compressed_mint_account(
        &mut cpi_instruction_struct.output_compressed_accounts[0],
        parsed_instruction_data.mint.spl_mint,
        parsed_instruction_data.mint.decimals,
        freeze_authority.map(|fa| *fa),
        mint_authority.map(|fa| *fa),
        supply.into(),
        mint_size_config,
        parsed_instruction_data.compressed_address,
        output_queue_index,
        parsed_instruction_data.mint.version,
        false, // Set is_decompressed = false for new mint creation
        parsed_instruction_data.mint.extensions.as_deref(),
        &mut token_context,
    )?;
    sol_log_compute_units();

    if let Some(executing) = validated_accounts.executing.as_ref() {
        // TODO: adapt cpi accounts offset.
        // 4. Execute CPI to light-system-program
        execute_cpi_invoke(
            &accounts[CreateCompressedMintAccounts::CPI_ACCOUNTS_OFFSET..],
            cpi_bytes,
            validated_accounts.tree_pubkeys().as_slice(),
            false, // no sol_pool_pda for create_compressed_mint
            None,
            executing.system.cpi_context.map(|x| *x.key()),
            false, // write to cpi hash_cache account
        )
    } else {
        execute_cpi_invoke(
            &accounts[CreateCompressedMintAccounts::CPI_ACCOUNTS_OFFSET..],
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
    // Build configuration for CPI instruction data using the generalized function
    let compressed_mint_with_freeze_authority =
        parsed_instruction_data.mint.freeze_authority.is_some();

    // Process extensions to get the proper config for CPI bytes allocation
    // The mint contains ZExtensionInstructionData, so we can use process_extensions_config directly
    let (_, extensions_config, _) = crate::extensions::process_extensions_config(
        parsed_instruction_data.mint.extensions.as_ref(),
    )?;

    let mut input = CpiConfigInput::mint_to_compressed(
        0, //parsed_instruction_data.recipients.len(), TODO: adapt
        parsed_instruction_data.proof.is_some(),
        compressed_mint_with_freeze_authority,
    );
    // Override the empty extensions_config with the actual one
    input.extensions_config = extensions_config;
    use light_ctoken_types::state::{CompressedMint, CompressedMintConfig};
    let mint_size_config = CompressedMintConfig {
        mint_authority: (input.compressed_mint_with_mint_authority, ()),
        freeze_authority: (input.compressed_mint_with_freeze_authority, ()),
        extensions: (
            !input.extensions_config.is_empty(),
            input.extensions_config.clone(),
        ),
    };
    let compressed_mint_config = CompressedAccountConfig {
        address: (true, ()), // Compressed mint has an address
        data: (
            true,
            CompressedAccountDataConfig {
                data: CompressedMint::byte_len(&mint_size_config) as u32,
            },
        ),
    };

    let config = cpi_bytes_config(input);
    let cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);

    Ok((config, cpi_bytes, mint_size_config))
}
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;

use light_hasher::{Hasher, Poseidon, Sha256};

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR, extensions::processor::create_extension_hash_chain,
};

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
