use anchor_lang::{prelude::ProgramError, pubkey};
use borsh::BorshDeserialize;
use light_account_checks::{checks::check_owner, AccountIterator};
use light_compressible::{config::CompressibleConfig, rent::get_rent_with_compression_cost};
use light_ctoken_types::{
    instructions::create_ctoken_account::CreateTokenAccountInstructionData,
    COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use pinocchio::{account_info::AccountInfo, instruction::AccountMeta};
use spl_pod::solana_msg::msg;

use crate::shared::{
    create_pda_account, initialize_token_account::initialize_token_account, CreatePdaAccountConfig,
};
use anchor_lang::solana_program::{rent::Rent, system_instruction, sysvar::Sysvar};

/// Validated accounts for the create token account instruction
pub struct CreateCTokenAccounts<'info> {
    /// The token account being created (signer, mutable)
    pub token_account: &'info AccountInfo,
    /// The mint for the token account (not actually used, kept for SPL compatibility)
    pub mint: &'info AccountInfo,
    /// Optional compressible configuration accounts
    pub compressible: Option<CompressibleAccounts<'info>>,
}

/// Accounts required when creating a compressible token account
pub struct CompressibleAccounts<'info> {
    /// Pays for the account creation (signer, mutable)
    pub payer: &'info AccountInfo,
    /// Contains rent configuration and authority settings
    pub config: &'info AccountInfo,
    /// Used for account creation CPI
    pub system_program: &'info AccountInfo,
    /// Either the rent recipient PDA or a custom fee payer
    pub fee_payer_pda: &'info AccountInfo,
    /// Parsed configuration from the config account
    pub parsed_config: CompressibleConfig,
}

impl<'info> CreateCTokenAccounts<'info> {
    /// Parse and validate accounts from the provided account infos
    pub fn parse(
        account_infos: &'info [AccountInfo],
        inputs: &CreateTokenAccountInstructionData,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(account_infos);

        // Required accounts
        let token_account = iter.next_signer_mut("token_account")?;
        let mint = iter.next_non_mut("mint")?;

        // Parse optional compressible accounts
        let compressible = if inputs.compressible_config.is_some() {
            let payer = iter.next_signer_mut("payer")?;
            let config_account = iter.next_non_mut("compressible config")?;

            // Validate config account owner
            check_owner(
                &pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX").to_bytes(),
                config_account,
            )?;

            // Parse config data
            let data = config_account.try_borrow_data().unwrap();
            let parsed_config = CompressibleConfig::deserialize(&mut &data[8..]).map_err(|e| {
                msg!("Failed to deserialize CompressibleConfig: {:?}", e);
                ProgramError::InvalidAccountData
            })?;

            let system_program = iter.next_account("system program")?;
            let fee_payer_pda = iter.next_account("fee payer pda")?;

            Some(CompressibleAccounts {
                payer,
                config: config_account,
                system_program,
                fee_payer_pda,
                parsed_config,
            })
        } else {
            None
        };

        Ok(Self {
            token_account,
            mint,
            compressible,
        })
    }
}

/// Process the create token account instruction
pub fn process_create_token_account(
    account_infos: &[AccountInfo],
    mut instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let mut padded_instruction_data = [0u8; 33];
    let inputs = if instruction_data.len() == 32 {
        // Extend instruction data with a zero option byte for initialize_3 spl_token instruction compatibility
        padded_instruction_data[0..32].copy_from_slice(instruction_data);
        CreateTokenAccountInstructionData::deserialize(&mut padded_instruction_data.as_slice())
            .map_err(ProgramError::from)?
    } else {
        CreateTokenAccountInstructionData::deserialize(&mut instruction_data)
            .map_err(ProgramError::from)?
    };

    // Parse and validate accounts
    let accounts = CreateCTokenAccounts::parse(account_infos, &inputs)?;

    // Create account via cpi
    let (compressible_config_account, custom_fee_payer) = if let Some(compressible) =
        accounts.compressible.as_ref()
    {
        let compressible_config = inputs
            .compressible_config
            .as_ref()
            .ok_or(ProgramError::InvalidInstructionData)?;

        if let Some(compress_to_pubkey) = compressible_config.compress_to_account_pubkey.as_ref() {
            compress_to_pubkey.check_seeds(accounts.token_account.key())?;
        }

        let account = &compressible.parsed_config;
        let account_size = COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize;
        let rent = get_rent_with_compression_cost(
            account.rent_config.min_rent as u64,
            account.rent_config.rent_per_byte as u64,
            account_size as u64,
            compressible_config.rent_payment,
            account.rent_config.full_compression_incentive as u64,
        );

        let custom_fee_payer =
            *compressible.fee_payer_pda.key() != account.rent_recipient.to_bytes();
        let custom_fee_payer = if custom_fee_payer {
            // custom fee payer for account creation -> pays rent exemption
            // Calculate rent
            let solana_rent = Rent::get()?;
            let lamports = solana_rent.minimum_balance(account_size) + rent;
            let create_account_ix = system_instruction::create_account(
                &solana_pubkey::Pubkey::new_from_array(*compressible.fee_payer_pda.key()),
                &solana_pubkey::Pubkey::new_from_array(*accounts.token_account.key()),
                lamports,
                account_size as u64,
                &solana_pubkey::Pubkey::new_from_array(crate::LIGHT_CPI_SIGNER.program_id),
            );
            let pinocchio_instruction = pinocchio::instruction::Instruction {
                program_id: &create_account_ix.program_id.to_bytes(),
                accounts: &[
                    AccountMeta::new(compressible.fee_payer_pda.key(), true, true),
                    AccountMeta::new(accounts.token_account.key(), true, true),
                    pinocchio::instruction::AccountMeta::readonly(
                        compressible.system_program.key(),
                    ),
                ],
                data: &create_account_ix.data,
            };

            match pinocchio::program::invoke(
                &pinocchio_instruction,
                &[
                    compressible.fee_payer_pda,
                    accounts.token_account,
                    compressible.system_program,
                ],
            ) {
                Ok(()) => Ok(()),
                Err(e) => Err(ProgramError::Custom(u64::from(e) as u32)),
            }?;
            Some(*compressible.fee_payer_pda.key())
        } else {
            // Rent recipient is fee payer for account creation -> pays rent exemption
            let version_bytes = account.version.to_le_bytes();
            let seeds = &[b"rent_recipient".as_slice(), version_bytes.as_slice()];
            let config = CreatePdaAccountConfig {
                seeds,
                bump: account.rent_recipient_bump,
                account_size,
                owner_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
                derivation_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
            };

            // PDA creates account with only rent-exempt balance
            create_pda_account(
                compressible.fee_payer_pda,
                accounts.token_account,
                compressible.system_program,
                config,
                None,
                None, // No additional lamports from PDA
            )?;

            // Payer transfers the additional rent (compression incentive)
            transfer_lamports_via_cpi(rent, compressible.payer, accounts.token_account)?;
            None
        };
        (Some(*account), custom_fee_payer)
    } else {
        (None, None)
    };

    // Initialize the token account (assumes account already exists and is owned by our program)
    initialize_token_account(
        accounts.token_account,
        accounts.mint.key(),
        &inputs.owner.to_bytes(),
        inputs.compressible_config,
        compressible_config_account,
        custom_fee_payer,
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
