use std::collections::HashSet;

use light_compressible::rent::RentConfig;
use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_loader_v3_interface::state::UpgradeableLoaderState;
use solana_msg::msg;
use solana_pubkey::Pubkey;
use solana_system_interface::instruction as system_instruction;
use solana_sysvar::{rent::Rent, Sysvar};

use crate::{error::LightSdkError, AnchorDeserialize, AnchorSerialize};

pub const COMPRESSIBLE_CONFIG_SEED: &[u8] = b"compressible_config";

// Re-export from sdk-types
pub use light_sdk_types::constants::RENT_SPONSOR_SEED;
pub const MAX_ADDRESS_TREES_PER_SPACE: usize = 1;
const BPF_LOADER_UPGRADEABLE_ID: Pubkey =
    Pubkey::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");

// TODO: add rent_authority + rent_func like in token.
/// Global configuration for compressible accounts
#[derive(Clone, AnchorDeserialize, AnchorSerialize, Debug)]
pub struct LightConfig {
    /// Config version for future upgrades
    pub version: u8,
    /// Lamports to top up on each write (heuristic)
    pub write_top_up: u32,
    /// Authority that can update the config
    pub update_authority: Pubkey,
    /// Account that receives rent from compressed PDAs
    pub rent_sponsor: Pubkey,
    /// Authority that can compress/close PDAs (distinct from rent_sponsor)
    pub compression_authority: Pubkey,
    /// Rent function parameters for compressibility and distribution
    pub rent_config: RentConfig,
    /// Config bump seed (0)
    pub config_bump: u8,
    /// PDA bump seed
    pub bump: u8,
    /// Address space for compressed accounts (currently 1 address_tree allowed)
    pub address_space: Vec<Pubkey>,
}

impl LightConfig {
    pub const LEN: usize = 1
        + 4
        + 32
        + 32
        + 32
        + core::mem::size_of::<RentConfig>()
        + 1
        + 1
        + 4
        + (32 * MAX_ADDRESS_TREES_PER_SPACE);

    /// Calculate the exact size needed for a LightConfig with the given
    /// number of address spaces
    pub fn size_for_address_space(num_address_trees: usize) -> usize {
        1 + 4
            + 32
            + 32
            + 32
            + core::mem::size_of::<RentConfig>()
            + 1
            + 1
            + 4
            + (32 * num_address_trees)
    }

    /// Derives the config PDA address with config bump
    pub fn derive_pda(program_id: &Pubkey, config_bump: u8) -> (Pubkey, u8) {
        // Convert u8 to u16 to match program-libs derivation (uses u16 with to_le_bytes)
        let config_bump_u16 = config_bump as u16;
        Pubkey::find_program_address(
            &[COMPRESSIBLE_CONFIG_SEED, &config_bump_u16.to_le_bytes()],
            program_id,
        )
    }

    /// Derives the default config PDA address (config_bump = 0)
    pub fn derive_default_pda(program_id: &Pubkey) -> (Pubkey, u8) {
        Self::derive_pda(program_id, 0)
    }

    /// Derives the rent sponsor PDA address for a program.
    /// Seeds: ["rent_sponsor"]
    pub fn derive_rent_sponsor_pda(program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[RENT_SPONSOR_SEED], program_id)
    }

    /// Checks the config account
    pub fn validate(&self) -> Result<(), crate::ProgramError> {
        if self.version != 1 {
            msg!(
                "LightConfig validation failed: Unsupported config version: {}",
                self.version
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }
        if self.address_space.len() != 1 {
            msg!(
                "LightConfig validation failed: Address space must contain exactly 1 pubkey, found: {}",
                self.address_space.len()
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }
        // For now, only allow config_bump = 0 to keep it simple
        if self.config_bump != 0 {
            msg!(
                "LightConfig validation failed: Config bump must be 0 for now, found: {}",
                self.config_bump
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }
        Ok(())
    }

    /// Loads and validates config from account, checking owner and PDA derivation
    #[inline(never)]
    pub fn load_checked(
        account: &AccountInfo,
        program_id: &Pubkey,
    ) -> Result<Self, crate::ProgramError> {
        if account.owner != program_id {
            msg!(
                "LightConfig::load_checked failed: Config account owner mismatch. Expected: {:?}. Found: {:?}.",
                program_id,
                account.owner
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }
        let data = account.try_borrow_data()?;
        let config = Self::try_from_slice(&data).map_err(|err| {
            msg!(
                "LightConfig::load_checked failed: Failed to deserialize config data: {:?}",
                err
            );
            LightSdkError::Borsh
        })?;
        config.validate()?;

        // CHECK: PDA derivation
        let (expected_pda, _) = Self::derive_pda(program_id, config.config_bump);
        if expected_pda != *account.key {
            msg!(
                "LightConfig::load_checked failed: Config account key mismatch. Expected PDA: {:?}. Found: {:?}.",
                expected_pda,
                account.key
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }

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
/// * `rent_sponsor` - Account that receives rent from compressed PDAs
/// * `compression_authority` - Authority that can compress/close PDAs
/// * `rent_config` - Rent function parameters
/// * `write_top_up` - Lamports to top up on each write
/// * `address_space` - Address space for compressed accounts (currently 1 address_tree allowed)
/// * `config_bump` - Config bump seed (must be 0 for now)
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
/// * `Err(ProgramError)` if there was an error
#[allow(clippy::too_many_arguments)]
pub fn process_initialize_light_config<'info>(
    config_account: &AccountInfo<'info>,
    update_authority: &AccountInfo<'info>,
    rent_sponsor: &Pubkey,
    compression_authority: &Pubkey,
    rent_config: RentConfig,
    write_top_up: u32,
    address_space: Vec<Pubkey>,
    config_bump: u8,
    payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    program_id: &Pubkey,
) -> Result<(), crate::ProgramError> {
    // CHECK: only 1 address_space
    if config_bump != 0 {
        msg!("Config bump must be 0 for now, found: {}", config_bump);
        return Err(LightSdkError::ConstraintViolation.into());
    }

    // CHECK: not already initialized
    if config_account.data_len() > 0 {
        msg!("Config account already initialized");
        return Err(LightSdkError::ConstraintViolation.into());
    }

    // CHECK: only 1 address_space
    if address_space.len() != 1 {
        msg!(
            "Address space must contain exactly 1 pubkey, found: {}",
            address_space.len()
        );
        return Err(LightSdkError::ConstraintViolation.into());
    }

    // CHECK: unique pubkeys in address_space
    validate_address_space_no_duplicates(&address_space)?;

    // CHECK: signer
    if !update_authority.is_signer {
        msg!("Update authority must be signer for initial config creation");
        return Err(LightSdkError::ConstraintViolation.into());
    }

    // CHECK: pda derivation
    let (derived_pda, bump) = LightConfig::derive_pda(program_id, config_bump);
    if derived_pda != *config_account.key {
        msg!("Invalid config PDA");
        return Err(LightSdkError::ConstraintViolation.into());
    }

    let rent = Rent::get().map_err(LightSdkError::from)?;
    let account_size = LightConfig::size_for_address_space(address_space.len());
    let rent_lamports = rent.minimum_balance(account_size);

    // Use u16 to_le_bytes to match derive_pda (2 bytes instead of 1)
    let config_bump_bytes = (config_bump as u16).to_le_bytes();
    let seeds = &[
        COMPRESSIBLE_CONFIG_SEED,
        config_bump_bytes.as_ref(),
        &[bump],
    ];
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
    )
    .map_err(LightSdkError::from)?;

    let config = LightConfig {
        version: 1,
        write_top_up,
        update_authority: *update_authority.key,
        rent_sponsor: *rent_sponsor,
        compression_authority: *compression_authority,
        rent_config,
        config_bump,
        address_space,
        bump,
    };

    let mut data = config_account
        .try_borrow_mut_data()
        .map_err(LightSdkError::from)?;
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
/// * `new_rent_sponsor` - Optional new rent recipient
/// * `new_compression_authority` - Optional new compression authority
/// * `new_rent_config` - Optional new rent function parameters
/// * `new_write_top_up` - Optional new write top-up amount
/// * `new_address_space` - Optional new address space (currently 1 address_tree allowed)
/// * `owner_program_id` - The program that owns the config
///
/// # Returns
/// * `Ok(())` if config was updated successfully
/// * `Err(ProgramError)` if there was an error
#[allow(clippy::too_many_arguments)]
pub fn process_update_light_config<'info>(
    config_account: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    new_update_authority: Option<&Pubkey>,
    new_rent_sponsor: Option<&Pubkey>,
    new_compression_authority: Option<&Pubkey>,
    new_rent_config: Option<RentConfig>,
    new_write_top_up: Option<u32>,
    new_address_space: Option<Vec<Pubkey>>,
    owner_program_id: &Pubkey,
) -> Result<(), crate::ProgramError> {
    // CHECK: PDA derivation
    let mut config = LightConfig::load_checked(config_account, owner_program_id)?;

    // CHECK: signer
    if !authority.is_signer {
        msg!("Update authority must be signer");
        return Err(LightSdkError::ConstraintViolation.into());
    }
    // CHECK: authority
    if *authority.key != config.update_authority {
        msg!("Invalid update authority");
        return Err(LightSdkError::ConstraintViolation.into());
    }

    if let Some(new_authority) = new_update_authority {
        config.update_authority = *new_authority;
    }
    if let Some(new_recipient) = new_rent_sponsor {
        config.rent_sponsor = *new_recipient;
    }
    if let Some(new_auth) = new_compression_authority {
        config.compression_authority = *new_auth;
    }
    if let Some(new_rcfg) = new_rent_config {
        config.rent_config = new_rcfg;
    }
    if let Some(new_top_up) = new_write_top_up {
        config.write_top_up = new_top_up;
    }
    if let Some(new_address_space) = new_address_space {
        // CHECK: address space length
        if new_address_space.len() != MAX_ADDRESS_TREES_PER_SPACE {
            msg!(
                "New address space must contain exactly 1 pubkey, found: {}",
                new_address_space.len()
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }

        validate_address_space_no_duplicates(&new_address_space)?;

        validate_address_space_only_adds(&config.address_space, &new_address_space)?;

        config.address_space = new_address_space;
    }

    let mut data = config_account.try_borrow_mut_data().map_err(|e| {
        msg!("Failed to borrow mut data for config_account: {:?}", e);
        LightSdkError::from(e)
    })?;
    config.serialize(&mut &mut data[..]).map_err(|e| {
        msg!("Failed to serialize updated config: {:?}", e);
        LightSdkError::Borsh
    })?;

    Ok(())
}

/// Checks that the signer is the program's upgrade authority
///
/// # Arguments
/// * `program_id` - The program to check
/// * `program_data_account` - The program's data account (ProgramData)
/// * `authority` - The authority to verify
///
/// # Returns
/// * `Ok(())` if authority is valid
/// * `Err(LightSdkError)` if authority is invalid or verification fails
pub fn check_program_upgrade_authority(
    program_id: &Pubkey,
    program_data_account: &AccountInfo,
    authority: &AccountInfo,
) -> Result<(), crate::ProgramError> {
    // CHECK: program data PDA
    let (expected_program_data, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &BPF_LOADER_UPGRADEABLE_ID);
    if program_data_account.key != &expected_program_data {
        msg!("Invalid program data account");
        return Err(LightSdkError::ConstraintViolation.into());
    }

    let data = program_data_account.try_borrow_data()?;
    let program_state: UpgradeableLoaderState = bincode::deserialize(&data).map_err(|_| {
        msg!("Failed to deserialize program data account");
        LightSdkError::ConstraintViolation
    })?;

    // Extract upgrade authority
    let upgrade_authority = match program_state {
        UpgradeableLoaderState::ProgramData {
            slot: _,
            upgrade_authority_address,
        } => {
            match upgrade_authority_address {
                Some(auth) => {
                    // Check for invalid zero authority when authority exists
                    if auth == Pubkey::default() {
                        msg!("Invalid state: authority is zero pubkey");
                        return Err(LightSdkError::ConstraintViolation.into());
                    }
                    auth
                }
                None => {
                    msg!("Program has no upgrade authority");
                    return Err(LightSdkError::ConstraintViolation.into());
                }
            }
        }
        _ => {
            msg!("Account is not ProgramData, found: {:?}", program_state);
            return Err(LightSdkError::ConstraintViolation.into());
        }
    };

    // CHECK: upgrade authority is signer
    if !authority.is_signer {
        msg!("Authority must be signer");
        return Err(LightSdkError::ConstraintViolation.into());
    }

    // CHECK: upgrade authority is program's upgrade authority
    if *authority.key != upgrade_authority {
        msg!(
            "Signer is not the program's upgrade authority. Signer: {:?}, Expected Authority: {:?}",
            authority.key,
            upgrade_authority
        );
        return Err(LightSdkError::ConstraintViolation.into());
    }

    Ok(())
}

/// Creates a new compressible config PDA.
///
/// # Arguments
/// * `config_account` - The config PDA account to initialize
/// * `update_authority` - Must be the program's upgrade authority
/// * `program_data_account` - The program's data account for validation
/// * `rent_sponsor` - Account that receives rent from compressed PDAs
/// * `compression_authority` - Authority that can compress/close PDAs
/// * `rent_config` - Rent function parameters
/// * `write_top_up` - Lamports to top up on each write
/// * `address_space` - Address spaces for compressed accounts (exactly 1
///   allowed)
/// * `config_bump` - Config bump seed (must be 0 for now)
/// * `payer` - Account paying for the PDA creation
/// * `system_program` - System program
/// * `program_id` - The program that owns the config
///
/// # Returns
/// * `Ok(())` if config was created successfully
/// * `Err(ProgramError)` if there was an error or authority validation fails
#[allow(clippy::too_many_arguments)]
pub fn process_initialize_light_config_checked<'info>(
    config_account: &AccountInfo<'info>,
    update_authority: &AccountInfo<'info>,
    program_data_account: &AccountInfo<'info>,
    rent_sponsor: &Pubkey,
    compression_authority: &Pubkey,
    rent_config: RentConfig,
    write_top_up: u32,
    address_space: Vec<Pubkey>,
    config_bump: u8,
    payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    program_id: &Pubkey,
) -> Result<(), crate::ProgramError> {
    msg!(
        "create_compression_config_checked program_data_account: {:?}",
        program_data_account.key
    );
    msg!(
        "create_compression_config_checked program_id: {:?}",
        program_id
    );
    // Verify the signer is the program's upgrade authority
    check_program_upgrade_authority(program_id, program_data_account, update_authority)?;

    // Create the config with validated authority
    process_initialize_light_config(
        config_account,
        update_authority,
        rent_sponsor,
        compression_authority,
        rent_config,
        write_top_up,
        address_space,
        config_bump,
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
