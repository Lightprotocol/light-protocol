use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize as BorshDeserialize, AnchorSerialize as BorshSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize, BorshSerialize};

use super::config::CompressibleConfig;

/// Instruction builders for compressible accounts, following Solana SDK patterns
/// These are generic builders that work with any program implementing the compressible pattern
pub struct CompressibleInstruction;

impl CompressibleInstruction {
    pub const INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR: [u8; 8] =
        [133, 228, 12, 169, 56, 76, 222, 61];

    pub const UPDATE_COMPRESSION_CONFIG_DISCRIMINATOR: [u8; 8] =
        [135, 215, 243, 81, 163, 146, 33, 70];

    /// Creates an initialize_compression_config instruction
    ///
    /// Following Solana SDK patterns like system_instruction::transfer()
    /// Returns Instruction directly - errors surface at execution time
    pub fn initialize_compression_config(
        program_id: &Pubkey,
        payer: &Pubkey,
        authority: &Pubkey,
        compression_delay: u32,
        rent_recipient: Pubkey,
        address_space: Vec<Pubkey>,
    ) -> Instruction {
        let (config_pda, _) = CompressibleConfig::derive_pda(program_id);

        // Get program data account for BPF Loader Upgradeable
        let bpf_loader_upgradeable_id =
            solana_pubkey::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");
        let (program_data_pda, _) =
            Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable_id);

        let system_program_id = solana_pubkey::pubkey!("11111111111111111111111111111111");
        let accounts = vec![
            AccountMeta::new(*payer, true),                      // payer
            AccountMeta::new(config_pda, false),                 // config
            AccountMeta::new_readonly(program_data_pda, false),  // program_data
            AccountMeta::new_readonly(*authority, true),         // authority
            AccountMeta::new_readonly(system_program_id, false), // system_program
        ];

        let instruction_data = InitializeCompressionConfigData {
            compression_delay,
            rent_recipient,
            address_space,
        };

        // Prepend discriminator to serialized data, following Solana SDK pattern
        let mut data = Vec::with_capacity(8 + instruction_data.try_to_vec().unwrap().len());
        data.extend_from_slice(&Self::INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR);
        data.extend_from_slice(
            &instruction_data
                .try_to_vec()
                .expect("Failed to serialize instruction data"),
        );

        Instruction {
            program_id: *program_id,
            accounts,
            data,
        }
    }

    /// Creates an update config instruction
    ///
    /// Following Solana SDK patterns - returns Instruction directly
    pub fn update_compression_config(
        program_id: &Pubkey,
        authority: &Pubkey,
        new_compression_delay: Option<u32>,
        new_rent_recipient: Option<Pubkey>,
        new_address_space: Option<Vec<Pubkey>>,
        new_update_authority: Option<Pubkey>,
    ) -> Instruction {
        let (config_pda, _) = CompressibleConfig::derive_pda(program_id);

        let accounts = vec![
            AccountMeta::new(config_pda, false),         // config
            AccountMeta::new_readonly(*authority, true), // authority
        ];

        let instruction_data = UpdateConfigData {
            new_compression_delay,
            new_rent_recipient,
            new_address_space,
            new_update_authority,
        };

        // Prepend discriminator to serialized data, following Solana SDK pattern
        let mut data = Vec::with_capacity(8 + instruction_data.try_to_vec().unwrap().len());
        data.extend_from_slice(&Self::UPDATE_COMPRESSION_CONFIG_DISCRIMINATOR);
        data.extend_from_slice(
            &instruction_data
                .try_to_vec()
                .expect("Failed to serialize instruction data"),
        );

        Instruction {
            program_id: *program_id,
            accounts,
            data,
        }
    }
}

/// Generic instruction data for initialize config
/// Note: Real programs should use their specific instruction format
#[derive(BorshSerialize, BorshDeserialize)]
struct InitializeCompressionConfigData {
    compression_delay: u32,
    rent_recipient: Pubkey,
    address_space: Vec<Pubkey>,
}

/// Generic instruction data for update config
/// Note: Real programs should use their specific instruction format  
#[derive(BorshSerialize, BorshDeserialize)]
struct UpdateConfigData {
    new_compression_delay: Option<u32>,
    new_rent_recipient: Option<Pubkey>,
    new_address_space: Option<Vec<Pubkey>>,
    new_update_authority: Option<Pubkey>,
}

// Re-export for easy access following Solana SDK patterns
pub use CompressibleInstruction as compressible_instruction;
