use light_compressed_token_types::{
    constants::{
        ACCOUNT_COMPRESSION_PROGRAM_ID, CPI_AUTHORITY_PDA, LIGHT_SYSTEM_PROGRAM_ID,
        NOOP_PROGRAM_ID, PROGRAM_ID as COMPRESSED_TOKEN_PROGRAM_ID,
    },
    cpi_accounts::CpiAccounts,
};
use solana_account_info::AccountInfo;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::error::{Result, TokenSdkError};

/// Convert CpiAccounts to AccountMeta vector for compressed token instructions
/// This follows the same pattern as light-sdk's to_account_metas function
pub fn to_compressed_token_account_metas(
    cpi_accounts: &CpiAccounts<'_, AccountInfo<'_>>,
) -> Result<Vec<AccountMeta>> {
    let mut account_metas = Vec::with_capacity(12); // Base compressed token accounts

    // fee_payer (mut, signer)
    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts.fee_payer().key,
        is_signer: true,
        is_writable: true,
    });

    // authority (signer)
    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts
            .authority()
            .map_err(|_| TokenSdkError::CpiError("Missing authority".to_string()))?
            .key,
        is_signer: true,
        is_writable: false,
    });

    // cpi_authority_pda
    account_metas.push(AccountMeta {
        pubkey: Pubkey::from(CPI_AUTHORITY_PDA),
        is_signer: false,
        is_writable: false,
    });

    // light_system_program
    account_metas.push(AccountMeta {
        pubkey: Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID),
        is_signer: false,
        is_writable: false,
    });

    // registered_program_pda
    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts
            .registered_program_pda()
            .map_err(|_| TokenSdkError::CpiError("Missing registered program PDA".to_string()))?
            .key,
        is_signer: false,
        is_writable: false,
    });

    // noop_program
    account_metas.push(AccountMeta {
        pubkey: Pubkey::from(NOOP_PROGRAM_ID),
        is_signer: false,
        is_writable: false,
    });

    // account_compression_authority
    account_metas.push(AccountMeta {
        pubkey: *cpi_accounts
            .account_compression_authority()
            .map_err(|_| TokenSdkError::CpiError("Missing compression authority".to_string()))?
            .key,
        is_signer: false,
        is_writable: false,
    });

    // account_compression_program
    account_metas.push(AccountMeta {
        pubkey: Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID),
        is_signer: false,
        is_writable: false,
    });

    // self_program (compressed token program)
    account_metas.push(AccountMeta {
        pubkey: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        is_signer: false,
        is_writable: false,
    });

    // token_pool_pda - if not configured, use compressed token program ID (like anchor does for None)
    if cpi_accounts.config().token_pool_pda {
        let account = cpi_accounts
            .sol_pool_pda()
            .map_err(|_| TokenSdkError::CpiError("Missing token pool PDA".to_string()))?;
        account_metas.push(AccountMeta {
            pubkey: *account.key,
            is_signer: false,
            is_writable: true,
        });
    } else {
        // Anchor represents None optional accounts as the program ID being invoked
        account_metas.push(AccountMeta {
            pubkey: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
            is_signer: false,
            is_writable: false,
        });
    }

    // compress_or_decompress_token_account - if not configured, use compressed token program ID
    if cpi_accounts.config().compress_or_decompress_token_account {
        let account = cpi_accounts.decompression_recipient().map_err(|_| {
            TokenSdkError::CpiError("Missing compress/decompress account".to_string())
        })?;
        account_metas.push(AccountMeta {
            pubkey: *account.key,
            is_signer: false,
            is_writable: true,
        });
    } else {
        // Anchor represents None optional accounts as the program ID being invoked
        account_metas.push(AccountMeta {
            pubkey: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
            is_signer: false,
            is_writable: false,
        });
    }

    // token_program - if not configured, use compressed token program ID
    if cpi_accounts.config().cpi_context {
        let account = cpi_accounts
            .cpi_context()
            .map_err(|_| TokenSdkError::CpiError("Missing token program".to_string()))?;
        account_metas.push(AccountMeta {
            pubkey: *account.key,
            is_signer: false,
            is_writable: false,
        });
    } else {
        // Anchor represents None optional accounts as the program ID being invoked
        account_metas.push(AccountMeta {
            pubkey: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
            is_signer: false,
            is_writable: false,
        });
    }

    // system_program (always last according to TransferInstruction definition)
    account_metas.push(AccountMeta {
        pubkey: Pubkey::default(), // System program ID
        is_signer: false,
        is_writable: false,
    });

    // Add any remaining tree accounts
    let system_len = account_metas.len();
    let tree_accounts = cpi_accounts
        .account_infos()
        .get(system_len..)
        .unwrap_or(&[]);

    for account in tree_accounts {
        account_metas.push(AccountMeta {
            pubkey: *account.key,
            is_signer: false,
            is_writable: account.is_writable,
        });
    }

    Ok(account_metas)
}
