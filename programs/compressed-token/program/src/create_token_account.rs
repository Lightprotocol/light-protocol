use anchor_lang::{prelude::ProgramError, pubkey};
use borsh::BorshDeserialize;
use light_account_checks::{
    checks::{check_discriminator, check_owner},
    AccountIterator,
};
use light_compressed_account::Pubkey;
use light_compressible::config::CompressibleConfig;
use light_ctoken_types::{
    instructions::create_ctoken_account::CreateTokenAccountInstructionData,
    COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use light_program_profiler::profile;
use pinocchio::{
    account_info::AccountInfo,
    instruction::Seed,
    sysvars::{rent::Rent, Sysvar},
};
use pinocchio_system::instructions::CreateAccount;
use spl_pod::{bytemuck, solana_msg::msg};

use crate::shared::{
    convert_program_error, create_pda_account,
    initialize_ctoken_account::initialize_ctoken_account, transfer_lamports_via_cpi,
};

/// Validated accounts for the create token account instruction
pub struct CreateCTokenAccounts<'info> {
    /// The token account being created (signer, mutable)
    pub token_account: &'info AccountInfo,
    /// The mint for the token account (only used for pubkey not checked)
    pub mint: &'info AccountInfo,
    /// Optional compressible configuration accounts
    pub compressible: Option<CompressibleAccounts<'info>>,
}

/// Accounts required when creating a compressible token account
pub struct CompressibleAccounts<'info> {
    /// Pays for the compression incentive rent when rent_payer is the rent recipient (signer, mutable)
    pub payer: &'info AccountInfo,
    /// Used for account creation CPI
    pub system_program: &'info AccountInfo,
    /// Either the rent recipient PDA or a custom fee payer
    pub rent_payer: &'info AccountInfo,
    /// Parsed configuration from the config account
    pub parsed_config: &'info CompressibleConfig,
}

impl<'info> CreateCTokenAccounts<'info> {
    /// Parse and validate accounts from the provided account infos
    #[profile]
    #[inline(always)]
    pub fn parse(
        account_infos: &'info [AccountInfo],
        inputs: &CreateTokenAccountInstructionData,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(account_infos);

        // Required accounts
        // For compressible accounts: token_account must be signer (account created via CPI)
        // For non-compressible accounts: token_account doesn't need to be signer (SPL compatibility - initialize_account3)
        let token_account = if inputs.compressible_config.is_some() {
            iter.next_signer_mut("token_account")?
        } else {
            iter.next_mut("token_account")?
        };
        let mint = iter.next_non_mut("mint")?;

        // Parse optional compressible accounts
        let compressible = if inputs.compressible_config.is_some() {
            let payer = iter.next_signer_mut("payer")?;

            let parsed_config = next_config_account(&mut iter)?;

            let system_program = iter.next_non_mut("system program")?;
            // Must be signer if custom rent payer.
            // Rent sponsor is not signer.
            let rent_payer = iter.next_mut("rent payer")?;

            Some(CompressibleAccounts {
                payer,
                parsed_config,
                system_program,
                rent_payer,
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

#[profile]
#[inline(always)]
pub fn parse_config_account<'info>(
    config_account: &'info AccountInfo,
) -> Result<&'info CompressibleConfig, ProgramError> {
    // Validate config account owner
    check_owner(
        &pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX").to_bytes(),
        config_account,
    )?;
    // Parse config data
    let data = unsafe { config_account.borrow_data_unchecked() };
    check_discriminator::<CompressibleConfig>(data)?;
    let config = bytemuck::pod_from_bytes::<CompressibleConfig>(&data[8..]).map_err(|e| {
        msg!("Failed to deserialize CompressibleConfig: {:?}", e);
        ProgramError::InvalidAccountData
    })?;

    Ok(config)
}

#[profile]
#[inline(always)]
pub fn next_config_account<'info>(
    iter: &mut AccountIterator<'info, AccountInfo>,
) -> Result<&'info CompressibleConfig, ProgramError> {
    let config_account = iter.next_non_mut("compressible config")?;
    let config = parse_config_account(config_account)?;

    // Validate config is active (only active allowed for account creation)
    config.validate_active().map_err(ProgramError::from)?;

    Ok(config)
}

/// Process the create token account instruction
#[profile]
pub fn process_create_token_account(
    account_infos: &[AccountInfo],
    mut instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let inputs = if instruction_data.len() == 32 {
        // Backward compatibility with spl token program instruction data.
        let mut instruction_data_array = [0u8; 32];
        instruction_data_array.copy_from_slice(instruction_data);
        CreateTokenAccountInstructionData {
            owner: Pubkey::from(instruction_data_array),
            compressible_config: None,
        }
    } else {
        CreateTokenAccountInstructionData::deserialize(&mut instruction_data)
            .map_err(ProgramError::from)?
    };

    // Parse and validate accounts
    let accounts = CreateCTokenAccounts::parse(account_infos, &inputs)?;

    // Create account via cpi
    let (compressible_config_account, custom_rent_payer) = if let Some(compressible) =
        accounts.compressible.as_ref()
    {
        let compressible_config = inputs
            .compressible_config
            .as_ref()
            .ok_or(ProgramError::InvalidInstructionData)?;

        // Validate that rent_payment is not exactly 1 epoch (footgun prevention)
        if compressible_config.rent_payment == 1 {
            msg!("Prefunding for exactly 1 epoch is not allowed. If the account is created near an epoch boundary, it could become immediately compressible. Use 0 or 2+ epochs.");
            return Err(anchor_compressed_token::ErrorCode::OneEpochPrefundingNotAllowed.into());
        }

        if let Some(compress_to_pubkey) = compressible_config.compress_to_account_pubkey.as_ref() {
            // Compress to pubkey specifies compression to account pubkey instead of the owner.
            // This is useful for pda token accounts that rely on pubkey derivation but have a program wide
            // authority pda as owner.
            // To prevent compressing ctokens to owners that cannot sign, prevent misconfiguration,
            // we check that the account is a pda and can be signer with known seeds.
            compress_to_pubkey.check_seeds(accounts.token_account.key())?;
        }

        let config_account = &compressible.parsed_config;
        let rent = compressible
            .parsed_config
            .rent_config
            .get_rent_with_compression_cost(
                COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
                compressible_config.rent_payment as u64,
            );
        let account_size = COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize;

        let custom_rent_payer =
            *compressible.rent_payer.key() != config_account.rent_sponsor.to_bytes();
        if custom_rent_payer {
            // custom rent payer for account creation -> pays rent exemption
            // rent payer must be signer.
            create_account_with_custom_rent_payer(
                compressible.rent_payer,
                accounts.token_account,
                account_size,
                rent,
            )
            .map_err(convert_program_error)?;

            (Some(*config_account), Some(*compressible.rent_payer.key()))
        } else {
            // Rent recipient is fee payer for account creation -> pays rent exemption
            let version_bytes = config_account.version.to_le_bytes();
            let bump_seed = [config_account.rent_sponsor_bump];
            let seeds = [
                Seed::from(b"rent_sponsor".as_ref()),
                Seed::from(version_bytes.as_ref()),
                Seed::from(bump_seed.as_ref()),
            ];

            let seeds_inputs = [seeds.as_slice()];

            // PDA creates account with only rent-exempt balance
            create_pda_account(
                compressible.rent_payer,
                accounts.token_account,
                account_size,
                seeds_inputs,
                None,
            )?;

            // Payer transfers the additional rent (compression incentive)
            transfer_lamports_via_cpi(rent, compressible.payer, accounts.token_account)
                .map_err(convert_program_error)?;
            (Some(*config_account), None)
        }
    } else {
        (None, None)
    };

    // Initialize the token account (assumes account already exists and is owned by our program)
    initialize_ctoken_account(
        accounts.token_account,
        accounts.mint.key(),
        &inputs.owner.to_bytes(),
        inputs.compressible_config,
        compressible_config_account,
        custom_rent_payer,
    )
}

#[profile]
#[inline(always)]
fn create_account_with_custom_rent_payer(
    rent_payer: &AccountInfo,
    token_account: &AccountInfo,
    account_size: usize,
    rent: u64,
) -> pinocchio::ProgramResult {
    let solana_rent = Rent::get()?;
    let lamports = solana_rent.minimum_balance(account_size) + rent;

    let create_account = CreateAccount {
        from: rent_payer,
        to: token_account,
        lamports,
        space: account_size as u64,
        owner: &crate::LIGHT_CPI_SIGNER.program_id,
    };
    create_account.invoke()
}
