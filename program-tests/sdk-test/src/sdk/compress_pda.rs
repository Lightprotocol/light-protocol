use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::DataHasher;
use light_sdk::{
    account::LightAccount,
    cpi::{CpiAccounts, CpiInputs},
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
    LightDiscriminator,
};
use solana_program::sysvar::Sysvar;
use solana_program::{
    account_info::AccountInfo, clock::Clock, msg, program_error::ProgramError, pubkey::Pubkey,
};

/// Trait for PDA accounts that can be compressed
pub trait PdaTimingData {
    fn last_written_slot(&self) -> u64;
    fn slots_until_compression(&self) -> u64;
    fn set_last_written_slot(&mut self, slot: u64);
}

/// Helper function to compress a PDA and reclaim rent.
///
/// 1. closes onchain PDA
/// 2. transfers PDA lamports to rent_recipient
/// 3. updates the empty compressed PDA with onchain PDA data
///
/// This requires the compressed PDA that is tied to the onchain PDA to already
/// exist.
///
/// # Arguments
/// * `pda_account` - The PDA account to compress (will be closed)
/// * `compressed_account_meta` - Metadata for the compressed account (must be
///   empty but have an address)
/// * `proof` - Validity proof
/// * `cpi_accounts` - Accounts needed for CPI
/// * `owner_program` - The program that will own the compressed account
/// * `rent_recipient` - The account to receive the PDA's rent
//
// TODO:
// - check if any explicit checks required for compressed account?
// - consider multiple accounts per ix.
pub fn compress_pda<A>(
    pda_account: &AccountInfo,
    compressed_account_meta: &CompressedAccountMeta,
    proof: ValidityProof,
    cpi_accounts: CpiAccounts,
    owner_program: &Pubkey,
    rent_recipient: &AccountInfo,
) -> Result<(), LightSdkError>
where
    A: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + PdaTimingData,
{
    // Check that the PDA account is owned by the caller program
    if pda_account.owner != owner_program {
        msg!(
            "Invalid PDA owner. Expected: {}. Found: {}.",
            owner_program,
            pda_account.owner
        );
        return Err(LightSdkError::ConstraintViolation);
    }

    let current_slot = Clock::get()?.slot;

    // Deserialize the PDA data to check timing fields
    let pda_data = pda_account.try_borrow_data()?;
    let pda_account_data = A::try_from_slice(&pda_data[8..]).map_err(|_| LightSdkError::Borsh)?;
    drop(pda_data);

    let last_written_slot = pda_account_data.last_written_slot();
    let slots_until_compression = pda_account_data.slots_until_compression();

    if current_slot < last_written_slot + slots_until_compression {
        msg!(
            "Cannot compress yet. {} slots remaining",
            (last_written_slot + slots_until_compression).saturating_sub(current_slot)
        );
        return Err(LightSdkError::ConstraintViolation);
    }

    // Get the PDA lamports before we close it
    let pda_lamports = pda_account.lamports();

    let mut compressed_account =
        LightAccount::<'_, A>::new_mut(owner_program, compressed_account_meta, A::default())?;

    compressed_account.account = pda_account_data;

    // Create CPI inputs
    let cpi_inputs = CpiInputs::new(proof, vec![compressed_account.to_account_info()?]);

    // Invoke light system program to create the compressed account
    cpi_inputs.invoke_light_system_program(cpi_accounts)?;

    // Close the PDA account
    // 1. Transfer all lamports to the rent recipient
    let dest_starting_lamports = rent_recipient.lamports();
    **rent_recipient.try_borrow_mut_lamports()? = dest_starting_lamports
        .checked_add(pda_lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    // 2. Decrement source account lamports
    **pda_account.try_borrow_mut_lamports()? = 0;
    // 3. Clear all account data
    pda_account.try_borrow_mut_data()?.fill(0);
    // 4. Assign ownership back to the system program
    pda_account.assign(&solana_program::system_program::ID);

    Ok(())
}
