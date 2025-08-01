use light_account_checks::AccountInfoTrait;

use crate::{
    error::{LightTokenSdkTypeError, Result},
    AnchorDeserialize, AnchorSerialize,
};

#[repr(usize)]
pub enum MintToAccountInfosIndex {
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
    MerkleTree,
    SelfProgram,
    SystemProgram,
    SolPoolPda,
}

pub struct MintToAccountInfos<'a, T: AccountInfoTrait + Clone> {
    fee_payer: &'a T,
    authority: &'a T,
    accounts: &'a [T],
    config: MintToAccountInfosConfig,
}

#[derive(Debug, Default, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct MintToAccountInfosConfig {
    pub cpi_context: bool,
    pub has_mint: bool,         // false for batch_compress, true for mint_to
    pub has_sol_pool_pda: bool, // can be Some or None in both cases
}

impl MintToAccountInfosConfig {
    pub const fn new() -> Self {
        Self {
            cpi_context: false,
            has_mint: true, // default to mint_to behavior
            has_sol_pool_pda: false,
        }
    }

    pub const fn new_batch_compress() -> Self {
        Self {
            cpi_context: false,
            has_mint: false, // batch_compress doesn't use mint account
            has_sol_pool_pda: false,
        }
    }

    pub const fn new_with_cpi_context() -> Self {
        Self {
            cpi_context: true,
            has_mint: true,
            has_sol_pool_pda: false,
        }
    }

    pub const fn new_with_sol_pool_pda() -> Self {
        Self {
            cpi_context: false,
            has_mint: true,
            has_sol_pool_pda: true,
        }
    }

    pub const fn new_batch_compress_with_sol_pool_pda() -> Self {
        Self {
            cpi_context: false,
            has_mint: false,
            has_sol_pool_pda: true,
        }
    }
}

impl<'a, T: AccountInfoTrait + Clone> MintToAccountInfos<'a, T> {
    pub fn new(fee_payer: &'a T, authority: &'a T, accounts: &'a [T]) -> Self {
        Self {
            fee_payer,
            authority,
            accounts,
            config: MintToAccountInfosConfig::new(),
        }
    }

    pub fn new_with_config(
        fee_payer: &'a T,
        authority: &'a T,
        accounts: &'a [T],
        config: MintToAccountInfosConfig,
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
        let index = MintToAccountInfosIndex::CpiAuthorityPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn mint(&self) -> Result<&'a T> {
        if !self.config.has_mint {
            return Err(LightTokenSdkTypeError::MintUndefinedForBatchCompress);
        }
        let index = MintToAccountInfosIndex::Mint as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn token_pool_pda(&self) -> Result<&'a T> {
        let index = MintToAccountInfosIndex::TokenPoolPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn token_program(&self) -> Result<&'a T> {
        let index = MintToAccountInfosIndex::TokenProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn light_system_program(&self) -> Result<&'a T> {
        let index = MintToAccountInfosIndex::LightSystemProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn registered_program_pda(&self) -> Result<&'a T> {
        let index = MintToAccountInfosIndex::RegisteredProgramPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn noop_program(&self) -> Result<&'a T> {
        let index = MintToAccountInfosIndex::NoopProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_authority(&self) -> Result<&'a T> {
        let index = MintToAccountInfosIndex::AccountCompressionAuthority as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_program(&self) -> Result<&'a T> {
        let index = MintToAccountInfosIndex::AccountCompressionProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn merkle_tree(&self) -> Result<&'a T> {
        let index = MintToAccountInfosIndex::MerkleTree as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn self_program(&self) -> Result<&'a T> {
        let index = MintToAccountInfosIndex::SelfProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn system_program(&self) -> Result<&'a T> {
        let index = MintToAccountInfosIndex::SystemProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn sol_pool_pda(&self) -> Result<&'a T> {
        if !self.config.has_sol_pool_pda {
            return Err(LightTokenSdkTypeError::SolPoolPdaUndefined);
        }
        let index = MintToAccountInfosIndex::SolPoolPda as usize;
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

    pub fn config(&self) -> &MintToAccountInfosConfig {
        &self.config
    }

    pub fn system_accounts_len(&self) -> usize {
        let mut len = 15; // Base accounts from the enum
        if !self.config.has_sol_pool_pda {
            len -= 1; // Remove sol_pool_pda if it's None
        }
        len
    }
}
