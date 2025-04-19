use anchor_lang::{error::ErrorCode, Owner, Result, ZeroCopy};

pub fn check_discriminator<T: ZeroCopy + Owner + std::fmt::Debug>(data: &[u8]) -> Result<()> {
    if &data[..8] != T::DISCRIMINATOR {
        return Err(ErrorCode::AccountDiscriminatorMismatch.into());
    }
    Ok(())
}
