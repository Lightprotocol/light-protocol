use anchor_lang::{
    prelude::{ProgramError, SolanaSysvar},
    solana_program::{rent::Rent, system_instruction},
};
use light_account_checks::AccountInfoTrait;
use light_zero_copy::borsh::Deserialize;
use pinocchio::account_info::AccountInfo;

use super::accounts::CreateAssociatedTokenAccountAccounts;
use crate::shared::initialize_token_account::initialize_token_account;
use light_ctoken_types::instructions::create_associated_token_account::CreateAssociatedTokenAccountInstructionData;

/// Note:
/// - we don't validate the mint because it would be very expensive with compressed mints
/// - it is possible to create an associated token account for non existing mints
/// - accounts with non existing mints can never have a balance
/// Process the create associated token account instruction
pub fn process_create_associated_token_account(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse instruction data using zero-copy
    let (inputs, _) = CreateAssociatedTokenAccountInstructionData::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;

    // Validate and get accounts
    let accounts = CreateAssociatedTokenAccountAccounts::get_checked(
        account_infos,
        &inputs.mint.to_bytes(),
        false,
    )?;

    {
        let owner = inputs.owner.to_bytes();
        let mint = inputs.mint.to_bytes();
        // Define the PDA seeds for signing
        use pinocchio::instruction::{Seed, Signer};
        let bump_bytes = [inputs.bump];
        let seed_array = [
            Seed::from(owner.as_ref()),
            Seed::from(crate::ID.as_ref()),
            Seed::from(mint.as_ref()),
            Seed::from(bump_bytes.as_ref()),
        ];
        let signer = Signer::from(&seed_array);

        // Calculate rent based on whether compressible extension is needed
        let token_account_size = if inputs.compressible_config.is_some() {
            light_ctoken_types::COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize
        } else {
            light_ctoken_types::BASIC_TOKEN_ACCOUNT_SIZE as usize
        };
        let rent = Rent::get()?;
        let rent_lamports = rent.minimum_balance(token_account_size);

        // Create the associated token account
        let fee_payer_key =
            solana_pubkey::Pubkey::new_from_array(AccountInfoTrait::key(accounts.fee_payer));
        let ata_key = solana_pubkey::Pubkey::new_from_array(AccountInfoTrait::key(
            accounts.associated_token_account,
        ));
        let create_account_instruction = system_instruction::create_account(
            &fee_payer_key,
            &ata_key,
            rent_lamports,
            token_account_size as u64,
            &crate::ID,
        );

        // Execute the create account instruction with PDA signing
        let instruction_data = create_account_instruction.data;
        let pinocchio_instruction = pinocchio::instruction::Instruction {
            program_id: &create_account_instruction.program_id.to_bytes(),
            accounts: &[
                pinocchio::instruction::AccountMeta {
                    pubkey: accounts.fee_payer.key(),
                    is_signer: true,
                    is_writable: true,
                },
                pinocchio::instruction::AccountMeta {
                    pubkey: accounts.associated_token_account.key(),
                    is_signer: true,
                    is_writable: true,
                },
                pinocchio::instruction::AccountMeta {
                    pubkey: accounts.system_program.key(),
                    is_signer: false,
                    is_writable: false,
                },
            ],
            data: &instruction_data,
        };

        match pinocchio::program::invoke_signed(
            &pinocchio_instruction,
            &[
                accounts.fee_payer,
                accounts.associated_token_account,
                accounts.system_program,
            ],
            &[signer],
        ) {
            Ok(()) => {}
            Err(e) => {
                anchor_lang::solana_program::msg!("invoke_signed failed: {:?}", e);
                return Err(ProgramError::Custom(u64::from(e) as u32));
            }
        }
    }

    // Initialize the token account using shared utility
    initialize_token_account(
        accounts.associated_token_account,
        &inputs.mint.to_bytes(),
        &inputs.owner.to_bytes(),
        inputs.compressible_config,
    )?;

    Ok(())
}
