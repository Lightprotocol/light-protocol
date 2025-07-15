use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
#[repr(u16)]
pub enum ExtensionType {
    // /// Used as padding if the account size would otherwise be 355, same as a
    // /// multisig
    // Uninitialized,
    // /// Includes transfer fee rate info and accompanying authorities to withdraw
    // /// and set the fee
    // TransferFeeConfig,
    // /// Includes withheld transfer fees
    // TransferFeeAmount,
    // /// Includes an optional mint close authority
    // MintCloseAuthority,
    // /// Auditor configuration for confidential transfers
    // ConfidentialTransferMint,
    // /// State for confidential transfers
    // ConfidentialTransferAccount,
    // /// Specifies the default Account::state for new Accounts
    // DefaultAccountState,
    // /// Indicates that the Account owner authority cannot be changed
    // ImmutableOwner,
    // /// Require inbound transfers to have memo
    // MemoTransfer,
    // /// Indicates that the tokens from this mint can't be transferred
    // NonTransferable,
    // /// Tokens accrue interest over time,
    // InterestBearingConfig,
    // /// Locks privileged token operations from happening via CPI
    // CpiGuard,
    // /// Includes an optional permanent delegate
    // PermanentDelegate,
    // /// Indicates that the tokens in this account belong to a non-transferable
    // /// mint
    // NonTransferableAccount,
    // /// Mint requires a CPI to a program implementing the "transfer hook"
    // /// interface
    // TransferHook,
    // /// Indicates that the tokens in this account belong to a mint with a
    // /// transfer hook
    // TransferHookAccount,
    // /// Includes encrypted withheld fees and the encryption public that they are
    // /// encrypted under
    // ConfidentialTransferFeeConfig,
    // /// Includes confidential withheld transfer fees
    // ConfidentialTransferFeeAmount,
    /// Mint contains a pointer to another account (or the same account) that
    /// holds metadata. Must not point to itself.
    MetadataPointer = 18,
    /// Mint contains token-metadata.
    /// Unlike token22 there is no metadata pointer.
    TokenMetadata = 19,
    // /// Mint contains a pointer to another account (or the same account) that
    // /// holds group configurations
    // GroupPointer,
    // /// Mint contains token group configurations
    // TokenGroup,
    // /// Mint contains a pointer to another account (or the same account) that
    // /// holds group member configurations
    // GroupMemberPointer,
    // /// Mint contains token group member configurations
    // TokenGroupMember,
    // /// Mint allowing the minting and burning of confidential tokens
    // ConfidentialMintBurn,
    // /// Tokens whose UI amount is scaled by a given amount
    // ScaledUiAmount,
    // /// Tokens where minting / burning / transferring can be paused
    // Pausable,
    // /// Indicates that the account belongs to a pausable mint
    // PausableAccount,
}

impl TryFrom<u16> for ExtensionType {
    type Error = crate::CTokenError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            18 => Ok(ExtensionType::MetadataPointer),
            19 => Ok(ExtensionType::TokenMetadata),
            _ => Err(crate::CTokenError::UnsupportedExtension),
        }
    }
}
