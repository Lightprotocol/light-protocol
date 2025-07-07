use anchor_lang::solana_program::{
    program_error::ProgramError, rent::Rent, system_instruction, sysvar::Sysvar,
};
use arrayvec::ArrayVec;
use light_zero_copy::{borsh::Deserialize, borsh_mut::DeserializeMut, ZeroCopyNew};
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use spl_token::solana_program::log::sol_log_compute_units;

use crate::{
    constants::POOL_SEED,
    create_spl_mint::{
        accounts::CreateSplMintAccounts,
        instructions::{CreateSplMintInstructionData, ZCreateSplMintInstructionData},
    },
    shared::cpi::execute_cpi_invoke,
};

pub fn process_create_spl_mint<'info>(
    program_id: Pubkey,
    accounts: &'info [AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();

    // Parse instruction data using zero-copy
    let (parsed_instruction_data, _) = CreateSplMintInstructionData::zero_copy_at(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    sol_log_compute_units();

    // Validate and parse accounts
    let validated_accounts = CreateSplMintAccounts::validate_and_parse(accounts, &program_id)?;

    // Verify mint PDA matches the spl_mint field in compressed mint inputs
    let expected_mint: [u8; 32] = parsed_instruction_data
        .compressed_mint_inputs
        .compressed_mint_input
        .spl_mint
        .into();
    if validated_accounts.mint.key() != &expected_mint {
        return Err(ProgramError::InvalidAccountData);
    }

    // Create the mint account manually (PDA derived from our program, owned by token program)
    create_mint_account(&validated_accounts, &program_id)?;

    // Initialize the mint account using Token-2022's initialize_mint2 instruction
    initialize_mint_account(&validated_accounts, &parsed_instruction_data)?;

    // Create the token pool account manually (PDA derived from our program, owned by token program)
    create_token_pool_account_manual(&validated_accounts, &program_id)?;

    // Initialize the token pool account
    initialize_token_pool_account(&validated_accounts)?;

    // Mint the existing supply to the token pool if there's any supply
    if parsed_instruction_data
        .compressed_mint_inputs
        .compressed_mint_input
        .supply
        > 0
    {
        mint_existing_supply_to_pool(&validated_accounts, &parsed_instruction_data)?;
    }

    // Update the compressed mint to mark it as is_decompressed = true
    update_compressed_mint_to_decompressed(
        accounts,
        &validated_accounts,
        &parsed_instruction_data,
        &program_id,
    )?;

    sol_log_compute_units();
    Ok(())
}

fn update_compressed_mint_to_decompressed<'info>(
    all_accounts: &'info [AccountInfo],
    accounts: &CreateSplMintAccounts<'info>,
    instruction_data: &ZCreateSplMintInstructionData,
    program_id: &pinocchio::pubkey::Pubkey,
) -> Result<(), ProgramError> {
    use crate::mint::{
        input::create_input_compressed_mint_account, output::create_output_compressed_mint_account,
    };
    use crate::shared::{
        context::TokenContext,
        cpi_bytes_size::{
            allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
        },
    };
    use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;

    // Build configuration for CPI instruction data - 1 input, 1 output, with optional proof
    let config_input = CpiConfigInput {
        input_accounts: ArrayVec::new(),
        output_accounts: ArrayVec::new(),
        has_proof: instruction_data.proof.is_some(),
        compressed_mint: true,
        compressed_mint_with_freeze_authority: instruction_data.freeze_authority.is_some(),
    };

    let config = cpi_bytes_config(config_input);
    let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);

    let (mut cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;

    cpi_instruction_struct.bump = crate::LIGHT_CPI_SIGNER.bump;
    cpi_instruction_struct.invoking_program_id = crate::LIGHT_CPI_SIGNER.program_id.into();

    let mut context = TokenContext::new();
    let hashed_mint_authority = context.get_or_hash_pubkey(accounts.authority.key());

    // Process input compressed mint account (before is_decompressed = true)
    create_input_compressed_mint_account(
        &mut cpi_instruction_struct.input_compressed_accounts[0],
        &mut context,
        &instruction_data.compressed_mint_inputs,
        &hashed_mint_authority,
    )?;

    // Process output compressed mint account (with is_decompressed = true)
    let mint_inputs = &instruction_data
        .compressed_mint_inputs
        .compressed_mint_input;
    let mint_pda = mint_inputs.spl_mint;
    let decimals = instruction_data.decimals;
    let freeze_authority = if mint_inputs.freeze_authority_is_set() {
        Some(mint_inputs.freeze_authority)
    } else {
        None
    };

    let mint_config = crate::mint::state::CompressedMintConfig {
        mint_authority: (true, ()),
        freeze_authority: (mint_inputs.freeze_authority_is_set(), ()),
    };
    let compressed_account_address = *instruction_data.compressed_mint_inputs.address;
    let supply = mint_inputs.supply; // Keep same supply, just mark as decompressed

    create_output_compressed_mint_account(
        &mut cpi_instruction_struct.output_compressed_accounts[0],
        mint_pda,
        decimals,
        freeze_authority,
        Some(instruction_data.mint_authority),
        supply,
        &program_id.into(),
        mint_config,
        compressed_account_address,
        instruction_data
            .compressed_mint_inputs
            .output_merkle_tree_index,
    )?;

    // Set proof data if provided
    if let Some(instruction_proof) = &instruction_data.proof {
        if let Some(proof) = cpi_instruction_struct.proof.as_deref_mut() {
            proof.a = instruction_proof.a;
            proof.b = instruction_proof.b;
            proof.c = instruction_proof.c;
        }
    }

    // Override the output compressed mint to set is_decompressed = true
    // The create_output_compressed_mint_account function sets is_decompressed = false by default
    {
        let output_account = &mut cpi_instruction_struct.output_compressed_accounts[0];
        if let Some(data) = output_account.compressed_account.data.as_mut() {
            let (mut compressed_mint, _) =
                crate::mint::state::CompressedMint::zero_copy_at_mut(data.data)
                    .map_err(ProgramError::from)?;
            compressed_mint.is_decompressed = 1; // Override to mark as decompressed (1 = true)

            // Recalculate hash with is_decompressed = true
            *data.data_hash = compressed_mint
                .hash()
                .map_err(|_| ProgramError::InvalidAccountData)?;
        }
    }

    // Extract tree accounts for the generalized CPI call
    let tree_accounts = [
        accounts.in_merkle_tree.key(),
        accounts.in_output_queue.key(),
        accounts.out_output_queue.key(),
    ];

    // Execute CPI to light system program to update the compressed mint
    execute_cpi_invoke(
        all_accounts,
        cpi_bytes,
        &tree_accounts,
        false, // no sol_pool_pda
        None,  // no cpi_context_account
    )?;

    Ok(())
}

/// Creates the mint account manually as a PDA derived from our program but owned by the token program
fn create_mint_account(
    accounts: &CreateSplMintAccounts<'_>,
    program_id: &pinocchio::pubkey::Pubkey,
) -> Result<(), ProgramError> {
    let mint_account_size = 82; // Size of Token-2022 Mint account
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(mint_account_size);

    // Derive the mint PDA seeds and bump
    let program_id_pubkey = solana_pubkey::Pubkey::new_from_array(*program_id);
    let (expected_mint, bump) = solana_pubkey::Pubkey::find_program_address(
        &[b"compressed_mint", accounts.mint_signer.key().as_ref()],
        &program_id_pubkey,
    );

    // Verify the provided mint account matches the expected PDA
    if accounts.mint.key() != &expected_mint.to_bytes() {
        return Err(ProgramError::InvalidAccountData);
    }

    use pinocchio::instruction::{Seed, Signer};
    let mint_signer_key = accounts.mint_signer.key();
    let bump_bytes = [bump];
    let seed_array = [
        Seed::from(b"compressed_mint"),
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

    pinocchio::program::invoke_signed(
        &pinocchio_instruction,
        &[accounts.fee_payer, accounts.mint, accounts.system_program],
        &[signer], // Signed with our program's PDA seeds
    )
    .map_err(|_| ProgramError::Custom(1))?;

    Ok(())
}

/// Initializes the mint account using Token-2022's initialize_mint2 instruction
fn initialize_mint_account(
    accounts: &CreateSplMintAccounts<'_>,
    instruction_data: &ZCreateSplMintInstructionData,
) -> Result<(), ProgramError> {
    let initialize_mint_ix = pinocchio::instruction::Instruction {
        program_id: accounts.token_program.key(),
        accounts: &[pinocchio::instruction::AccountMeta::new(
            accounts.mint.key(),
            false,
            false,
        )],
        data: &spl_token_2022::instruction::initialize_mint2(
            &solana_pubkey::Pubkey::new_from_array(*accounts.token_program.key()),
            &solana_pubkey::Pubkey::new_from_array(*accounts.mint.key()),
            &solana_pubkey::Pubkey::new_from_array(instruction_data.mint_authority.into()),
            instruction_data
                .freeze_authority
                .as_ref()
                .map(|f| solana_pubkey::Pubkey::new_from_array((**f).into()))
                .as_ref(),
            instruction_data.decimals,
        )?
        .data,
    };

    pinocchio::program::invoke(
        &initialize_mint_ix,
        &[accounts.mint, accounts.token_program],
    )
    .map_err(|_| ProgramError::Custom(1))?;

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

    pinocchio::program::invoke_signed(
        &pinocchio_instruction,
        &[
            accounts.fee_payer,
            accounts.token_pool_pda,
            accounts.system_program,
        ],
        &[signer], // Signed with our program's PDA seeds
    )
    .map_err(|_| ProgramError::Custom(1))?;

    Ok(())
}

/// Initializes the token pool account (assumes account already exists)
fn initialize_token_pool_account(accounts: &CreateSplMintAccounts<'_>) -> Result<(), ProgramError> {
    let initialize_account_ix = pinocchio::instruction::Instruction {
        program_id: accounts.token_program.key(),
        accounts: &[
            pinocchio::instruction::AccountMeta::new(accounts.token_pool_pda.key(), false, false),
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

    pinocchio::program::invoke(
        &initialize_account_ix,
        &[
            accounts.token_pool_pda,
            accounts.mint,
            accounts.token_program,
        ],
    )
    .map_err(|_| ProgramError::Custom(1))?;

    Ok(())
}

/// Mints the existing supply from compressed mint to the token pool
fn mint_existing_supply_to_pool(
    accounts: &CreateSplMintAccounts<'_>,
    instruction_data: &ZCreateSplMintInstructionData,
) -> Result<(), ProgramError> {
    // Only mint if the authority matches
    if accounts.authority.key() != &instruction_data.mint_authority.to_bytes() {
        return Err(ProgramError::InvalidAccountData);
    }

    let supply = instruction_data
        .compressed_mint_inputs
        .compressed_mint_input
        .supply
        .into();

    // Mint tokens to the pool
    let mint_to_ix = pinocchio::instruction::Instruction {
        program_id: accounts.token_program.key(),
        accounts: &[
            pinocchio::instruction::AccountMeta::new(accounts.mint.key(), false, false),
            pinocchio::instruction::AccountMeta::new(accounts.token_pool_pda.key(), false, false),
            pinocchio::instruction::AccountMeta::readonly(accounts.authority.key()),
        ],
        data: &spl_token_2022::instruction::mint_to(
            &solana_pubkey::Pubkey::new_from_array(*accounts.token_program.key()),
            &solana_pubkey::Pubkey::new_from_array(*accounts.mint.key()),
            &solana_pubkey::Pubkey::new_from_array(*accounts.token_pool_pda.key()),
            &solana_pubkey::Pubkey::new_from_array(*accounts.authority.key()),
            &[],
            supply,
        )?
        .data,
    };

    pinocchio::program::invoke(
        &mint_to_ix,
        &[
            accounts.mint,
            accounts.token_pool_pda,
            accounts.authority,
            accounts.token_program,
        ],
    )
    .map_err(|_| ProgramError::Custom(1))?;

    Ok(())
}
