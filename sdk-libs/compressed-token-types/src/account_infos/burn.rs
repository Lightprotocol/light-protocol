use light_account_checks::AccountInfoTrait;

use crate::{
    error::{LightTokenSdkTypeError, Result},
    AnchorDeserialize, AnchorSerialize,
};

#[repr(usize)]
pub enum BurnAccountInfosIndex {
    FeePayer,
    Authority,
    CpiAuthorityPda,
    Mint,
    TokenPoolPda,
    TokenProgram,
    LightSystemProgram,
    RegisteredProgramPda,
    NoopProgram,
    AccountCompressionAuthority,
    AccountCompressionProgram,
    SelfProgram,
    SystemProgram,
}

pub struct BurnAccountInfos<'a, T: AccountInfoTrait + Clone> {
    fee_payer: &'a T,
    authority: &'a T,
    accounts: &'a [T],
    config: BurnAccountInfosConfig,
}

#[derive(Debug, Default, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct BurnAccountInfosConfig {
    pub cpi_context: bool,
}

impl BurnAccountInfosConfig {
    pub const fn new() -> Self {
        Self { cpi_context: false }
    }

    pub const fn new_with_cpi_context() -> Self {
        Self { cpi_context: true }
    }
}

impl<'a, T: AccountInfoTrait + Clone> BurnAccountInfos<'a, T> {
    pub fn new(fee_payer: &'a T, authority: &'a T, accounts: &'a [T]) -> Self {
        Self {
            fee_payer,
            authority,
            accounts,
            config: BurnAccountInfosConfig::new(),
        }
    }

    pub fn new_with_config(
        fee_payer: &'a T,
        authority: &'a T,
        accounts: &'a [T],
        config: BurnAccountInfosConfig,
    ) -> Self {
        Self {
            fee_payer,
            authority,
            accounts,
            config,
        }
    }

    pub fn fee_payer(&self) -> &'a T {
        self.fee_payer
    }

    pub fn authority(&self) -> &'a T {
        self.authority
    }

    pub fn cpi_authority_pda(&self) -> Result<&'a T> {
        let index = BurnAccountInfosIndex::CpiAuthorityPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn mint(&self) -> Result<&'a T> {
        let index = BurnAccountInfosIndex::Mint as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn token_pool_pda(&self) -> Result<&'a T> {
        let index = BurnAccountInfosIndex::TokenPoolPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn token_program(&self) -> Result<&'a T> {
        let index = BurnAccountInfosIndex::TokenProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn light_system_program(&self) -> Result<&'a T> {
        let index = BurnAccountInfosIndex::LightSystemProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn registered_program_pda(&self) -> Result<&'a T> {
        let index = BurnAccountInfosIndex::RegisteredProgramPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn noop_program(&self) -> Result<&'a T> {
        let index = BurnAccountInfosIndex::NoopProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_authority(&self) -> Result<&'a T> {
        let index = BurnAccountInfosIndex::AccountCompressionAuthority as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_program(&self) -> Result<&'a T> {
        let index = BurnAccountInfosIndex::AccountCompressionProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn self_program(&self) -> Result<&'a T> {
        let index = BurnAccountInfosIndex::SelfProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn system_program(&self) -> Result<&'a T> {
        let index = BurnAccountInfosIndex::SystemProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn get_account_info(&self, index: usize) -> Result<&'a T> {
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_infos(&self) -> &'a [T] {
        self.accounts
    }

    pub fn config(&self) -> &BurnAccountInfosConfig {
        &self.config
    }

    pub fn system_accounts_len(&self) -> usize {
        // BurnInstruction has a fixed number of accounts
        13 // All accounts from the enum
    }
}
