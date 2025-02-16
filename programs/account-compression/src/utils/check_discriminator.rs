use anchor_lang::{error::ErrorCode, Owner, Result, ZeroCopy};

pub fn check_discriminator<T: ZeroCopy + Owner + std::fmt::Debug>(data: &[u8]) -> Result<()> {
    let disc_bytes: [u8; 8] = data[0..8].try_into().unwrap();
    if disc_bytes != T::discriminator() {
        return Err(ErrorCode::AccountDiscriminatorMismatch.into());
    }
    Ok(())
}
