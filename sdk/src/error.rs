use anchor_lang::prelude::error_code;

#[error_code]
pub enum LightSdkError {
    #[msg("Constraint violation")]
    ConstraintViolation,
    #[msg("Invalid light-system-program ID")]
    InvalidLightSystemProgram,
}
