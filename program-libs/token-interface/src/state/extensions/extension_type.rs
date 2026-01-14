use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
#[repr(u8)] // Note: token22 uses u16
pub enum ExtensionType {
    Placeholder0,
    Placeholder1,
    Placeholder2,
    Placeholder3,
    Placeholder4,
    Placeholder5,
    Placeholder6,
    Placeholder7,
    Placeholder8,
    Placeholder9,
    Placeholder10,
    Placeholder11,
    Placeholder12,
    Placeholder13,
    Placeholder14,
    Placeholder15,
    Placeholder16,
    Placeholder17,
    Placeholder18,
    /// Mint contains token-metadata.
    /// Unlike token22 there is no metadata pointer.
    TokenMetadata = 19,
    Placeholder20,
    Placeholder21,
    Placeholder22,
    Placeholder23,
    Placeholder24,
    Placeholder25,
    /// Reserved for Token-2022 Pausable compatibility
    Placeholder26,
    /// Marker extension indicating the account belongs to a pausable mint.
    /// When the SPL mint has PausableConfig and is paused, token operations are blocked.
    PausableAccount = 27,
    /// Marker extension indicating the account belongs to a mint with permanent delegate.
    /// When the SPL mint has PermanentDelegate extension, the delegate can transfer/burn any tokens.
    PermanentDelegateAccount = 28,
    /// Transfer fee extension storing withheld fees from transfers.
    TransferFeeAccount = 29,
    /// Marker extension indicating the account belongs to a mint with transfer hook.
    /// We only support mints where program_id is nil (no hook invoked).
    TransferHookAccount = 30,
    /// CompressedOnly extension for compressed token accounts.
    /// Marks account as decompress-only (cannot be transferred) and stores delegated amount.
    CompressedOnly = 31,
    /// Account contains compressible timing data and rent authority
    Compressible = 32,
}

impl TryFrom<u8> for ExtensionType {
    type Error = crate::TokenError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            19 => Ok(ExtensionType::TokenMetadata),
            27 => Ok(ExtensionType::PausableAccount),
            28 => Ok(ExtensionType::PermanentDelegateAccount),
            29 => Ok(ExtensionType::TransferFeeAccount),
            30 => Ok(ExtensionType::TransferHookAccount),
            31 => Ok(ExtensionType::CompressedOnly),
            32 => Ok(ExtensionType::Compressible),
            _ => Err(crate::TokenError::UnsupportedExtension),
        }
    }
}
