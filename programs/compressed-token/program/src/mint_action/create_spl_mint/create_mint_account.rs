use anchor_lang::prelude::msg;
use anchor_lang::solana_program::{
    program_error::ProgramError, rent::Rent, system_instruction, sysvar::Sysvar,
};
use pinocchio::instruction::{Seed, Signer};

use light_ctoken_types::COMPRESSED_MINT_SEED;

use crate::LIGHT_CPI_SIGNER;

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
