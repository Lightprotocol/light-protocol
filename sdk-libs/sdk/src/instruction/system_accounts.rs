use light_sdk_types::constants::{
    ACCOUNT_COMPRESSION_PROGRAM_ID, CPI_AUTHORITY_PDA_SEED, LIGHT_SYSTEM_PROGRAM_ID,
    NOOP_PROGRAM_ID,
};

use crate::{find_cpi_signer_macro, AccountMeta, Pubkey};

/// Configuration for Light system program accounts when building instructions.
///
/// This struct specifies which system accounts to include when using
/// [`PackedAccounts::add_system_accounts()`](crate::instruction::PackedAccounts::add_system_accounts)
/// or `PackedAccounts::add_system_accounts_v2()`.
///
/// # Required Fields
///
/// - **`self_program`**: Your program's ID (the one calling the Light system program).
///   Used to derive the CPI signer PDA.
///
/// # Optional Fields
///
/// - **`cpi_context`**: CPI context account for batched operations (v2 only).
///   Required when using CPI context for multi-step compressed account operations.
///
/// - **`sol_compression_recipient`**: Account to receive decompressed SOL.
///   Required when decompressing SOL from compressed accounts.
///
/// - **`sol_pool_pda`**: SOL pool PDA for SOL compression/decompression.
///   Required when compressing or decompressing SOL.
///
/// # Examples
///
/// Basic usage (no SOL operations):
///
/// ```rust
/// # use light_sdk::instruction::SystemAccountMetaConfig;
/// # use solana_pubkey::Pubkey;
/// let program_id = Pubkey::new_unique();
/// let config = SystemAccountMetaConfig::new(program_id);
/// ```
///
/// With CPI context (v2 batched operations):
///
#[cfg_attr(not(feature = "cpi-context"), doc = "```ignore")]
#[cfg_attr(feature = "cpi-context", doc = "```rust")]
/// # use light_sdk::instruction::SystemAccountMetaConfig;
/// # use solana_pubkey::Pubkey;
/// let program_id = Pubkey::new_unique();
/// let cpi_context_account = Pubkey::new_unique();
/// let config = SystemAccountMetaConfig::new_with_cpi_context(program_id, cpi_context_account);
/// ```
///
/// With SOL compression:
///
/// ```rust
/// # use light_sdk::instruction::SystemAccountMetaConfig;
/// # use solana_pubkey::Pubkey;
/// let program_id = Pubkey::new_unique();
/// let sol_pool_pda = Pubkey::new_unique();
/// let recipient = Pubkey::new_unique();
///
/// let mut config = SystemAccountMetaConfig::new(program_id);
/// config.sol_pool_pda = Some(sol_pool_pda);
/// config.sol_compression_recipient = Some(recipient);
/// ```
#[derive(Debug, Default, Copy, Clone)]
#[non_exhaustive]
pub struct SystemAccountMetaConfig {
    /// Your program's ID (optional). Used to derive the CPI signer PDA.
    /// When None, the CPI signer is not included (for registry CPI flow).
    pub self_program: Option<Pubkey>,
    /// Optional CPI context account for batched operations (v2 only).
    #[cfg(feature = "cpi-context")]
    pub cpi_context: Option<Pubkey>,
    /// Optional account to receive decompressed SOL.
    pub sol_compression_recipient: Option<Pubkey>,
    /// Optional SOL pool PDA for SOL compression/decompression.
    pub sol_pool_pda: Option<Pubkey>,
}

impl SystemAccountMetaConfig {
    /// Creates a basic configuration with only the program ID.
    ///
    /// Use this for simple compressed account operations without SOL compression
    /// or CPI context.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use light_sdk::instruction::SystemAccountMetaConfig;
    /// # use solana_pubkey::Pubkey;
    /// let program_id = Pubkey::new_unique();
    /// let config = SystemAccountMetaConfig::new(program_id);
    /// ```
    pub fn new(self_program: Pubkey) -> Self {
        Self {
            self_program: Some(self_program),
            #[cfg(feature = "cpi-context")]
            cpi_context: None,
            sol_compression_recipient: None,
            sol_pool_pda: None,
        }
    }

    /// Creates a configuration with CPI context for batched operations (v2 only).
    ///
    /// Use this when you need to batch multiple compressed account operations
    /// using a CPI context account.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use light_sdk::instruction::SystemAccountMetaConfig;
    /// # use solana_pubkey::Pubkey;
    /// let program_id = Pubkey::new_unique();
    /// let cpi_context_account = Pubkey::new_unique();
    /// let config = SystemAccountMetaConfig::new_with_cpi_context(
    ///     program_id,
    ///     cpi_context_account
    /// );
    /// ```
    #[cfg(feature = "cpi-context")]
    pub fn new_with_cpi_context(self_program: Pubkey, cpi_context: Pubkey) -> Self {
        Self {
            self_program: Some(self_program),
            cpi_context: Some(cpi_context),
            sol_compression_recipient: None,
            sol_pool_pda: None,
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
            light_sytem_program: Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID),
            system_program: Pubkey::default(),
            account_compression_program: Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID),
            account_compression_authority: Pubkey::find_program_address(
                &[CPI_AUTHORITY_PDA_SEED],
                &Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID),
            )
            .0,
            registered_program_pda: Pubkey::find_program_address(
                &[LIGHT_SYSTEM_PROGRAM_ID.as_slice()],
                &Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID),
            )
            .0,
            noop_program: Pubkey::from(NOOP_PROGRAM_ID),
            sol_pool_pda: Pubkey::find_program_address(
                &[b"sol_pool_pda"],
                &Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID),
            )
            .0,
        }
    }
}

/// InvokeSystemCpi v1.
pub fn get_light_system_account_metas(config: SystemAccountMetaConfig) -> Vec<AccountMeta> {
    let default_pubkeys = SystemAccountPubkeys::default();

    let mut vec = if let Some(self_program) = &config.self_program {
        let cpi_signer = find_cpi_signer_macro!(self_program).0;
        vec![
            AccountMeta::new_readonly(default_pubkeys.light_sytem_program, false),
            AccountMeta::new_readonly(cpi_signer, false),
            AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
            AccountMeta::new_readonly(default_pubkeys.noop_program, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
            AccountMeta::new_readonly(*self_program, false),
        ]
    } else {
        vec![
            AccountMeta::new_readonly(default_pubkeys.light_sytem_program, false),
            AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
            AccountMeta::new_readonly(default_pubkeys.noop_program, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
        ]
    };

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
    #[cfg(feature = "cpi-context")]
    if let Some(pubkey) = config.cpi_context {
        vec.push(AccountMeta {
            pubkey,
            is_signer: false,
            is_writable: true,
        });
    }
    vec
}

/// Can be used in client to add system account metas.
///
/// We need the program id account infos in the outer instruction.
/// Account Metas:
/// 1. Light System Program
/// 2. Account Compression Program
/// 3. System Program
/// 4. CPI Signer
/// 5. Registered Program PDA
/// 6. Account Compression Authority
#[cfg(feature = "v2")]
pub fn get_light_system_account_metas_v2(config: SystemAccountMetaConfig) -> Vec<AccountMeta> {
    let default_pubkeys = SystemAccountPubkeys::default();

    let mut vec = if let Some(self_program) = &config.self_program {
        let cpi_signer = find_cpi_signer_macro!(self_program).0;
        vec![
            AccountMeta::new_readonly(default_pubkeys.light_sytem_program, false),
            AccountMeta::new_readonly(cpi_signer, false), // authority (cpi_signer)
            AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
            AccountMeta::new_readonly(default_pubkeys.system_program, false),
        ]
    } else {
        vec![
            AccountMeta::new_readonly(default_pubkeys.light_sytem_program, false),
            AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
            AccountMeta::new_readonly(default_pubkeys.system_program, false),
        ]
    };

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
    #[cfg(feature = "cpi-context")]
    if let Some(pubkey) = config.cpi_context {
        vec.push(AccountMeta {
            pubkey,
            is_signer: false,
            is_writable: true,
        });
    }
    vec
}
