use light_account_checks::AccountInfoTrait;

use crate::{
    error::{LightSdkTypesError, Result},
    CpiAccountsConfig, CpiSigner,
};

#[repr(usize)]
pub enum CompressionCpiAccountIndexSmall {
    LightSystemProgram,          // index 0 - hardcoded in cpi hence no getter.
    Authority, // index 1 - Cpi authority of the custom program, used to invoke the light system program.
    RegisteredProgramPda, // index 2 - registered_program_pda
    AccountCompressionAuthority, // index 3 - account_compression_authority
    AccountCompressionProgram, // index 4 - account_compression_program
    SystemProgram, // index 5 - system_program
    SolPoolPda, // index 6 - Optional
    DecompressionRecipient, // index 7 - Optional
    CpiContext, // index 8 - Optional
}

pub const PROGRAM_ACCOUNTS_LEN: usize = 0; // No program accounts in CPI
                                           // 6 base accounts + 3 optional accounts
pub const SMALL_SYSTEM_ACCOUNTS_LEN: usize = 9;

#[derive(Clone)]
pub struct CpiAccountsSmall<'a, T: AccountInfoTrait + Clone> {
    fee_payer: &'a T,
    accounts: &'a [T],
    config: CpiAccountsConfig,
}

impl<'a, T: AccountInfoTrait + Clone> CpiAccountsSmall<'a, T> {
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

    pub fn authority(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndexSmall::Authority as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn registered_program_pda(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndexSmall::RegisteredProgramPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_authority(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndexSmall::AccountCompressionAuthority as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn account_compression_program(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndexSmall::AccountCompressionProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn system_program(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndexSmall::SystemProgram as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn sol_pool_pda(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndexSmall::SolPoolPda as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn decompression_recipient(&self) -> Result<&'a T> {
        let index = CompressionCpiAccountIndexSmall::DecompressionRecipient as usize;
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn cpi_context(&self) -> Result<&'a T> {
        let mut index = CompressionCpiAccountIndexSmall::CpiContext as usize;
        if !self.config.sol_pool_pda {
            index -= 1;
        }
        if !self.config.sol_compression_recipient {
            index -= 1;
        }
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
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

    pub fn get_account_info(&self, index: usize) -> Result<&'a T> {
        self.accounts
            .get(index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(index))
    }

    pub fn tree_accounts(&self) -> Result<&'a [T]> {
        let system_offset = self.system_accounts_end_offset();
        self.accounts
            .get(system_offset..)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(
                system_offset,
            ))
    }

    pub fn get_tree_account_info(&self, tree_index: usize) -> Result<&'a T> {
        let tree_accounts = self.tree_accounts()?;
        tree_accounts
            .get(tree_index)
            .ok_or(LightSdkTypesError::CpiAccountsIndexOutOfBounds(
                self.system_accounts_end_offset() + tree_index,
            ))
    }

    // TODO: unify with get_tree_account_info
    pub fn get_tree_address(&self, tree_index: u8) -> Result<&'a T> {
        let tree_accounts = self.tree_accounts()?;
        tree_accounts.get(tree_index as usize).ok_or(
            LightSdkTypesError::CpiAccountsIndexOutOfBounds(
                self.system_accounts_end_offset() + tree_index as usize,
            ),
        )
    }

    /// Create a vector of account info references
    pub fn to_account_infos(&self) -> Vec<T> {
        let mut account_infos = Vec::with_capacity(1 + self.accounts.len());
        account_infos.push(self.fee_payer().clone());
        // Skip system light program
        self.accounts[1..]
            .iter()
            .for_each(|acc| account_infos.push(acc.clone()));
        account_infos
    }
    pub fn bump(&self) -> u8 {
        self.config.cpi_signer.bump
    }

    pub fn invoking_program(&self) -> [u8; 32] {
        self.config.cpi_signer.program_id
    }
    pub fn account_infos_slice(&self) -> &[T] {
        &self.accounts[PROGRAM_ACCOUNTS_LEN..]
    }

    pub fn tree_pubkeys(&self) -> Result<Vec<T::Pubkey>> {
        Ok(self
            .tree_accounts()?
            .iter()
            .map(|x| x.pubkey())
            .collect::<Vec<T::Pubkey>>())
    }
}
