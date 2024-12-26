use bytemuck::Pod;
use light_hasher::Discriminator;
use light_utils::account::DISCRIMINATOR_LEN;
use thiserror::Error;

// TODO: move file to bounded vec crate and rename to light-zero-copy
#[derive(Debug, Error, PartialEq)]
pub enum ZeroCopyError {
    #[error("Invalid Account size.")]
    InvalidAccountSize,
}

#[cfg(feature = "solana")]
impl From<ZeroCopyError> for u32 {
    fn from(e: ZeroCopyError) -> u32 {
        match e {
            ZeroCopyError::InvalidAccountSize => 14401,
        }
    }
}

#[cfg(feature = "solana")]
impl From<ZeroCopyError> for solana_program::program_error::ProgramError {
    fn from(e: ZeroCopyError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}

pub fn bytes_to_struct_unchecked<T: Clone + Copy + Pod + Discriminator>(
    bytes: &mut [u8],
) -> Result<*mut T, ZeroCopyError> {
    // Base address for alignment check of T.
    let base_address = bytes.as_ptr() as usize + DISCRIMINATOR_LEN;
    if bytes.len() < std::mem::size_of::<T>() || base_address % std::mem::align_of::<T>() != 0 {
        return Err(ZeroCopyError::InvalidAccountSize);
    }

    Ok(bytes[DISCRIMINATOR_LEN..].as_mut_ptr() as *mut T)
}
