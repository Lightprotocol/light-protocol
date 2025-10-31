use crate::{
    discriminator::{Discriminator, DISCRIMINATOR_LEN},
    error::AccountError,
    AccountInfoTrait,
};

/// Sets discriminator in account data.
pub fn account_info_init<T: Discriminator, A: AccountInfoTrait>(
    account_info: &A,
) -> Result<(), AccountError> {
    set_discriminator::<T>(
        &mut account_info
            .try_borrow_mut_data()
            .map_err(|_| AccountError::BorrowAccountDataFailed)?,
    )?;
    Ok(())
}

/// Checks:
/// 1. account is mutable
/// 2. account owned by program_id
/// 3. account discriminator
pub fn check_account_info_mut<T: Discriminator, A: AccountInfoTrait>(
    program_id: &[u8; 32],
    account_info: &A,
) -> Result<(), AccountError> {
    check_mut(account_info)?;
    check_account_info::<T, A>(program_id, account_info)
}

/// Checks:
/// 1. account is not mutable
/// 2. account owned by program_id
/// 3. account discriminator
pub fn check_account_info_non_mut<T: Discriminator, A: AccountInfoTrait>(
    program_id: &[u8; 32],
    account_info: &A,
) -> Result<(), AccountError> {
    check_non_mut(account_info)?;
    check_account_info::<T, A>(program_id, account_info)
}

pub fn check_non_mut<A: AccountInfoTrait>(account_info: &A) -> Result<(), AccountError> {
    if account_info.is_writable() {
        return Err(AccountError::AccountMutable);
    }
    Ok(())
}

/// Checks:
/// 1. account owned by program_id
/// 2. account discriminator
pub fn check_account_info<T: Discriminator, A: AccountInfoTrait>(
    program_id: &[u8; 32],
    account_info: &A,
) -> Result<(), AccountError> {
    check_owner(program_id, account_info)?;

    let account_data = &account_info
        .try_borrow_data()
        .map_err(|_| AccountError::BorrowAccountDataFailed)?;
    check_discriminator::<T>(account_data)
}

/// Checks:
/// 1. discriminator is uninitialized
/// 2. sets discriminator
pub fn set_discriminator<T: Discriminator>(bytes: &mut [u8]) -> Result<(), AccountError> {
    check_data_is_zeroed::<DISCRIMINATOR_LEN>(bytes)
        .map_err(|_| AccountError::AlreadyInitialized)?;
    bytes[0..DISCRIMINATOR_LEN].copy_from_slice(&T::LIGHT_DISCRIMINATOR);
    Ok(())
}

/// Checks:
/// 1. account size is at least U
/// 2. account discriminator
pub fn check_discriminator<T: Discriminator>(bytes: &[u8]) -> Result<(), AccountError> {
    if bytes.len() < DISCRIMINATOR_LEN {
        return Err(AccountError::InvalidAccountSize);
    }

    if T::LIGHT_DISCRIMINATOR != bytes[0..DISCRIMINATOR_LEN] {
        #[cfg(all(feature = "msg", feature = "std"))]
        solana_msg::msg!(
            "expected discriminator {:?} != {:?} actual",
            T::LIGHT_DISCRIMINATOR,
            &bytes[0..DISCRIMINATOR_LEN]
        );
        return Err(AccountError::InvalidDiscriminator);
    }
    Ok(())
}

/// Checks that the account balance is greater or eqal to rent exemption.
pub fn check_account_balance_is_rent_exempt<A: AccountInfoTrait>(
    account_info: &A,
    expected_size: usize,
) -> Result<u64, AccountError> {
    let account_size = account_info.data_len();
    if account_size != expected_size {
        return Err(AccountError::InvalidAccountSize);
    }
    let lamports = account_info.lamports();
    #[cfg(target_os = "solana")]
    {
        let rent_exemption = A::get_min_rent_balance(expected_size)?;
        if lamports < rent_exemption {
            return Err(AccountError::InvalidAccountBalance);
        }
        Ok(rent_exemption)
    }
    #[cfg(not(target_os = "solana"))]
    {
        #[cfg(feature = "std")]
        println!("Rent exemption check skipped since not target_os solana.");
        Ok(lamports)
    }
}

pub fn check_signer<A: AccountInfoTrait>(account_info: &A) -> Result<(), AccountError> {
    if !account_info.is_signer() {
        return Err(AccountError::InvalidSigner);
    }
    Ok(())
}

pub fn check_mut<A: AccountInfoTrait>(account_info: &A) -> Result<(), AccountError> {
    if !account_info.is_writable() {
        return Err(AccountError::AccountNotMutable);
    }
    Ok(())
}

pub fn check_owner<A: AccountInfoTrait>(
    owner: &[u8; 32],
    account_info: &A,
) -> Result<(), AccountError> {
    if !account_info.is_owned_by(owner) {
        return Err(AccountError::AccountOwnedByWrongProgram);
    }
    Ok(())
}

pub fn check_program<A: AccountInfoTrait>(
    program_id: &[u8; 32],
    account_info: &A,
) -> Result<(), AccountError> {
    if account_info.key() != *program_id {
        return Err(AccountError::InvalidProgramId);
    }
    if !account_info.executable() {
        return Err(AccountError::ProgramNotExecutable);
    }
    Ok(())
}

pub fn check_pda_seeds<A: AccountInfoTrait>(
    seeds: &[&[u8]],
    program_id: &[u8; 32],
    account_info: &A,
) -> Result<(), AccountError> {
    let (derived_key, _) = A::find_program_address(seeds, program_id);
    if derived_key != account_info.key() {
        return Err(AccountError::InvalidSeeds);
    }
    Ok(())
}

pub fn check_pda_seeds_with_bump<A: AccountInfoTrait>(
    seeds: &[&[u8]],
    program_id: &[u8; 32],
    account_info: &A,
) -> Result<(), AccountError> {
    let derived_key = A::create_program_address(seeds, program_id)?;
    if derived_key != account_info.key() {
        return Err(AccountError::InvalidSeeds);
    }
    Ok(())
}

/// Check that an account is not initialized by checking it's discriminator is zeroed.
///
/// Equivalent functionality to anchor #[account(zero)].
pub fn check_data_is_zeroed<const N: usize>(data: &[u8]) -> Result<(), AccountError> {
    if data[..N].iter().any(|&byte| byte != 0) {
        return Err(AccountError::AccountNotZeroed);
    }
    Ok(())
}
