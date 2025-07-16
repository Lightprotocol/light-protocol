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
use std::collections::HashSet;

pub const COMPRESSIBLE_CONFIG_SEED: &[u8] = b"compressible_config";
pub const MAX_ADDRESS_TREES_PER_SPACE: usize = 4;

/// BPF Loader Upgradeable Program ID
/// BPFLoaderUpgradeab1e11111111111111111111111
// const BPF_LOADER_UPGRADEABLE_ID: Pubkey = Pubkey::new_from_array([
//     2, 168, 246, 145, 78, 136, 161, 110, 57, 90, 225, 40, 148, 143, 250, 105, 86, 147, 55, 104, 24,
//     221, 71, 67, 82, 33, 243, 198, 0, 0, 0, 0,
// ]);
const BPF_LOADER_UPGRADEABLE_ID: Pubkey =
    Pubkey::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");

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
    /// Address space for compressed accounts (1-4 address_treess allowed)
    pub address_space: Vec<Pubkey>,
    /// PDA bump seed
    pub bump: u8,
}

impl Default for CompressibleConfig {
    fn default() -> Self {
        Self {
            version: 0,
            discriminator: CompressibleConfig::LIGHT_DISCRIMINATOR,
            compression_delay: 216_000, // 24h
            update_authority: Pubkey::default(),
            rent_recipient: Pubkey::default(),
            address_space: vec![Pubkey::default()],
            bump: 0,
        }
    }
}

impl CompressibleConfig {
    pub const LEN: usize = 1 + 8 + 4 + 32 + 32 + 4 + (32 * MAX_ADDRESS_TREES_PER_SPACE) + 1; // 241 bytes max

    /// Calculate the exact size needed for a CompressibleConfig with the given number of address spaces
    pub fn size_for_address_spaces(num_address_spaces: usize) -> usize {
        1 + 8 + 4 + 32 + 32 + 4 + (32 * num_address_spaces) + 1
    }

    /// Derives the config PDA address
    pub fn derive_pda(program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[COMPRESSIBLE_CONFIG_SEED], program_id)
    }

    /// Returns the primary address space (first in the list)
    pub fn primary_address_space(&self) -> &Pubkey {
        &self.address_space[0]
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
        if self.address_space.is_empty() || self.address_space.len() > MAX_ADDRESS_TREES_PER_SPACE {
            msg!(
                "Invalid number of address spaces: {}",
                self.address_space.len()
            );
            return Err(LightSdkError::ConstraintViolation);
        }
        Ok(())
    }

    /// Loads and validates config from account, checking owner
    pub fn load_checked(account: &AccountInfo, program_id: &Pubkey) -> Result<Self, LightSdkError> {
        if account.owner != program_id {
            msg!(
                "Config account owner mismatch. Expected: {}. Found: {}.",
                program_id,
                account.owner
            );
            return Err(LightSdkError::ConstraintViolation);
        }
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
/// * `address_space` - Address spaces for compressed accounts (1-4 allowed)
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
    address_space: Vec<Pubkey>,
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

    // Validate address spaces
    if address_space.is_empty() || address_space.len() > MAX_ADDRESS_TREES_PER_SPACE {
        msg!("Invalid number of address spaces: {}", address_space.len());
        return Err(LightSdkError::ConstraintViolation);
    }

    // Validate no duplicate pubkeys in address_space
    validate_address_space_no_duplicates(&address_space)?;

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

    // Get rent for the exact size needed
    let rent = Rent::get()?;
    let account_size = CompressibleConfig::size_for_address_spaces(address_space.len());
    let rent_lamports = rent.minimum_balance(account_size);

    // Create the account with exact size
    let seeds = &[COMPRESSIBLE_CONFIG_SEED, &[bump]];
    let create_account_ix = system_instruction::create_account(
        payer.key,
        config_account.key,
        rent_lamports,
        account_size as u64,
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
        address_space,
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
/// * `new_address_space` - Optional new address spaces (1-4 allowed)
/// * `new_compression_delay` - Optional new compression delay
/// * `owner_program_id` - The program that owns the config
///
/// # Returns
/// * `Ok(())` if config was updated successfully
/// * `Err(LightSdkError)` if there was an error
pub fn update_compression_config<'info>(
    config_account: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    new_update_authority: Option<&Pubkey>,
    new_rent_recipient: Option<&Pubkey>,
    new_address_space: Option<Vec<Pubkey>>,
    new_compression_delay: Option<u32>,
    owner_program_id: &Pubkey,
) -> Result<(), LightSdkError> {
    // Load and validate existing config
    let mut config = CompressibleConfig::load_checked(config_account, owner_program_id)?;

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
    if let Some(new_spaces) = new_address_space {
        if new_spaces.is_empty() || new_spaces.len() > MAX_ADDRESS_TREES_PER_SPACE {
            msg!("Invalid number of address spaces: {}", new_spaces.len());
            return Err(LightSdkError::ConstraintViolation);
        }

        // Validate no duplicate pubkeys in new address_space
        validate_address_space_no_duplicates(&new_spaces)?;

        // Validate that we're only adding, not removing existing pubkeys
        validate_address_space_only_adds(&config.address_space, &new_spaces)?;

        config.address_space = new_spaces;
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
/// * `address_space` - Address spaces for compressed accounts (1-4 allowed)
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
    address_space: Vec<Pubkey>,
    compression_delay: u32,
    payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    program_id: &Pubkey,
) -> Result<(), LightSdkError> {
    msg!(
        "create_compression_config_checked program_data_account: {:?}",
        program_data_account.key.log()
    );
    msg!(
        "create_compression_config_checked program_id: {:?}",
        program_id.log()
    );
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

/// Validates that address_space contains no duplicate pubkeys
fn validate_address_space_no_duplicates(address_space: &[Pubkey]) -> Result<(), LightSdkError> {
    let mut seen = HashSet::new();
    for pubkey in address_space {
        if !seen.insert(pubkey) {
            msg!("Duplicate pubkey found in address_space: {}", pubkey);
            return Err(LightSdkError::ConstraintViolation);
        }
    }
    Ok(())
}

/// Validates that new_address_space only adds to existing address_space (no removals)
fn validate_address_space_only_adds(
    existing_address_space: &[Pubkey],
    new_address_space: &[Pubkey],
) -> Result<(), LightSdkError> {
    // Check that all existing pubkeys are still present in new address space
    for existing_pubkey in existing_address_space {
        if !new_address_space.contains(existing_pubkey) {
            msg!(
                "Cannot remove existing pubkey from address_space: {}",
                existing_pubkey
            );
            return Err(LightSdkError::ConstraintViolation);
        }
    }
    Ok(())
}
