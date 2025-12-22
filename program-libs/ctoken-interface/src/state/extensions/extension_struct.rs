use light_zero_copy::ZeroCopy;
use spl_pod::solana_msg::msg;

use crate::{
    state::extensions::{
        CompressedOnlyExtension, CompressedOnlyExtensionConfig, ExtensionType,
        PausableAccountExtension, PausableAccountExtensionConfig,
        PermanentDelegateAccountExtension, PermanentDelegateAccountExtensionConfig, TokenMetadata,
        TokenMetadataConfig, TransferFeeAccountExtension, TransferFeeAccountExtensionConfig,
        TransferHookAccountExtension, TransferHookAccountExtensionConfig,
        ZPausableAccountExtensionMut, ZPermanentDelegateAccountExtensionMut, ZTokenMetadataMut,
        ZTransferFeeAccountExtensionMut, ZTransferHookAccountExtensionMut,
    },
    AnchorDeserialize, AnchorSerialize,
};

#[derive(Debug, Clone, Hash, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
#[repr(C)]
pub enum ExtensionStruct {
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
    TokenMetadata(TokenMetadata),
    Placeholder20,
    Placeholder21,
    Placeholder22,
    Placeholder23,
    Placeholder24,
    Placeholder25,
    /// Reserved for Token-2022 Pausable compatibility
    Placeholder26,
    /// Marker extension indicating the account belongs to a pausable mint
    PausableAccount(PausableAccountExtension),
    /// Marker extension indicating the account belongs to a mint with permanent delegate
    PermanentDelegateAccount(PermanentDelegateAccountExtension),
    /// Transfer fee extension storing withheld fees from transfers
    TransferFeeAccount(TransferFeeAccountExtension),
    /// Marker extension indicating the account belongs to a mint with transfer hook
    TransferHookAccount(TransferHookAccountExtension),
    /// CompressedOnly extension for compressed token accounts (stores delegated amount)
    CompressedOnly(CompressedOnlyExtension),
    /// Reserved - CompressionInfo is now embedded directly in CToken and CompressedMint structs
    Placeholder32,
}

#[derive(Debug)]
pub enum ZExtensionStructMut<'a> {
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
    TokenMetadata(ZTokenMetadataMut<'a>),
    Placeholder20,
    Placeholder21,
    Placeholder22,
    Placeholder23,
    Placeholder24,
    Placeholder25,
    /// Reserved for Token-2022 Pausable compatibility
    Placeholder26,
    /// Marker extension indicating the account belongs to a pausable mint
    PausableAccount(ZPausableAccountExtensionMut<'a>),
    /// Marker extension indicating the account belongs to a mint with permanent delegate
    PermanentDelegateAccount(ZPermanentDelegateAccountExtensionMut<'a>),
    /// Transfer fee extension storing withheld fees from transfers
    TransferFeeAccount(ZTransferFeeAccountExtensionMut<'a>),
    /// Marker extension indicating the account belongs to a mint with transfer hook
    TransferHookAccount(ZTransferHookAccountExtensionMut<'a>),
    /// CompressedOnly extension for compressed token accounts
    CompressedOnly(
        <CompressedOnlyExtension as light_zero_copy::traits::ZeroCopyAtMut<'a>>::ZeroCopyAtMut,
    ),
    /// Reserved - CompressionInfo is now embedded directly in CToken and CompressedMint structs
    Placeholder32,
}

impl<'a> light_zero_copy::traits::ZeroCopyAtMut<'a> for ExtensionStruct {
    type ZeroCopyAtMut = ZExtensionStructMut<'a>;

    fn zero_copy_at_mut(
        data: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        // Read discriminant (first 1 byte for borsh enum)
        if data.is_empty() {
            return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                1,
                data.len(),
            ));
        }

        let discriminant = data[0];
        let remaining_data = &mut data[1..];
        let extension_type = ExtensionType::try_from(discriminant)
            .map_err(|_| light_zero_copy::errors::ZeroCopyError::InvalidConversion)?;

        match extension_type {
            ExtensionType::TokenMetadata => {
                let (token_metadata, remaining_bytes) =
                    TokenMetadata::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::TokenMetadata(token_metadata),
                    remaining_bytes,
                ))
            }
            ExtensionType::PausableAccount => {
                let (pausable_ext, remaining_bytes) =
                    PausableAccountExtension::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::PausableAccount(pausable_ext),
                    remaining_bytes,
                ))
            }
            ExtensionType::PermanentDelegateAccount => {
                let (permanent_delegate_ext, remaining_bytes) =
                    PermanentDelegateAccountExtension::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::PermanentDelegateAccount(permanent_delegate_ext),
                    remaining_bytes,
                ))
            }
            ExtensionType::TransferFeeAccount => {
                let (transfer_fee_ext, remaining_bytes) =
                    TransferFeeAccountExtension::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::TransferFeeAccount(transfer_fee_ext),
                    remaining_bytes,
                ))
            }
            ExtensionType::TransferHookAccount => {
                let (transfer_hook_ext, remaining_bytes) =
                    TransferHookAccountExtension::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::TransferHookAccount(transfer_hook_ext),
                    remaining_bytes,
                ))
            }
            ExtensionType::CompressedOnly => {
                let (compressed_only_ext, remaining_bytes) =
                    CompressedOnlyExtension::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::CompressedOnly(compressed_only_ext),
                    remaining_bytes,
                ))
            }
            _ => Err(light_zero_copy::errors::ZeroCopyError::InvalidConversion),
        }
    }
}

impl<'a> light_zero_copy::ZeroCopyNew<'a> for ExtensionStruct {
    type ZeroCopyConfig = ExtensionStructConfig;
    type Output = ZExtensionStructMut<'a>;

    fn byte_len(
        config: &Self::ZeroCopyConfig,
    ) -> Result<usize, light_zero_copy::errors::ZeroCopyError> {
        Ok(match config {
            ExtensionStructConfig::TokenMetadata(token_metadata_config) => {
                // 1 byte for discriminant + TokenMetadata size
                1 + TokenMetadata::byte_len(token_metadata_config)?
            }
            ExtensionStructConfig::PausableAccount(config) => {
                // 1 byte for discriminant + 0 bytes for marker extension
                1 + PausableAccountExtension::byte_len(config)?
            }
            ExtensionStructConfig::PermanentDelegateAccount(config) => {
                // 1 byte for discriminant + 0 bytes for marker extension
                1 + PermanentDelegateAccountExtension::byte_len(config)?
            }
            ExtensionStructConfig::TransferFeeAccount(config) => {
                // 1 byte for discriminant + 8 bytes for withheld_amount
                1 + TransferFeeAccountExtension::byte_len(config)?
            }
            ExtensionStructConfig::TransferHookAccount(config) => {
                // 1 byte for discriminant + 1 byte for transferring flag
                1 + TransferHookAccountExtension::byte_len(config)?
            }
            ExtensionStructConfig::CompressedOnly(_) => {
                // 1 byte for discriminant + 16 bytes for CompressedOnlyExtension (2 * u64)
                1 + CompressedOnlyExtension::LEN
            }
            _ => {
                msg!("Invalid extension type returning");
                return Err(light_zero_copy::errors::ZeroCopyError::InvalidConversion);
            }
        })
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        match config {
            ExtensionStructConfig::TokenMetadata(config) => {
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = ExtensionType::TokenMetadata as u8;

                let (token_metadata, remaining_bytes) =
                    TokenMetadata::new_zero_copy(&mut bytes[1..], config)?;
                Ok((
                    ZExtensionStructMut::TokenMetadata(token_metadata),
                    remaining_bytes,
                ))
            }
            ExtensionStructConfig::PausableAccount(config) => {
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = ExtensionType::PausableAccount as u8;

                let (pausable_ext, remaining_bytes) =
                    PausableAccountExtension::new_zero_copy(&mut bytes[1..], config)?;
                Ok((
                    ZExtensionStructMut::PausableAccount(pausable_ext),
                    remaining_bytes,
                ))
            }
            ExtensionStructConfig::PermanentDelegateAccount(config) => {
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = ExtensionType::PermanentDelegateAccount as u8;

                let (permanent_delegate_ext, remaining_bytes) =
                    PermanentDelegateAccountExtension::new_zero_copy(&mut bytes[1..], config)?;
                Ok((
                    ZExtensionStructMut::PermanentDelegateAccount(permanent_delegate_ext),
                    remaining_bytes,
                ))
            }
            ExtensionStructConfig::TransferFeeAccount(config) => {
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = ExtensionType::TransferFeeAccount as u8;

                let (transfer_fee_ext, remaining_bytes) =
                    TransferFeeAccountExtension::new_zero_copy(&mut bytes[1..], config)?;
                Ok((
                    ZExtensionStructMut::TransferFeeAccount(transfer_fee_ext),
                    remaining_bytes,
                ))
            }
            ExtensionStructConfig::TransferHookAccount(config) => {
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = ExtensionType::TransferHookAccount as u8;

                let (transfer_hook_ext, remaining_bytes) =
                    TransferHookAccountExtension::new_zero_copy(&mut bytes[1..], config)?;
                Ok((
                    ZExtensionStructMut::TransferHookAccount(transfer_hook_ext),
                    remaining_bytes,
                ))
            }
            ExtensionStructConfig::CompressedOnly(config) => {
                if bytes.len() < 1 + CompressedOnlyExtension::LEN {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1 + CompressedOnlyExtension::LEN,
                        bytes.len(),
                    ));
                }
                bytes[0] = ExtensionType::CompressedOnly as u8;

                let (compressed_only_ext, remaining_bytes) =
                    CompressedOnlyExtension::new_zero_copy(&mut bytes[1..], config)?;
                Ok((
                    ZExtensionStructMut::CompressedOnly(compressed_only_ext),
                    remaining_bytes,
                ))
            }
            _ => Err(light_zero_copy::errors::ZeroCopyError::InvalidConversion),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ExtensionStructConfig {
    #[default]
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
    Placeholder18, // MetadataPointer(MetadataPointerConfig),
    TokenMetadata(TokenMetadataConfig),
    Placeholder20,
    Placeholder21,
    Placeholder22,
    Placeholder23,
    Placeholder24,
    Placeholder25,
    /// Reserved for Token-2022 Pausable compatibility
    Placeholder26,
    PausableAccount(PausableAccountExtensionConfig),
    PermanentDelegateAccount(PermanentDelegateAccountExtensionConfig),
    TransferFeeAccount(TransferFeeAccountExtensionConfig),
    TransferHookAccount(TransferHookAccountExtensionConfig),
    CompressedOnly(CompressedOnlyExtensionConfig),
    /// Reserved - CompressionInfo is now embedded directly in CToken and CompressedMint structs
    Placeholder32,
}
