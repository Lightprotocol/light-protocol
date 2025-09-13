use anchor_lang::{prelude::ProgramError, pubkey};
use light_account_checks::{checks::check_owner, AccountIterator};
use light_compressible::{config::CompressibleConfig, rent::get_rent_with_compression_cost};
use light_ctoken_types::COMPRESSIBLE_TOKEN_ACCOUNT_SIZE;
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
    let compressible_config_account =
        if let Some(compressible_config) = inputs.compressible_config.as_ref() {
            // Not os solana we assume that the accoun already exists and just transfer funds
            let payer = iter.next_signer_mut("payer")?;
            let config_account = iter.next_non_mut("compressible config")?;
            check_owner(
                &pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX").to_bytes(),
                config_account,
            )?;
            let data = config_account.try_borrow_data().unwrap();

            // Skip Anchor's 8-byte discriminator and deserialize the actual CompressibleConfig
            use borsh::BorshDeserialize;
            // Anchor accounts have an 8-byte discriminator at the beginning
            let account = CompressibleConfig::deserialize(&mut &data[8..]).map_err(|e| {
                msg!("Failed to deserialize CompressibleConfig: {:?}", e);
                ProgramError::InvalidAccountData
            })?;

            let account_size = COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize;
            let rent = get_rent_with_compression_cost(
                account.rent_config.min_rent as u64,
                account.rent_config.rent_per_byte as u64,
                account_size as u64,
                compressible_config.rent_payment.get(),
                account.rent_config.full_compression_incentive as u64,
            );
            msg!(
                "Calculating rent for {} bytes, {} epochs: {} lamports",
                token_account.data_len(),
                compressible_config.rent_payment.get(),
                rent
            );
            {
                use crate::shared::{create_pda_account, CreatePdaAccountConfig};
                let system_program = iter.next_account("system program")?;
                let fee_payer_pda = iter.next_account("fee payer pda")?;

                let version_bytes = account.version.to_le_bytes();
                let seeds = &[b"rent_recipient".as_slice(), version_bytes.as_slice(), &[0]];
                let config = CreatePdaAccountConfig {
                    seeds,
                    bump: account.rent_recipient_bump,
                    account_size,
                    owner_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
                    derivation_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
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
            Some(account)
        } else {
            None
        };

    // Initialize the token account (assumes account already exists and is owned by our program)
    initialize_token_account(
        token_account,
        mint.key(),
        &inputs.owner.to_bytes(),
        inputs.compressible_config,
        compressible_config_account,
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
