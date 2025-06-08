#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use light_sdk_types::CpiSigner;

use crate::{
    error::{LightSdkError, Result},
    AccountInfo, AccountMeta, Pubkey,
};

#[derive(Debug, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CpiAccountsConfig {
    pub cpi_context: bool,
    pub sol_compression_recipient: bool,
    pub sol_pool_pda: bool,
    pub cpi_signer: CpiSigner,
}

impl CpiAccountsConfig {
    pub fn new(cpi_signer: CpiSigner) -> Self {
        Self {
            cpi_context: false,
            sol_compression_recipient: false,
            sol_pool_pda: false,
            cpi_signer,
        }
    }

    pub fn new_with_cpi_context(cpi_signer: CpiSigner) -> Self {
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
    DecompressionRecipent,
    SystemProgram,
    CpiContext,
}

pub const SYSTEM_ACCOUNTS_LEN: usize = 11;

// TODO: add unit tests
pub struct CpiAccounts<'c, 'info> {
    fee_payer: &'c AccountInfo<'info>,
    accounts: &'c [AccountInfo<'info>],
    config: CpiAccountsConfig,
}

impl<'c, 'info> CpiAccounts<'c, 'info> {
    pub fn new(
        fee_payer: &'c AccountInfo<'info>,
        accounts: &'c [AccountInfo<'info>],
        cpi_signer: CpiSigner,
    ) -> Result<Self> {
        let new = Self {
            fee_payer,
            accounts,
            config: CpiAccountsConfig::new(cpi_signer),
        };
        if accounts.len() < new.system_accounts_len() {
            crate::msg!("accounts len {}", accounts.len());
            return Err(LightSdkError::FewerAccountsThanSystemAccounts);
        }
        Ok(new)
    }

    pub fn new_with_config(
        fee_payer: &'c AccountInfo<'info>,
        accounts: &'c [AccountInfo<'info>],
        config: CpiAccountsConfig,
    ) -> Result<Self> {
        let new = Self {
            fee_payer,
            accounts,
            config,
        };
        if accounts.len() < new.system_accounts_len() {
            crate::msg!("accounts len {}", accounts.len());
            return Err(LightSdkError::FewerAccountsThanSystemAccounts);
        }
        Ok(new)
    }

    pub fn fee_payer(&self) -> &'c AccountInfo<'info> {
        self.fee_payer
    }

    pub fn light_system_program(&self) -> &'c AccountInfo<'info> {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(CompressionCpiAccountIndex::LightSystemProgram as usize)
            .unwrap()
    }

    pub fn authority(&self) -> &'c AccountInfo<'info> {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(CompressionCpiAccountIndex::Authority as usize)
            .unwrap()
    }

    pub fn invoking_program(&self) -> &'c AccountInfo<'info> {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(CompressionCpiAccountIndex::InvokingProgram as usize)
            .unwrap()
    }

    pub fn self_program_id(&self) -> Pubkey {
        Pubkey::new_from_array(self.config.cpi_signer.program_id)
    }

    pub fn bump(&self) -> u8 {
        self.config.cpi_signer.bump
    }

    pub fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        let mut account_infos = Vec::with_capacity(1 + SYSTEM_ACCOUNTS_LEN);
        account_infos.push(self.fee_payer.clone());
        self.accounts[1..]
            .iter()
            .for_each(|acc| account_infos.push(acc.clone()));
        account_infos
    }

    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        let mut account_metas = Vec::with_capacity(1 + SYSTEM_ACCOUNTS_LEN);
        account_metas.push(AccountMeta {
            pubkey: *self.fee_payer.key,
            is_signer: true,
            is_writable: true,
        });
        account_metas.push(AccountMeta {
            pubkey: *self.authority().key,
            is_signer: true,
            is_writable: false,
        });

        account_metas.push(AccountMeta {
            pubkey: *self.accounts[CompressionCpiAccountIndex::RegisteredProgramPda as usize].key,
            is_signer: false,
            is_writable: false,
        });
        account_metas.push(AccountMeta {
            pubkey: *self.accounts[CompressionCpiAccountIndex::NoopProgram as usize].key,
            is_signer: false,
            is_writable: false,
        });
        account_metas.push(AccountMeta {
            pubkey: *self.accounts
                [CompressionCpiAccountIndex::AccountCompressionAuthority as usize]
                .key,
            is_signer: false,
            is_writable: false,
        });
        account_metas.push(AccountMeta {
            pubkey: *self.accounts[CompressionCpiAccountIndex::AccountCompressionProgram as usize]
                .key,
            is_signer: false,
            is_writable: false,
        });
        account_metas.push(AccountMeta {
            pubkey: *self.accounts[CompressionCpiAccountIndex::InvokingProgram as usize].key,
            is_signer: false,
            is_writable: false,
        });
        let mut current_index = 7;
        if !self.config.sol_pool_pda {
            account_metas.push(AccountMeta {
                pubkey: *self.light_system_program().key,
                is_signer: false,
                is_writable: false,
            });
        } else {
            account_metas.push(AccountMeta {
                pubkey: *self.accounts[current_index].key,
                is_signer: false,
                is_writable: true,
            });
            current_index += 1;
        }

        if !self.config.sol_compression_recipient {
            account_metas.push(AccountMeta {
                pubkey: *self.light_system_program().key,
                is_signer: false,
                is_writable: false,
            });
        } else {
            account_metas.push(AccountMeta {
                pubkey: *self.accounts[current_index].key,
                is_signer: false,
                is_writable: true,
            });
            current_index += 1;
        }
        // System program
        account_metas.push(AccountMeta {
            pubkey: Pubkey::default(),
            is_signer: false,
            is_writable: false,
        });
        current_index += 1;

        if !self.config.cpi_context {
            account_metas.push(AccountMeta {
                pubkey: *self.light_system_program().key,
                is_signer: false,
                is_writable: false,
            });
        } else {
            account_metas.push(AccountMeta {
                pubkey: *self.accounts[current_index].key,
                is_signer: false,
                is_writable: true,
            });
            current_index += 1;
        }
        //self.system_accounts_len()
        self.accounts[current_index..].iter().for_each(|acc| {
            account_metas.push(AccountMeta {
                pubkey: *acc.key,
                is_signer: false,
                is_writable: true,
            });
        });
        account_metas
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

    pub fn account_infos(&self) -> &'c [AccountInfo<'info>] {
        self.accounts
    }

    pub fn tree_accounts(&self) -> &'c [AccountInfo<'info>] {
        &self.accounts[self.system_accounts_len()..]
    }
}
