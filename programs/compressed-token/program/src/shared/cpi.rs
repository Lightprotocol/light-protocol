use std::mem::MaybeUninit;

use account_compression::utils::constants::NOOP_PUBKEY;
use anchor_lang::solana_program::program_error::ProgramError;
use light_sdk_types::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, CPI_AUTHORITY_PDA_SEED,
    LIGHT_SYSTEM_PROGRAM_ID, REGISTERED_PROGRAM_PDA,
};
use pinocchio::{
    account_info::{AccountInfo, BorrowState},
    cpi::{invoke_signed_unchecked, MAX_CPI_ACCOUNTS},
    instruction::{Account, AccountMeta, Instruction, Seed, Signer},
    msg,
    pubkey::Pubkey,
};

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
pub fn execute_cpi_invoke(
    accounts: &[AccountInfo],
    cpi_bytes: Vec<u8>,
    tree_accounts: &[&Pubkey],
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
    let inner_pool =
        solana_pubkey::pubkey!("CHK57ywWSDncAoRu1F8QgwYJeXuAJyyBYT4LixLXvMZ1").to_bytes();
    let sol_pool_pda = if with_sol_pool {
        AccountMeta::new(&inner_pool, true, false)
    } else {
        AccountMeta::new(&LIGHT_SYSTEM_PROGRAM_ID, false, false)
    };
    account_metas.extend_from_slice(&[
        AccountMeta::new(accounts[0].key(), true, true), // 0 fee_payer (signer, mutable)
        AccountMeta::new(&LIGHT_CPI_SIGNER.cpi_signer, false, true), // 1 authority (cpi_authority_pda)
        AccountMeta::new(&REGISTERED_PROGRAM_PDA, false, false),     // 2 registered_program_pda
        AccountMeta::new(&NOOP_PUBKEY, false, false),                // 3 noop_program
        AccountMeta::new(&ACCOUNT_COMPRESSION_AUTHORITY_PDA, false, false), // 4 account_compression_authority
        AccountMeta::new(&ACCOUNT_COMPRESSION_PROGRAM_ID, false, false), // 5 account_compression_program
        AccountMeta::new(&LIGHT_CPI_SIGNER.program_id, false, false), // 6 invoking_program (self_program)
        sol_pool_pda,                                                 // 7 sol_pool_pda
        AccountMeta::new(&LIGHT_SYSTEM_PROGRAM_ID, false, false), // 8 decompression_recipient (None, using default)
        AccountMeta::new(&[0u8; 32], false, false),               // system_program
        AccountMeta::new(
            if let Some(cpi_context) = cpi_context_account.as_ref() {
                cpi_context
            } else {
                &LIGHT_SYSTEM_PROGRAM_ID
            },
            false,
            false,
        ), // cpi_context_account
    ]);

    // Append dynamic tree accounts (merkle trees, queues, etc.) as mutable accounts
    for tree_account in tree_accounts {
        account_metas.push(AccountMeta::new(tree_account, true, false));
    }

    let instruction = Instruction {
        program_id: &LIGHT_SYSTEM_PROGRAM_ID,
        accounts: account_metas.as_slice(),
        data: cpi_bytes.as_slice(),
    };

    // Use the precomputed CPI signer and bump from the config
    let bump_seed = [LIGHT_CPI_SIGNER.bump];
    let seed_array = [
        Seed::from(CPI_AUTHORITY_PDA_SEED),
        Seed::from(bump_seed.as_slice()),
    ];
    let signer = Signer::from(&seed_array);
    let mut account_vec = Vec::with_capacity(accounts.len());
    accounts.iter().for_each(|a| account_vec.push(a));
    match slice_invoke_signed(&instruction, account_vec.as_slice(), &[signer]) {
        Ok(()) => {}
        Err(e) => {
            msg!(format!("slice_invoke_signed failed: {:?}", e).as_str());
            return Err(ProgramError::InvalidArgument);
        }
    }

    Ok(())
}

#[inline]
pub fn slice_invoke_signed(
    instruction: &Instruction,
    account_infos: &[&AccountInfo],
    signers_seeds: &[Signer],
) -> pinocchio::ProgramResult {
    use pinocchio::program_error::ProgramError;
    if instruction.accounts.len() < account_infos.len() {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    if account_infos.len() > MAX_CPI_ACCOUNTS {
        return Err(ProgramError::InvalidArgument);
    }

    const UNINIT: MaybeUninit<Account> = MaybeUninit::<Account>::uninit();
    let mut accounts = [UNINIT; MAX_CPI_ACCOUNTS];
    let mut len = 0;

    for (account_info, account_meta) in account_infos.iter().zip(
        instruction
            .accounts
            .iter()
            .filter(|x| x.pubkey != instruction.program_id),
    ) {
        // if account_info.key() == instruction.program_id {
        //     // skip anchor None account infos
        //     continue;
        // }
        if account_info.key() != account_meta.pubkey {
            use std::format;
            msg!(format!(
                "Received account key: {:?}",
                solana_pubkey::Pubkey::new_from_array(*account_info.key())
            )
            .as_str());
            msg!(format!(
                "Expected account key: {:?}",
                solana_pubkey::Pubkey::new_from_array(*account_meta.pubkey)
            )
            .as_str());

            return Err(ProgramError::InvalidArgument);
        }

        let state = if account_meta.is_writable {
            BorrowState::Borrowed
        } else {
            BorrowState::MutablyBorrowed
        };

        if account_info.is_borrowed(state) {
            return Err(ProgramError::AccountBorrowFailed);
        }

        // SAFETY: The number of accounts has been validated to be less than
        // `MAX_CPI_ACCOUNTS`.
        unsafe {
            accounts
                .get_unchecked_mut(len)
                .write(Account::from(*account_info));
        }

        len += 1;
    }
    // SAFETY: The accounts have been validated.
    unsafe {
        invoke_signed_unchecked(
            instruction,
            core::slice::from_raw_parts(accounts.as_ptr() as _, len),
            signers_seeds,
        );
    }

    Ok(())
}
