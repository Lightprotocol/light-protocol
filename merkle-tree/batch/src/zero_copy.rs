use bytemuck::Pod;
use light_hasher::Discriminator;
#[cfg(target_os = "solana")]
use solana_program::msg;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ZeroCopyError {
    #[error("Invalid Account size.")]
    InvalidAccountSize,
    #[error("Invalid Discriminator.")]
    InvalidDiscriminator,
}

#[cfg(feature = "solana")]
impl From<ZeroCopyError> for u32 {
    fn from(e: ZeroCopyError) -> u32 {
        match e {
            ZeroCopyError::InvalidAccountSize => 14301,
            ZeroCopyError::InvalidDiscriminator => 14302,
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

pub fn bytes_to_struct_checked<T: Clone + Copy + Pod + Discriminator, const INIT: bool>(
    bytes: &mut [u8],
) -> Result<*mut T, ZeroCopyError> {
    // Base address for alignment check of T.
    let base_address = bytes.as_ptr() as usize + DISCRIMINATOR_LEN;
    if bytes.len() < std::mem::size_of::<T>() || base_address % std::mem::align_of::<T>() != 0 {
        return Err(ZeroCopyError::InvalidAccountSize);
    }

    if INIT {
        if bytes[0..DISCRIMINATOR_LEN] != [0; DISCRIMINATOR_LEN] {
            #[cfg(target_os = "solana")]
            msg!("Discriminator bytes must be zero for initialization.");
            return Err(ZeroCopyError::InvalidDiscriminator);
        }
        bytes[0..DISCRIMINATOR_LEN].copy_from_slice(&T::DISCRIMINATOR);
    } else if T::DISCRIMINATOR != bytes[0..DISCRIMINATOR_LEN] {
        #[cfg(target_os = "solana")]
        msg!(
            "Expected discriminator: {:?}, actual {:?} ",
            T::DISCRIMINATOR,
            bytes[0..DISCRIMINATOR_LEN].to_vec()
        );
        return Err(ZeroCopyError::InvalidDiscriminator);
    }

    Ok(bytes[DISCRIMINATOR_LEN..].as_mut_ptr() as *mut T)
}
