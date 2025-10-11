use light_account_checks::AccountInfoTrait;

use crate::CpiSigner;
// TODO: move to ctoken types
#[derive(Clone, Debug)]
pub struct CpiContextWriteAccounts<'a, T: AccountInfoTrait + Clone> {
    pub fee_payer: &'a T,
    pub authority: &'a T,
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

    pub fn to_account_infos(&self) -> [T; 3] {
        [
            self.fee_payer.clone(),
            self.authority.clone(),
            self.cpi_context.clone(),
        ]
    }

    pub fn to_account_info_refs(&self) -> [&T; 3] {
        [self.fee_payer, self.authority, self.cpi_context]
    }
}
