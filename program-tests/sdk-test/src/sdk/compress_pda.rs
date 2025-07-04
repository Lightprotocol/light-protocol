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
    fn last_touched_slot(&self) -> u64;
    fn slots_buffer(&self) -> u64;
    fn set_last_written_slot(&mut self, slot: u64);
}

const DECOMP_SEED: &[u8] = b"decomp";

/// Check that the PDA account is owned by the caller program and derived from the correct seeds.
///
/// # Arguments
/// * `custom_seeds` - Custom seeds to check against
/// * `c_pda_address` - The address of the compressed PDA
/// * `pda_account` - The address of the PDA account
/// * `caller_program` - The program that owns the PDA.
pub fn check_pda(
    custom_seeds: &[&[u8]],
    c_pda_address: &[u8; 32],
    pda_account: &Pubkey,
    caller_program: &Pubkey,
) -> Result<(), ProgramError> {
    // Create seeds array: [custom_seeds..., c_pda_address, "decomp"]
    let mut seeds: Vec<&[u8]> = custom_seeds.to_vec();
    seeds.push(c_pda_address);
    seeds.push(DECOMP_SEED);

    let derived_pda =
        Pubkey::create_program_address(&seeds, caller_program).expect("Invalid PDA seeds.");

    if derived_pda != *pda_account {
        msg!(
            "Invalid PDA provided. Expected: {}. Found: {}.",
            derived_pda,
            pda_account
        );
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
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
/// * `proof` - Optional validity proof
/// * `cpi_accounts` - Accounts needed for CPI starting from
///   system_accounts_offset
/// * `system_accounts_offset` - Offset where CPI accounts start
/// * `fee_payer` - The fee payer account
/// * `cpi_signer` - The CPI signer for the calling program
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
    custom_seeds: &[&[u8]],
) -> Result<(), LightSdkError>
where
    A: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + PdaTimingData,
{
    // Check that the PDA account is owned by the caller program and derived from the address of the compressed PDA.
    check_pda(
        custom_seeds,
        &compressed_account_meta.address,
        pda_account.key,
        owner_program,
    )?;

    let current_slot = Clock::get()?.slot;

    // Deserialize the PDA data to check timing fields
    let pda_data = pda_account.try_borrow_data()?;
    let pda_account_data = A::try_from_slice(&pda_data[8..]).map_err(|_| LightSdkError::Borsh)?;
    drop(pda_data);

    let last_touched_slot = pda_account_data.last_touched_slot();
    let slots_buffer = pda_account_data.slots_buffer();

    if current_slot < last_touched_slot + slots_buffer {
        msg!(
            "Cannot compress yet. {} slots remaining",
            (last_touched_slot + slots_buffer).saturating_sub(current_slot)
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
