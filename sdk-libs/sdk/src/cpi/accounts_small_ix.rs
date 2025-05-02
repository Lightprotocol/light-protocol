#![cfg(feature = "v2")]
use crate::{
    cpi::accounts::CompressionCpiAccountsConfig, error::Result, msg, AccountInfo, AccountMeta,
    Pubkey,
};

#[repr(usize)]
pub enum CompressionCpiAccountIndexSmall {
    LightSystemProgram,        // Only exposed to outer instruction
    AccountCompressionProgram, // Only exposed to outer instruction
    SystemProgram,             // Only exposed to outer instruction
    Authority, // Cpi authority of the custom program, used to invoke the light system program.
    RegisteredProgramPda,
    AccountCompressionAuthority,
    SolPoolPda,            // Optional
    DecompressionRecipent, // Optional
    CpiContext,            // Optional
}

pub const PROGRAM_ACCOUNTS_LEN: usize = 3;
// 6 + 3 program ids, fee payer is extra.
pub const SMALL_SYSTEM_ACCOUNTS_LEN: usize = 9;

// TODO: add unit tests
pub struct CompressionCpiAccounts<'c, 'info> {
    fee_payer: &'c AccountInfo<'info>,
    accounts: &'c [AccountInfo<'info>],
    config: CompressionCpiAccountsConfig,
}

impl<'c, 'info> CompressionCpiAccounts<'c, 'info> {
    // TODO: consider to pass num of trees to split remaining accounts
    pub fn new(
        fee_payer: &'c AccountInfo<'info>,
        accounts: &'c [AccountInfo<'info>],
        program_id: Pubkey,
    ) -> Result<Self> {
        // if accounts.len() < SYSTEM_ACCOUNTS_LEN {
        //     msg!("accounts len {}", accounts.len());
        //     return Err(LightSdkError::FewerAccountsThanSystemAccounts);
        // }
        Ok(Self {
            fee_payer,
            accounts,
            config: CompressionCpiAccountsConfig {
                self_program: program_id,
                ..Default::default()
            },
        })
    }

    pub fn new_with_config(
        fee_payer: &'c AccountInfo<'info>,
        accounts: &'c [AccountInfo<'info>],
        config: CompressionCpiAccountsConfig,
    ) -> Result<Self> {
        msg!("config {:?}", config);
        // if accounts.len() < SYSTEM_ACCOUNTS_LEN {
        //     msg!("accounts len {}", accounts.len());
        //     return Err(LightSdkError::FewerAccountsThanSystemAccounts);
        // }
        Ok(Self {
            fee_payer,
            accounts,
            config,
        })
    }

    pub fn fee_payer(&self) -> &'c AccountInfo<'info> {
        self.fee_payer
    }

    pub fn authority(&self) -> &'c AccountInfo<'info> {
        self.accounts
            .get(CompressionCpiAccountIndexSmall::Authority as usize)
            .unwrap()
    }

    pub fn self_program_id(&self) -> &Pubkey {
        &self.config.self_program
    }

    /// Account infos for cpi to light system program.
    pub fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        // TODO: do a version with a const array instead of vector.
        let mut account_infos = Vec::with_capacity(1 + self.accounts.len() - PROGRAM_ACCOUNTS_LEN);
        account_infos.push(self.fee_payer.clone());
        self.accounts[PROGRAM_ACCOUNTS_LEN..]
            .iter()
            .for_each(|acc| account_infos.push(acc.clone()));
        account_infos
    }

    /// Account metas for cpi to light system program.
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        // TODO: do a version with a const array instead of vector.
        let mut account_metas = Vec::with_capacity(1 + self.accounts.len() - PROGRAM_ACCOUNTS_LEN);

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
            pubkey: *self.accounts[CompressionCpiAccountIndexSmall::RegisteredProgramPda as usize]
                .key,
            is_signer: false,
            is_writable: false,
        });
        account_metas.push(AccountMeta {
            pubkey: *self.accounts
                [CompressionCpiAccountIndexSmall::AccountCompressionAuthority as usize]
                .key,
            is_signer: false,
            is_writable: false,
        });

        let mut index = CompressionCpiAccountIndexSmall::SolPoolPda as usize;
        if self.config.sol_pool_pda {
            account_metas.push(AccountMeta {
                pubkey: *self.accounts[index].key,
                is_signer: false,
                is_writable: true,
            });
            index += 1;
        }

        if self.config.sol_compression_recipient {
            account_metas.push(AccountMeta {
                pubkey: *self.accounts[index].key,
                is_signer: false,
                is_writable: true,
            });
            index += 1;
        }

        if self.config.cpi_context {
            account_metas.push(AccountMeta {
                pubkey: *self.accounts[index].key,
                is_signer: false,
                is_writable: true,
            });
            index += 1;
        }
        assert_eq!(self.system_accounts_end_offset(), index);

        self.accounts[index..].iter().for_each(|acc| {
            account_metas.push(AccountMeta {
                pubkey: *acc.key,
                is_signer: false,
                is_writable: true,
            });
        });
        account_metas
    }

    pub fn config(&self) -> &CompressionCpiAccountsConfig {
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

    pub fn account_infos(&self) -> &'c [AccountInfo<'info>] {
        self.accounts
    }

    pub fn tree_accounts(&self) -> &'c [AccountInfo<'info>] {
        &self.accounts[self.system_accounts_end_offset()..]
    }
}
