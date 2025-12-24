use core::ops::Deref;

use aligned_sized::aligned_sized;
use light_compressed_account::Pubkey;
use light_compressible::compression_info::CompressionInfo;
use light_program_profiler::profile;
use light_zero_copy::{
    traits::{ZeroCopyAt, ZeroCopyAtMut},
    ZeroCopy, ZeroCopyMut, ZeroCopyNew,
};
use spl_pod::solana_msg::msg;

use super::compressed_mint::{CompressedMintMetadata, ACCOUNT_TYPE_MINT};
use crate::{
    instructions::mint_action::CompressedMintInstructionData,
    state::{
        CompressedMint, ExtensionStruct, ExtensionStructConfig, TokenDataVersion, ZExtensionStruct,
        ZExtensionStructMut,
    },
    AnchorDeserialize, AnchorSerialize, CTokenError, BASE_TOKEN_ACCOUNT_SIZE,
};

/// Optimized CompressedMint zero copy struct.
/// Uses derive macros to generate ZCompressedMintZeroCopyMeta<'a> and ZCompressedMintZeroCopyMetaMut<'a>.
#[derive(
    Debug, PartialEq, Eq, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
#[aligned_sized]
struct CompressedMintZeroCopyMeta {
    // BaseMint fields with flattened COptions (SPL format: 4 bytes discriminator + 32 bytes pubkey)
    mint_authority_option_prefix: u32,
    mint_authority: Pubkey,
    /// Total supply of tokens.
    pub supply: u64,
    /// Number of base 10 digits to the right of the decimal place.
    pub decimals: u8,
    /// Is initialized - for SPL compatibility
    pub is_initialized: u8,
    freeze_authority_option_prefix: u32,
    freeze_authority: Pubkey,
    // CompressedMintMetadata
    pub metadata: CompressedMintMetadata,
    /// Reserved bytes for T22 layout compatibility (padding to reach byte 165)
    pub reserved: [u8; 49],
    /// Account type discriminator at byte 165 (1 = Mint, 2 = Account)
    pub account_type: u8,
    /// Compression info embedded directly in the mint
    pub compression: CompressionInfo,
    /// Extensions flag
    has_extensions: bool,
}

/// Zero-copy view of CompressedMint with base and optional extensions
#[derive(Debug)]
pub struct ZCompressedMint<'a> {
    pub base: ZCompressedMintZeroCopyMeta<'a>,
    pub extensions: Option<Vec<ZExtensionStruct<'a>>>,
}

/// Mutable zero-copy view of CompressedMint with base and optional extensions
#[derive(Debug)]
pub struct ZCompressedMintMut<'a> {
    pub base: ZCompressedMintZeroCopyMetaMut<'a>,
    pub extensions: Option<Vec<ZExtensionStructMut<'a>>>,
}

/// Configuration for creating a new CompressedMint via ZeroCopyNew
#[derive(Debug, Clone, PartialEq)]
pub struct CompressedMintConfig {
    /// Extension configurations
    pub extensions: Option<Vec<ExtensionStructConfig>>,
}

impl<'a> ZeroCopyNew<'a> for CompressedMint {
    type ZeroCopyConfig = CompressedMintConfig;
    type Output = ZCompressedMintMut<'a>;

    fn byte_len(
        config: &Self::ZeroCopyConfig,
    ) -> Result<usize, light_zero_copy::errors::ZeroCopyError> {
        // Use derived byte_len for meta struct
        let meta_config = CompressedMintZeroCopyMetaConfig {
            metadata: (),
            compression: light_compressible::compression_info::CompressionInfoConfig {
                rent_config: (),
            },
        };
        let mut size = CompressedMintZeroCopyMeta::byte_len(&meta_config)?;

        // Add extension sizes if present
        if let Some(ref extensions) = config.extensions {
            // Vec length prefix (4 bytes) + each extension's size
            size += 4;
            for ext_config in extensions {
                size += ExtensionStruct::byte_len(ext_config)?;
            }
        }

        Ok(size)
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        // Use derived new_zero_copy for meta struct
        let meta_config = CompressedMintZeroCopyMetaConfig {
            metadata: (),
            compression: light_compressible::compression_info::CompressionInfoConfig {
                rent_config: (),
            },
        };
        let (mut base, remaining) =
            <CompressedMintZeroCopyMeta as ZeroCopyNew<'a>>::new_zero_copy(bytes, meta_config)?;
        *base.account_type = ACCOUNT_TYPE_MINT;
        base.is_initialized = 1;

        // Initialize extensions if present
        if let Some(extensions_config) = config.extensions {
            *base.has_extensions = 1u8;
            let (extensions, remaining) = <Vec<ExtensionStruct> as ZeroCopyNew<'a>>::new_zero_copy(
                remaining,
                extensions_config,
            )?;

            Ok((
                ZCompressedMintMut {
                    base,
                    extensions: Some(extensions),
                },
                remaining,
            ))
        } else {
            Ok((
                ZCompressedMintMut {
                    base,
                    extensions: None,
                },
                remaining,
            ))
        }
    }
}

impl<'a> ZeroCopyAt<'a> for CompressedMint {
    type ZeroCopyAt = ZCompressedMint<'a>;

    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(Self::ZeroCopyAt, &'a [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (base, bytes) = <CompressedMintZeroCopyMeta as ZeroCopyAt<'a>>::zero_copy_at(bytes)?;
        // has_extensions already consumed the Option discriminator byte
        if base.has_extensions() {
            let (extensions, bytes) =
                <Vec<ExtensionStruct> as ZeroCopyAt<'a>>::zero_copy_at(bytes)?;
            Ok((
                ZCompressedMint {
                    base,
                    extensions: Some(extensions),
                },
                bytes,
            ))
        } else {
            Ok((
                ZCompressedMint {
                    base,
                    extensions: None,
                },
                bytes,
            ))
        }
    }
}

impl<'a> ZeroCopyAtMut<'a> for CompressedMint {
    type ZeroCopyAtMut = ZCompressedMintMut<'a>;

    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (base, bytes) =
            <CompressedMintZeroCopyMeta as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)?;
        // has_extensions already consumed the Option discriminator byte
        if base.has_extensions() {
            let (extensions, bytes) =
                <Vec<ExtensionStruct> as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)?;
            Ok((
                ZCompressedMintMut {
                    base,
                    extensions: Some(extensions),
                },
                bytes,
            ))
        } else {
            Ok((
                ZCompressedMintMut {
                    base,
                    extensions: None,
                },
                bytes,
            ))
        }
    }
}

// Deref implementations for field access
impl<'a> Deref for ZCompressedMint<'a> {
    type Target = ZCompressedMintZeroCopyMeta<'a>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<'a> Deref for ZCompressedMintMut<'a> {
    type Target = ZCompressedMintZeroCopyMetaMut<'a>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

// Getters on ZCompressedMintZeroCopyMeta (immutable)
impl ZCompressedMintZeroCopyMeta<'_> {
    /// Checks if account_type matches CMint discriminator value
    #[inline(always)]
    pub fn is_cmint_account(&self) -> bool {
        self.account_type == ACCOUNT_TYPE_MINT
    }

    /// Checks if account is initialized
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.is_initialized != 0
    }

    /// Get mint_authority if set (COption discriminator == 1)
    pub fn mint_authority(&self) -> Option<&Pubkey> {
        if u32::from(self.mint_authority_option_prefix) == 1 {
            Some(&self.mint_authority)
        } else {
            None
        }
    }

    /// Get freeze_authority if set (COption discriminator == 1)
    pub fn freeze_authority(&self) -> Option<&Pubkey> {
        if u32::from(self.freeze_authority_option_prefix) == 1 {
            Some(&self.freeze_authority)
        } else {
            None
        }
    }
}

// Getters on ZCompressedMintZeroCopyMetaMut (mutable)
impl ZCompressedMintZeroCopyMetaMut<'_> {
    /// Checks if account_type matches CMint discriminator value
    #[inline(always)]
    pub fn is_cmint_account(&self) -> bool {
        *self.account_type == ACCOUNT_TYPE_MINT
    }

    /// Checks if account is initialized
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.is_initialized == 1
    }

    /// Get mint_authority if set (COption discriminator == 1)
    pub fn mint_authority(&self) -> Option<&Pubkey> {
        if u32::from(self.mint_authority_option_prefix) == 1 {
            Some(&self.mint_authority)
        } else {
            None
        }
    }

    /// Set mint_authority using COption format
    pub fn set_mint_authority(&mut self, pubkey: Option<Pubkey>) {
        if let Some(pubkey) = pubkey {
            self.mint_authority_option_prefix = 1u32.into();
            self.mint_authority = pubkey;
        } else {
            self.mint_authority_option_prefix = 0u32.into();
            self.mint_authority = Pubkey::default();
        }
    }

    /// Get freeze_authority if set (COption discriminator == 1)
    pub fn freeze_authority(&self) -> Option<&Pubkey> {
        if u32::from(self.freeze_authority_option_prefix) == 1 {
            Some(&self.freeze_authority)
        } else {
            None
        }
    }

    /// Set freeze_authority using COption format
    pub fn set_freeze_authority(&mut self, pubkey: Option<Pubkey>) {
        if let Some(pubkey) = pubkey {
            self.freeze_authority_option_prefix = 1u32.into();
            self.freeze_authority = pubkey;
        } else {
            self.freeze_authority_option_prefix = 0u32.into();
            self.freeze_authority = Pubkey::default();
        }
    }
}

// Checked methods on CompressedMint
impl CompressedMint {
    /// Zero-copy deserialization with initialization and account_type check.
    /// Returns an error if:
    /// - Account is not initialized (is_initialized == false)
    /// - Account type is not ACCOUNT_TYPE_MINT (byte 165 != 1)
    #[profile]
    pub fn zero_copy_at_checked(bytes: &[u8]) -> Result<(ZCompressedMint<'_>, &[u8]), CTokenError> {
        // Check minimum size for account_type at byte 165
        if bytes.len() < BASE_TOKEN_ACCOUNT_SIZE as usize {
            return Err(CTokenError::InvalidAccountData);
        }

        // Proceed with deserialization first
        let (mint, remaining) = CompressedMint::zero_copy_at(bytes)
            .map_err(|_| CTokenError::CMintDeserializationFailed)?;

        // Verify account_type using the method
        if !mint.is_cmint_account() {
            return Err(CTokenError::InvalidAccountType);
        }

        // Check is_initialized
        if !mint.is_initialized() {
            return Err(CTokenError::CMintNotInitialized);
        }

        Ok((mint, remaining))
    }

    /// Mutable zero-copy deserialization with initialization and account_type check.
    /// Returns an error if:
    /// - Account is not initialized (is_initialized == false)
    /// - Account type is not ACCOUNT_TYPE_MINT
    #[profile]
    pub fn zero_copy_at_mut_checked(
        bytes: &mut [u8],
    ) -> Result<(ZCompressedMintMut<'_>, &mut [u8]), CTokenError> {
        // Check minimum size
        if bytes.len() < BASE_TOKEN_ACCOUNT_SIZE as usize {
            msg!(
                "zero_copy_at_checked bytes.len() < BASE_TOKEN_ACCOUNT_SIZE {}",
                bytes.len()
            );
            return Err(CTokenError::InvalidAccountData);
        }

        let (mint, remaining) = CompressedMint::zero_copy_at_mut(bytes)
            .map_err(|_| CTokenError::CMintDeserializationFailed)?;

        if !mint.is_initialized() {
            return Err(CTokenError::CMintNotInitialized);
        }
        if !mint.is_cmint_account() {
            return Err(CTokenError::InvalidAccountType);
        }

        Ok((mint, remaining))
    }
}

// Helper methods on ZCompressedMint
impl ZCompressedMint<'_> {
    /// Checks if account_type matches CMint discriminator value
    #[inline(always)]
    pub fn is_cmint_account(&self) -> bool {
        self.base.is_cmint_account()
    }

    /// Checks if account is initialized
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.base.is_initialized()
    }
}

// Helper methods on ZCompressedMintMut
impl ZCompressedMintMut<'_> {
    /// Checks if account_type matches CMint discriminator value
    #[inline(always)]
    pub fn is_cmint_account(&self) -> bool {
        self.base.is_cmint_account()
    }

    /// Checks if account is initialized
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.base.is_initialized()
    }

    /// Set all fields of the CompressedMint struct at once
    #[inline]
    #[profile]
    pub fn set(
        &mut self,
        ix_data: &<CompressedMintInstructionData as light_zero_copy::traits::ZeroCopyAt<'_>>::ZeroCopyAt,
        cmint_decompressed: bool,
    ) -> Result<(), CTokenError> {
        if ix_data.metadata.version != TokenDataVersion::ShaFlat as u8 {
            #[cfg(feature = "solana")]
            msg!(
                "Only shaflat version 3 is supported got {}",
                ix_data.metadata.version
            );
            return Err(CTokenError::InvalidTokenMetadataVersion);
        }
        // Set metadata fields from instruction data
        self.base.metadata.version = ix_data.metadata.version;
        self.base.metadata.mint = ix_data.metadata.mint;
        self.base.metadata.cmint_decompressed = if cmint_decompressed { 1 } else { 0 };

        // Set base fields
        self.base.supply = ix_data.supply;
        self.base.decimals = ix_data.decimals;
        self.base.is_initialized = 1; // Always initialized for compressed mints

        if let Some(mint_authority) = ix_data.mint_authority.as_deref() {
            self.base.set_mint_authority(Some(*mint_authority));
        }
        // Set freeze authority using COption format
        if let Some(freeze_authority) = ix_data.freeze_authority.as_deref() {
            self.base.set_freeze_authority(Some(*freeze_authority));
        }

        // account_type is already set in new_zero_copy
        // extensions are handled separately
        Ok(())
    }
}
