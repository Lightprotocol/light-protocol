//! LightConfig state struct and methods.

use light_account_checks::{
    checks::check_discriminator,
    discriminator::{Discriminator, DISCRIMINATOR_LEN},
};
use light_compressible::rent::RentConfig;
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_pubkey::Pubkey;

use super::{COMPRESSIBLE_CONFIG_SEED, MAX_ADDRESS_TREES_PER_SPACE, RENT_SPONSOR_SEED};
use crate::{error::LightSdkError, AnchorDeserialize, AnchorSerialize};

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
    /// Config PDA bump seed
    pub bump: u8,
    /// Rent sponsor PDA bump seed
    pub rent_sponsor_bump: u8,
    /// Address space for compressed accounts (currently 1 address_tree allowed)
    pub address_space: Vec<Pubkey>,
}

/// Implement the Light Discriminator trait for LightConfig
impl Discriminator for LightConfig {
    const LIGHT_DISCRIMINATOR: [u8; 8] = *b"LightCfg";
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}

impl LightConfig {
    /// Total account size including discriminator
    pub const LEN: usize = DISCRIMINATOR_LEN
        + 1
        + 4
        + 32
        + 32
        + 32
        + core::mem::size_of::<RentConfig>()
        + 1
        + 1
        + 1
        + 4
        + (32 * MAX_ADDRESS_TREES_PER_SPACE);

    /// Calculate the exact size needed for a LightConfig with the given
    /// number of address spaces (includes discriminator)
    pub fn size_for_address_space(num_address_trees: usize) -> usize {
        DISCRIMINATOR_LEN
            + 1
            + 4
            + 32
            + 32
            + 32
            + core::mem::size_of::<RentConfig>()
            + 1
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

    /// Validates rent_sponsor matches config and returns stored bump for signing.
    pub fn validate_rent_sponsor(
        &self,
        rent_sponsor: &AccountInfo,
    ) -> Result<u8, crate::ProgramError> {
        if *rent_sponsor.key != self.rent_sponsor {
            msg!(
                "rent_sponsor mismatch: expected {:?}, got {:?}",
                self.rent_sponsor,
                rent_sponsor.key
            );
            return Err(LightSdkError::InvalidRentSponsor.into());
        }
        Ok(self.rent_sponsor_bump)
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

    /// Loads and validates config from account, checking owner, discriminator, and PDA derivation
    #[inline(never)]
    pub fn load_checked(
        account: &AccountInfo,
        program_id: &Pubkey,
    ) -> Result<Self, crate::ProgramError> {
        // CHECK: Owner
        if account.owner != program_id {
            msg!(
                "LightConfig::load_checked failed: Config account owner mismatch. Expected: {:?}. Found: {:?}.",
                program_id,
                account.owner
            );
            return Err(LightSdkError::ConstraintViolation.into());
        }

        let data = account.try_borrow_data()?;

        // CHECK: Discriminator using light-account-checks
        check_discriminator::<Self>(&data).map_err(|e| {
            msg!("LightConfig::load_checked failed: {:?}", e);
            LightSdkError::ConstraintViolation
        })?;

        // Deserialize from offset after discriminator
        let config = Self::try_from_slice(&data[DISCRIMINATOR_LEN..]).map_err(|err| {
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
