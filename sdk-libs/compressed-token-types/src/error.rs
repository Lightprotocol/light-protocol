use thiserror::Error;

pub type Result<T> = std::result::Result<T, LightTokenSdkTypeError>;

#[derive(Debug, Error)]
pub enum LightTokenSdkTypeError {
    #[error("CPI accounts index out of bounds: {0}")]
    CpiAccountsIndexOutOfBounds(usize),
}