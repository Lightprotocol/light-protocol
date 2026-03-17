//! Create Light Token associated token account instruction builder.
//!
//! Ported from `sdk-libs/token-sdk/src/instruction/create_ata.rs`.

use borsh::BorshSerialize;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{
    error::KoraLightError,
    pda::get_associated_token_address,
    program_ids::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_PROGRAM_ID, RENT_SPONSOR_V1, SYSTEM_PROGRAM_ID},
    types::{CompressibleExtensionInstructionData, CreateAssociatedTokenAccountInstructionData},
};

const CREATE_ATA_DISCRIMINATOR: u8 = 100;
const CREATE_ATA_IDEMPOTENT_DISCRIMINATOR: u8 = 102;

/// Default pre-pay epochs for rent
const DEFAULT_PREPAY_EPOCHS: u8 = 16;
/// Default write top-up in lamports (covers ~3 hours of rent)
const DEFAULT_WRITE_TOP_UP: u32 = 766;

/// Builder for CreateAssociatedTokenAccount instructions.
#[derive(Debug, Clone)]
pub struct CreateAta {
    pub payer: Pubkey,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub idempotent: bool,
    /// Compressible config PDA (default: LIGHT_TOKEN_CONFIG)
    pub compressible_config: Pubkey,
    /// Rent sponsor PDA (default: RENT_SPONSOR_V1)
    pub rent_sponsor: Pubkey,
    /// Pre-pay rent epochs (default: 16)
    pub pre_pay_num_epochs: u8,
    /// Write top-up in lamports (default: 766)
    pub write_top_up: u32,
    /// Compression-only flag (default: true for ATAs)
    pub compression_only: bool,
}

impl CreateAta {
    /// Create a new CreateAta builder with default rent-free settings.
    pub fn new(payer: Pubkey, owner: Pubkey, mint: Pubkey) -> Self {
        Self {
            payer,
            owner,
            mint,
            idempotent: false,
            compressible_config: LIGHT_TOKEN_CONFIG,
            rent_sponsor: RENT_SPONSOR_V1,
            pre_pay_num_epochs: DEFAULT_PREPAY_EPOCHS,
            write_top_up: DEFAULT_WRITE_TOP_UP,
            compression_only: true,
        }
    }

    /// Make this an idempotent create (no-op if ATA already exists).
    pub fn idempotent(mut self) -> Self {
        self.idempotent = true;
        self
    }

    /// Build the instruction.
    pub fn instruction(&self) -> Result<Instruction, KoraLightError> {
        let ata = get_associated_token_address(&self.owner, &self.mint);

        let instruction_data = CreateAssociatedTokenAccountInstructionData {
            compressible_config: Some(CompressibleExtensionInstructionData {
                token_account_version: 0, // ShaFlat
                rent_payment: self.pre_pay_num_epochs,
                compression_only: if self.compression_only { 1 } else { 0 },
                write_top_up: self.write_top_up,
                compress_to_account_pubkey: None,
            }),
        };

        let discriminator = if self.idempotent {
            CREATE_ATA_IDEMPOTENT_DISCRIMINATOR
        } else {
            CREATE_ATA_DISCRIMINATOR
        };

        let mut data = Vec::new();
        data.push(discriminator);
        instruction_data.serialize(&mut data)?;

        let accounts = vec![
            AccountMeta::new_readonly(self.owner, false),
            AccountMeta::new_readonly(self.mint, false),
            AccountMeta::new(self.payer, true),
            AccountMeta::new(ata, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(self.compressible_config, false),
            AccountMeta::new(self.rent_sponsor, false),
        ];

        Ok(Instruction {
            program_id: LIGHT_TOKEN_PROGRAM_ID,
            accounts,
            data,
        })
    }
}

/// Convenience function: build an idempotent CreateAta instruction with defaults.
pub fn create_ata_idempotent_instruction(
    payer: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<Instruction, KoraLightError> {
    CreateAta::new(*payer, *owner, *mint)
        .idempotent()
        .instruction()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ata_instruction_builds() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ix = CreateAta::new(payer, owner, mint).instruction().unwrap();

        assert_eq!(ix.program_id, LIGHT_TOKEN_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 7);
        // First byte is discriminator
        assert_eq!(ix.data[0], CREATE_ATA_DISCRIMINATOR);

        // Account order: owner, mint, payer, ata, system, config, sponsor
        assert_eq!(ix.accounts[0].pubkey, owner);
        assert!(!ix.accounts[0].is_signer);
        assert_eq!(ix.accounts[1].pubkey, mint);
        assert_eq!(ix.accounts[2].pubkey, payer);
        assert!(ix.accounts[2].is_signer);
        assert_eq!(ix.accounts[4].pubkey, SYSTEM_PROGRAM_ID);
        assert_eq!(ix.accounts[5].pubkey, LIGHT_TOKEN_CONFIG);
        assert_eq!(ix.accounts[6].pubkey, RENT_SPONSOR_V1);
    }

    #[test]
    fn test_create_ata_idempotent() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ix = CreateAta::new(payer, owner, mint)
            .idempotent()
            .instruction()
            .unwrap();

        assert_eq!(ix.data[0], CREATE_ATA_IDEMPOTENT_DISCRIMINATOR);
    }

    #[test]
    fn test_create_ata_ata_address_matches_pda() {
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ix = CreateAta::new(Pubkey::new_unique(), owner, mint)
            .instruction()
            .unwrap();

        let expected_ata = get_associated_token_address(&owner, &mint);
        assert_eq!(ix.accounts[3].pubkey, expected_ata);
    }

    #[test]
    fn test_convenience_function() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let ix = create_ata_idempotent_instruction(&payer, &owner, &mint).unwrap();
        assert_eq!(ix.data[0], CREATE_ATA_IDEMPOTENT_DISCRIMINATOR);
    }
}
