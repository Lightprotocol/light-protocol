use anchor_lang::{prelude::ProgramError, pubkey};
use borsh::BorshDeserialize;
use light_account_checks::{
    checks::{check_discriminator, check_owner},
    AccountIterator,
};
use light_compressed_account::Pubkey;
use light_compressible::config::CompressibleConfig;
use light_ctoken_interface::instructions::create_ctoken_account::CreateTokenAccountInstructionData;
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, instruction::Seed};
use spl_pod::{bytemuck, solana_msg::msg};

use crate::{
    extensions::has_mint_extensions,
    shared::{
        convert_program_error, create_pda_account,
        initialize_ctoken_account::{initialize_ctoken_account, CTokenInitConfig},
        transfer_lamports_via_cpi,
    },
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
pub fn parse_config_account(
    config_account: &AccountInfo,
) -> Result<&CompressibleConfig, ProgramError> {
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
    let (compressible_config_account, custom_rent_payer, mint_extensions) = if let Some(
        compressible,
    ) =
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

        // Check which extensions the mint has (single deserialization)
        let mint_extensions = has_mint_extensions(accounts.mint)?;

        // If restricted extensions exist, compression_only must be set
        if mint_extensions.has_restricted_extensions() && compressible_config.compression_only == 0
        {
            msg!("Mint has restricted extensions - compression_only must be set");
            return Err(anchor_compressed_token::ErrorCode::CompressionOnlyRequired.into());
        }

        // Calculate account size based on extensions
        let account_size = mint_extensions.calculate_account_size(true /* has_compressible */);

        let config_account = &compressible.parsed_config;
        let rent = compressible
            .parsed_config
            .rent_config
            .get_rent_with_compression_cost(account_size, compressible_config.rent_payment as u64);
        let account_size = account_size as usize;

        let custom_rent_payer =
            *compressible.rent_payer.key() != config_account.rent_sponsor.to_bytes();

        // Prevents setting executable accounts as rent_sponsor
        if custom_rent_payer && !compressible.rent_payer.is_signer() {
            msg!("Custom rent payer must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Build fee_payer seeds (rent_sponsor PDA or None for custom keypair)
        let version_bytes = config_account.version.to_le_bytes();
        let bump_seed = [config_account.rent_sponsor_bump];
        let rent_sponsor_seeds = [
            Seed::from(b"rent_sponsor".as_ref()),
            Seed::from(version_bytes.as_ref()),
            Seed::from(bump_seed.as_ref()),
        ];

        // fee_payer_seeds: Some for rent_sponsor PDA, None for custom keypair
        // new_account_seeds: None (token_account is always a keypair signer)
        let fee_payer_seeds = if custom_rent_payer {
            None
        } else {
            Some(rent_sponsor_seeds.as_slice())
        };

        // Create token account (handles DoS prevention internally)
        create_pda_account(
            compressible.rent_payer,
            accounts.token_account,
            account_size,
            fee_payer_seeds,
            None, // token_account is keypair signer
            None, // no additional lamports here
        )?;

        // Payer transfers the additional rent (compression incentive)
        transfer_lamports_via_cpi(rent, compressible.payer, accounts.token_account)
            .map_err(convert_program_error)?;

        if custom_rent_payer {
            (
                Some(*config_account),
                Some(*compressible.rent_payer.key()),
                mint_extensions,
            )
        } else {
            (Some(*config_account), None, mint_extensions)
        }
    } else {
        // Non-compressible accounts cannot be created for mints with restricted extensions
        let mint_extensions = has_mint_extensions(accounts.mint)?;
        if mint_extensions.has_restricted_extensions() {
            msg!("Mints with restricted extensions require compressible accounts");
            return Err(anchor_compressed_token::ErrorCode::CompressibleRequired.into());
        }
        (None, None, mint_extensions)
    };

    // Initialize the token account (assumes account already exists and is owned by our program)
    initialize_ctoken_account(
        accounts.token_account,
        CTokenInitConfig {
            mint: accounts.mint.key(),
            owner: &inputs.owner.to_bytes(),
            compressible: inputs.compressible_config,
            compressible_config_account,
            custom_rent_payer,
            mint_extensions,
            mint_account: accounts.mint,
        },
    )
}
