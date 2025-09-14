use bytemuck::{Pod, Zeroable};
use light_account_checks::discriminator::Discriminator;
use solana_pubkey::{pubkey, Pubkey};

use crate::{rent::RentConfig, AnchorDeserialize, AnchorSerialize};

pub const COMPRESSIBLE_CONFIG_SEED: &[u8] = b"compressible_config";

#[derive(Clone, Debug, AnchorDeserialize, PartialEq, AnchorSerialize, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct CompressibleConfig {
    /// Config version for future upgrades
    pub version: u16,
    pub active: u8,
    /// PDA bump seed
    pub bump: u8,
    pub update_authority: Pubkey,
    pub withdrawal_authority: Pubkey,
    // rent_recipient, rent_authority have fixed derivation
    pub rent_recipient: Pubkey,
    pub rent_authority: Pubkey,
    pub rent_recipient_bump: u8,
    pub rent_authority_bump: u8,
    pub rent_config: RentConfig,

    /// Address space for compressed accounts (currently 1 address_tree allowed)
    pub address_space: [Pubkey; 4],
    pub _place_holder: [u8; 32],
}

#[cfg(feature = "anchor")]
impl anchor_lang::Discriminator for CompressibleConfig {
    const DISCRIMINATOR: &'static [u8] = &[1, 2, 3, 4, 5, 6, 7, 8];
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
    const LIGHT_DISCRIMINATOR: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
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
        address_space[0] = pubkey!("EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK");
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

    pub fn get_rent_authority_seeds(version: u16) -> [Vec<u8>; 2] {
        [b"rent_authority".to_vec(), version.to_le_bytes().to_vec()]
    }

    pub fn get_rent_recipient_seeds(version: u16) -> [Vec<u8>; 2] {
        [b"rent_recipient".to_vec(), version.to_le_bytes().to_vec()]
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        version: u16,
        active: bool,
        update_authority: Pubkey,
        withdrawal_authority: Pubkey,
        rent_recipient_program_id: &Pubkey,
        owner_program_id: &Pubkey,
        address_space: [Pubkey; 4],
        rent_config: RentConfig,
    ) -> Self {
        let version_bytes = version.to_le_bytes();
        let rent_authority_seeds = [b"rent_authority".as_slice(), version_bytes.as_slice()];
        let rent_recipient_seeds = [b"rent_recipient".as_slice(), version_bytes.as_slice()];
        let (rent_authority, rent_authority_bump) = solana_pubkey::Pubkey::find_program_address(
            rent_authority_seeds.as_slice(),
            owner_program_id,
        );
        let (rent_recipient, rent_recipient_bump) = solana_pubkey::Pubkey::find_program_address(
            rent_recipient_seeds.as_slice(),
            rent_recipient_program_id,
        );
        let (_, bump) = Self::derive_pda(owner_program_id, version);

        Self {
            version,
            active: active as u8,
            bump,
            update_authority,
            withdrawal_authority,
            rent_recipient,
            rent_authority,
            rent_recipient_bump,
            rent_authority_bump,
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
