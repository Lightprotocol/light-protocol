#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use light_account_checks::AccountInfoTrait;

use crate::CpiSigner;

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

    pub fn new_with_config(fee_payer: &'a T, accounts: &'a [T], config: CpiAccountsConfig) -> Self {
        Self {
            fee_payer,
            accounts,
            config,
        }
    }

    pub fn fee_payer(&self) -> &'a T {
        self.fee_payer
    }

    pub fn light_system_program(&self) -> &'a T {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(CompressionCpiAccountIndex::LightSystemProgram as usize)
            .unwrap()
    }

    pub fn authority(&self) -> &'a T {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(CompressionCpiAccountIndex::Authority as usize)
            .unwrap()
    }

    pub fn invoking_program(&self) -> &'a T {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(CompressionCpiAccountIndex::InvokingProgram as usize)
            .unwrap()
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

    pub fn tree_accounts(&self) -> &'a [T] {
        &self.accounts[self.system_accounts_len()..]
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
