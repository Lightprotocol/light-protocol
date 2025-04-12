use crate::{
    error::Result, find_cpi_signer_macro, AccountInfo, AccountMeta, Pubkey, CPI_AUTHORITY_PDA_SEED,
    PROGRAM_ID_ACCOUNT_COMPRESSION, PROGRAM_ID_LIGHT_SYSTEM, PROGRAM_ID_NOOP,
};

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

        self.accounts[2..].iter().enumerate().for_each(|(i, acc)| {
            if i < self.system_accounts_len() - 2 {
                account_metas.push(AccountMeta {
                    pubkey: *acc.key,
                    is_signer: false,
                    is_writable: false,
                });
            }
        });

        if !self.config.sol_pool_pda {
            account_metas.insert(
                CompressionCpiAccountIndex::SolPoolPda as usize,
                AccountMeta {
                    pubkey: *self.light_system_program().key,
                    is_signer: false,
                    is_writable: false,
                },
            );
        }

        if !self.config.sol_compression_recipient {
            account_metas.insert(
                CompressionCpiAccountIndex::DecompressionRecipent as usize,
                AccountMeta {
                    pubkey: *self.light_system_program().key,
                    is_signer: false,
                    is_writable: false,
                },
            );
        }
        if !self.config.cpi_context {
            account_metas.insert(
                CompressionCpiAccountIndex::CpiContext as usize,
                AccountMeta {
                    pubkey: *self.light_system_program().key,
                    is_signer: false,
                    is_writable: false,
                },
            );
        }
        self.accounts[self.system_accounts_len()..]
            .iter()
            .for_each(|acc| {
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

// Offchain
#[derive(Debug, Default, Copy, Clone)]
pub struct SystemAccountMetaConfig {
    pub self_program: Pubkey,
    pub cpi_context: Option<Pubkey>,
    pub sol_compression_recipient: Option<Pubkey>,
    pub sol_pool_pda: Option<Pubkey>,
}

impl SystemAccountMetaConfig {
    pub fn new(self_program: Pubkey) -> Self {
        Self {
            self_program,
            cpi_context: None,
            sol_compression_recipient: None,
            sol_pool_pda: None,
        }
    }

    pub fn new_with_cpi_context(self_program: Pubkey, cpi_context: Pubkey) -> Self {
        Self {
            self_program,
            cpi_context: Some(cpi_context),
            sol_compression_recipient: None,
            sol_pool_pda: None,
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct CompressionCpiAccountsConfig {
    pub self_program: Pubkey,
    // TODO: move to instructiond data
    pub cpi_context: bool,
    pub sol_compression_recipient: bool,
    pub sol_pool_pda: bool,
}

impl CompressionCpiAccountsConfig {
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

#[derive(Debug, Copy, Clone)]
pub struct SystemAccountPubkeys {
    pub light_sytem_program: Pubkey,
    pub system_program: Pubkey,
    pub account_compression_program: Pubkey,
    pub account_compression_authority: Pubkey,
    pub registered_program_pda: Pubkey,
    pub noop_program: Pubkey,
    pub sol_pool_pda: Pubkey,
}

impl Default for SystemAccountPubkeys {
    fn default() -> Self {
        Self {
            light_sytem_program: PROGRAM_ID_LIGHT_SYSTEM,
            system_program: Pubkey::default(),
            account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
            account_compression_authority: Pubkey::find_program_address(
                &[CPI_AUTHORITY_PDA_SEED],
                &PROGRAM_ID_LIGHT_SYSTEM,
            )
            .0,
            registered_program_pda: Pubkey::find_program_address(
                &[PROGRAM_ID_LIGHT_SYSTEM.to_bytes().as_slice()],
                &PROGRAM_ID_ACCOUNT_COMPRESSION,
            )
            .0,
            noop_program: PROGRAM_ID_NOOP,
            // TODO: add correct pubkey
            sol_pool_pda: Pubkey::default(),
        }
    }
}

pub fn get_light_system_account_metas(config: SystemAccountMetaConfig) -> Vec<AccountMeta> {
    let cpi_signer = find_cpi_signer_macro!(&config.self_program).0;
    let default_pubkeys = SystemAccountPubkeys::default();
    let mut vec = vec![
        AccountMeta::new_readonly(default_pubkeys.light_sytem_program, false),
        AccountMeta::new_readonly(cpi_signer, false),
        AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
        AccountMeta::new_readonly(default_pubkeys.noop_program, false),
        AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
        AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
        AccountMeta::new_readonly(config.self_program, false),
        // sol_pool_pda,
        // decompression_recipient,
        // AccountMeta::new_readonly(default_pubkeys.system_program, false),
        // cpi_context,
    ];
    if let Some(pubkey) = config.sol_pool_pda {
        vec.push(AccountMeta {
            pubkey,
            is_signer: false,
            is_writable: true,
        });
    }
    if let Some(pubkey) = config.sol_compression_recipient {
        vec.push(AccountMeta {
            pubkey,
            is_signer: false,
            is_writable: true,
        });
    }
    vec.push(AccountMeta::new_readonly(
        default_pubkeys.system_program,
        false,
    ));
    if let Some(pubkey) = config.cpi_context {
        vec.push(AccountMeta {
            pubkey,
            is_signer: false,
            is_writable: true,
        });
    }
    vec
}
