use anchor_lang::solana_program::{
    program_error::ProgramError, rent::Rent, system_instruction, sysvar::Sysvar,
};
use arrayvec::ArrayVec;
use light_compressed_account::{
    instruction_data::cpi_context::CompressedCpiContext, pubkey::AsPubkey,
};
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::create_spl_mint::{CreateSplMintInstructionData, ZCreateSplMintInstructionData},
    state::{CompressedMint, CompressedMintConfig},
    COMPRESSED_MINT_SEED,
};
use light_sdk::instruction::PackedMerkleContext;
use light_zero_copy::{borsh::Deserialize, borsh_mut::DeserializeMut, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;
use spl_token::solana_program::log::sol_log_compute_units;

use crate::{
    constants::POOL_SEED,
    create_spl_mint::accounts::CreateSplMintAccounts,
    shared::{cpi::execute_cpi_invoke, mint_to_token_pool},
    LIGHT_CPI_SIGNER,
};

// TODO: add test which asserts spl mint and compressed mint equivalence.
// TODO: check and handle extensions
pub fn process_create_spl_mint(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();

    // Parse instruction data using zero-copy
    let (parsed_instruction_data, _) = CreateSplMintInstructionData::zero_copy_at(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    sol_log_compute_units();
    let with_cpi_context = parsed_instruction_data.cpi_context();
    // Validate and parse accounts
    let validated_accounts = CreateSplMintAccounts::validate_and_parse(accounts, with_cpi_context)?;

    // Check mint authority if it exists.
    if let Some(ix_data_mint_authority) = parsed_instruction_data.mint.mint.mint_authority {
        if *validated_accounts.authority.key() != ix_data_mint_authority.to_bytes() {
            return Err(ProgramError::InvalidAccountData);
        }
    }
    // Verify mint PDA matches the spl_mint field in compressed mint inputs
    // TODO: set it instead of passing it, to eliminate duplicate ix data.
    let expected_mint: [u8; 32] = parsed_instruction_data.mint.mint.spl_mint.to_bytes();
    if validated_accounts.mint.key() != &expected_mint {
        return Err(ProgramError::InvalidAccountData);
    }

    // Create the mint account manually (PDA derived from our program, owned by token program)
    create_mint_account(
        &validated_accounts,
        &crate::LIGHT_CPI_SIGNER.program_id,
        parsed_instruction_data.mint_bump,
    )?;

    // Initialize the mint account using Token-2022's initialize_mint2 instruction
    initialize_mint_account(&validated_accounts, &parsed_instruction_data)?;

    // Create the token pool account manually (PDA derived from our program, owned by token program)
    create_token_pool_account_manual(&validated_accounts, &crate::LIGHT_CPI_SIGNER.program_id)?;

    // Initialize the token pool account
    initialize_token_pool_account(&validated_accounts)?;

    // Mint the existing supply to the token pool if there's any supply
    if parsed_instruction_data.mint.mint.supply > 0 {
        mint_to_token_pool(
            validated_accounts.mint,
            validated_accounts.token_pool_pda,
            validated_accounts.token_program,
            validated_accounts.cpi_authority_pda,
            parsed_instruction_data.mint.mint.supply.into(),
        )?;
    }
    if parsed_instruction_data.mint_authority_is_none() {
        // TODO: remove mint authority from spl mint.
    }

    // Update the compressed mint to mark it as is_decompressed = true
    update_compressed_mint_to_decompressed(
        accounts,
        &validated_accounts,
        &parsed_instruction_data,
        with_cpi_context,
    )?;

    sol_log_compute_units();
    Ok(())
}

const IN_TREE: u8 = 0;
const IN_OUTPUT_QUEUE: u8 = 1;

const OUT_OUTPUT_QUEUE: u8 = 2;

fn update_compressed_mint_to_decompressed<'info>(
    all_accounts: &'info [AccountInfo],
    accounts: &CreateSplMintAccounts<'info>,
    instruction_data: &ZCreateSplMintInstructionData,
    with_cpi_context: bool,
) -> Result<(), ProgramError> {
    use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;

    use crate::{
        mint::{
            mint_input::create_input_compressed_mint_account,
            mint_output::create_output_compressed_mint_account,
        },
        shared::cpi_bytes_size::{
            allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
        },
    };

    // Process extensions from input mint
    let mint_inputs = &instruction_data.mint.mint;
    let (_, extensions_config, _) =
        crate::extensions::process_extensions_config(mint_inputs.extensions.as_ref())?;

    // Build configuration for CPI instruction data - 1 input, 1 output, with optional proof
    let config_input = CpiConfigInput {
        input_accounts: ArrayVec::new(),
        output_accounts: ArrayVec::new(),
        has_proof: instruction_data.proof.is_some(),
        compressed_mint: true,
        compressed_mint_with_freeze_authority: mint_inputs.freeze_authority.is_some(),
        compressed_mint_with_mint_authority: true, // create_spl_mint always creates with mint authority
        extensions_config,
    };

    let config = cpi_bytes_config(config_input);
    let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);

    {
        let (mut cpi_instruction_struct, _) =
            InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
                .map_err(ProgramError::from)?;
        cpi_instruction_struct.initialize(
            crate::LIGHT_CPI_SIGNER.bump,
            &crate::LIGHT_CPI_SIGNER.program_id.into(),
            instruction_data.proof,
            &Option::<CompressedCpiContext>::None,
        )?;

        let mut hash_cache = HashCache::new();

        // Process input compressed mint account (before is_decompressed = true)
        create_input_compressed_mint_account(
            &mut cpi_instruction_struct.input_compressed_accounts[0],
            &mut hash_cache,
            &instruction_data.mint,
            PackedMerkleContext {
                leaf_index: instruction_data.mint.leaf_index.into(),
                prove_by_index: instruction_data.mint.prove_by_index(),
                merkle_tree_pubkey_index: IN_TREE,
                queue_pubkey_index: IN_OUTPUT_QUEUE,
            },
        )?;

        // Process output compressed mint account (with is_decompressed = true)
        let mint_inputs = &instruction_data.mint.mint;
        let mint_pda = mint_inputs.spl_mint;
        let decimals = mint_inputs.decimals;
        let freeze_authority = mint_inputs
            .freeze_authority
            .as_ref()
            .map(|fa| fa.to_bytes().into());
        let mint_authority = if instruction_data.mint_authority_is_none() {
            None
        } else {
            Some(accounts.authority.key().to_pubkey_bytes().into())
        };

        // Reuse the extensions config we already processed
        let (has_extensions_output, extensions_config_output, _) =
            crate::extensions::process_extensions_config(mint_inputs.extensions.as_ref())?;

        let mint_config = CompressedMintConfig {
            mint_authority: (true, ()),
            freeze_authority: (mint_inputs.freeze_authority.is_some(), ()),
            extensions: (has_extensions_output, extensions_config_output),
        };
        let mut token_context = HashCache::new();

        create_output_compressed_mint_account(
            &mut cpi_instruction_struct.output_compressed_accounts[0],
            mint_pda,
            decimals,
            freeze_authority,
            mint_authority,
            mint_inputs.supply,
            mint_config,
            instruction_data.mint.address,
            OUT_OUTPUT_QUEUE,
            instruction_data.mint.mint.version,
            true, // Set is_decompressed = true for create_spl_mint
            mint_inputs.extensions.as_deref(),
            &mut token_context,
        )?;

        // Override the output compressed mint to set is_decompressed = true
        // The create_output_compressed_mint_account function sets is_decompressed = false by default
        {
            let output_account = &mut cpi_instruction_struct.output_compressed_accounts[0];
            if let Some(data) = output_account.compressed_account.data.as_mut() {
                let (mut compressed_mint, _) =
                    CompressedMint::zero_copy_at_mut(data.data).map_err(ProgramError::from)?;
                compressed_mint.is_decompressed = 1; // Override to mark as decompressed (1 = true)
            }
        }
    }
    // Execute CPI to light system program to update the compressed mint
    execute_cpi_invoke(
        &all_accounts[CreateSplMintAccounts::SYSTEM_ACCOUNTS_OFFSET..],
        cpi_bytes,
        accounts.tree_pubkeys().as_slice(),
        false, // no sol_pool_pda
        None,
        accounts.cpi_context.map(|cpi_context| *cpi_context.key()),
        with_cpi_context,
    )?;

    Ok(())
}

/// Creates the mint account manually as a PDA derived from our program but owned by the token program
fn create_mint_account(
    accounts: &CreateSplMintAccounts<'_>,
    program_id: &pinocchio::pubkey::Pubkey,
    mint_bump: u8,
) -> Result<(), ProgramError> {
    let mint_account_size = 82; // Size of Token-2022 Mint account
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(mint_account_size);

    // Derive the mint PDA seeds using provided bump
    let program_id_pubkey = solana_pubkey::Pubkey::new_from_array(*program_id);
    let expected_mint = solana_pubkey::Pubkey::create_program_address(
        &[
            COMPRESSED_MINT_SEED,
            accounts.mint_signer.key().as_ref(),
            &[mint_bump],
        ],
        &program_id_pubkey,
    )
    .map_err(|_| ProgramError::InvalidAccountData)?;

    // Verify the provided mint account matches the expected PDA
    if accounts.mint.key() != &expected_mint.to_bytes() {
        return Err(ProgramError::InvalidAccountData);
    }

    use pinocchio::instruction::{Seed, Signer};
    let mint_signer_key = accounts.mint_signer.key();
    let bump_bytes = [mint_bump];
    let seed_array = [
        Seed::from(COMPRESSED_MINT_SEED),
        Seed::from(mint_signer_key.as_ref()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&seed_array);

    // Create account owned by token program but derived from our program
    let fee_payer_pubkey = solana_pubkey::Pubkey::new_from_array(*accounts.fee_payer.key());
    let mint_pubkey = solana_pubkey::Pubkey::new_from_array(*accounts.mint.key());
    let token_program_pubkey = solana_pubkey::Pubkey::new_from_array(*accounts.token_program.key());
    let create_account_ix = system_instruction::create_account(
        &fee_payer_pubkey,
        &mint_pubkey,
        lamports,
        mint_account_size as u64,
        &token_program_pubkey, // Owned by token program
    );

    let pinocchio_instruction = pinocchio::instruction::Instruction {
        program_id: &create_account_ix.program_id.to_bytes(),
        accounts: &[
            pinocchio::instruction::AccountMeta::new(accounts.fee_payer.key(), true, true),
            pinocchio::instruction::AccountMeta::new(accounts.mint.key(), true, true),
            pinocchio::instruction::AccountMeta::readonly(accounts.system_program.key()),
        ],
        data: &create_account_ix.data,
    };

    match pinocchio::program::invoke_signed(
        &pinocchio_instruction,
        &[
            accounts.system.fee_payer,
            accounts.mint,
            accounts.system_program,
        ],
        &[signer], // Signed with our program's PDA seeds
    ) {
        Ok(()) => {}
        Err(e) => {
            return Err(ProgramError::Custom(u64::from(e) as u32));
        }
    }

    Ok(())
}

/// Initializes the mint account using Token-2022's initialize_mint2 instruction
fn initialize_mint_account(
    accounts: &CreateSplMintAccounts<'_>,
    instruction_data: &ZCreateSplMintInstructionData,
) -> Result<(), ProgramError> {
    let spl_ix = spl_token_2022::instruction::initialize_mint2(
        &solana_pubkey::Pubkey::new_from_array(*accounts.token_program.key()),
        &solana_pubkey::Pubkey::new_from_array(*accounts.mint.key()),
        // cpi_signer is spl mint authority for compressed mints.
        &solana_pubkey::Pubkey::new_from_array(LIGHT_CPI_SIGNER.cpi_signer),
        instruction_data
            .mint
            .mint
            .freeze_authority
            .as_ref()
            .map(|f| solana_pubkey::Pubkey::new_from_array(f.to_bytes()))
            .as_ref(),
        instruction_data.mint.mint.decimals,
    )?;

    let initialize_mint_ix = pinocchio::instruction::Instruction {
        program_id: accounts.token_program.key(),
        accounts: &[pinocchio::instruction::AccountMeta::new(
            accounts.mint.key(),
            true, // is_writable: true (we're initializing the mint)
            false,
        )],
        data: &spl_ix.data,
    };

    match pinocchio::program::invoke(&initialize_mint_ix, &[accounts.mint]) {
        Ok(()) => {}
        Err(e) => {
            return Err(ProgramError::Custom(u64::from(e) as u32));
        }
    }

    Ok(())
}

/// Creates the token pool account manually as a PDA derived from our program but owned by the token program
fn create_token_pool_account_manual(
    accounts: &CreateSplMintAccounts<'_>,
    program_id: &pinocchio::pubkey::Pubkey,
) -> Result<(), ProgramError> {
    let token_account_size = 165; // Size of Token account
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(token_account_size);

    // Derive the token pool PDA seeds and bump
    let mint_key = accounts.mint.key();
    let program_id_pubkey = solana_pubkey::Pubkey::new_from_array(*program_id);
    let (expected_token_pool, bump) = solana_pubkey::Pubkey::find_program_address(
        &[POOL_SEED, mint_key.as_ref()],
        &program_id_pubkey,
    );

    // Verify the provided token pool account matches the expected PDA
    if accounts.token_pool_pda.key() != &expected_token_pool.to_bytes() {
        return Err(ProgramError::InvalidAccountData);
    }

    use pinocchio::instruction::{Seed, Signer};
    let bump_bytes = [bump];
    let seed_array = [
        Seed::from(POOL_SEED),
        Seed::from(mint_key.as_ref()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&seed_array);

    // Create account owned by token program but derived from our program
    let fee_payer_pubkey = solana_pubkey::Pubkey::new_from_array(*accounts.fee_payer.key());
    let token_pool_pubkey = solana_pubkey::Pubkey::new_from_array(*accounts.token_pool_pda.key());
    let token_program_pubkey = solana_pubkey::Pubkey::new_from_array(*accounts.token_program.key());
    let create_account_ix = system_instruction::create_account(
        &fee_payer_pubkey,
        &token_pool_pubkey,
        lamports,
        token_account_size as u64,
        &token_program_pubkey, // Owned by token program
    );

    let pinocchio_instruction = pinocchio::instruction::Instruction {
        program_id: &create_account_ix.program_id.to_bytes(),
        accounts: &[
            pinocchio::instruction::AccountMeta::new(accounts.fee_payer.key(), true, true),
            pinocchio::instruction::AccountMeta::new(accounts.token_pool_pda.key(), true, true),
            pinocchio::instruction::AccountMeta::readonly(accounts.system_program.key()),
        ],
        data: &create_account_ix.data,
    };

    match pinocchio::program::invoke_signed(
        &pinocchio_instruction,
        &[
            accounts.fee_payer,
            accounts.token_pool_pda,
            accounts.system_program,
        ],
        &[signer], // Signed with our program's PDA seeds
    ) {
        Ok(()) => {}
        Err(e) => {
            return Err(ProgramError::Custom(u64::from(e) as u32));
        }
    }

    Ok(())
}

/// Initializes the token pool account (assumes account already exists)
fn initialize_token_pool_account(accounts: &CreateSplMintAccounts<'_>) -> Result<(), ProgramError> {
    let initialize_account_ix = pinocchio::instruction::Instruction {
        program_id: accounts.token_program.key(),
        accounts: &[
            pinocchio::instruction::AccountMeta::new(accounts.token_pool_pda.key(), true, false), // writable=true for initialization
            pinocchio::instruction::AccountMeta::readonly(accounts.mint.key()),
        ],
        data: &spl_token_2022::instruction::initialize_account3(
            &solana_pubkey::Pubkey::new_from_array(*accounts.token_program.key()),
            &solana_pubkey::Pubkey::new_from_array(*accounts.token_pool_pda.key()),
            &solana_pubkey::Pubkey::new_from_array(*accounts.mint.key()),
            &solana_pubkey::Pubkey::new_from_array(*accounts.cpi_authority_pda.key()),
        )?
        .data,
    };

    match pinocchio::program::invoke(
        &initialize_account_ix,
        &[accounts.token_pool_pda, accounts.mint],
    ) {
        Ok(()) => {}
        Err(e) => {
            return Err(ProgramError::Custom(u64::from(e) as u32));
        }
    }
    Ok(())
}
