use light_compressed_token_types::CPI_AUTHORITY_PDA;
use light_sdk::constants::LIGHT_SYSTEM_PROGRAM_ID;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::instructions::CTokenDefaultAccounts;

/// Account metadata configuration for compressed token multi-transfer instructions
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Transfer2AccountsMetaConfig {
    pub fee_payer: Option<Pubkey>,
    pub sol_pool_pda: Option<Pubkey>,
    pub sol_decompression_recipient: Option<Pubkey>,
    pub cpi_context: Option<Pubkey>,
    pub with_sol_pool: bool,
    pub decompressed_accounts_only: bool,
    pub packed_accounts: Option<Vec<AccountMeta>>, // TODO: check whether this can ever be None
}

impl Transfer2AccountsMetaConfig {
    pub fn new(fee_payer: Pubkey, packed_accounts: Vec<AccountMeta>) -> Self {
        Self {
            fee_payer: Some(fee_payer),
            decompressed_accounts_only: false,
            sol_pool_pda: None,
            sol_decompression_recipient: None,
            cpi_context: None,
            with_sol_pool: false,
            packed_accounts: Some(packed_accounts),
        }
    }

    pub fn new_decompressed_accounts_only(
        fee_payer: Pubkey,
        packed_accounts: Vec<AccountMeta>,
    ) -> Self {
        Self {
            fee_payer: Some(fee_payer),
            sol_pool_pda: None,
            sol_decompression_recipient: None,
            cpi_context: None,
            with_sol_pool: false,
            decompressed_accounts_only: true,
            packed_accounts: Some(packed_accounts),
        }
    }
}

/// Get the standard account metas for a compressed token multi-transfer instruction
pub fn get_transfer2_instruction_account_metas(
    config: Transfer2AccountsMetaConfig,
) -> Vec<AccountMeta> {
    let default_pubkeys = CTokenDefaultAccounts::default();
    let packed_accounts_len = if let Some(packed_accounts) = config.packed_accounts.as_ref() {
        packed_accounts.len()
    } else {
        0
    };

    // Build the account metas following the order expected by Transfer2ValidatedAccounts
    let mut metas = Vec::with_capacity(10 + packed_accounts_len);
    if !config.decompressed_accounts_only {
        metas.push(AccountMeta::new_readonly(
            Pubkey::new_from_array(LIGHT_SYSTEM_PROGRAM_ID),
            false,
        ));
        // Add fee payer and authority if provided (for direct invoke)
        if let Some(fee_payer) = config.fee_payer {
            metas.push(AccountMeta::new(fee_payer, true));
        }

        // Core system accounts (always present)
        metas.extend([
            AccountMeta::new_readonly(Pubkey::new_from_array(CPI_AUTHORITY_PDA), false),
            // registered_program_pda
            AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
            // account_compression_authority
            AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
            // account_compression_program
            AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
        ]);

        // system_program (always present)
        metas.push(AccountMeta::new_readonly(
            default_pubkeys.system_program,
            false,
        ));

        // Optional sol pool accounts
        if config.with_sol_pool {
            if let Some(sol_pool_pda) = config.sol_pool_pda {
                metas.push(AccountMeta::new(sol_pool_pda, false));
            }
            if let Some(sol_decompression_recipient) = config.sol_decompression_recipient {
                metas.push(AccountMeta::new(sol_decompression_recipient, false));
            }
        }
        if let Some(cpi_context) = config.cpi_context {
            metas.push(AccountMeta::new(cpi_context, false));
        }
    } else if config.cpi_context.is_some() || config.with_sol_pool {
        // TODO: replace with error
        unimplemented!(
            "config.cpi_context.is_some() {}, config.with_sol_pool {} must both be false",
            config.cpi_context.is_some(),
            config.with_sol_pool
        );
    } else {
        // For decompressed accounts only, add compressions_only_cpi_authority_pda first
        metas.push(AccountMeta::new_readonly(
            Pubkey::new_from_array(CPI_AUTHORITY_PDA),
            false,
        ));
        // Then add compressions_only_fee_payer if provided
        if let Some(fee_payer) = config.fee_payer {
            metas.push(AccountMeta::new(fee_payer, true));
        }
    }
    if let Some(packed_accounts) = config.packed_accounts.as_ref() {
        for account in packed_accounts {
            metas.push(account.clone());
        }
    }

    metas
}
