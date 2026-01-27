use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_pubkey::Pubkey;
use solana_system_interface::instruction as system_instruction;

use crate::error::LightSdkError;

/// Cold path: Account already has lamports (e.g., attacker donation).
/// Uses Assign + Allocate + Transfer instead of CreateAccount which would fail.
#[cold]
fn create_pda_account_with_lamports<'info>(
    rent_sponsor: &AccountInfo<'info>,
    solana_account: &AccountInfo<'info>,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
    seeds: &[&[u8]],
    system_program: &AccountInfo<'info>,
) -> Result<(), LightSdkError> {
    let current_lamports = solana_account.lamports();

    // Assign owner
    let assign_ix = system_instruction::assign(solana_account.key, owner);
    invoke_signed(
        &assign_ix,
        &[solana_account.clone(), system_program.clone()],
        &[seeds],
    )
    .map_err(LightSdkError::ProgramError)?;

    // Allocate space
    let allocate_ix = system_instruction::allocate(solana_account.key, space);
    invoke_signed(
        &allocate_ix,
        &[solana_account.clone(), system_program.clone()],
        &[seeds],
    )
    .map_err(LightSdkError::ProgramError)?;

    // Transfer remaining lamports for rent-exemption if needed
    if lamports > current_lamports {
        let transfer_ix = system_instruction::transfer(
            rent_sponsor.key,
            solana_account.key,
            lamports - current_lamports,
        );
        invoke_signed(
            &transfer_ix,
            &[
                rent_sponsor.clone(),
                solana_account.clone(),
                system_program.clone(),
            ],
            &[],
        )
        .map_err(LightSdkError::ProgramError)?;
    }

    Ok(())
}

/// Creates a PDA account, handling the case where the account already has lamports.
///
/// This function handles the edge case where an attacker might have donated lamports
/// to the PDA address before decompression. In that case, `CreateAccount` would fail,
/// so we fall back to `Assign + Allocate + Transfer`.
#[inline(never)]
pub fn create_pda_account<'info>(
    rent_sponsor: &AccountInfo<'info>,
    solana_account: &AccountInfo<'info>,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
    seeds: &[&[u8]],
    system_program: &AccountInfo<'info>,
) -> Result<(), LightSdkError> {
    // Cold path: account already has lamports (e.g., attacker donation)
    if solana_account.lamports() > 0 {
        return create_pda_account_with_lamports(
            rent_sponsor,
            solana_account,
            lamports,
            space,
            owner,
            seeds,
            system_program,
        );
    }

    // Normal path: CreateAccount
    let create_account_ix = system_instruction::create_account(
        rent_sponsor.key,
        solana_account.key,
        lamports,
        space,
        owner,
    );

    invoke_signed(
        &create_account_ix,
        &[
            rent_sponsor.clone(),
            solana_account.clone(),
            system_program.clone(),
        ],
        &[seeds],
    )
    .map_err(LightSdkError::ProgramError)
}
