use aligned_sized::aligned_sized;
use light_zero_copy::{ZeroCopy, ZeroCopyMut};
use spl_pod::solana_msg::msg;

use crate::{
    state::extensions::{
        CompressedOnlyExtension, CompressedOnlyExtensionConfig, CompressionInfo,
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
    /// Account contains compressible timing data and rent authority
    Compressible(CompressibleExtension),
}

#[derive(
    Debug,
    ZeroCopy,
    ZeroCopyMut,
    Clone,
    Copy,
    PartialEq,
    Hash,
    Eq,
    AnchorSerialize,
    AnchorDeserialize,
)]
#[repr(C)]
#[aligned_sized]
pub struct CompressibleExtension {
    pub compression_only: bool,
    /// Mint decimals (if has_decimals is set).
    /// Cached from mint at account creation for transfer_checked optimization.
    pub decimals: u8,
    /// 1 if decimals is set, 0 otherwise.
    /// Separate flag needed because decimals=0 is valid for some tokens.
    pub has_decimals: u8,
    pub info: CompressionInfo,
}

impl CompressibleExtension {
    /// Get cached decimals if set.
    /// Returns Some(decimals) if decimals were cached at account creation, None otherwise.
    pub fn get_decimals(&self) -> Option<u8> {
        if self.has_decimals != 0 {
            Some(self.decimals)
        } else {
            None
        }
    }
}

impl<'a> ZCompressibleExtensionMut<'a> {
    /// Get cached decimals if set.
    /// Returns Some(decimals) if decimals were cached at account creation, None otherwise.
    pub fn get_decimals(&self) -> Option<u8> {
        if self.has_decimals != 0 {
            Some(self.decimals)
        } else {
            None
        }
    }

    /// Set cached decimals from mint.
    /// Call this during account initialization when mint is available.
    pub fn set_decimals(&mut self, decimals: u8) {
        self.decimals = decimals;
        self.has_decimals = 1;
    }
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
    /// Account contains compressible timing data and rent authority
    Compressible(
        <CompressibleExtension as light_zero_copy::traits::ZeroCopyAtMut<'a>>::ZeroCopyAtMut,
    ),
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
        match discriminant {
            19 => {
                let (token_metadata, remaining_bytes) =
                    TokenMetadata::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::TokenMetadata(token_metadata),
                    remaining_bytes,
                ))
            }
            32 => {
                // Compressible variant (index 32 to avoid Token-2022 overlap)
                let (compressible_ext, remaining_bytes) =
                    CompressibleExtension::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::Compressible(compressible_ext),
                    remaining_bytes,
                ))
            }
            27 => {
                // PausableAccount variant (marker extension, no data)
                let (pausable_ext, remaining_bytes) =
                    PausableAccountExtension::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::PausableAccount(pausable_ext),
                    remaining_bytes,
                ))
            }
            28 => {
                // PermanentDelegateAccount variant (marker extension, no data)
                let (permanent_delegate_ext, remaining_bytes) =
                    PermanentDelegateAccountExtension::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::PermanentDelegateAccount(permanent_delegate_ext),
                    remaining_bytes,
                ))
            }
            29 => {
                // TransferFeeAccount variant
                let (transfer_fee_ext, remaining_bytes) =
                    TransferFeeAccountExtension::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::TransferFeeAccount(transfer_fee_ext),
                    remaining_bytes,
                ))
            }
            30 => {
                // TransferHookAccount variant
                let (transfer_hook_ext, remaining_bytes) =
                    TransferHookAccountExtension::zero_copy_at_mut(remaining_data)?;
                Ok((
                    ZExtensionStructMut::TransferHookAccount(transfer_hook_ext),
                    remaining_bytes,
                ))
            }
            31 => {
                // CompressedOnly variant
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
            ExtensionStructConfig::Compressible(config) => {
                // 1 byte for discriminant + CompressibleExtension size
                1 + CompressibleExtension::byte_len(config)?
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
                // Write discriminant (19 for TokenMetadata)
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = 19u8;

                let (token_metadata, remaining_bytes) =
                    TokenMetadata::new_zero_copy(&mut bytes[1..], config)?;
                Ok((
                    ZExtensionStructMut::TokenMetadata(token_metadata),
                    remaining_bytes,
                ))
            }
            ExtensionStructConfig::Compressible(config) => {
                // Write discriminant (32 for Compressible - avoids Token-2022 overlap)
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = 32u8;

                let (compressible_ext, remaining_bytes) =
                    CompressibleExtension::new_zero_copy(&mut bytes[1..], config)?;
                Ok((
                    ZExtensionStructMut::Compressible(compressible_ext),
                    remaining_bytes,
                ))
            }
            ExtensionStructConfig::PausableAccount(config) => {
                // Write discriminant (27 for PausableAccount)
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = 27u8;

                let (pausable_ext, remaining_bytes) =
                    PausableAccountExtension::new_zero_copy(&mut bytes[1..], config)?;
                Ok((
                    ZExtensionStructMut::PausableAccount(pausable_ext),
                    remaining_bytes,
                ))
            }
            ExtensionStructConfig::PermanentDelegateAccount(config) => {
                // Write discriminant (28 for PermanentDelegateAccount)
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = 28u8;

                let (permanent_delegate_ext, remaining_bytes) =
                    PermanentDelegateAccountExtension::new_zero_copy(&mut bytes[1..], config)?;
                Ok((
                    ZExtensionStructMut::PermanentDelegateAccount(permanent_delegate_ext),
                    remaining_bytes,
                ))
            }
            ExtensionStructConfig::TransferFeeAccount(config) => {
                // Write discriminant (29 for TransferFeeAccount)
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = 29u8;

                let (transfer_fee_ext, remaining_bytes) =
                    TransferFeeAccountExtension::new_zero_copy(&mut bytes[1..], config)?;
                Ok((
                    ZExtensionStructMut::TransferFeeAccount(transfer_fee_ext),
                    remaining_bytes,
                ))
            }
            ExtensionStructConfig::TransferHookAccount(config) => {
                // Write discriminant (30 for TransferHookAccount)
                if bytes.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        bytes.len(),
                    ));
                }
                bytes[0] = 30u8;

                let (transfer_hook_ext, remaining_bytes) =
                    TransferHookAccountExtension::new_zero_copy(&mut bytes[1..], config)?;
                Ok((
                    ZExtensionStructMut::TransferHookAccount(transfer_hook_ext),
                    remaining_bytes,
                ))
            }
            ExtensionStructConfig::CompressedOnly(config) => {
                // Write discriminant (31 for CompressedOnly)
                if bytes.len() < 1 + CompressedOnlyExtension::LEN {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1 + CompressedOnlyExtension::LEN,
                        bytes.len(),
                    ));
                }
                bytes[0] = 31u8;

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

#[derive(Debug, Clone, PartialEq)]
pub enum ExtensionStructConfig {
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
    Compressible(CompressibleExtensionConfig),
}
