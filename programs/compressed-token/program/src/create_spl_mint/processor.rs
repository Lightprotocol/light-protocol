use anchor_lang::solana_program::{
    program_error::ProgramError, rent::Rent, system_instruction, sysvar::Sysvar,
};

use light_ctoken_types::COMPRESSED_MINT_SEED;

use crate::{constants::POOL_SEED, LIGHT_CPI_SIGNER};
/*
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

const IN_TREE: u8 = 0;
const IN_OUTPUT_QUEUE: u8 = 1;
const OUT_OUTPUT_QUEUE: u8 = 2;
}
*/

/// Creates the mint account manually as a PDA derived from our program but owned by the token program
pub fn create_mint_account(
    executing_accounts: &crate::mint_action::accounts::ExecutingAccounts<'_>,
    program_id: &pinocchio::pubkey::Pubkey,
    mint_bump: u8,
    mint_signer: &pinocchio::account_info::AccountInfo,
) -> Result<(), ProgramError> {
    let mint_account_size = 82; // Size of Token-2022 Mint account
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(mint_account_size);

    // Derive the mint PDA seeds using provided bump
    let program_id_pubkey = solana_pubkey::Pubkey::new_from_array(*program_id);
    let expected_mint = solana_pubkey::Pubkey::create_program_address(
        &[
            COMPRESSED_MINT_SEED,
            mint_signer.key().as_ref(),
            &[mint_bump],
        ],
        &program_id_pubkey,
    )
    .map_err(|_| ProgramError::InvalidAccountData)?;

    // Verify the provided mint account matches the expected PDA
    let mint_account = executing_accounts
        .mint
        .ok_or(ProgramError::InvalidAccountData)?;
    if mint_account.key() != &expected_mint.to_bytes() {
        return Err(ProgramError::InvalidAccountData);
    }

    use pinocchio::instruction::{Seed, Signer};
    let mint_signer_key = mint_signer.key();
    let bump_bytes = [mint_bump];
    let seed_array = [
        Seed::from(COMPRESSED_MINT_SEED),
        Seed::from(mint_signer_key.as_ref()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&seed_array);

    // Create account owned by token program but derived from our program
    let fee_payer_pubkey =
        solana_pubkey::Pubkey::new_from_array(*executing_accounts.system.fee_payer.key());
    let mint_pubkey = solana_pubkey::Pubkey::new_from_array(*mint_account.key());
    let token_program_pubkey = solana_pubkey::Pubkey::new_from_array(
        *executing_accounts
            .token_program
            .ok_or(ProgramError::InvalidAccountData)?
            .key(),
    );
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
            pinocchio::instruction::AccountMeta::new(
                executing_accounts.system.fee_payer.key(),
                true,
                true,
            ),
            pinocchio::instruction::AccountMeta::new(mint_account.key(), true, true),
            pinocchio::instruction::AccountMeta::readonly(
                executing_accounts.system.system_program.key(),
            ),
        ],
        data: &create_account_ix.data,
    };

    match pinocchio::program::invoke_signed(
        &pinocchio_instruction,
        &[
            executing_accounts.system.fee_payer,
            mint_account,
            executing_accounts.system.system_program,
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
pub fn initialize_mint_account_for_action(
    executing_accounts: &crate::mint_action::accounts::ExecutingAccounts<'_>,
    mint_data: &light_ctoken_types::instructions::create_compressed_mint::ZCompressedMintInstructionData<'_>,
) -> Result<(), ProgramError> {
    let mint_account = executing_accounts
        .mint
        .ok_or(ProgramError::InvalidAccountData)?;
    let token_program = executing_accounts
        .token_program
        .ok_or(ProgramError::InvalidAccountData)?;

    let spl_ix = spl_token_2022::instruction::initialize_mint2(
        &solana_pubkey::Pubkey::new_from_array(*token_program.key()),
        &solana_pubkey::Pubkey::new_from_array(*mint_account.key()),
        // cpi_signer is spl mint authority for compressed mints.
        &solana_pubkey::Pubkey::new_from_array(LIGHT_CPI_SIGNER.cpi_signer),
        mint_data
            .freeze_authority
            .as_ref()
            .map(|f| solana_pubkey::Pubkey::new_from_array(f.to_bytes()))
            .as_ref(),
        mint_data.decimals,
    )?;

    let initialize_mint_ix = pinocchio::instruction::Instruction {
        program_id: token_program.key(),
        accounts: &[pinocchio::instruction::AccountMeta::new(
            mint_account.key(),
            true, // is_writable: true (we're initializing the mint)
            false,
        )],
        data: &spl_ix.data,
    };

    match pinocchio::program::invoke(&initialize_mint_ix, &[mint_account]) {
        Ok(()) => {}
        Err(e) => {
            return Err(ProgramError::Custom(u64::from(e) as u32));
        }
    }

    Ok(())
}

/// Creates the token pool account manually as a PDA derived from our program but owned by the token program
pub fn create_token_pool_account_manual(
    executing_accounts: &crate::mint_action::accounts::ExecutingAccounts<'_>,
    program_id: &pinocchio::pubkey::Pubkey,
) -> Result<(), ProgramError> {
    let token_account_size = 165; // Size of Token account
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(token_account_size);

    // Derive the token pool PDA seeds and bump
    let mint_account = executing_accounts
        .mint
        .ok_or(ProgramError::InvalidAccountData)?;
    let token_pool_pda = executing_accounts
        .token_pool_pda
        .ok_or(ProgramError::InvalidAccountData)?;
    let token_program = executing_accounts
        .token_program
        .ok_or(ProgramError::InvalidAccountData)?;

    let mint_key = mint_account.key();
    let program_id_pubkey = solana_pubkey::Pubkey::new_from_array(*program_id);
    let (expected_token_pool, bump) = solana_pubkey::Pubkey::find_program_address(
        &[POOL_SEED, mint_key.as_ref()],
        &program_id_pubkey,
    );

    // Verify the provided token pool account matches the expected PDA
    if token_pool_pda.key() != &expected_token_pool.to_bytes() {
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
    let fee_payer_pubkey =
        solana_pubkey::Pubkey::new_from_array(*executing_accounts.system.fee_payer.key());
    let token_pool_pubkey = solana_pubkey::Pubkey::new_from_array(*token_pool_pda.key());
    let token_program_pubkey = solana_pubkey::Pubkey::new_from_array(*token_program.key());
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
            pinocchio::instruction::AccountMeta::new(
                executing_accounts.system.fee_payer.key(),
                true,
                true,
            ),
            pinocchio::instruction::AccountMeta::new(token_pool_pda.key(), true, true),
            pinocchio::instruction::AccountMeta::readonly(
                executing_accounts.system.system_program.key(),
            ),
        ],
        data: &create_account_ix.data,
    };

    match pinocchio::program::invoke_signed(
        &pinocchio_instruction,
        &[
            executing_accounts.system.fee_payer,
            token_pool_pda,
            executing_accounts.system.system_program,
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
pub fn initialize_token_pool_account_for_action(
    executing_accounts: &crate::mint_action::accounts::ExecutingAccounts<'_>,
) -> Result<(), ProgramError> {
    let mint_account = executing_accounts
        .mint
        .ok_or(ProgramError::InvalidAccountData)?;
    let token_pool_pda = executing_accounts
        .token_pool_pda
        .ok_or(ProgramError::InvalidAccountData)?;
    let token_program = executing_accounts
        .token_program
        .ok_or(ProgramError::InvalidAccountData)?;

    let initialize_account_ix = pinocchio::instruction::Instruction {
        program_id: token_program.key(),
        accounts: &[
            pinocchio::instruction::AccountMeta::new(token_pool_pda.key(), true, false), // writable=true for initialization
            pinocchio::instruction::AccountMeta::readonly(mint_account.key()),
        ],
        data: &spl_token_2022::instruction::initialize_account3(
            &solana_pubkey::Pubkey::new_from_array(*token_program.key()),
            &solana_pubkey::Pubkey::new_from_array(*token_pool_pda.key()),
            &solana_pubkey::Pubkey::new_from_array(*mint_account.key()),
            &solana_pubkey::Pubkey::new_from_array(
                *executing_accounts.system.cpi_authority_pda.key(),
            ),
        )?
        .data,
    };

    match pinocchio::program::invoke(&initialize_account_ix, &[token_pool_pda, mint_account]) {
        Ok(()) => {}
        Err(e) => {
            return Err(ProgramError::Custom(u64::from(e) as u32));
        }
    }
    Ok(())
}
