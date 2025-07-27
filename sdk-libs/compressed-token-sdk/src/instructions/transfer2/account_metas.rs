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
    pub packed_accounts: Option<Vec<AccountMeta>>,
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
        // noop_program
        AccountMeta::new_readonly(default_pubkeys.noop_program, false),
        // account_compression_authority
        AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
        // account_compression_program
        AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
        // invoking_program (self program)
        AccountMeta::new_readonly(default_pubkeys.self_program, false),
    ]);

    // Optional sol pool accounts
    if config.with_sol_pool {
        if let Some(sol_pool_pda) = config.sol_pool_pda {
            metas.push(AccountMeta::new(sol_pool_pda, false));
        }
        if let Some(sol_decompression_recipient) = config.sol_decompression_recipient {
            metas.push(AccountMeta::new(sol_decompression_recipient, false));
        }
    }

    // system_program (always present)
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.system_program,
        false,
    ));
    if let Some(cpi_context) = config.cpi_context {
        metas.push(AccountMeta::new(cpi_context, false));
    }
    if let Some(packed_accounts) = config.packed_accounts.as_ref() {
        for account in packed_accounts {
            metas.push(account.clone());
        }
    }

    metas
}
