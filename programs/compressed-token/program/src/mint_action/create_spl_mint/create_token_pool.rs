use anchor_lang::solana_program::{
    program_error::ProgramError, rent::Rent, system_instruction, sysvar::Sysvar,
};

use crate::constants::POOL_SEED;

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
