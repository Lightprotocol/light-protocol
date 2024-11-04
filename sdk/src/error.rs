use anchor_lang::prelude::error_code;

#[error_code]
pub enum LightSdkError {
    #[msg("Constraint violation")]
    ConstraintViolation,
    #[msg("Invalid light-system-program ID")]
    InvalidLightSystemProgram,
    #[msg("Expected accounts in the instruction")]
    ExpectedAccounts,
    #[msg("Expected root index to be provided")]
    ExpectedRootIndex,
    #[msg("Address parameters are expected when deriving the address")]
    ExpectedAddressParams,
    #[msg("Expected address root index to be provided")]
    ExpectedAddressRootIndex,
    #[msg("Accounts with a specified input are expected to have data")]
    ExpectedData,
    #[msg("Accounts with specified data are expected to have a discriminator")]
    ExpectedDiscriminator,
    #[msg("Accounts with specified data are expected to have a hash")]
    ExpectedHash,
    #[msg("Cannot transfer lamports from an account without input")]
    TransferFromNoInput,
    #[msg("Cannot transfer from an account without lamports")]
    TransferFromNoLamports,
    #[msg("Account, from which a transfer was attempted, has insufficient amount of lamports")]
    TransferFromInsufficientLamports,
    #[msg("Integer overflow resulting from too large resulting amount")]
    TransferIntegerOverflow,
}
