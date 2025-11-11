use anchor_lang::prelude::*;

#[repr(u32)]
pub enum ErrorCode {
    InvalidAccountCount,
    InvalidRentRecipient,
    MintCreationFailed,
    MissingCompressedTokenProgram,
    MissingCompressedTokenProgramAuthorityPDA,
    RentRecipientMismatch,
    InvalidAccountDiscriminator,
    DerivedTokenAccountMismatch,
    MissingAuthority,
    MissingCpiContext,
}

#[automatically_derived]
impl ::core::fmt::Debug for ErrorCode {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(
            f,
            match self {
                ErrorCode::InvalidAccountCount => "InvalidAccountCount",
                ErrorCode::InvalidRentRecipient => "InvalidRentRecipient",
                ErrorCode::MintCreationFailed => "MintCreationFailed",
                ErrorCode::MissingCompressedTokenProgram => "MissingCompressedTokenProgram",
                ErrorCode::MissingCompressedTokenProgramAuthorityPDA => {
                    "MissingCompressedTokenProgramAuthorityPDA"
                }
                ErrorCode::RentRecipientMismatch => "RentRecipientMismatch",
                ErrorCode::InvalidAccountDiscriminator => "InvalidAccountDiscriminator",
                ErrorCode::DerivedTokenAccountMismatch => "DerivedTokenAccountMismatch",
                ErrorCode::MissingAuthority => "MissingAuthority",
                ErrorCode::MissingCpiContext => "MissingCpiContext",
            },
        )
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            ErrorCode::InvalidAccountCount => fmt.write_fmt(format_args!(
                "Invalid account count: PDAs and compressed accounts must match",
            )),
            ErrorCode::InvalidRentRecipient => {
                fmt.write_fmt(format_args!("Rent recipient does not match config"))
            }
            ErrorCode::MintCreationFailed => {
                fmt.write_fmt(format_args!("Failed to create compressed mint"))
            }
            ErrorCode::MissingCompressedTokenProgram => fmt.write_fmt(format_args!(
                "Compressed token program account not found in remaining accounts",
            )),
            ErrorCode::MissingCompressedTokenProgramAuthorityPDA => fmt.write_fmt(format_args!(
                "Compressed token program authority PDA account not found in remaining accounts",
            )),
            ErrorCode::RentRecipientMismatch => {
                fmt.write_fmt(format_args!("Rent recipient does not match config"))
            }
            ErrorCode::InvalidAccountDiscriminator => fmt.write_fmt(format_args!(
                "Trying to compress account with invalid discriminator"
            )),
            ErrorCode::DerivedTokenAccountMismatch => fmt.write_fmt(format_args!(
                "Derived token account address must match owner_info.key"
            )),
            ErrorCode::MissingAuthority => fmt.write_fmt(format_args!(
                "Authority account is missing from CPI accounts"
            )),
            ErrorCode::MissingCpiContext => fmt.write_fmt(format_args!(
                "CPI context account is missing from CPI accounts"
            )),
        }
    }
}

impl From<ErrorCode> for ProgramError {
    fn from(e: ErrorCode) -> Self {
        ProgramError::Custom(e as u32)
    }
}

#[repr(u32)]
pub enum CompressibleInstructionError {
    InvalidRentRecipient,
    CTokenDecompressionNotImplemented,
    PdaDecompressionNotImplemented,
    TokenCompressionNotImplemented,
    PdaCompressionNotImplemented,
}
