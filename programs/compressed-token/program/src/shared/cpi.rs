use std::mem::MaybeUninit;

use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::constants::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, CPI_AUTHORITY_PDA_SEED,
    LIGHT_SYSTEM_PROGRAM_ID, REGISTERED_PROGRAM_PDA,
};
use light_program_profiler::profile;
use pinocchio::{
    address::Address,
    cpi::{invoke_signed_unchecked, CpiAccount, Seed, Signer},
    instruction::{InstructionAccount, InstructionView},
    AccountView as AccountInfo,
};
use solana_msg::msg;

use crate::LIGHT_CPI_SIGNER;

/// Cast `&[u8; 32]` to `&Address` without creating a stack copy.
/// SAFETY: Address is a single-field newtype wrapper around [u8; 32].
#[inline(always)]
fn as_addr(bytes: &[u8; 32]) -> &Address {
    unsafe { &*(bytes as *const [u8; 32] as *const Address) }
}

/// Executes CPI to light-system-program using the new InvokeCpiInstructionSmall format
#[inline(never)]
#[profile]
pub fn execute_cpi_invoke(
    accounts: &[AccountInfo],
    cpi_bytes: Vec<u8>,
    tree_accounts: &[&[u8; 32]],
    with_sol_pool: bool,
    decompress_sol: Option<&[u8; 32]>,
    cpi_context_account: Option<[u8; 32]>,
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
    account_metas.push(InstructionAccount::new(accounts[0].address(), true, true)); // fee_payer (signer, mutable)
    account_metas.push(InstructionAccount::new(
        as_addr(&LIGHT_CPI_SIGNER.cpi_signer),
        false,
        true,
    )); // authority (cpi_authority_pda, signer)

    if !write_to_cpi_context {
        // Execution mode - include all execution accounts
        account_metas.push(InstructionAccount::new(
            as_addr(&REGISTERED_PROGRAM_PDA),
            false,
            false,
        ));
        account_metas.push(InstructionAccount::new(
            as_addr(&ACCOUNT_COMPRESSION_AUTHORITY_PDA),
            false,
            false,
        ));
        account_metas.push(InstructionAccount::new(
            as_addr(&ACCOUNT_COMPRESSION_PROGRAM_ID),
            false,
            false,
        ));
        static SYSTEM_PROGRAM: [u8; 32] = [0u8; 32];
        account_metas.push(InstructionAccount::new(
            as_addr(&SYSTEM_PROGRAM),
            false,
            false,
        ));

        // Optional SOL pool
        if with_sol_pool {
            static INNER_POOL: [u8; 32] =
                solana_pubkey::pubkey!("CHK57ywWSDncAoRu1F8QgwYJeXuAJyyBYT4LixLXvMZ1").to_bytes();
            account_metas.push(InstructionAccount::new(as_addr(&INNER_POOL), true, false));
        }

        if let Some(decompress_sol_bytes) = decompress_sol {
            account_metas.push(InstructionAccount::new(
                as_addr(decompress_sol_bytes),
                true,
                false,
            ));
        }
        if let Some(ref cpi_context_bytes) = cpi_context_account {
            account_metas.push(InstructionAccount::new(
                as_addr(cpi_context_bytes),
                true,
                false,
            ));
        }
        // Append dynamic tree accounts
        for tree_account in tree_accounts {
            account_metas.push(InstructionAccount::new(as_addr(tree_account), true, false));
        }
    } else if let Some(ref cpi_context_bytes) = cpi_context_account {
        account_metas.push(InstructionAccount::new(
            as_addr(cpi_context_bytes),
            true,
            false,
        ));
    }

    let instruction = InstructionView {
        program_id: as_addr(&LIGHT_SYSTEM_PROGRAM_ID),
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

/// Equivalent to pinocchio::cpi::invoke_signed_with_slice except:
/// 1. account_infos: &[&AccountInfo] ->  &[AccountInfo]
/// 2. Error prints
#[inline(never)]
#[profile]
pub fn slice_invoke_signed(
    instruction: &InstructionView,
    account_infos: &[AccountInfo],
    signers_seeds: &[Signer],
) -> pinocchio::ProgramResult {
    use pinocchio::error::ProgramError;
    if instruction.accounts.len() < account_infos.len() {
        msg!(
            "instruction.accounts.len() account metas {}< account_infos.len() account infos {}",
            instruction.accounts.len(),
            account_infos.len()
        );
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    const LOCAL_MAX: usize = crate::MAX_ACCOUNTS;
    if account_infos.len() > LOCAL_MAX {
        return Err(ProgramError::InvalidArgument);
    }

    const UNINIT: MaybeUninit<CpiAccount> = MaybeUninit::<CpiAccount>::uninit();
    let mut accounts = [UNINIT; LOCAL_MAX];
    let mut len = 0;

    for (account_info, account_meta) in account_infos.iter().zip(instruction.accounts.iter()) {
        if account_info.address() != account_meta.address {
            use std::format;
            msg!(format!(
                "Received account key: {:?}",
                solana_pubkey::Pubkey::new_from_array(account_info.address().to_bytes())
            )
            .as_str());
            msg!(format!(
                "Expected account key: {:?}",
                solana_pubkey::Pubkey::new_from_array(account_meta.address.to_bytes())
            )
            .as_str());
            return Err(ProgramError::InvalidArgument);
        }

        if account_meta.is_writable {
            if account_info.is_borrowed_mut() {
                return Err(ProgramError::AccountBorrowFailed);
            }
        } else if account_info.is_borrowed() {
            return Err(ProgramError::AccountBorrowFailed);
        }

        // SAFETY: The number of accounts has been validated to be less than
        // `MAX_CPI_ACCOUNTS`.
        unsafe {
            accounts
                .get_unchecked_mut(len)
                .write(CpiAccount::from(account_info));
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
