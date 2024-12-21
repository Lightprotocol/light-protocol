use bytemuck::Pod;
use light_hasher::Discriminator;
use solana_program::account_info;
#[cfg(target_os = "solana")]
use solana_program::msg;
use thiserror::Error;

// TODO: move file to bounded vec crate and rename to light-zero-copy
#[derive(Debug, Error, PartialEq)]
pub enum ZeroCopyError {
    #[error("Invalid Account size.")]
    InvalidAccountSize,
    #[error("Invalid Discriminator.")]
    InvalidDiscriminator,
    #[error("Account owned by wrong program.")]
    AccountOwnedByWrongProgram,
    #[error("Account not mutable.")]
    AccountNotMutable,
    #[error("Borrow account data failed.")]
    BorrowAccountDataFailed,
}

#[cfg(feature = "solana")]
impl From<ZeroCopyError> for u32 {
    fn from(e: ZeroCopyError) -> u32 {
        match e {
            ZeroCopyError::InvalidAccountSize => 14401,
            ZeroCopyError::InvalidDiscriminator => 14402,
            ZeroCopyError::AccountOwnedByWrongProgram => 14403,
            ZeroCopyError::AccountNotMutable => 14404,
            ZeroCopyError::BorrowAccountDataFailed => 14405,
        }
    }
}

#[cfg(feature = "solana")]
impl From<ZeroCopyError> for solana_program::program_error::ProgramError {
    fn from(e: ZeroCopyError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}

pub const DISCRIMINATOR_LEN: usize = 8;

pub fn check_account_info_init<T: Discriminator>(
    program_id: solana_program::pubkey::Pubkey,
    account_info: &account_info::AccountInfo,
) -> Result<(), ZeroCopyError> {
    if *account_info.owner != program_id {
        return Err(ZeroCopyError::AccountOwnedByWrongProgram);
    }
    if !account_info.is_writable {
        return Err(ZeroCopyError::AccountNotMutable);
    }
    let account_data = &mut account_info
        .try_borrow_mut_data()
        .map_err(|_| ZeroCopyError::BorrowAccountDataFailed)?;
    set_discriminator::<T>(account_data)
}

pub fn check_account_info_mut<T: Discriminator>(
    program_id: &solana_program::pubkey::Pubkey,
    account_info: &account_info::AccountInfo,
) -> Result<(), ZeroCopyError> {
    if *account_info.owner != *program_id {
        return Err(ZeroCopyError::AccountOwnedByWrongProgram);
    }
    if !account_info.is_writable {
        return Err(ZeroCopyError::AccountNotMutable);
    }
    let account_data = &account_info
        .try_borrow_data()
        .map_err(|_| ZeroCopyError::BorrowAccountDataFailed)?;
    check_discriminator::<T>(account_data)
}

pub fn set_discriminator<T: Discriminator>(bytes: &mut [u8]) -> Result<(), ZeroCopyError> {
    if bytes[0..DISCRIMINATOR_LEN] != [0; DISCRIMINATOR_LEN] {
        #[cfg(target_os = "solana")]
        msg!("Discriminator bytes must be zero for initialization.");
        return Err(ZeroCopyError::InvalidDiscriminator);
    }
    bytes[0..DISCRIMINATOR_LEN].copy_from_slice(&T::DISCRIMINATOR);
    Ok(())
}

pub fn check_discriminator<T: Discriminator>(bytes: &[u8]) -> Result<(), ZeroCopyError> {
    if bytes.len() < DISCRIMINATOR_LEN {
        return Err(ZeroCopyError::InvalidAccountSize);
    }

    if T::DISCRIMINATOR != bytes[0..DISCRIMINATOR_LEN] {
        #[cfg(target_os = "solana")]
        msg!(
            "Expected discriminator: {:?}, actual {:?} ",
            T::DISCRIMINATOR,
            bytes[0..DISCRIMINATOR_LEN].to_vec()
        );
        return Err(ZeroCopyError::InvalidDiscriminator);
    }
    Ok(())
}

pub fn bytes_to_struct_unchecked<T: Clone + Copy + Pod>(
    bytes: &mut [u8],
) -> Result<*mut T, ZeroCopyError> {
    // Base address for alignment check of T.
    let base_address = bytes.as_ptr() as usize + DISCRIMINATOR_LEN;
    if bytes.len() < std::mem::size_of::<T>() || base_address % std::mem::align_of::<T>() != 0 {
        return Err(ZeroCopyError::InvalidAccountSize);
    }

    Ok(bytes[DISCRIMINATOR_LEN..].as_mut_ptr() as *mut T)
}
