use account_compression::utils::constants::NOOP_PUBKEY;
use anchor_lang::{
    prelude::AccountMeta,
    solana_program::{account_info::AccountInfo, program_error::ProgramError},
};
use light_sdk::cpi::invoke_light_system_program;
use light_sdk_types::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, LIGHT_SYSTEM_PROGRAM_ID, REGISTERED_PROGRAM_PDA,
};
use solana_pubkey::Pubkey;

use crate::LIGHT_CPI_SIGNER;

/// Generalized CPI function for invoking light-system-program
///
/// This function builds the standard account meta structure for light-system-program CPI
/// and appends dynamic tree accounts (merkle trees, queues, etc.) to the account metas.
///
/// # Arguments
/// * `accounts` - All account infos passed to the instruction
/// * `cpi_bytes` - The CPI instruction data bytes
/// * `tree_accounts` - Slice of tree account pubkeys to append (will be marked as mutable)
/// * `sol_pool_pda` - Optional sol pool PDA pubkey
/// * `cpi_context_account` - Optional CPI context account pubkey
///
/// # Returns
/// * `Result<(), ProgramError>` - Success or error from the CPI call
pub fn execute_cpi_invoke<'info>(
    accounts: &'info [AccountInfo<'info>],
    cpi_bytes: Vec<u8>,
    tree_accounts: &[Pubkey],
    with_sol_pool: bool,
    cpi_context_account: Option<Pubkey>,
) -> Result<(), ProgramError> {
    // Build account metas with capacity for standard accounts + dynamic tree accounts
    let capacity = 11 + tree_accounts.len(); // 11 standard accounts + dynamic tree accounts
    let mut account_metas = Vec::with_capacity(capacity);

    // Standard account metas for light-system-program CPI
    // Account order must match light-system program's InvokeCpiInstruction expectation:
    // 0: fee_payer, 1: authority, 2: registered_program_pda, 3: noop_program,
    // 4: account_compression_authority, 5: account_compression_program, 6: invoking_program,
    // 7: sol_pool_pda (optional), 8: decompression_recipient (optional), 9: system_program,
    // 10: cpi_context_account (optional), then remaining accounts (merkle trees, etc.)
    let sol_pool_pda = if with_sol_pool {
        AccountMeta::new(
            solana_pubkey::pubkey!("CHK57ywWSDncAoRu1F8QgwYJeXuAJyyBYT4LixLXvMZ1"),
            false,
        )
    } else {
        AccountMeta::new_readonly(LIGHT_SYSTEM_PROGRAM_ID.into(), false)
    };
    account_metas.extend_from_slice(&[
        AccountMeta::new(*accounts[0].key, true), // fee_payer (signer, mutable)
        AccountMeta::new_readonly(LIGHT_CPI_SIGNER.cpi_signer.into(), true), // authority (cpi_authority_pda)
        AccountMeta::new_readonly(REGISTERED_PROGRAM_PDA.into(), false), // registered_program_pda
        AccountMeta::new_readonly(NOOP_PUBKEY.into(), false),            // noop_program
        AccountMeta::new_readonly(ACCOUNT_COMPRESSION_AUTHORITY_PDA.into(), false), // account_compression_authority
        AccountMeta::new_readonly(account_compression::ID, false), // account_compression_program
        AccountMeta::new_readonly(LIGHT_CPI_SIGNER.program_id.into(), false), // invoking_program (self_program)
        sol_pool_pda,                                                         // sol_pool_pda
        AccountMeta::new_readonly(LIGHT_SYSTEM_PROGRAM_ID.into(), false), // decompression_recipient (None, using default)
        AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false), // system_program
        AccountMeta::new_readonly(
            if let Some(cpi_context) = cpi_context_account {
                cpi_context
            } else {
                LIGHT_SYSTEM_PROGRAM_ID.into()
            },
            false,
        ), // cpi_context_account
    ]);

    // Append dynamic tree accounts (merkle trees, queues, etc.) as mutable accounts
    for tree_account in tree_accounts {
        account_metas.push(AccountMeta::new(*tree_account, false));
    }

    let instruction = anchor_lang::solana_program::instruction::Instruction {
        program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
        accounts: account_metas,
        data: cpi_bytes,
    };

    invoke_light_system_program(accounts, instruction, LIGHT_CPI_SIGNER.bump)?;

    Ok(())
}
