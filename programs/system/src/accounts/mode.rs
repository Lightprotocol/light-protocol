use crate::errors::SystemProgramError;

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum AccountMode {
    /// Deserialize optional accounts consistently with anchor.
    Anchor,
    /// Do not send optional accounts if not required.
    /// Use instruction data to signal whether an optional account is expected.
    Small,
}

impl TryFrom<u8> for AccountMode {
    type Error = SystemProgramError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(AccountMode::Anchor),
            1 => Ok(AccountMode::Small),
            _ => Err(SystemProgramError::InvalidAccountMode),
        }
    }
}

impl From<AccountMode> for u8 {
    fn from(value: AccountMode) -> Self {
        match value {
            AccountMode::Anchor => 0u8,
            AccountMode::Small => 1u8,
        }
    }
}
