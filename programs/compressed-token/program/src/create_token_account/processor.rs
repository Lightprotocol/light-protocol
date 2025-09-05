use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountIterator;
use light_ctoken_types::{
    state::{get_rent_with_compression_cost, COMPRESSION_COST, COMPRESSION_INCENTIVE},
    COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use light_zero_copy::traits::ZeroCopyAt;
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use super::instruction_data::CreateTokenAccountInstructionData;
use crate::shared::initialize_token_account::initialize_token_account;

/// Process the create token account instruction
pub fn process_create_token_account(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let mut padded_instruction_data = [0u8; 33];
    let (inputs, _) = if instruction_data.len() == 32 {
        // Extend instruction data with a zero option byte for initialize_3 spl_token instruction compatibility
        padded_instruction_data[0..32].copy_from_slice(instruction_data);
        CreateTokenAccountInstructionData::zero_copy_at(padded_instruction_data.as_slice())
            .map_err(ProgramError::from)?
    } else {
        CreateTokenAccountInstructionData::zero_copy_at(instruction_data)
            .map_err(ProgramError::from)?
    };

    let mut iter = AccountIterator::new(account_infos);
    let token_account = iter.next_signer_mut("token_account")?;
    // Mint is not required and not used.
    // Mint is specified for compatibility with solana.
    // TODO: provide either mint or decimals. our trick with pubkey derivation based on mint
    // only works for compressed not for spl mints.
    let mint: &AccountInfo = iter.next_non_mut("mint")?;

    // Create account via cpi
    let rent = if let Some(compressible_config) = inputs.compressible_config.as_ref() {
        // Not os solana we assume that the accoun already exists and just transfer funds
        let payer = iter.next_signer_mut("payer")?;
        let account_size = COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize;
        let rent = get_rent_with_compression_cost(
            account_size as u64,
            compressible_config.rent_payment.get(),
        );
        msg!(
            "Calculating rent for {} bytes, {} epochs: {} lamports",
            token_account.data_len(),
            compressible_config.rent_payment.get(),
            rent
        );
        #[cfg(target_os = "solana")]
        {
            use crate::shared::{create_pda_account, CreatePdaAccountConfig};
            let system_program = iter.next_account("system program")?;
            let fee_payer_pda = iter.next_account("fee payer pda")?;
            // Check derivation
            // payer pda pays for account creation
            let seeds2 = [
                b"pool".as_slice(),
                compressible_config.rent_authority.as_ref(),
            ];
            let derived_pool_pda = pinocchio_pubkey::derive_address(
                &seeds2,
                Some(compressible_config.payer_pda_bump),
                crate::ID.as_array(),
            );
            // TODO: also compare the rent recipient and rent authority
            let config = if compressible_config.has_rent_recipient != 0
                // && compressible_config.rent_authority == derived_pool_pda
                && compressible_config.rent_recipient == derived_pool_pda
            {
                CreatePdaAccountConfig {
                    seeds: seeds2.as_slice(),
                    bump: compressible_config.payer_pda_bump,
                    account_size,
                    owner_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
                    derivation_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
                }
            } else {
                return Err(ProgramError::InvalidInstructionData);
            };

            // PDA creates account with only rent-exempt balance
            create_pda_account(
                fee_payer_pda,
                token_account,
                system_program,
                config,
                None,
                None, // No additional lamports from PDA
            )?;
        }

        // Payer transfers the additional rent (compression incentive)
        transfer_lamports_via_cpi(rent, payer, token_account)?;
        rent - COMPRESSION_COST - COMPRESSION_INCENTIVE
    } else {
        0
    };

    // Initialize the token account (assumes account already exists and is owned by our program)
    initialize_token_account(
        token_account,
        mint.key(),
        &inputs.owner.to_bytes(),
        inputs.compressible_config,
        Some(rent),
    )?;

    Ok(())
}

pub fn transfer_lamports(
    amount: u64,
    from: &AccountInfo,
    to: &AccountInfo,
) -> Result<(), ProgramError> {
    let from_lamports: u64 = *from
        .try_borrow_lamports()
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
    let to_lamports: u64 = *to
        .try_borrow_lamports()
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
    if from_lamports < amount {
        msg!("payer lamports {}", from_lamports);
        msg!("required lamports {}", amount);
        return Err(ProgramError::InsufficientFunds);
    }

    let from_lamports = from_lamports
        .checked_sub(amount)
        .ok_or(ProgramError::InsufficientFunds)?;
    let to_lamports = to_lamports
        .checked_add(amount)
        .ok_or(ProgramError::InsufficientFunds)?;
    *from
        .try_borrow_mut_lamports()
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))? = from_lamports;
    *to.try_borrow_mut_lamports()
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))? = to_lamports;
    Ok(())
}

/// Transfer lamports using CPI to system program
/// This is needed when transferring from accounts not owned by our program
pub fn transfer_lamports_via_cpi(
    amount: u64,
    from: &AccountInfo,
    to: &AccountInfo,
) -> Result<(), ProgramError> {
    use anchor_lang::solana_program::system_instruction;
    use pinocchio::program::invoke;

    // Create system transfer instruction
    let transfer_ix = system_instruction::transfer(
        &solana_pubkey::Pubkey::new_from_array(*from.key()),
        &solana_pubkey::Pubkey::new_from_array(*to.key()),
        amount,
    );

    // Convert to pinocchio instruction format
    let pinocchio_ix = pinocchio::instruction::Instruction {
        program_id: &transfer_ix.program_id.to_bytes(),
        accounts: &[
            pinocchio::instruction::AccountMeta::new(from.key(), true, true),
            pinocchio::instruction::AccountMeta::new(to.key(), true, false),
        ],
        data: &transfer_ix.data,
    };

    // Invoke the system program to transfer lamports
    match invoke(&pinocchio_ix, &[from, to]) {
        Ok(()) => {
            msg!("Successfully transferred {} lamports", amount);
            Ok(())
        }
        Err(e) => {
            msg!("Failed to transfer lamports: {:?}", e);
            Err(ProgramError::Custom(u64::from(e) as u32))
        }
    }
}
