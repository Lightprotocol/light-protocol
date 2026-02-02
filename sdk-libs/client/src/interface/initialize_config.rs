//! Helper for initializing config with sensible defaults.

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

/// Default address tree v2 pubkey.
pub const ADDRESS_TREE_V2: Pubkey =
    solana_pubkey::pubkey!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx");

/// Default write top-up value (5000 lamports).
pub const DEFAULT_INIT_WRITE_TOP_UP: u32 = 5_000;

/// Instruction data format matching anchor-generated `initialize_compression_config`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeCompressionConfigAnchorData {
    pub write_top_up: u32,
    pub rent_sponsor: Pubkey,
    pub compression_authority: Pubkey,
    pub rent_config: light_compressible::rent::RentConfig,
    pub address_space: Vec<Pubkey>,
}

/// Builder for `initialize_compression_config` instruction with sensible defaults.
pub struct InitializeRentFreeConfig {
    program_id: Pubkey,
    fee_payer: Pubkey,
    program_data_pda: Pubkey,
    authority: Option<Pubkey>,
    rent_sponsor: Pubkey,
    compression_authority: Pubkey,
    rent_config: light_compressible::rent::RentConfig,
    write_top_up: u32,
    address_space: Vec<Pubkey>,
    config_bump: u8,
}

impl InitializeRentFreeConfig {
    pub fn new(
        program_id: &Pubkey,
        fee_payer: &Pubkey,
        program_data_pda: &Pubkey,
        rent_sponsor: Pubkey,
        compression_authority: Pubkey,
    ) -> Self {
        Self {
            program_id: *program_id,
            fee_payer: *fee_payer,
            program_data_pda: *program_data_pda,
            authority: None,
            rent_sponsor,
            compression_authority,
            rent_config: light_compressible::rent::RentConfig::default(),
            write_top_up: DEFAULT_INIT_WRITE_TOP_UP,
            address_space: vec![ADDRESS_TREE_V2],
            config_bump: 0,
        }
    }

    pub fn authority(mut self, authority: Pubkey) -> Self {
        self.authority = Some(authority);
        self
    }

    pub fn rent_config(mut self, rent_config: light_compressible::rent::RentConfig) -> Self {
        self.rent_config = rent_config;
        self
    }

    pub fn write_top_up(mut self, write_top_up: u32) -> Self {
        self.write_top_up = write_top_up;
        self
    }

    pub fn address_space(mut self, address_space: Vec<Pubkey>) -> Self {
        self.address_space = address_space;
        self
    }

    pub fn config_bump(mut self, config_bump: u8) -> Self {
        self.config_bump = config_bump;
        self
    }

    pub fn build(self) -> (Instruction, Pubkey) {
        let authority = self.authority.unwrap_or(self.fee_payer);
        let config_bump_u16 = self.config_bump as u16;
        let (config_pda, _) = Pubkey::find_program_address(
            &[
                light_sdk::COMPRESSIBLE_CONFIG_SEED,
                &config_bump_u16.to_le_bytes(),
            ],
            &self.program_id,
        );

        let accounts = vec![
            AccountMeta::new(self.fee_payer, true), // payer
            AccountMeta::new(config_pda, false),    // config
            AccountMeta::new_readonly(self.program_data_pda, false), // program_data
            AccountMeta::new_readonly(authority, true), // authority
            AccountMeta::new_readonly(
                solana_pubkey::pubkey!("11111111111111111111111111111111"),
                false,
            ), // system_program
        ];

        let instruction_data = InitializeCompressionConfigAnchorData {
            write_top_up: self.write_top_up,
            rent_sponsor: self.rent_sponsor,
            compression_authority: self.compression_authority,
            rent_config: self.rent_config,
            address_space: self.address_space,
        };

        // Anchor discriminator for "initialize_compression_config"
        // SHA256("global:initialize_compression_config")[..8]
        const DISCRIMINATOR: [u8; 8] = [133, 228, 12, 169, 56, 76, 222, 61];

        let serialized_data = instruction_data
            .try_to_vec()
            .expect("Failed to serialize instruction data");

        let mut data = Vec::with_capacity(DISCRIMINATOR.len() + serialized_data.len());
        data.extend_from_slice(&DISCRIMINATOR);
        data.extend_from_slice(&serialized_data);

        let instruction = Instruction {
            program_id: self.program_id,
            accounts,
            data,
        };

        (instruction, config_pda)
    }
}
