use bytemuck::{Pod, Zeroable};
use light_account_checks::discriminator::Discriminator;
use solana_pubkey::Pubkey;

use crate::{rent::RentConfig, AnchorDeserialize, AnchorSerialize};

pub const COMPRESSIBLE_CONFIG_SEED: &[u8] = b"compressible_config";
pub const MAX_ADDRESS_TREES_PER_SPACE: usize = 1;

#[derive(Clone, AnchorDeserialize, AnchorSerialize, Copy, Pod, Zeroable)]
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
impl anchor_lang::AccountDeserialize for CompressibleConfig {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        // Use the AnchorDeserialize implementation
        Self::deserialize(buf)
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::AccountSerialize for CompressibleConfig {
    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
        // Use the AnchorSerialize implementation
        self.serialize(writer)
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotSerialize.into())
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::Owner for CompressibleConfig {
    fn owner() -> anchor_lang::prelude::Pubkey {
        // This should return the program ID that owns this account type
        // For now, return a default - this should be set to your actual program ID
        anchor_lang::prelude::Pubkey::default()
    }
}

impl Discriminator for CompressibleConfig {
    const LIGHT_DISCRIMINATOR: [u8; 8] = [1u8; 8];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = Self::LIGHT_DISCRIMINATOR.as_slice();
}

impl CompressibleConfig {
    pub const LEN: usize = std::mem::size_of::<Self>();
    pub const DISCRIMINATOR: [u8; 8] = [1u8; 8];

    /// Derives the config PDA address with config bump
    pub fn derive_pda(program_id: &Pubkey, config_bump: u8) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[COMPRESSIBLE_CONFIG_SEED, &[config_bump]], program_id)
    }

    /// Derives the default config PDA address (config_bump = 0)
    pub fn derive_default_pda(program_id: &Pubkey) -> (Pubkey, u8) {
        Self::derive_pda(program_id, 0)
    }
}
