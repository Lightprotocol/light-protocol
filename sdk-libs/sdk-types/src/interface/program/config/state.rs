//! LightConfig state struct and methods.

use light_account_checks::{
    checks::check_discriminator,
    discriminator::{Discriminator, DISCRIMINATOR_LEN},
    AccountInfoTrait,
};
use light_compressible::rent::RentConfig;

use super::{COMPRESSIBLE_CONFIG_SEED, MAX_ADDRESS_TREES_PER_SPACE};
use crate::{error::LightSdkTypesError, AnchorDeserialize, AnchorSerialize};

/// Global configuration for compressible accounts
#[derive(Clone, AnchorDeserialize, AnchorSerialize, Debug)]
pub struct LightConfig {
    /// Config version for future upgrades
    pub version: u8,
    /// Lamports to top up on each write (heuristic)
    pub write_top_up: u32,
    /// Authority that can update the config
    pub update_authority: [u8; 32],
    /// Account that receives rent from compressed PDAs
    pub rent_sponsor: [u8; 32],
    /// Authority that can compress/close PDAs (distinct from rent_sponsor)
    pub compression_authority: [u8; 32],
    /// Rent function parameters for compressibility and distribution
    pub rent_config: RentConfig,
    /// Config bump seed (0)
    pub config_bump: u8,
    /// Config PDA bump seed
    pub bump: u8,
    /// Rent sponsor PDA bump seed
    pub rent_sponsor_bump: u8,
    /// Address space for compressed accounts (currently 1 address_tree allowed)
    pub address_space: Vec<[u8; 32]>,
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

    /// Derives the config PDA address (returns raw bytes).
    /// Generic over AccountInfoTrait for framework-agnostic PDA derivation.
    pub fn derive_pda_bytes<AI: AccountInfoTrait>(
        program_id: &[u8; 32],
        config_bump: u8,
    ) -> ([u8; 32], u8) {
        let config_bump_u16 = config_bump as u16;
        AI::find_program_address(
            &[COMPRESSIBLE_CONFIG_SEED, &config_bump_u16.to_le_bytes()],
            program_id,
        )
    }

    /// Derives the rent sponsor PDA address (returns raw bytes).
    pub fn derive_rent_sponsor_pda_bytes<AI: AccountInfoTrait>(
        program_id: &[u8; 32],
    ) -> ([u8; 32], u8) {
        AI::find_program_address(&[super::RENT_SPONSOR_SEED], program_id)
    }

    /// Validates rent_sponsor matches config and returns stored bump for signing.
    pub fn validate_rent_sponsor_account<AI: AccountInfoTrait>(
        &self,
        rent_sponsor: &AI,
    ) -> Result<u8, LightSdkTypesError> {
        if rent_sponsor.key() != self.rent_sponsor {
            return Err(LightSdkTypesError::InvalidRentSponsor);
        }
        Ok(self.rent_sponsor_bump)
    }

    /// Checks the config account
    pub fn validate(&self) -> Result<(), LightSdkTypesError> {
        if self.version != 1 {
            return Err(LightSdkTypesError::ConstraintViolation);
        }
        if self.address_space.len() != 1 {
            return Err(LightSdkTypesError::ConstraintViolation);
        }
        // For now, only allow config_bump = 0 to keep it simple
        if self.config_bump != 0 {
            return Err(LightSdkTypesError::ConstraintViolation);
        }
        Ok(())
    }

    /// Loads and validates config from account, checking owner, discriminator, and PDA derivation.
    /// Generic over AccountInfoTrait - works with both solana and pinocchio.
    #[inline(never)]
    pub fn load_checked<AI: AccountInfoTrait>(
        account: &AI,
        program_id: &[u8; 32],
    ) -> Result<Self, LightSdkTypesError> {
        // CHECK: Owner
        if !account.is_owned_by(program_id) {
            return Err(LightSdkTypesError::ConstraintViolation);
        }

        let data = account
            .try_borrow_data()
            .map_err(|_| LightSdkTypesError::ConstraintViolation)?;

        // CHECK: Discriminator using light-account-checks
        check_discriminator::<Self>(&data).map_err(|_| LightSdkTypesError::ConstraintViolation)?;

        // Deserialize from offset after discriminator
        let config = Self::try_from_slice(&data[DISCRIMINATOR_LEN..])
            .map_err(|_| LightSdkTypesError::Borsh)?;
        config.validate()?;

        // CHECK: PDA derivation
        let (expected_pda, _) = AI::find_program_address(
            &[
                COMPRESSIBLE_CONFIG_SEED,
                &(config.config_bump as u16).to_le_bytes(),
            ],
            program_id,
        );
        if expected_pda != account.key() {
            return Err(LightSdkTypesError::ConstraintViolation);
        }

        Ok(config)
    }
}
