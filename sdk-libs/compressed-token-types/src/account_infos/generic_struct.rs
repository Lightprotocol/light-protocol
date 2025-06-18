use std::marker::PhantomData;

use light_account_checks::AccountInfoTrait;

use crate::{
    account_infos::TransferAccountInfosConfig,
    error::{LightTokenSdkTypeError, Result},
};

pub trait AccountInfoIndexGetter {
    const SYSTEM_ACCOUNTS_LEN: usize;
    fn cpi_authority_index() -> usize;
    fn light_system_program_index() -> usize;
    fn registered_program_pda_index() -> usize;
    fn noop_program_index() -> usize;
    fn account_compression_authority_index() -> usize;
    fn account_compression_program_index() -> usize;
    fn ctoken_program_index() -> usize;
    fn token_pool_pda_index() -> usize;
    fn decompression_recipient_index() -> usize;
    fn spl_token_program_index() -> usize;
    fn system_program_index() -> usize;
    fn cpi_context_index() -> usize;
}

pub struct AccountInfos<'a, T: AccountInfoTrait + Clone, I: AccountInfoIndexGetter> {
    fee_payer: &'a T,
    authority: &'a T,
    accounts: &'a [T],
    config: TransferAccountInfosConfig,
    _p: PhantomData<I>,
}

impl<'a, T: AccountInfoTrait + Clone, I: AccountInfoIndexGetter> AccountInfos<'a, T, I> {
    pub fn new(fee_payer: &'a T, authority: &'a T, accounts: &'a [T]) -> Self {
        Self {
            fee_payer,
            authority,
            accounts,
            config: TransferAccountInfosConfig::default(),
            _p: PhantomData,
        }
    }

    pub fn new_compress(fee_payer: &'a T, authority: &'a T, accounts: &'a [T]) -> Self {
        Self {
            fee_payer,
            authority,
            accounts,
            config: TransferAccountInfosConfig::new_compress(),
            _p: PhantomData,
        }
    }

    pub fn new_decompress(fee_payer: &'a T, authority: &'a T, accounts: &'a [T]) -> Self {
        Self {
            fee_payer,
            authority,
            accounts,
            config: TransferAccountInfosConfig::new_decompress(),
            _p: PhantomData,
        }
    }

    pub fn new_with_config(
        fee_payer: &'a T,
        authority: &'a T,
        accounts: &'a [T],
        config: TransferAccountInfosConfig,
    ) -> Self {
        Self {
            fee_payer,
            authority,
            accounts,
            config,
            _p: PhantomData,
        }
    }

    pub fn fee_payer(&self) -> &'a T {
        self.fee_payer
    }

    pub fn light_system_program(&self) -> Result<&'a T> {
        let index = I::light_system_program_index();
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn authority(&self) -> &'a T {
        self.authority
    }

    pub fn ctoken_program(&self) -> Result<&'a T> {
        let index = I::ctoken_program_index();
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn spl_token_program(&self) -> Result<&'a T> {
        let index = I::spl_token_program_index();
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn registered_program_pda(&self) -> Result<&'a T> {
        let index = I::registered_program_pda_index();
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn noop_program(&self) -> Result<&'a T> {
        let index = I::noop_program_index();
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_authority(&self) -> Result<&'a T> {
        let index = I::account_compression_authority_index();
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_program(&self) -> Result<&'a T> {
        let index = I::account_compression_program_index();
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn token_pool_pda(&self) -> Result<&'a T> {
        let index = I::token_pool_pda_index();
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn decompression_recipient(&self) -> Result<&'a T> {
        if !self.config.decompress {
            return Err(LightTokenSdkTypeError::DecompressionRecipientTokenAccountDoesOnlyExistInDecompressedMode);
        };
        let index = I::decompression_recipient_index();
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn sender_token_account(&self) -> Result<&'a T> {
        if !self.config.compress {
            return Err(LightTokenSdkTypeError::SenderTokenAccountDoesOnlyExistInCompressedMode);
        };
        let index = I::decompression_recipient_index();
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn system_program(&self) -> Result<&'a T> {
        let index = I::system_program_index();
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn cpi_context(&self) -> Result<&'a T> {
        let index = I::cpi_context_index();
        self.accounts
            .get(index)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn config(&self) -> &TransferAccountInfosConfig {
        &self.config
    }

    pub fn system_accounts_len(&self) -> usize {
        let mut len = I::SYSTEM_ACCOUNTS_LEN;
        if !self.config.is_compress_or_decompress() {
            solana_msg::msg!("System accounts length calculation");
            // Token pool pda & compression sender or decompression recipient
            len -= 3;
        }
        if !self.config.cpi_context {
            solana_msg::msg!("System accounts length calculation");
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
        solana_msg::msg!("Tree accounts length calculation {}", system_len);
        self.accounts
            .get(system_len..)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(
                system_len,
            ))
    }

    pub fn tree_pubkeys(&self) -> Result<Vec<T::Pubkey>> {
        let system_len = self.system_accounts_len();
        Ok(self
            .accounts
            .get(system_len..)
            .ok_or(LightTokenSdkTypeError::CpiAccountsIndexOutOfBounds(
                system_len,
            ))?
            .iter()
            .map(|account| account.pubkey())
            .collect::<Vec<T::Pubkey>>())
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
        let mut account_infos = Vec::with_capacity(1 + I::SYSTEM_ACCOUNTS_LEN);
        account_infos.push(self.fee_payer());
        self.account_infos()[1..]
            .iter()
            .for_each(|acc| account_infos.push(acc));
        account_infos
    }

    /// Create a vector of account info references
    pub fn to_account_infos(&self) -> Vec<T> {
        let mut account_infos = Vec::with_capacity(1 + I::SYSTEM_ACCOUNTS_LEN);
        account_infos.push(self.fee_payer().clone());
        self.account_infos()
            .iter()
            .for_each(|acc| account_infos.push(acc.clone()));
        account_infos
    }
}
