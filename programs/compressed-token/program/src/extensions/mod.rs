use anchor_compressed_token::ErrorCode;
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::ZeroCopyNew;

pub mod instruction_data;
pub use instruction_data::{ExtensionInstructionData, ZExtensionInstructionData};
pub mod metadata_pointer;
pub mod processor;
pub mod state;
pub mod token_metadata;

use metadata_pointer::{MetadataPointer, MetadataPointerConfig};
use state::ExtensionStructConfig;
use token_metadata::{AdditionalMetadataConfig, MetadataConfig, TokenMetadata, TokenMetadataConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
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
    type Error = ErrorCode;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            18 => Ok(ExtensionType::MetadataPointer),
            19 => Ok(ExtensionType::TokenMetadata),
            _ => Err(ErrorCode::InvalidExtensionType),
        }
    }
}

/// Processes extension instruction data and returns the configuration tuple and additional data length
/// Returns: (has_extensions, extension_configs, additional_data_len)
pub fn process_extensions_config(
    extensions: Option<&Vec<ZExtensionInstructionData>>,
) -> (bool, Vec<ExtensionStructConfig>, usize) {
    if let Some(extensions) = extensions {
        let mut additional_mint_data_len = 0;
        let mut config_vec = Vec::new();
        
        for extension in extensions.iter() {
            match extension {
                ZExtensionInstructionData::MetadataPointer(extension) => {
                    let config = MetadataPointerConfig {
                        authority: (extension.authority.is_some(), ()),
                        metadata_address: (extension.metadata_address.is_some(), ()),
                    };
                    let byte_len = MetadataPointer::byte_len(&config);
                    additional_mint_data_len += byte_len;
                    config_vec.push(ExtensionStructConfig::MetadataPointer(config));
                }
                ZExtensionInstructionData::TokenMetadata(token_metadata_data) => {
                    let additional_metadata_configs = if let Some(ref additional_metadata) =
                        token_metadata_data.additional_metadata
                    {
                        additional_metadata
                            .iter()
                            .map(|item| AdditionalMetadataConfig {
                                key: item.key.len() as u32,
                                value: item.value.len() as u32,
                            })
                            .collect()
                    } else {
                        vec![]
                    };

                    let config = TokenMetadataConfig {
                        update_authority: (token_metadata_data.update_authority.is_some(), ()),
                        metadata: MetadataConfig {
                            name: token_metadata_data.metadata.name.len() as u32,
                            symbol: token_metadata_data.metadata.symbol.len() as u32,
                            uri: token_metadata_data.metadata.uri.len() as u32,
                        },
                        additional_metadata: additional_metadata_configs,
                    };
                    let byte_len = TokenMetadata::byte_len(&config);
                    additional_mint_data_len += byte_len;
                    config_vec.push(ExtensionStructConfig::TokenMetadata(config));
                }
            }
        }
        (true, config_vec, additional_mint_data_len)
    } else {
        (false, Vec::new(), 0)
    }
}

