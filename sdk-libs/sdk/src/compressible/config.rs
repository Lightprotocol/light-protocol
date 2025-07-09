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
/// # Arguments
/// * `config_account` - The config PDA account to initialize
/// * `update_authority` - Authority that can update the config
/// * `rent_recipient` - Account that receives rent from compressed PDAs
/// * `address_space` - Address space for compressed accounts
/// * `compression_delay` - Number of slots to wait before compression
/// * `payer` - Account paying for the PDA creation
/// * `program_id` - The program that owns the config
///
/// # Returns
/// * `Ok(())` if config was created successfully
/// * `Err(LightSdkError)` if there was an error
pub fn create_config<'info>(
    config_account: &AccountInfo<'info>,
    update_authority: &Pubkey,
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
        update_authority: *update_authority,
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
