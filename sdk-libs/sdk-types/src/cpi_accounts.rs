#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use light_account_checks::AccountInfoTrait;

use crate::{
    error::{LightSdkTypesError, Result},
    CpiSigner, CPI_CONTEXT_ACCOUNT_DISCRIMINATOR, LIGHT_SYSTEM_PROGRAM_ID, SOL_POOL_PDA,
};

#[derive(Debug, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CpiAccountsConfig {
    pub cpi_context: bool,
    pub sol_compression_recipient: bool,
    pub sol_pool_pda: bool,
    pub cpi_signer: CpiSigner,
}

impl CpiAccountsConfig {
    pub const fn new(cpi_signer: CpiSigner) -> Self {
        Self {
            cpi_context: false,
            sol_compression_recipient: false,
            sol_pool_pda: false,
            cpi_signer,
        }
    }

    pub const fn new_with_cpi_context(cpi_signer: CpiSigner) -> Self {
        Self {
            cpi_context: true,
            sol_compression_recipient: false,
            sol_pool_pda: false,
            cpi_signer,
        }
    }

    pub fn cpi_signer(&self) -> [u8; 32] {
        self.cpi_signer.cpi_signer
    }

    pub fn bump(&self) -> u8 {
        self.cpi_signer.bump
    }
}

#[repr(usize)]
pub enum CompressionCpiAccountIndex {
    LightSystemProgram,
    Authority,
    RegisteredProgramPda,
    NoopProgram,
    AccountCompressionAuthority,
    AccountCompressionProgram,
    InvokingProgram,
    SolPoolPda,
    DecompressionRecipient,
    SystemProgram,
    CpiContext,
}

pub const SYSTEM_ACCOUNTS_LEN: usize = 11;

pub struct CpiAccounts<'a, T: AccountInfoTrait> {
    fee_payer: &'a T,
    accounts: &'a [T],
    config: CpiAccountsConfig,
}

impl<'a, T: AccountInfoTrait> CpiAccounts<'a, T> {
    pub fn new(fee_payer: &'a T, accounts: &'a [T], cpi_signer: CpiSigner) -> Self {
        Self {
            fee_payer,
            accounts,
            config: CpiAccountsConfig::new(cpi_signer),
        }
    }

    pub fn try_new(fee_payer: &'a T, accounts: &'a [T], cpi_signer: CpiSigner) -> Result<Self> {
        if accounts[0].key() != LIGHT_SYSTEM_PROGRAM_ID {
            return Err(LightSdkTypesError::InvalidCpiAccountsOffset);
        }
        Ok(Self {
            fee_payer,
            accounts,
            config: CpiAccountsConfig::new(cpi_signer),
        })
    }

    pub fn new_with_config(fee_payer: &'a T, accounts: &'a [T], config: CpiAccountsConfig) -> Self {
        Self {
            fee_payer,
            accounts,
            config,
        }
    }

    pub fn try_new_with_config(
        fee_payer: &'a T,
        accounts: &'a [T],
        config: CpiAccountsConfig,
    ) -> Result<Self> {
        let res = Self {
            fee_payer,
            accounts,
            config,
        };
        if accounts[0].key() != LIGHT_SYSTEM_PROGRAM_ID {
            return Err(LightSdkTypesError::InvalidCpiAccountsOffset);
        }
        if res.config().cpi_context {
            let cpi_context = res.cpi_context()?;
            let discriminator_bytes = &cpi_context.try_borrow_data()?[..8];
            if discriminator_bytes != CPI_CONTEXT_ACCOUNT_DISCRIMINATOR.as_slice() {
                solana_msg::msg!("Invalid CPI context account: {:?}", cpi_context.pubkey());
                return Err(LightSdkTypesError::InvalidCpiContextAccount);
            }
        }

        if res.config().sol_pool_pda && res.sol_pool_pda()?.key() != SOL_POOL_PDA {
            return Err(LightSdkTypesError::InvalidSolPoolPdaAccount);
        }

        Ok(res)
    }

    pub fn fee_payer(&self) -> &'a T {
        self.fee_payer
    }

    pub fn light_system_program(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::LightSystemProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn authority(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::Authority as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn invoking_program(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::InvokingProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn registered_program_pda(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::RegisteredProgramPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn noop_program(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::NoopProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_authority(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::AccountCompressionAuthority as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_program(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::AccountCompressionProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn sol_pool_pda(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::SolPoolPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn decompression_recipient(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::DecompressionRecipient as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn system_program(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::SystemProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn cpi_context(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndex::CpiContext as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn self_program_id(&self) -> T::Pubkey {
        T::pubkey_from_bytes(self.config.cpi_signer.program_id)
    }

    pub fn bump(&self) -> u8 {
        self.config.cpi_signer.bump
    }

    pub fn config(&self) -> &CpiAccountsConfig {
        &self.config
    }

    pub fn system_accounts_len(&self) -> usize {
        let mut len = SYSTEM_ACCOUNTS_LEN;
        if !self.config.sol_pool_pda {
            len -= 1;
        }
        if !self.config.sol_compression_recipient {
            len -= 1;
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
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn tree_accounts(&self) -> Result<&'a [T]> {
        let system_len = self.system_accounts_len();
        self.accounts
            .get(system_len..)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(system_len))
    }

    pub fn get_tree_account_info(&self, tree_index: usize) -> Result<&'a T> {
        let tree_accounts = self.tree_accounts()?;
        tree_accounts
            .get(tree_index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(
                self.system_accounts_len() + tree_index,
            ))
    }

    /// Create a vector of account info references
    pub fn to_account_infos(&self) -> Vec<&'a T> {
        let mut account_infos = Vec::with_capacity(1 + SYSTEM_ACCOUNTS_LEN);
        account_infos.push(self.fee_payer());
        self.account_infos()[1..]
            .iter()
            .for_each(|acc| account_infos.push(acc));
        account_infos
    }
}
