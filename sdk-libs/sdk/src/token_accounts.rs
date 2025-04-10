use crate::{
    error::{LightSdkError, Result},
    find_cpi_signer_macro, AccountInfo, AccountMeta, Pubkey, CPI_AUTHORITY_PDA_SEED,
    PROGRAM_ID_ACCOUNT_COMPRESSION, PROGRAM_ID_LIGHT_SYSTEM, PROGRAM_ID_LIGHT_TOKEN,
    PROGRAM_ID_NOOP,
};

#[repr(usize)]
pub enum CompressedTokenCpiAccountIndex {
    CompressedTokenProgram,
    Authority,
    TokenProgramCpiAuthorityPda,
    LightSystemProgram,
    RegisteredProgramPda,
    NoopProgram,
    AccountCompressionAuthority,
    AccountCompressionProgram,
    TokenPoolPda,
    DecompressionRecipent,
    SplTokenProgram,
    SystemProgram,
}

/// Index in combination with system program accounts.
/// Does not contain any accounts the light system program already needs.
/// Duplicate accounts are:
/// 1. Authority
/// 2. LightSystemProgram,
/// 3. RegisteredProgramPda,
/// 4. NoopProgram,
/// 5. AccountCompressionAuthority,
/// 6. AccountCompressionProgram,
/// 7. SystemProgram
#[repr(usize)]
pub enum CompressedTokenCpiAccountIndexSmall {
    CompressedTokenProgram,
    Authority,
    TokenProgramCpiAuthorityPda,
    TokenPoolPda,
    DecompressionRecipent,
    SplTokenProgram,
}

const TOKEN_ACCOUNTS_LEN: usize = 12;
const TOKEN_ACCOUNTS_LEN_SMALL: usize = 6;

// TODO: add unit tests
pub struct CompressionCpiAccounts<'c, 'info> {
    fee_payer: &'c AccountInfo<'info>,
    system_accounts: &'c [AccountInfo<'info>],
    token_accounts: &'c [AccountInfo<'info>],
    config: CompressionCpiAccountsConfig,
}

impl<'c, 'info> CompressionCpiAccounts<'c, 'info> {
    // TODO: consider to pass num of trees to split remaining accounts
    pub fn new(
        fee_payer: &'c AccountInfo<'info>,
        token_accounts: &'c [AccountInfo<'info>],
        program_id: Pubkey,
    ) -> Result<Self> {
        if *token_accounts[0].key != PROGRAM_ID_LIGHT_TOKEN {
            return Err(LightSdkError::InvalidLightSystemProgramAccountInfo);
        }
        if token_accounts.len() < TOKEN_ACCOUNTS_LEN_SMALL + 1 {
            // msg!("accounts len {}", accounts.len());
            return Err(LightSdkError::FewerAccountsThanSystemAccounts);
        }
        Ok(Self {
            fee_payer,
            system_accounts: &[],
            token_accounts,
            config: CompressionCpiAccountsConfig {
                self_program: program_id,
                ..Default::default()
            },
        })
    }

    /// Expectes token accounts in order:
    /// 1. CompressedTokenProgram
    /// 2. Authority
    /// 3. TokenProgramCpiAuthorityPda
    /// 4. TokenPoolPda
    /// 5. DecompressionRecipent
    /// 6. SplTokenProgram
    ///
    /// Expects these system accounts in this order:
    /// 1. Cpi Authority
    /// 2. LightSystemProgram,
    /// 3. RegisteredProgramPda,
    /// 4. NoopProgram,
    /// 5. AccountCompressionAuthority,
    /// 6. AccountCompressionProgram,
    /// 7. SystemProgram
    pub fn new_with_separate_system_accounts(
        fee_payer: &'c AccountInfo<'info>,
        system_accounts: &'c [AccountInfo<'info>],
        token_accounts: &'c [AccountInfo<'info>],
        program_id: Pubkey,
    ) -> Result<Self> {
        if *token_accounts[0].key != PROGRAM_ID_LIGHT_TOKEN {
            return Err(LightSdkError::InvalidLightSystemProgramAccountInfo);
        }
        if *system_accounts[0].key != find_cpi_signer_macro!(&program_id).0 {
            return Err(LightSdkError::InvalidLightSystemProgramAccountInfo);
        }

        Ok(Self {
            fee_payer,
            system_accounts,
            token_accounts,
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
            .get(CompressedTokenCpiAccountIndex::LightSystemProgram as usize)
            .unwrap()
    }

    pub fn authority(&self) -> &'c AccountInfo<'info> {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(CompressedTokenCpiAccountIndex::Authority as usize)
            .unwrap()
    }

    pub fn invoking_program(&self) -> &'c AccountInfo<'info> {
        // PANICS: We are sure about the bounds of the slice.
        self.accounts
            .get(CompressedTokenCpiAccountIndex::InvokingProgram as usize)
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
                CompressedTokenCpiAccountIndex::SolPoolPda as usize,
                AccountMeta {
                    pubkey: *self.light_system_program().key,
                    is_signer: false,
                    is_writable: false,
                },
            );
        }

        if !self.config.sol_compression_recipient {
            account_metas.insert(
                CompressedTokenCpiAccountIndex::DecompressionRecipent as usize,
                AccountMeta {
                    pubkey: *self.light_system_program().key,
                    is_signer: false,
                    is_writable: false,
                },
            );
        }
        // if !self.config.cpi_context {
        //     account_metas.insert(
        //         CompressedTokenCpiAccountIndex::CpiContext as usize,
        //         AccountMeta {
        //             pubkey: *self.light_system_program().key,
        //             is_signer: false,
        //             is_writable: false,
        //         },
        //     );
        // }
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
}

// Offchain
#[derive(Debug, Default, Clone)]
pub struct TokenAccountMetaConfig {
    pub token_pool_pdas: Option<Vec<Pubkey>>,
    pub de_compression_recipient: Option<Pubkey>,
    pub cpi_context: Option<Pubkey>,
}

// TODO: add compress, decompress
impl TokenAccountMetaConfig {
    pub fn new() -> Self {
        Self {
            cpi_context: None,
            token_pool_pdas: None,
            de_compression_recipient: None,
        }
    }

    pub fn new_with_cpi_context(cpi_context: Pubkey) -> Self {
        Self {
            cpi_context: Some(cpi_context),
            token_pool_pdas: None,
            de_compression_recipient: None,
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct CompressionCpiAccountsConfig {
    pub self_program: Pubkey,
    // TODO: move to instruction data
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
pub struct TokenAccountPubkeys {
    pub light_sytem_program: Pubkey,
    pub system_program: Pubkey,
    pub account_compression_program: Pubkey,
    pub account_compression_authority: Pubkey,
    pub registered_program_pda: Pubkey,
    pub noop_program: Pubkey,
    pub cpi_authority_pda: Pubkey,
    pub compressed_token_program: Pubkey,
}

impl Default for TokenAccountPubkeys {
    fn default() -> Self {
        Self {
            light_sytem_program: PROGRAM_ID_LIGHT_SYSTEM,
            cpi_authority_pda: find_cpi_signer_macro!(&PROGRAM_ID_LIGHT_TOKEN).0,
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
            compressed_token_program: PROGRAM_ID_LIGHT_TOKEN,
        }
    }
}

pub fn get_compressed_token_account_metas(
    config: TokenAccountMetaConfig,
) -> Result<Vec<AccountMeta>> {
    let default_pubkeys = TokenAccountPubkeys::default();
    let mut vec = vec![
        AccountMeta::new_readonly(default_pubkeys.cpi_authority_pda, false),
        AccountMeta::new_readonly(default_pubkeys.light_sytem_program, false),
        AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
        AccountMeta::new_readonly(default_pubkeys.noop_program, false),
        AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
        AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
        AccountMeta::new_readonly(default_pubkeys.compressed_token_program, false), // self_program (token program)
                                                                                    // sol_pool_pda,
                                                                                    // decompression_recipient,
                                                                                    // AccountMeta::new_readonly(default_pubkeys.system_program, false),
                                                                                    // cpi_context,
    ];

    if let Some(pubkey) = config.token_pool_pdas.as_ref() {
        vec.push(AccountMeta {
            pubkey: pubkey[0],
            is_signer: false,
            is_writable: true,
        });
        if pubkey.len() > 1 {
            println!("Multiple token pool pdas are currently unsupportend in PackedAccounts. You can use multiple token pools manually.");
            return Err(LightSdkError::Unsupported);
        }
    }
    if let Some(pubkey) = config.de_compression_recipient {
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
    // Additional token pool pubkeys are transmitted in the first remaining accounts.
    // if let Some(pubkeys) = config.token_pool_pdas.as_ref() {
    //     if pubkeys.len() > 1 {
    //         for pubkey in pubkeys.iter() {
    //             vec.push(AccountMeta {
    //                 pubkey: *pubkey,
    //                 is_signer: false,
    //                 is_writable: true,
    //             });
    //         }
    //     }
    // }
    vec
}
