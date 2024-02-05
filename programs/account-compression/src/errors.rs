use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("InvalidAuthority")]
    InvalidAuthority,
    #[msg("InvalidVerifier")]
    InvalidVerifier,
    #[msg(
        "Leaves <> remaining accounts missmatch. The number of remaining accounts must match the number of leaves."
    )]
    NumberOfLeavesMismatch,
    #[msg("Integer overflow, value too large")]
    IntegerOverflow,
    #[msg("Provided noop program public key is invalid")]
    InvalidNoopPubkey,
    #[msg("Emitting an event requires at least one changelog entry")]
    EventNoChangelogEntry,
}
