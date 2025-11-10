use std::collections::HashSet;

use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_msg::msg;
use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_pubkey::Pubkey;
use solana_system_interface::instruction as system_instruction;
use solana_sysvar::{rent::Rent, Sysvar};

use crate::{error::LightSdkError, AnchorDeserialize, AnchorSerialize};

pub const COMPRESSIBLE_CONFIG_SEED: &[u8] = b"compressible_config";
pub const MAX_ADDRESS_TREES_PER_SPACE: usize = 1;
const BPF_LOADER_UPGRADEABLE_ID: Pubkey =
    Pubkey::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");

// TODO: add rent_authority + rent_func like in ctoken.
/// Global configuration for compressible accounts
#[derive(Clone, AnchorDeserialize, AnchorSerialize, Debug)]
pub struct CompressibleConfig {
    /// Config version for future upgrades
    pub version: u8,
    /// Number of slots to wait before compression is allowed
    pub compression_delay: u32,
    /// Authority that can update the config
    pub update_authority: Pubkey,
    /// Account that receives rent from compressed PDAs
    pub rent_recipient: Pubkey,
    /// Config bump seed (currently always 0)å
    pub config_bump: u8,
    /// PDA bump seed
    pub bump: u8,
    /// Address space for compressed accounts (currently 1 address_tree allowed)
    pub address_space: Vec<Pubkey>,
}

impl CompressibleConfig {
    pub const LEN: usize = 1 + 4 + 32 + 32 + 1 + 4 + (32 * MAX_ADDRESS_TREES_PER_SPACE) + 1; // 107 bytes max

    /// Calculate the exact size needed for a CompressibleConfig with the given
    /// number of address spaces
    pub fn size_for_address_space(num_address_trees: usize) -> usize {
        1 + 4 + 32 + 32 + 1 + 4 + (32 * num_address_trees) + 1
    }

    /// Derives the config PDA address with config bump
    pub fn derive_pda(program_id: &Pubkey, config_bump: u8) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[COMPRESSIBLE_CONFIG_SEED, &[config_bump]], program_id)
    }

    /// Derives the default config PDA address (config_bump = 0)
    pub fn derive_default_pda(program_id: &Pubkey) -> (Pubkey, u8) {
        Self::derive_pda(program_id, 0)
    }

    /// Checks the config account
    pub fn validate(&self) -> Result<(), crate::ProgramError> {
        if self.version != 1 {
            msg!(
                "CompressibleConfig validation failed: Unsupported config version: {}",
                self.version
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }
        if self.address_space.len() != 1 {
            msg!(
                "CompressibleConfig validation failed: Address space must contain exactly 1 pubkey, found: {}",
                self.address_space.len()
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }
        // For now, only allow config_bump = 0 to keep it simple
        if self.config_bump != 0 {
            msg!(
                "CompressibleConfig validation failed: Config bump must be 0 for now, found: {}",
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
                "CompressibleConfig::load_checked failed: Config account owner mismatch. Expected: {:?}. Found: {:?}.",
                program_id,
                account.owner
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }
        let data = account.try_borrow_data()?;
        let config = Self::try_from_slice(&data).map_err(|err| {
            msg!(
                "CompressibleConfig::load_checked failed: Failed to deserialize config data: {:?}",
                err
            );
            LightSdkError::Borsh
        })?;
        config.validate()?;

        // CHECK: PDA derivation
        let (expected_pda, _) = Self::derive_pda(program_id, config.config_bump);
        if expected_pda != *account.key {
            msg!(
                "CompressibleConfig::load_checked failed: Config account key mismatch. Expected PDA: {:?}. Found: {:?}.",
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
/// * `rent_recipient` - Account that receives rent from compressed PDAs
/// * `address_space` - Address space for compressed accounts (currently 1 address_tree allowed)
/// * `compression_delay` - Number of slots to wait before compression
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
pub fn process_initialize_compression_config_account_info<'info>(
    config_account: &AccountInfo<'info>,
    update_authority: &AccountInfo<'info>,
    rent_recipient: &Pubkey,
    address_space: Vec<Pubkey>,
    compression_delay: u32,
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
    let (derived_pda, bump) = CompressibleConfig::derive_pda(program_id, config_bump);
    if derived_pda != *config_account.key {
        msg!("Invalid config PDA");
        return Err(LightSdkError::ConstraintViolation.into());
    }

    let rent = Rent::get().map_err(LightSdkError::from)?;
    let account_size = CompressibleConfig::size_for_address_space(address_space.len());
    let rent_lamports = rent.minimum_balance(account_size);

    let seeds = &[COMPRESSIBLE_CONFIG_SEED, &[config_bump], &[bump]];
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

    let config = CompressibleConfig {
        version: 1,
        compression_delay,
        update_authority: *update_authority.key,
        rent_recipient: *rent_recipient,
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
/// * `new_rent_recipient` - Optional new rent recipient
/// * `new_address_space` - Optional new address space (currently 1 address_tree allowed)
/// * `new_compression_delay` - Optional new compression delay
/// * `owner_program_id` - The program that owns the config
///
/// # Returns
/// * `Ok(())` if config was updated successfully
/// * `Err(ProgramError)` if there was an error
pub fn process_update_compression_config<'info>(
    config_account: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    new_update_authority: Option<&Pubkey>,
    new_rent_recipient: Option<&Pubkey>,
    new_address_space: Option<Vec<Pubkey>>,
    new_compression_delay: Option<u32>,
    owner_program_id: &Pubkey,
) -> Result<(), crate::ProgramError> {
    // CHECK: PDA derivation
    let mut config = CompressibleConfig::load_checked(config_account, owner_program_id)?;

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
    if let Some(new_recipient) = new_rent_recipient {
        config.rent_recipient = *new_recipient;
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
    if let Some(new_delay) = new_compression_delay {
        config.compression_delay = new_delay;
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
/// * `rent_recipient` - Account that receives rent from compressed PDAs
/// * `address_space` - Address spaces for compressed accounts (exactly 1
///   allowed)
/// * `compression_delay` - Number of slots to wait before compression
/// * `config_bump` - Config bump seed (must be 0 for now)
/// * `payer` - Account paying for the PDA creation
/// * `system_program` - System program
/// * `program_id` - The program that owns the config
///
/// # Returns
/// * `Ok(())` if config was created successfully
/// * `Err(ProgramError)` if there was an error or authority validation fails
#[allow(clippy::too_many_arguments)]
pub fn process_initialize_compression_config_checked<'info>(
    config_account: &AccountInfo<'info>,
    update_authority: &AccountInfo<'info>,
    program_data_account: &AccountInfo<'info>,
    rent_recipient: &Pubkey,
    address_space: Vec<Pubkey>,
    compression_delay: u32,
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
    process_initialize_compression_config_account_info(
        config_account,
        update_authority,
        rent_recipient,
        address_space,
        compression_delay,
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
