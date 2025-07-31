pub mod account_metas;
pub mod instruction;

pub use account_metas::{
    get_mint_to_compressed_instruction_account_metas,
    get_mint_to_compressed_instruction_account_metas_cpi_write, MintToCompressedMetaConfig,
    MintToCompressedMetaConfigCpiWrite,
};
pub use instruction::{
    create_mint_to_compressed_cpi_write, create_mint_to_compressed_instruction,
    DecompressedMintConfig, MintToCompressedInputs, MintToCompressedInputsCpiWrite,
    MINT_TO_COMPRESSED_DISCRIMINATOR,
};

use light_account_checks::AccountInfoTrait;
use light_sdk::cpi::CpiSigner;

#[derive(Clone, Debug)]
pub struct MintToCompressedCpiContextWriteAccounts<'a, T: AccountInfoTrait + Clone> {
    pub mint_authority: &'a T,
    pub light_system_program: &'a T,
    pub fee_payer: &'a T,
    pub cpi_authority_pda: &'a T,
    pub cpi_context: &'a T,
    pub cpi_signer: CpiSigner,
}

impl<'a, T: AccountInfoTrait + Clone> MintToCompressedCpiContextWriteAccounts<'a, T> {
    pub fn bump(&self) -> u8 {
        self.cpi_signer.bump
    }

    pub fn invoking_program(&self) -> [u8; 32] {
        self.cpi_signer.program_id
    }

    pub fn to_account_infos(&self) -> Vec<T> {
        // The 5 accounts expected by mint_to_compressed_cpi_write:
        // [light_system_program, mint_authority, fee_payer, cpi_authority_pda, cpi_context]
        vec![
            self.light_system_program.clone(),
            self.mint_authority.clone(),
            self.fee_payer.clone(),
            self.cpi_authority_pda.clone(),
            self.cpi_context.clone(),
        ]
    }
}
