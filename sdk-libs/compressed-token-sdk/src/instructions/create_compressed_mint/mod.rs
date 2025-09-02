pub mod account_metas;
pub mod instruction;

pub use account_metas::{
    get_create_compressed_mint_instruction_account_metas, CreateCompressedMintMetaConfig,
};
pub use instruction::{
    create_compressed_mint, create_compressed_mint_cpi, derive_compressed_mint_address,
    derive_compressed_mint_from_spl_mint, find_spl_mint_address, CreateCompressedMintInputs,
    CREATE_COMPRESSED_MINT_DISCRIMINATOR,
};
use light_account_checks::AccountInfoTrait;
use light_sdk::cpi::CpiSigner;

#[derive(Clone, Debug)]
pub struct CpiContextWriteAccounts<'a, T: AccountInfoTrait + Clone> {
    pub mint_signer: &'a T,
    pub light_system_program: &'a T,
    pub fee_payer: &'a T,
    pub cpi_authority_pda: &'a T,
    pub cpi_context: &'a T,
    pub cpi_signer: CpiSigner,
}

impl<T: AccountInfoTrait + Clone> CpiContextWriteAccounts<'_, T> {
    pub fn bump(&self) -> u8 {
        self.cpi_signer.bump
    }

    pub fn invoking_program(&self) -> [u8; 32] {
        self.cpi_signer.program_id
    }

    pub fn to_account_infos(&self) -> Vec<T> {
        // The 5 accounts expected by create_compressed_mint_cpi_write:
        // [mint_signer, light_system_program, fee_payer, cpi_authority_pda, cpi_context]
        vec![
            self.mint_signer.clone(),
            self.light_system_program.clone(),
            self.fee_payer.clone(),
            self.cpi_authority_pda.clone(),
            self.cpi_context.clone(),
        ]
    }

    pub fn to_account_info_refs(&self) -> [&T; 3] {
        [self.mint_signer, self.fee_payer, self.cpi_context]
    }
}
