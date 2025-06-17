use crate::{AnchorDeserialize, AnchorSerialize};
use light_account_checks::AccountInfoTrait;

use crate::error::{LightTokenSdkTypeError, Result};

#[derive(Debug, Default, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CpiAccountsConfig {
    pub cpi_context: bool,
    pub compress: bool,
    pub decompress: bool,
}

impl CpiAccountsConfig {
    pub const fn new_with_cpi_context() -> Self {
        Self {
            cpi_context: true,
            compress: false,
            decompress: false,
        }
    }

    pub fn new_compress() -> Self {
        Self {
            cpi_context: false,
            compress: true,
            decompress: false,
        }
    }

    pub fn new_decompress() -> Self {
        Self {
            cpi_context: true,
            compress: false,
            decompress: true,
        }
    }

    pub fn is_compress_or_decompress(&self) -> bool {
        self.compress || self.decompress
    }
}

#[repr(usize)]
pub enum CompressionCpiAccountIndex {
    CpiAuthority,
    LightSystemProgram,
    RegisteredProgramPda,
    NoopProgram,
    AccountCompressionAuthority,
    AccountCompressionProgram,
    CTokenProgram,
    TokenPoolPda,
    DecompressionRecipient,
    SplTokenProgram,
    SystemProgram,
    CpiContext,
}

pub const SYSTEM_ACCOUNTS_LEN: usize = 12;

pub struct CpiAccounts<'a, T: AccountInfoTrait + Clone> {
    fee_payer: &'a T,
    authority: &'a T,
    accounts: &'a [T],
    config: CpiAccountsConfig,
}

impl<'a, T: AccountInfoTrait + Clone> CpiAccounts<'a, T> {
    pub fn new(fee_payer: &'a T, authority: &'a T, accounts: &'a [T]) -> Self {
        Self {
            fee_payer,
            authority,
            accounts,
            config: CpiAccountsConfig::default(),
        }
    }

    pub fn new_compress(fee_payer: &'a T, authority: &'a T, accounts: &'a [T]) -> Self {
        Self {
            fee_payer,
            authority,
            accounts,
            config: CpiAccountsConfig::new_compress(),
        }
    }

    pub fn new_decompress(fee_payer: &'a T, authority: &'a T, accounts: &'a [T]) -> Self {
        Self {
            fee_payer,
            authority,
            accounts,
            config: CpiAccountsConfig::new_decompress(),
        }
    }

    pub fn new_with_config(
        fee_payer: &'a T,
        authority: &'a T,
        accounts: &'a [T],
        config: CpiAccountsConfig,
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

    pub fn light_system_program(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::LightSystemProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn authority(&self) -> &'a T {
        self.authority
    }

    pub fn ctoken_program(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::CTokenProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn spl_token_program(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::SplTokenProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn registered_program_pda(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::RegisteredProgramPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn noop_program(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::NoopProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_authority(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::AccountCompressionAuthority as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_program(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::AccountCompressionProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn token_pool_pda(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::TokenPoolPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn decompression_recipient(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::DecompressionRecipient as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn system_program(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::SystemProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn cpi_context(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::CpiContext as usize;
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn config(&self) -> &CpiAccountsConfig {
        &self.config
    }

    pub fn system_accounts_len(&self) -> usize {
        let mut len = SYSTEM_ACCOUNTS_LEN;
        if !self.config.is_compress_or_decompress() {
            // Token pool pda & compression sender or decompression recipient
            len -= 2;
        }
        if !self.config.cpi_context {
            len -= 1;
        }
        len
    }

    pub fn account_infos(&self) -> &'a [T] {
        self.accounts
    }

    pub fn get_account_info(&self, index: usize) -> Result<&'a T> {
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn tree_accounts(&self) -> Result<&'a [T]> {
        let system_len = self.system_accounts_len();
        self.accounts
            .get(system_len..)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(
                system_len,
            ))
    }

    pub fn get_tree_account_info(&self, tree_index: usize) -> Result<&'a T> {
        let tree_accounts = self.tree_accounts()?;
        tree_accounts
            .get(tree_index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(
                self.system_accounts_len() + tree_index,
            ))
    }

    /// Create a vector of account info references
    pub fn to_account_info_refs(&self) -> Vec<&'a T> {
        let mut account_infos = Vec::with_capacity(1 + SYSTEM_ACCOUNTS_LEN);
        account_infos.push(self.fee_payer());
        self.account_infos()[1..]
            .iter()
            .for_each(|acc| account_infos.push(acc));
        account_infos
    }
    /// Create a vector of account info references
    pub fn to_account_infos(&self) -> Vec<T> {
        let mut account_infos = Vec::with_capacity(1 + SYSTEM_ACCOUNTS_LEN);
        account_infos.push(self.fee_payer().clone());
        self.account_infos()
            .iter()
            .for_each(|acc| account_infos.push(acc.clone()));
        account_infos
    }
}
