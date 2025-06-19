use light_account_checks::AccountInfoTrait;

use crate::{
    account_infos::MintToAccountInfosConfig,
    error::{LightTokenSdkTypeError, Result},
};

#[repr(usize)]
pub enum BatchCompressAccountInfosIndex {
    // FeePayer,
    // Authority,
    CpiAuthorityPda,
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
    SenderTokenAccount,
}

pub struct BatchCompressAccountInfos<'a, T: AccountInfoTrait + Clone> {
    fee_payer: &'a T,
    authority: &'a T,
    accounts: &'a [T],
    config: MintToAccountInfosConfig,
}

impl<'a, T: AccountInfoTrait + Clone> BatchCompressAccountInfos<'a, T> {
    pub fn new(fee_payer: &'a T, authority: &'a T, accounts: &'a [T]) -> Self {
        Self {
            fee_payer,
            authority,
            accounts,
            config: MintToAccountInfosConfig::new_batch_compress(),
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
        let index = BatchCompressAccountInfosIndex::CpiAuthorityPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn token_pool_pda(&self) -> Result<&'a T> {
        let index = BatchCompressAccountInfosIndex::TokenPoolPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn token_program(&self) -> Result<&'a T> {
        let index = BatchCompressAccountInfosIndex::TokenProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn light_system_program(&self) -> Result<&'a T> {
        let index = BatchCompressAccountInfosIndex::LightSystemProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn registered_program_pda(&self) -> Result<&'a T> {
        let index = BatchCompressAccountInfosIndex::RegisteredProgramPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn noop_program(&self) -> Result<&'a T> {
        let index = BatchCompressAccountInfosIndex::NoopProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_authority(&self) -> Result<&'a T> {
        let index = BatchCompressAccountInfosIndex::AccountCompressionAuthority as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_program(&self) -> Result<&'a T> {
        let index = BatchCompressAccountInfosIndex::AccountCompressionProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn merkle_tree(&self) -> Result<&'a T> {
        let index = BatchCompressAccountInfosIndex::MerkleTree as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn self_program(&self) -> Result<&'a T> {
        let index = BatchCompressAccountInfosIndex::SelfProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn system_program(&self) -> Result<&'a T> {
        let index = BatchCompressAccountInfosIndex::SystemProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn sol_pool_pda(&self) -> Result<&'a T> {
        if !self.config.has_sol_pool_pda {
            return Err(LightTokenSdkTypeError::SolPoolPdaUndefined);
        }
        let index = BatchCompressAccountInfosIndex::SolPoolPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn sender_token_account(&self) -> Result<&'a T> {
        let mut index = BatchCompressAccountInfosIndex::SenderTokenAccount as usize;
        if !self.config.has_sol_pool_pda {
            index -= 1;
        }
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn get_account_info(&self, index: usize) -> Result<&'a T> {
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }
    pub fn to_account_infos(&self) -> Vec<T> {
        [
            vec![self.fee_payer.clone()],
            vec![self.authority.clone()],
            self.accounts.to_vec(),
        ]
        .concat()
    }

    pub fn account_infos(&self) -> &'a [T] {
        self.accounts
    }

    pub fn config(&self) -> &MintToAccountInfosConfig {
        &self.config
    }

    pub fn system_accounts_len(&self) -> usize {
        let mut len = 13; // Base accounts from the enum (including sender_token_account)
        if !self.config.has_sol_pool_pda {
            len -= 1; // Remove sol_pool_pda if it's None
        }
        len
    }
}
