use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

use crate::{
    error::{LightSdkError, Result},
    BorshDeserialize, BorshSerialize,
};

#[derive(Debug, Default, Copy, Clone, BorshSerialize, BorshDeserialize)]
pub struct CpiAccountsConfig {
    pub self_program: Pubkey,
    pub cpi_context: bool,
    pub sol_compression_recipient: bool,
    pub sol_pool_pda: bool,
}

impl CpiAccountsConfig {
    pub fn new(self_program: Pubkey) -> Self {
        Self {
            self_program,
            cpi_context: false,
            sol_compression_recipient: false,
            sol_pool_pda: false,
        }
    }

    pub fn new_with_cpi_context(self_program: Pubkey) -> Self {
        Self {
            self_program,
            cpi_context: true,
            sol_compression_recipient: false,
            sol_pool_pda: false,
        }
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
    DecompressionRecipent,
    SystemProgram,
    CpiContext,
}

pub const SYSTEM_ACCOUNTS_LEN: usize = 11;

pub struct CpiAccounts<'a> {
    fee_payer: &'a AccountInfo,
    accounts: &'a [AccountInfo],
    config: CpiAccountsConfig,
}

impl<'a> CpiAccounts<'a> {
    pub fn new(
        fee_payer: &'a AccountInfo,
        accounts: &'a [AccountInfo],
        program_id: Pubkey,
    ) -> Result<Self> {
        let new = Self {
            fee_payer,
            accounts,
            config: CpiAccountsConfig {
                self_program: program_id,
                ..Default::default()
            },
        };
        if accounts.len() < new.system_accounts_len() {
            return Err(LightSdkError::FewerAccountsThanSystemAccounts);
        }
        Ok(new)
    }

    pub fn new_with_config(
        fee_payer: &'a AccountInfo,
        accounts: &'a [AccountInfo],
        config: CpiAccountsConfig,
    ) -> Result<Self> {
        let new = Self {
            fee_payer,
            accounts,
            config,
        };
        if accounts.len() < new.system_accounts_len() {
            return Err(LightSdkError::FewerAccountsThanSystemAccounts);
        }
        Ok(new)
    }

    pub fn fee_payer(&self) -> &'a AccountInfo {
        self.fee_payer
    }

    pub fn light_system_program(&self) -> &'a AccountInfo {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(CompressionCpiAccountIndex::LightSystemProgram as usize)
            .unwrap()
    }

    pub fn authority(&self) -> &'a AccountInfo {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(CompressionCpiAccountIndex::Authority as usize)
            .unwrap()
    }

    pub fn invoking_program(&self) -> &'a AccountInfo {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(CompressionCpiAccountIndex::InvokingProgram as usize)
            .unwrap()
    }

    pub fn self_program_id(&self) -> &Pubkey {
        &self.config.self_program
    }

    pub fn to_account_infos(&self) -> Vec<&'a AccountInfo> {
        let mut account_infos = Vec::with_capacity(1 + SYSTEM_ACCOUNTS_LEN);
        account_infos.push(self.fee_payer);
        // Skip the first account (light_system_program) and add the rest
        self.accounts[1..]
            .iter()
            .for_each(|acc| account_infos.push(acc));
        let mut current_index = 7;
        if !self.config.sol_pool_pda {
            account_infos.insert(current_index, self.light_system_program());
        }
        current_index += 1;

        if !self.config.sol_compression_recipient {
            account_infos.insert(current_index, self.light_system_program());
        }
        current_index += 1;
        // system program
        current_index += 1;

        if !self.config.cpi_context {
            account_infos.insert(current_index, self.light_system_program());
        }
        account_infos
    }

    pub fn to_account_metas(&self) -> Vec<pinocchio::instruction::AccountMeta> {
        use pinocchio::instruction::AccountMeta;
        msg!("pre account_metas");
        msg!(format!("{}", self.accounts.len()).as_str());
        let mut account_metas = Vec::with_capacity(1 + SYSTEM_ACCOUNTS_LEN);
        account_metas.push(AccountMeta::writable_signer(self.fee_payer.key()));
        account_metas.push(AccountMeta::readonly_signer(self.authority().key()));

        account_metas.push(AccountMeta::readonly(
            self.accounts[CompressionCpiAccountIndex::RegisteredProgramPda as usize].key(),
        ));
        account_metas.push(AccountMeta::readonly(
            self.accounts[CompressionCpiAccountIndex::NoopProgram as usize].key(),
        ));
        account_metas.push(AccountMeta::readonly(
            self.accounts[CompressionCpiAccountIndex::AccountCompressionAuthority as usize].key(),
        ));
        account_metas.push(AccountMeta::readonly(
            self.accounts[CompressionCpiAccountIndex::AccountCompressionProgram as usize].key(),
        ));
        account_metas.push(AccountMeta::readonly(
            self.accounts[CompressionCpiAccountIndex::InvokingProgram as usize].key(),
        ));
        let mut current_index = 7;
        if !self.config.sol_pool_pda {
            account_metas.push(AccountMeta::readonly(self.light_system_program().key()));
        } else {
            account_metas.push(AccountMeta::writable(self.accounts[current_index].key()));
            current_index += 1;
        }

        if !self.config.sol_compression_recipient {
            account_metas.push(AccountMeta::readonly(self.light_system_program().key()));
        } else {
            account_metas.push(AccountMeta::writable(self.accounts[current_index].key()));
            current_index += 1;
        }

        // System program - use default (all zeros)
        account_metas.push(AccountMeta::readonly(&[0u8; 32]));
        current_index += 1;

        if !self.config.cpi_context {
            account_metas.push(AccountMeta::readonly(self.light_system_program().key()));
        } else {
            account_metas.push(AccountMeta::writable(self.accounts[current_index].key()));
            current_index += 1;
        }

        // Add remaining tree accounts
        self.accounts[current_index..].iter().for_each(|acc| {
            let account_meta = if acc.is_writable() {
                AccountMeta::writable(acc.key())
            } else {
                AccountMeta::readonly(acc.key())
            };
            account_metas.push(account_meta);
        });

        account_metas
    }

    pub fn system_accounts_len(&self) -> usize {
        let mut len = 7; // Base system accounts

        if self.config.sol_pool_pda {
            len += 1;
        }

        if self.config.sol_compression_recipient {
            len += 1;
        }

        if self.config.cpi_context {
            len += 1;
        }

        len + 1 // Add system program
    }

    pub fn account_infos(&self) -> &'a [AccountInfo] {
        self.accounts
    }

    pub fn tree_accounts(&self) -> &'a [AccountInfo] {
        msg!(format!("tree_accounts: {}", self.accounts.len()).as_str());
        msg!(format!("offset {}", self.system_accounts_len()).as_str());

        // Debug print all accounts
        for (i, acc) in self.accounts.iter().enumerate() {
            msg!(format!("  accounts[{}] = {:?}", i, acc.key()).as_str());
        }

        &self.accounts[self.system_accounts_len()..]
    }
}
