use crate::{error::LightSdkError, LightDiscriminator};
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize as BorshDeserialize, AnchorSerialize as BorshSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize, BorshSerialize};
use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_msg::msg;
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_system_interface::instruction as system_instruction;
use solana_sysvar::Sysvar;

pub const COMPRESSIBLE_CONFIG_SEED: &[u8] = b"compressible_config";

/// BPF Loader Upgradeable Program ID
/// BPFLoaderUpgradeab1e11111111111111111111111
const BPF_LOADER_UPGRADEABLE_ID: Pubkey = Pubkey::new_from_array([
    2, 168, 246, 145, 78, 136, 161, 110, 57, 90, 225, 40, 148, 143, 250, 105, 86, 147, 55, 104, 24,
    221, 71, 67, 82, 33, 243, 198, 0, 0, 0, 0,
]);

/// Global configuration for compressible accounts
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, LightDiscriminator)]
pub struct CompressibleConfig {
    /// Config version for future upgrades
    pub version: u8,
    /// Discriminator for account validation
    pub discriminator: [u8; 8],
    /// Number of slots to wait before compression is allowed
    pub compression_delay: u32,
    /// Authority that can update the config
    pub update_authority: Pubkey,
    /// Account that receives rent from compressed PDAs
    pub rent_recipient: Pubkey,
    /// Address space for compressed accounts
    pub address_space: Pubkey,
    /// PDA bump seed
    pub bump: u8,
}

impl Default for CompressibleConfig {
    fn default() -> Self {
        Self {
            version: 1,
            discriminator: CompressibleConfig::LIGHT_DISCRIMINATOR,
            compression_delay: 100,
            update_authority: Pubkey::default(),
            rent_recipient: Pubkey::default(),
            address_space: Pubkey::default(),
            bump: 0,
        }
    }
}

impl CompressibleConfig {
    pub const LEN: usize = 1 + 8 + 4 + 32 + 32 + 32 + 1; // 110 bytes

    /// Derives the config PDA address
    pub fn derive_pda(program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[COMPRESSIBLE_CONFIG_SEED], program_id)
    }

    /// Validates the config account
    pub fn validate(&self) -> Result<(), LightSdkError> {
        if self.discriminator != Self::LIGHT_DISCRIMINATOR {
            msg!("Invalid config discriminator");
            return Err(LightSdkError::ConstraintViolation);
        }
        if self.version != 1 {
            msg!("Unsupported config version: {}", self.version);
            return Err(LightSdkError::ConstraintViolation);
        }
        Ok(())
    }

    /// Loads and validates config from account
    pub fn load(account: &AccountInfo) -> Result<Self, LightSdkError> {
        let data = account.try_borrow_data()?;
        let config = Self::try_from_slice(&data).map_err(|_| LightSdkError::Borsh)?;
        config.validate()?;
        Ok(config)
    }
}

/// Creates a new compressible config PDA
///
/// # Security - Solana Best Practice
/// This function follows the standard Solana pattern where only the program's
/// upgrade authority can create the initial config. This prevents unauthorized
/// parties from hijacking the config system.
///
/// # Arguments
/// * `config_account` - The config PDA account to initialize
/// * `update_authority` - Authority that can update the config after creation
/// * `rent_recipient` - Account that receives rent from compressed PDAs
/// * `address_space` - Address space for compressed accounts
/// * `compression_delay` - Number of slots to wait before compression
/// * `payer` - Account paying for the PDA creation
/// * `system_program` - System program
/// * `program_id` - The program that owns the config
///
/// # Required Validation (must be done by caller)
/// The caller MUST validate that the signer is the program's upgrade authority
/// by checking against the program data account. This cannot be done in the SDK
/// due to dependency constraints.
///
/// # Returns
/// * `Ok(())` if config was created successfully
/// * `Err(LightSdkError)` if there was an error
pub fn create_compression_config_unchecked<'info>(
    config_account: &AccountInfo<'info>,
    update_authority: &AccountInfo<'info>,
    rent_recipient: &Pubkey,
    address_space: &Pubkey,
    compression_delay: u32,
    payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    program_id: &Pubkey,
) -> Result<(), LightSdkError> {
    // Check if already initialized
    if config_account.data_len() > 0 {
        msg!("Config account already initialized");
        return Err(LightSdkError::ConstraintViolation);
    }

    // Verify update authority is signer
    if !update_authority.is_signer {
        msg!("Update authority must be signer for initial config creation");
        return Err(LightSdkError::ConstraintViolation);
    }

    // Derive PDA and verify
    let (derived_pda, bump) = CompressibleConfig::derive_pda(program_id);
    if derived_pda != *config_account.key {
        msg!("Invalid config PDA");
        return Err(LightSdkError::ConstraintViolation);
    }

    // Get rent
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(CompressibleConfig::LEN);

    // Create the account
    let seeds = &[COMPRESSIBLE_CONFIG_SEED, &[bump]];
    let create_account_ix = system_instruction::create_account(
        payer.key,
        config_account.key,
        rent_lamports,
        CompressibleConfig::LEN as u64,
        program_id,
    );

    invoke_signed(
        &create_account_ix,
        &[
            payer.clone(),
            config_account.clone(),
            system_program.clone(),
        ],
        &[seeds],
    )?;

    // Initialize config data
    let config = CompressibleConfig {
        version: 1,
        discriminator: CompressibleConfig::LIGHT_DISCRIMINATOR,
        compression_delay,
        update_authority: *update_authority.key,
        rent_recipient: *rent_recipient,
        address_space: *address_space,
        bump,
    };

    // Write to account
    let mut data = config_account.try_borrow_mut_data()?;
    config
        .serialize(&mut &mut data[..])
        .map_err(|_| LightSdkError::Borsh)?;

    Ok(())
}

/// Updates an existing compressible config
///
/// # Arguments
/// * `config_account` - The config PDA account to update
/// * `authority` - Current update authority (must match config)
/// * `new_update_authority` - Optional new update authority
/// * `new_rent_recipient` - Optional new rent recipient
/// * `new_address_space` - Optional new address space
/// * `new_compression_delay` - Optional new compression delay
///
/// # Returns
/// * `Ok(())` if config was updated successfully
/// * `Err(LightSdkError)` if there was an error
pub fn update_config<'info>(
    config_account: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    new_update_authority: Option<&Pubkey>,
    new_rent_recipient: Option<&Pubkey>,
    new_address_space: Option<&Pubkey>,
    new_compression_delay: Option<u32>,
) -> Result<(), LightSdkError> {
    // Load and validate existing config
    let mut config = CompressibleConfig::load(config_account)?;

    // Check authority
    if !authority.is_signer {
        msg!("Update authority must be signer");
        return Err(LightSdkError::ConstraintViolation);
    }
    if *authority.key != config.update_authority {
        msg!("Invalid update authority");
        return Err(LightSdkError::ConstraintViolation);
    }

    // Apply updates
    if let Some(new_authority) = new_update_authority {
        config.update_authority = *new_authority;
    }
    if let Some(new_recipient) = new_rent_recipient {
        config.rent_recipient = *new_recipient;
    }
    if let Some(new_space) = new_address_space {
        config.address_space = *new_space;
    }
    if let Some(new_delay) = new_compression_delay {
        config.compression_delay = new_delay;
    }

    // Write updated config
    let mut data = config_account.try_borrow_mut_data()?;
    config
        .serialize(&mut &mut data[..])
        .map_err(|_| LightSdkError::Borsh)?;

    Ok(())
}

/// Verifies that the signer is the program's upgrade authority
///
/// # Arguments
/// * `program_id` - The program to check
/// * `program_data_account` - The program's data account (ProgramData)
/// * `authority` - The authority to verify
///
/// # Returns
/// * `Ok(())` if authority is valid
/// * `Err(LightSdkError)` if authority is invalid or verification fails
pub fn verify_program_upgrade_authority(
    program_id: &Pubkey,
    program_data_account: &AccountInfo,
    authority: &AccountInfo,
) -> Result<(), LightSdkError> {
    // Verify program data account PDA
    let (expected_program_data, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &BPF_LOADER_UPGRADEABLE_ID);
    if program_data_account.key != &expected_program_data {
        msg!("Invalid program data account");
        return Err(LightSdkError::ConstraintViolation);
    }

    // Verify that the signer is the program's upgrade authority
    let data = program_data_account.try_borrow_data()?;

    // The UpgradeableLoaderState::ProgramData format:
    // 4 bytes discriminator + 8 bytes slot + 1 byte option + 32 bytes authority
    if data.len() < 45 {
        msg!("Program data account too small");
        return Err(LightSdkError::ConstraintViolation);
    }

    // Check discriminator (should be 3 for ProgramData)
    let discriminator = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if discriminator != 3 {
        msg!("Invalid program data discriminator");
        return Err(LightSdkError::ConstraintViolation);
    }

    // Skip slot (8 bytes) and check if authority exists (1 byte flag)
    let has_authority = data[12] == 1;
    if !has_authority {
        msg!("Program has no upgrade authority");
        return Err(LightSdkError::ConstraintViolation);
    }

    // Read the upgrade authority pubkey (32 bytes)
    let mut authority_bytes = [0u8; 32];
    authority_bytes.copy_from_slice(&data[13..45]);
    let upgrade_authority = Pubkey::new_from_array(authority_bytes);

    // Verify the signer matches the upgrade authority
    if !authority.is_signer {
        msg!("Authority must be signer");
        return Err(LightSdkError::ConstraintViolation);
    }

    if *authority.key != upgrade_authority {
        msg!("Signer is not the program's upgrade authority");
        return Err(LightSdkError::ConstraintViolation);
    }

    Ok(())
}

/// Creates a new compressible config PDA with program upgrade authority validation
///
/// # Security
/// This function verifies that the signer is the program's upgrade authority
/// before creating the config. This ensures only the program deployer can
/// initialize the configuration.
///
/// # Arguments
/// * `config_account` - The config PDA account to initialize
/// * `update_authority` - Must be the program's upgrade authority
/// * `program_data_account` - The program's data account for validation
/// * `rent_recipient` - Account that receives rent from compressed PDAs
/// * `address_space` - Address space for compressed accounts
/// * `compression_delay` - Number of slots to wait before compression
/// * `payer` - Account paying for the PDA creation
/// * `system_program` - System program
/// * `program_id` - The program that owns the config
///
/// # Returns
/// * `Ok(())` if config was created successfully
/// * `Err(LightSdkError)` if there was an error or authority validation fails
pub fn create_compression_config_checked<'info>(
    config_account: &AccountInfo<'info>,
    update_authority: &AccountInfo<'info>,
    program_data_account: &AccountInfo<'info>,
    rent_recipient: &Pubkey,
    address_space: &Pubkey,
    compression_delay: u32,
    payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    program_id: &Pubkey,
) -> Result<(), LightSdkError> {
    // Verify the signer is the program's upgrade authority
    verify_program_upgrade_authority(program_id, program_data_account, update_authority)?;

    // Create the config with validated authority
    create_compression_config_unchecked(
        config_account,
        update_authority,
        rent_recipient,
        address_space,
        compression_delay,
        payer,
        system_program,
        program_id,
    )
}
