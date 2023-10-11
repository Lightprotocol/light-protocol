use std::convert::TryInto;

use anchor_lang::{
    err,
    prelude::AccountLoader,
    solana_program::{
        account_info::AccountInfo, msg, program::invoke_signed, program_error::ProgramError,
        pubkey::Pubkey, system_instruction, sysvar::rent::Rent,
    },
    Key, Owner, ZeroCopy,
};

use crate::{errors::ErrorCode, indexed_merkle_tree::IndexedMerkleTree};

#[allow(clippy::too_many_arguments)]
pub fn create_and_check_pda<'a, 'b>(
    program_id: &Pubkey,
    signer_account: &'a AccountInfo<'b>,
    passed_in_pda: &'a AccountInfo<'b>,
    system_program: &'a AccountInfo<'b>,
    rent: &Rent,
    _instruction_data: &[u8],
    domain_separation_seed: &[u8],
    number_storage_bytes: u64,
    lamports: u64,
    rent_exempt: bool,
) -> Result<(), ProgramError> {
    let derived_pubkey =
        Pubkey::find_program_address(&[_instruction_data, domain_separation_seed], program_id);

    if derived_pubkey.0 != *passed_in_pda.key {
        msg!("Passed-in pda pubkey != on-chain derived pda pubkey.");
        msg!("On-chain derived pda pubkey {:?}", derived_pubkey);
        msg!("Passed-in pda pubkey {:?}", *passed_in_pda.key);
        msg!("Instruction data seed  {:?}", _instruction_data);
        return Err(ProgramError::InvalidInstructionData);
    }

    let mut account_lamports = lamports;
    if rent_exempt {
        account_lamports += rent.minimum_balance(number_storage_bytes.try_into().unwrap());
    }

    invoke_signed(
        &system_instruction::create_account(
            signer_account.key,   // from_pubkey
            passed_in_pda.key,    // to_pubkey
            account_lamports,     // lamports
            number_storage_bytes, // space
            program_id,           // owner
        ),
        &[
            signer_account.clone(),
            passed_in_pda.clone(),
            system_program.clone(),
        ],
        &[&[
            _instruction_data,
            domain_separation_seed,
            &[derived_pubkey.1],
        ]],
    )?;

    // Check for rent exemption
    if rent_exempt
        && !rent.is_exempt(
            **passed_in_pda.lamports.borrow(),
            number_storage_bytes.try_into().unwrap(),
        )
    {
        msg!("Account is not rent exempt.");
        return Err(ProgramError::AccountNotRentExempt);
    }
    Ok(())
}

/// Validates the old Merkle tree account. If valid, it also checks whether
/// it's the currently newest one, then afterwards marks it as not the newest.
/// Should be called only when initializing new Merkle trees.
pub fn deserialize_and_update_old_merkle_tree<T>(
    account: &AccountInfo,
    seed: &[u8],
    program_id: &Pubkey,
) -> anchor_lang::Result<()>
where
    T: IndexedMerkleTree + ZeroCopy + Owner,
{
    let loader: AccountLoader<T> = AccountLoader::try_from(account)?;
    let pubkey = loader.key();
    let mut merkle_tree = loader.load_mut()?;
    let index = merkle_tree.get_index();

    let (expected_pubkey, _) =
        Pubkey::find_program_address(&[seed, index.to_le_bytes().as_ref()], program_id);
    if pubkey != expected_pubkey {
        return err!(ErrorCode::InvalidOldMerkleTree);
    }

    if !merkle_tree.is_newest() {
        return err!(ErrorCode::NotNewestOldMerkleTree);
    }
    merkle_tree.set_newest(false);

    Ok(())
}
