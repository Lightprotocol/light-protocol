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

use super::compressed_mint::{MintMetadata, ACCOUNT_TYPE_MINT, IS_INITIALIZED_OFFSET};
use crate::{
    instructions::mint_action::MintInstructionData,
    state::{
        ExtensionStruct, ExtensionStructConfig, Mint, TokenDataVersion, ZExtensionStruct,
        ZExtensionStructMut,
    },
    AnchorDeserialize, AnchorSerialize, TokenError,
};

/// Base size for Mint accounts (without extensions)
pub const BASE_MINT_ACCOUNT_SIZE: u64 = MintZeroCopyMeta::LEN as u64;

/// Optimized Mint zero copy struct.
/// Uses derive macros to generate ZMintZeroCopyMeta<'a> and ZMintZeroCopyMetaMut<'a>.
#[derive(
    Debug, PartialEq, Eq, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
#[aligned_sized]
struct MintZeroCopyMeta {
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
    // MintMetadata
    pub metadata: MintMetadata,
    /// Reserved bytes for T22 layout compatibility (padding to reach byte 165)
    pub reserved: [u8; 16],
    /// Account type discriminator at byte 165 (1 = Mint, 2 = Account)
    pub account_type: u8,
    /// Compression info embedded directly in the mint
    pub compression: CompressionInfo,
    /// Extensions flag
    has_extensions: bool,
}

/// Zero-copy view of Mint with base and optional extensions
#[derive(Debug)]
pub struct ZMint<'a> {
    pub base: ZMintZeroCopyMeta<'a>,
    pub extensions: Option<Vec<ZExtensionStruct<'a>>>,
}

/// Mutable zero-copy view of Mint with base and optional extensions
#[derive(Debug)]
pub struct ZMintMut<'a> {
    pub base: ZMintZeroCopyMetaMut<'a>,
    pub extensions: Option<Vec<ZExtensionStructMut<'a>>>,
}

/// Configuration for creating a new Mint via ZeroCopyNew
#[derive(Debug, Clone, PartialEq)]
pub struct MintConfig {
    /// Extension configurations
    pub extensions: Option<Vec<ExtensionStructConfig>>,
}

impl<'a> ZeroCopyNew<'a> for Mint {
    type ZeroCopyConfig = MintConfig;
    type Output = ZMintMut<'a>;

    fn byte_len(
        config: &Self::ZeroCopyConfig,
    ) -> Result<usize, light_zero_copy::errors::ZeroCopyError> {
        // Use derived byte_len for meta struct
        let meta_config = MintZeroCopyMetaConfig {
            metadata: (),
            compression: light_compressible::compression_info::CompressionInfoConfig {
                rent_config: (),
            },
        };
        let mut size = MintZeroCopyMeta::byte_len(&meta_config)?;

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
        // Check that the account is not already initialized
        if bytes.len() > IS_INITIALIZED_OFFSET && bytes[IS_INITIALIZED_OFFSET] != 0 {
            return Err(light_zero_copy::errors::ZeroCopyError::MemoryNotZeroed);
        }
        // Use derived new_zero_copy for meta struct
        let meta_config = MintZeroCopyMetaConfig {
            metadata: (),
            compression: light_compressible::compression_info::CompressionInfoConfig {
                rent_config: (),
            },
        };
        let (mut base, remaining) =
            <MintZeroCopyMeta as ZeroCopyNew<'a>>::new_zero_copy(bytes, meta_config)?;
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
                ZMintMut {
                    base,
                    extensions: Some(extensions),
                },
                remaining,
            ))
        } else {
            Ok((
                ZMintMut {
                    base,
                    extensions: None,
                },
                remaining,
            ))
        }
    }
}

impl<'a> ZeroCopyAt<'a> for Mint {
    type ZeroCopyAt = ZMint<'a>;

    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(Self::ZeroCopyAt, &'a [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (base, bytes) = <MintZeroCopyMeta as ZeroCopyAt<'a>>::zero_copy_at(bytes)?;
        // has_extensions already consumed the Option discriminator byte
        if base.has_extensions() {
            let (extensions, bytes) =
                <Vec<ExtensionStruct> as ZeroCopyAt<'a>>::zero_copy_at(bytes)?;
            Ok((
                ZMint {
                    base,
                    extensions: Some(extensions),
                },
                bytes,
            ))
        } else {
            Ok((
                ZMint {
                    base,
                    extensions: None,
                },
                bytes,
            ))
        }
    }
}

impl<'a> ZeroCopyAtMut<'a> for Mint {
    type ZeroCopyAtMut = ZMintMut<'a>;

    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (base, bytes) = <MintZeroCopyMeta as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)?;
        // has_extensions already consumed the Option discriminator byte
        if base.has_extensions() {
            let (extensions, bytes) =
                <Vec<ExtensionStruct> as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)?;
            Ok((
                ZMintMut {
                    base,
                    extensions: Some(extensions),
                },
                bytes,
            ))
        } else {
            Ok((
                ZMintMut {
                    base,
                    extensions: None,
                },
                bytes,
            ))
        }
    }
}

// Deref implementations for field access
impl<'a> Deref for ZMint<'a> {
    type Target = ZMintZeroCopyMeta<'a>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<'a> Deref for ZMintMut<'a> {
    type Target = ZMintZeroCopyMetaMut<'a>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

// Getters on ZMintZeroCopyMeta (immutable)
impl ZMintZeroCopyMeta<'_> {
    /// Checks if account_type matches Mint discriminator value
    #[inline(always)]
    pub fn is_mint_account(&self) -> bool {
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

// Getters on ZMintZeroCopyMetaMut (mutable)
impl ZMintZeroCopyMetaMut<'_> {
    /// Checks if account_type matches Mint discriminator value
    #[inline(always)]
    pub fn is_mint_account(&self) -> bool {
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

// Checked methods on Mint
impl Mint {
    /// Zero-copy deserialization with initialization and account_type check.
    /// Returns an error if:
    /// - Account is not initialized (is_initialized == false)
    /// - Account type is not ACCOUNT_TYPE_MINT (byte 165 != 1)
    #[profile]
    pub fn zero_copy_at_checked(bytes: &[u8]) -> Result<(ZMint<'_>, &[u8]), TokenError> {
        // Check minimum size (use Mint-specific size, not Token size)
        if bytes.len() < BASE_MINT_ACCOUNT_SIZE as usize {
            return Err(TokenError::InvalidAccountData);
        }

        // Proceed with deserialization first
        let (mint, remaining) =
            Mint::zero_copy_at(bytes).map_err(|_| TokenError::MintDeserializationFailed)?;

        // Verify account_type using the method
        if !mint.is_mint_account() {
            return Err(TokenError::InvalidAccountType);
        }

        // Check is_initialized
        if !mint.is_initialized() {
            return Err(TokenError::MintNotInitialized);
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
    ) -> Result<(ZMintMut<'_>, &mut [u8]), TokenError> {
        // Check minimum size (use Mint-specific size, not Token size)
        if bytes.len() < BASE_MINT_ACCOUNT_SIZE as usize {
            msg!(
                "zero_copy_at_mut_checked bytes.len() < BASE_MINT_ACCOUNT_SIZE {}",
                bytes.len()
            );
            return Err(TokenError::InvalidAccountData);
        }

        let (mint, remaining) =
            Mint::zero_copy_at_mut(bytes).map_err(|_| TokenError::MintDeserializationFailed)?;

        if !mint.is_initialized() {
            return Err(TokenError::MintNotInitialized);
        }
        if !mint.is_mint_account() {
            return Err(TokenError::InvalidAccountType);
        }

        Ok((mint, remaining))
    }
}

// Helper methods on ZMint
impl ZMint<'_> {
    /// Checks if account_type matches Mint discriminator value
    #[inline(always)]
    pub fn is_mint_account(&self) -> bool {
        self.base.is_mint_account()
    }

    /// Checks if account is initialized
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.base.is_initialized()
    }
}

// Helper methods on ZMintMut
impl ZMintMut<'_> {
    /// Checks if account_type matches Mint discriminator value
    #[inline(always)]
    pub fn is_mint_account(&self) -> bool {
        self.base.is_mint_account()
    }

    /// Checks if account is initialized
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.base.is_initialized()
    }

    /// Set all fields of the Mint struct at once
    #[inline]
    #[profile]
    pub fn set(
        &mut self,
        ix_data: &<MintInstructionData as light_zero_copy::traits::ZeroCopyAt<'_>>::ZeroCopyAt,
        mint_decompressed: bool,
    ) -> Result<(), TokenError> {
        if ix_data.metadata.version != TokenDataVersion::ShaFlat as u8 {
            #[cfg(feature = "solana")]
            msg!(
                "Only shaflat version 3 is supported got {}",
                ix_data.metadata.version
            );
            return Err(TokenError::InvalidTokenMetadataVersion);
        }
        // Set metadata fields from instruction data
        self.base.metadata.version = ix_data.metadata.version;
        self.base.metadata.mint = ix_data.metadata.mint;
        self.base.metadata.mint_decompressed = if mint_decompressed { 1 } else { 0 };
        self.base.metadata.mint_signer = ix_data.metadata.mint_signer;
        self.base.metadata.bump = ix_data.metadata.bump;

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
