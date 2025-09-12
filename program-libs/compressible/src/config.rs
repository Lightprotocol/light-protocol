use bytemuck::{Pod, Zeroable};
use solana_pubkey::Pubkey;

use crate::{rent::RentConfig, AnchorDeserialize, AnchorSerialize};

pub const COMPRESSIBLE_CONFIG_SEED: &[u8] = b"compressible_config";
pub const MAX_ADDRESS_TREES_PER_SPACE: usize = 1;

// // TODO: add rent_authority + rent_func like in ctoken.
// /// Global configuration for compressible accounts
// #[derive(Clone, AnchorDeserialize, AnchorSerialize)]
// pub struct CompressibleConfig {
//     /// Config version for future upgrades
//     pub version: u8,
//     /// Number of slots to wait before compression is allowed
//     pub compression_delay: u32,
//     /// Authority that can update the config
//     pub update_authority: Pubkey,
//     /// Account that receives rent from compressed PDAs
//     pub rent_recipient: Pubkey,
//     /// Config bump seed (currently always 0)å
//     pub config_bump: u8,
//     /// PDA bump seed
//     pub bump: u8,
//     /// Address space for compressed accounts (currently 1 address_tree allowed)
//     pub address_space: Vec<Pubkey>,
// }
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

impl CompressibleConfig {
    pub const LEN: usize = std::mem::size_of::<Self>();
    pub const DISCRIMINATOR: [u8; 8] = [1u8; 8];

    /// Calculate the exact size needed for a CompressibleConfig with the given
    /// number of address spaces
    pub fn size_for_address_space(num_address_trees: usize) -> usize {
        1 + 4 + 32 + 32 + 1 + 4 + (32 * num_address_trees) + 1
    }

    /// Derives the config PDA address with config bump
    pub fn derive_pda(program_id: &Pubkey, config_bump: u8) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[COMPRESSIBLE_CONFIG_SEED, &[config_bump]],
            program_id.into(),
        )
        .into()
    }

    /// Derives the default config PDA address (config_bump = 0)
    pub fn derive_default_pda(program_id: &Pubkey) -> (Pubkey, u8) {
        Self::derive_pda(program_id, 0)
    }

    // /// Checks the config account
    // pub fn validate(&self) -> Result<(), ProgramError> {
    //     if self.version != 1 {
    //         msg!(
    //             "CompressibleConfig validation failed: Unsupported config version: {}",
    //             self.version
    //         );
    //         return Err(CompressibleError::ConstraintViolation.into());
    //     }
    //     if self.address_space.len() != 1 {
    //         msg!(
    //             "CompressibleConfig validation failed: Address space must contain exactly 1 pubkey, found: {}",
    //             self.address_space.len()
    //         );
    //         return Err(CompressibleError::ConstraintViolation.into());
    //     }
    //     // For now, only allow config_bump = 0 to keep it simple
    //     if self.config_bump != 0 {
    //         msg!(
    //             "CompressibleConfig validation failed: Config bump must be 0 for now, found: {}",
    //             self.config_bump
    //         );
    //         return Err(CompressibleError::ConstraintViolation.into());
    //     }
    //     Ok(())
    // }

    // /// Loads and validates config from account, checking owner and PDA derivation
    // pub fn load_checked(
    //     account: &AccountInfo,
    //     program_id: &Pubkey,
    // ) -> Result<Self, CompressibleError> {
    //     if account.owner != program_id {
    //         msg!(
    //             "CompressibleConfig::load_checked failed: Config account owner mismatch. Expected: {:?}. Found: {:?}.",
    //             program_id,
    //             account.owner
    //         );
    //         return Err(CompressibleError::ConstraintViolation.into());
    //     }
    //     let data = account.try_borrow_data()?;
    //     let config = Self::try_from_slice(&data).map_err(|err| {
    //         msg!(
    //             "CompressibleConfig::load_checked failed: Failed to deserialize config data: {:?}",
    //             err
    //         );
    //         CompressibleError::Borsh
    //     })?;
    //     config.validate()?;

    //     // CHECK: PDA derivation
    //     let (expected_pda, _) = Self::derive_pda(program_id, config.config_bump);
    //     if expected_pda != *account.key {
    //         msg!(
    //             "CompressibleConfig::load_checked failed: Config account key mismatch. Expected PDA: {:?}. Found: {:?}.",
    //             expected_pda,
    //             account.key
    //         );
    //         return Err(CompressibleError::ConstraintViolation.into());
    //     }

    //     Ok(config)
    // }
}
