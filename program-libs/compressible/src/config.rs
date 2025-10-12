use bytemuck::{Pod, Zeroable};
use light_account_checks::discriminator::Discriminator;
use solana_pubkey::{pubkey, Pubkey};

use crate::{error::CompressibleError, rent::RentConfig, AnchorDeserialize, AnchorSerialize};

pub const COMPRESSIBLE_CONFIG_SEED: &[u8] = b"compressible_config";

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum CompressibleConfigState {
    Inactive,
    Active,
    Deprecated,
}

impl TryFrom<u8> for CompressibleConfigState {
    type Error = CompressibleError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CompressibleConfigState::Inactive),
            1 => Ok(CompressibleConfigState::Active),
            2 => Ok(CompressibleConfigState::Deprecated),
            _ => Err(CompressibleError::InvalidState(value)),
        }
    }
}

#[derive(Clone, Debug, AnchorDeserialize, PartialEq, AnchorSerialize, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct CompressibleConfig {
    /// Config version for future upgrades
    pub version: u16,
    /// 1 Compressible Config pda is active, 0 is inactive, 2 is deprecated.
    /// - inactive, config cannot be used
    /// - active, config can be used
    /// - deprecated, no new ctoken account can be created with this config, other instructions work.
    pub state: u8,
    /// CompressibleConfig PDA bump seed
    pub bump: u8,
    /// Update authority can update the CompressibleConfig.
    pub update_authority: Pubkey,
    /// Withdrawal authority can withdraw funds from the rent recipient pda.
    pub withdrawal_authority: Pubkey,
    /// CToken program pda:
    /// 1. pays rent exemption at compressible ctoken account creation
    /// 2. receives rent exemption at compressible ctoken account closure
    /// 3. receives rent from compressible ctoken accounts with Claim, or compress and close instructions.
    pub rent_sponsor: Pubkey,
    /// Registry program pda, can Claim from and compress and close compressible ctoken accounts.
    pub compression_authority: Pubkey,
    pub rent_sponsor_bump: u8,
    pub compression_authority_bump: u8,
    /// Rent function parameters,
    /// used to calculate whether the account is compressible.
    pub rent_config: RentConfig,

    /// Address space for compressed accounts (currently 1 address_tree allowed)
    pub address_space: [Pubkey; 4],
    pub _place_holder: [u8; 32],
}

impl CompressibleConfig {
    /// Validates that the config is active (can be used for all operations)
    pub fn validate_active(&self) -> Result<(), CompressibleError> {
        let state = CompressibleConfigState::try_from(self.state)?;
        if state != CompressibleConfigState::Active {
            return Err(CompressibleError::InvalidState(self.state));
        }
        Ok(())
    }

    /// Validates that the config is not inactive (can be used for new account creation)
    pub fn validate_not_inactive(&self) -> Result<(), CompressibleError> {
        let state = CompressibleConfigState::try_from(self.state)?;
        if state == CompressibleConfigState::Inactive {
            return Err(CompressibleError::InvalidState(self.state));
        }
        Ok(())
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::Discriminator for CompressibleConfig {
    const DISCRIMINATOR: &'static [u8] = &[180, 4, 231, 26, 220, 144, 55, 168];
}

#[cfg(feature = "anchor")]
impl anchor_lang::AccountDeserialize for CompressibleConfig {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        // Skip the discriminator (first 8 bytes) and deserialize the rest
        let mut data: &[u8] = &buf[8..];
        Self::deserialize(&mut data)
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
    }

    fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        use anchor_lang::Discriminator;

        // Check discriminator first
        if buf.len() < 8 {
            return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into());
        }

        let given_disc = &buf[..8];
        if given_disc != Self::DISCRIMINATOR {
            return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into());
        }

        Self::try_deserialize_unchecked(buf)
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::AccountSerialize for CompressibleConfig {
    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
        use anchor_lang::Discriminator;

        // Write discriminator first
        if writer.write_all(Self::DISCRIMINATOR).is_err() {
            return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
        }

        // Then serialize the actual account data
        if self.serialize(writer).is_err() {
            return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
        }
        Ok(())
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::Owner for CompressibleConfig {
    fn owner() -> anchor_lang::prelude::Pubkey {
        pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX")
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::Space for CompressibleConfig {
    const INIT_SPACE: usize = 8 + std::mem::size_of::<Self>(); // 8 bytes for discriminator + struct size
}

impl Discriminator for CompressibleConfig {
    const LIGHT_DISCRIMINATOR: [u8; 8] = [180, 4, 231, 26, 220, 144, 55, 168];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = Self::LIGHT_DISCRIMINATOR.as_slice();
}

impl CompressibleConfig {
    pub const LEN: usize = std::mem::size_of::<Self>();

    pub fn ctoken_v1(update_authority: Pubkey, withdrawal_authority: Pubkey) -> Self {
        Self::new_ctoken(
            1,
            true,
            update_authority,
            withdrawal_authority,
            RentConfig::default(),
        )
    }

    pub fn new_ctoken(
        version: u16,
        active: bool,
        update_authority: Pubkey,
        withdrawal_authority: Pubkey,
        rent_config: RentConfig,
    ) -> Self {
        let mut address_space = [Pubkey::default(); 4];
        address_space[0] = pubkey!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx");
        Self::new(
            version,
            active,
            update_authority,
            withdrawal_authority,
            &pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"),
            &pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX"),
            address_space,
            rent_config,
        )
    }

    pub fn get_compression_authority_seeds(version: u16) -> [Vec<u8>; 2] {
        [
            b"compression_authority".to_vec(),
            version.to_le_bytes().to_vec(),
        ]
    }

    pub fn get_rent_sponsor_seeds(version: u16) -> [Vec<u8>; 2] {
        [b"rent_sponsor".to_vec(), version.to_le_bytes().to_vec()]
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        version: u16,
        active: bool,
        update_authority: Pubkey,
        withdrawal_authority: Pubkey,
        rent_sponsor_program_id: &Pubkey,
        owner_program_id: &Pubkey,
        address_space: [Pubkey; 4],
        rent_config: RentConfig,
    ) -> Self {
        let version_bytes = version.to_le_bytes();
        let compression_authority_seeds = [
            b"compression_authority".as_slice(),
            version_bytes.as_slice(),
        ];
        let rent_sponsor_seeds = [b"rent_sponsor".as_slice(), version_bytes.as_slice()];
        let (compression_authority, compression_authority_bump) =
            solana_pubkey::Pubkey::find_program_address(
                compression_authority_seeds.as_slice(),
                owner_program_id,
            );
        let (rent_sponsor, rent_sponsor_bump) = solana_pubkey::Pubkey::find_program_address(
            rent_sponsor_seeds.as_slice(),
            rent_sponsor_program_id,
        );
        let (_, bump) = Self::derive_pda(owner_program_id, version);

        Self {
            version,
            state: active as u8,
            bump,
            update_authority,
            withdrawal_authority,
            rent_sponsor,
            compression_authority,
            rent_sponsor_bump,
            compression_authority_bump,
            rent_config,
            address_space,
            _place_holder: [0u8; 32],
        }
    }

    /// Derives the config PDA address with config bump
    pub fn derive_pda(program_id: &Pubkey, config_bump: u16) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[COMPRESSIBLE_CONFIG_SEED, &config_bump.to_le_bytes()],
            program_id,
        )
    }

    /// Derives the default config PDA address (config_bump = 1)
    pub fn ctoken_v1_config_pda() -> Pubkey {
        Self::derive_pda(&pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX"), 1).0
    }

    /// Derives the default config PDA address (config_bump = 1)
    pub fn derive_v1_config_pda(program_id: &Pubkey) -> (Pubkey, u8) {
        Self::derive_pda(program_id, 1)
    }

    /// Derives the default config PDA address (config_bump = 0)
    pub fn derive_default_pda(program_id: &Pubkey) -> (Pubkey, u8) {
        Self::derive_pda(program_id, 0)
    }
}
