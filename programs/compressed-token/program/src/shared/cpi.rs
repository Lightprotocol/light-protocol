use std::mem::MaybeUninit;

use anchor_lang::solana_program::program_error::ProgramError;
use light_program_profiler::profile;
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

/// Executes CPI to light-system-program using the new InvokeCpiInstructionSmall format
///
/// This function follows the same pattern as the system program's InvokeCpiInstructionSmall
/// and properly handles AccountOptions for determining execution vs cpi context writing.
///
/// # Arguments
/// * `accounts` - All account infos passed to the instruction
/// * `cpi_bytes` - The CPI instruction data bytes
/// * `tree_accounts` - Slice of tree account pubkeys to append (will be marked as mutable)
/// * `with_sol_pool` - Whether SOL pool is being used
/// * `cpi_context_account` - Optional CPI cpi context account pubkey
///
/// # Returns
/// * `Result<(), ProgramError>` - Success or error from the CPI call
#[profile]
pub fn execute_cpi_invoke(
    accounts: &[AccountInfo],
    cpi_bytes: Vec<u8>,
    tree_accounts: &[&Pubkey],
    with_sol_pool: bool,
    decompress_sol: Option<&Pubkey>,
    cpi_context_account: Option<Pubkey>,
    write_to_cpi_context: bool,
) -> Result<(), ProgramError> {
    if cpi_bytes[9] == 0 {
        msg!("Bump not set in cpi struct.");
        return Err(ProgramError::InvalidInstructionData);
    }

    // Build account metas following InvokeCpiInstructionSmall format
    let base_capacity = if write_to_cpi_context {
        3
    } else {
        8 + tree_accounts.len()
    };
    let mut sol_pool_capacity = if with_sol_pool { 1 } else { 0 };
    if decompress_sol.is_some() {
        sol_pool_capacity += 1
    };
    let cpi_context_capacity = if cpi_context_account.is_some() { 1 } else { 0 };
    let total_capacity = base_capacity + sol_pool_capacity + cpi_context_capacity;

    let mut account_metas = Vec::with_capacity(total_capacity);

    // Always include: fee_payer and authority
    account_metas.push(AccountMeta::new(accounts[0].key(), true, true)); // fee_payer (signer, mutable)
    account_metas.push(AccountMeta::new(&LIGHT_CPI_SIGNER.cpi_signer, false, true)); // authority (cpi_authority_pda, signer)

    if !write_to_cpi_context {
        // Execution mode - include all execution accounts
        account_metas.push(AccountMeta::new(&REGISTERED_PROGRAM_PDA, false, false)); // registered_program_pda
        account_metas.push(AccountMeta::new(
            &ACCOUNT_COMPRESSION_AUTHORITY_PDA,
            false,
            false,
        )); // account_compression_authority
        account_metas.push(AccountMeta::new(
            &ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            false,
        )); // account_compression_program
        account_metas.push(AccountMeta::new(&[0u8; 32], false, false)); // system_program

        // Optional SOL pool
        if with_sol_pool {
            const INNER_POOL: [u8; 32] =
                solana_pubkey::pubkey!("CHK57ywWSDncAoRu1F8QgwYJeXuAJyyBYT4LixLXvMZ1").to_bytes();
            account_metas.push(AccountMeta::new(&INNER_POOL, true, false)); // sol_pool_pda
        }

        // No decompression_recipient for compressed token operations
        if let Some(decompress_sol) = decompress_sol {
            account_metas.push(AccountMeta::new(decompress_sol, true, false));
        }
        // Optional CPI context account (for both execution and cpi context writing modes)
        if let Some(cpi_context) = cpi_context_account.as_ref() {
            account_metas.push(AccountMeta::new(cpi_context, true, false)); // cpi_context_account
        }
        // Append dynamic tree accounts (merkle trees, queues, etc.)
        for tree_account in tree_accounts {
            account_metas.push(AccountMeta::new(tree_account, true, false));
        }
    } else {
        // Optional CPI context account (for both execution and cpi context writing modes)
        if let Some(cpi_context) = cpi_context_account.as_ref() {
            account_metas.push(AccountMeta::new(cpi_context, true, false)); // cpi_context_account
        }
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

    match slice_invoke_signed(&instruction, accounts, &[signer]) {
        Ok(()) => {}
        Err(e) => {
            msg!(format!("slice_invoke_signed failed: {:?}", e).as_str());
            return Err(ProgramError::InvalidArgument);
        }
    }

    Ok(())
}

/// Eqivalent to pinocchio::cpi::slice_invoke_signed except:
/// 1. account_infos: &[&AccountInfo] ->  &[AccountInfo]
/// 2. Error prints
#[inline]
#[profile]
pub fn slice_invoke_signed(
    instruction: &Instruction,
    account_infos: &[AccountInfo],
    signers_seeds: &[Signer],
) -> pinocchio::ProgramResult {
    use pinocchio::program_error::ProgramError;
    if instruction.accounts.len() < account_infos.len() {
        msg!(
            "instruction.accounts.len() account metas {}< account_infos.len() account infos {}",
            instruction.accounts.len(),
            account_infos.len()
        );
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    if account_infos.len() > MAX_CPI_ACCOUNTS {
        return Err(ProgramError::InvalidArgument);
    }

    const UNINIT: MaybeUninit<Account> = MaybeUninit::<Account>::uninit();
    let mut accounts = [UNINIT; MAX_CPI_ACCOUNTS];
    let mut len = 0;

    for (account_info, account_meta) in account_infos.iter().zip(
        instruction.accounts.iter(), //   .filter(|x| x.pubkey != instruction.program_id),
    ) {
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
                .write(Account::from(account_info));
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
