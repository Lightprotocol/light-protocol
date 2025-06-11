use light_account_checks::AccountInfoTrait;

use crate::{CpiAccountsConfig, CpiSigner};

#[repr(usize)]
pub enum CompressionCpiAccountIndexSmall {
    LightSystemProgram,        // Only exposed to outer instruction
    AccountCompressionProgram, // Only exposed to outer instruction
    SystemProgram,             // Only exposed to outer instruction
    Authority, // Cpi authority of the custom program, used to invoke the light system program.
    RegisteredProgramPda,
    AccountCompressionAuthority,
    SolPoolPda,             // Optional
    DecompressionRecipient, // Optional
    CpiContext,             // Optional
}

pub const PROGRAM_ACCOUNTS_LEN: usize = 3;
// 6 + 3 program ids, fee payer is extra.
pub const SMALL_SYSTEM_ACCOUNTS_LEN: usize = 9;

pub struct CpiAccountsSmall<'a, T: AccountInfoTrait> {
    fee_payer: &'a T,
    accounts: &'a [T],
    config: CpiAccountsConfig,
}

impl<'a, T: AccountInfoTrait> CpiAccountsSmall<'a, T> {
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

    pub fn authority(&self) -> &'a T {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(CompressionCpiAccountIndexSmall::Authority as usize)
            .unwrap()
    }

    pub fn self_program_id(&self) -> T::Pubkey {
        T::pubkey_from_bytes(self.config.cpi_signer.program_id)
    }

    pub fn config(&self) -> &CpiAccountsConfig {
        &self.config
    }

    pub fn system_accounts_end_offset(&self) -> usize {
        let mut len = SMALL_SYSTEM_ACCOUNTS_LEN;
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
        &self.accounts[self.system_accounts_end_offset()..]
    }

    /// Create a vector of account info references
    pub fn to_account_infos(&self) -> Vec<&'a T> {
        let mut account_infos = Vec::with_capacity(1 + self.accounts.len() - PROGRAM_ACCOUNTS_LEN);
        account_infos.push(self.fee_payer());
        self.accounts[PROGRAM_ACCOUNTS_LEN..]
            .iter()
            .for_each(|acc| account_infos.push(acc));
        account_infos
    }

    pub fn account_infos_slice(&self) -> &[T] {
        &self.accounts[PROGRAM_ACCOUNTS_LEN..]
    }
}
