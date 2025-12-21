use aligned_sized::aligned_sized;
use core::ops::Deref;
use light_compressed_account::Pubkey;
use light_compressible::compression_info::CompressionInfo;
use light_program_profiler::profile;
use light_zero_copy::{
    traits::{ZeroCopyAt, ZeroCopyAtMut},
    ZeroCopy, ZeroCopyMut, ZeroCopyNew,
};
use spl_pod::solana_msg::msg;

use crate::state::CToken;
use crate::{
    state::{
        ExtensionStruct, ExtensionStructConfig, ZExtensionStruct, ZExtensionStructMut,
        ACCOUNT_TYPE_TOKEN_ACCOUNT,
    },
    AnchorDeserialize, AnchorSerialize,
};
pub const BASE_TOKEN_ACCOUNT_SIZE: u64 = CTokenZeroCopyMeta::LEN as u64;

/// Optimized CToken zero copy struct.
/// Uses derive macros to generate ZCToken<'a> and ZCTokenMut<'a>.
#[derive(
    Debug, PartialEq, Eq, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
#[aligned_sized]
struct CTokenZeroCopyMeta {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    delegate_option_prefix: u32,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    delegate: Pubkey,
    /// The account's state
    pub state: u8,
    /// If `is_some`, this is a native token, and the value logs the rent-exempt
    /// reserve. An Account is required to be rent-exempt, so the value is
    /// used by the Processor to ensure that wrapped SOL accounts do not
    /// drop below this threshold.
    is_native_option_prefix: u32,
    is_native: u64,
    /// The amount delegated
    pub delegated_amount: u64,
    /// Optional authority to close the account.
    close_authority_option_prefix: u32,
    close_authority: Pubkey,
    // End of spl-token compatible layout
    /// Account type discriminator at byte 165 (always 2 for CToken accounts)
    pub account_type: u8, // t22 compatible account type - end of t22 compatible layout
    decimal_option_prefix: u8,
    decimals: u8,
    pub compression_only: bool,
    pub compression: CompressionInfo,
    has_extensions: bool,
}

/// Zero-copy view of CToken with meta and optional extensions
#[derive(Debug)]
pub struct ZCToken<'a> {
    pub meta: ZCTokenZeroCopyMeta<'a>,
    pub extensions: Option<Vec<ZExtensionStruct<'a>>>,
}

/// Mutable zero-copy view of CToken with meta and optional extensions
#[derive(Debug)]
pub struct ZCTokenMut<'a> {
    pub meta: ZCTokenZeroCopyMetaMut<'a>,
    pub extensions: Option<Vec<ZExtensionStructMut<'a>>>,
}

/// Configuration for creating a new CToken via ZeroCopyNew
#[derive(Debug, Clone, PartialEq)]
pub struct CompressedTokenConfig {
    /// Extension configurations
    pub extensions: Option<Vec<ExtensionStructConfig>>,
}

impl<'a> ZeroCopyNew<'a> for CToken {
    type ZeroCopyConfig = CompressedTokenConfig;
    type Output = ZCTokenMut<'a>;

    fn byte_len(
        config: &Self::ZeroCopyConfig,
    ) -> Result<usize, light_zero_copy::errors::ZeroCopyError> {
        // Use derived byte_len for meta struct
        let meta_config = CTokenZeroCopyMetaConfig {
            compression: light_compressible::compression_info::CompressionInfoConfig {
                rent_config: (),
            },
        };
        let mut size = CTokenZeroCopyMeta::byte_len(&meta_config)?;

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
        let meta_config = CTokenZeroCopyMetaConfig {
            compression: light_compressible::compression_info::CompressionInfoConfig {
                rent_config: (),
            },
        };
        let (mut meta, remaining) =
            <CTokenZeroCopyMeta as ZeroCopyNew<'a>>::new_zero_copy(bytes, meta_config)?;
        meta.account_type = ACCOUNT_TYPE_TOKEN_ACCOUNT;

        // Initialize extensions if present
        if let Some(extensions_config) = config.extensions {
            *meta.has_extensions = 1u8;
            let (extensions, remaining) = <Vec<ExtensionStruct> as ZeroCopyNew<'a>>::new_zero_copy(
                remaining,
                extensions_config,
            )?;

            Ok((
                ZCTokenMut {
                    meta,
                    extensions: Some(extensions),
                },
                remaining,
            ))
        } else {
            Ok((
                ZCTokenMut {
                    meta,
                    extensions: None,
                },
                remaining,
            ))
        }
    }
}

impl<'a> ZeroCopyAt<'a> for CToken {
    type ZeroCopyAt = ZCToken<'a>;

    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(Self::ZeroCopyAt, &'a [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (meta, bytes) = <CTokenZeroCopyMeta as ZeroCopyAt<'a>>::zero_copy_at(bytes)?;
        // has_extensions already consumed the Option discriminator byte
        if meta.has_extensions() {
            let (extensions, bytes) =
                <Vec<ExtensionStruct> as ZeroCopyAt<'a>>::zero_copy_at(bytes)?;
            Ok((
                ZCToken {
                    meta,
                    extensions: Some(extensions),
                },
                bytes,
            ))
        } else {
            Ok((
                ZCToken {
                    meta,
                    extensions: None,
                },
                bytes,
            ))
        }
    }
}

impl<'a> ZeroCopyAtMut<'a> for CToken {
    type ZeroCopyAtMut = ZCTokenMut<'a>;

    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (meta, bytes) = <CTokenZeroCopyMeta as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)?;
        // has_extensions already consumed the Option discriminator byte
        if meta.has_extensions() {
            let (extensions, bytes) =
                <Vec<ExtensionStruct> as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)?;
            Ok((
                ZCTokenMut {
                    meta,
                    extensions: Some(extensions),
                },
                bytes,
            ))
        } else {
            Ok((
                ZCTokenMut {
                    meta,
                    extensions: None,
                },
                bytes,
            ))
        }
    }
}

// Deref implementations for field access
impl<'a> Deref for ZCToken<'a> {
    type Target = ZCTokenZeroCopyMeta<'a>;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl<'a> Deref for ZCTokenMut<'a> {
    type Target = ZCTokenZeroCopyMetaMut<'a>;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

// Getters on ZCTokenZeroCopyMeta (immutable)
impl ZCTokenZeroCopyMeta<'_> {
    /// Checks if account_type matches CToken discriminator value
    #[inline(always)]
    pub fn is_ctoken_account(&self) -> bool {
        self.account_type == ACCOUNT_TYPE_TOKEN_ACCOUNT
    }

    /// Checks if account is initialized (state == 1 or state == 2)
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.state != 0
    }

    /// Checks if account is frozen (state == 2)
    #[inline(always)]
    pub fn is_frozen(&self) -> bool {
        self.state == 2
    }

    /// Get delegate if set (COption discriminator == 1)
    pub fn delegate(&self) -> Option<&Pubkey> {
        if u32::from(self.delegate_option_prefix) == 1 {
            Some(&self.delegate)
        } else {
            None
        }
    }

    /// Get is_native value if set (COption discriminator == 1)
    pub fn is_native_value(&self) -> Option<u64> {
        if u32::from(self.is_native_option_prefix) == 1 {
            Some(u64::from(self.is_native))
        } else {
            None
        }
    }

    /// Get close_authority if set (COption discriminator == 1)
    pub fn close_authority(&self) -> Option<&Pubkey> {
        if u32::from(self.close_authority_option_prefix) == 1 {
            Some(&self.close_authority)
        } else {
            None
        }
    }

    /// Get decimals if set (option prefix == 1)
    pub fn decimals(&self) -> Option<u8> {
        if self.decimal_option_prefix == 1 {
            Some(self.decimals)
        } else {
            None
        }
    }
}

// Getters on ZCTokenZeroCopyMetaMut (mutable)
impl ZCTokenZeroCopyMetaMut<'_> {
    /// Checks if account_type matches CToken discriminator value
    #[inline(always)]
    pub fn is_ctoken_account(&self) -> bool {
        self.account_type == ACCOUNT_TYPE_TOKEN_ACCOUNT
    }

    /// Checks if account is initialized (state == 1 or state == 2)
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.state != 0
    }

    /// Checks if account is frozen (state == 2)
    #[inline(always)]
    pub fn is_frozen(&self) -> bool {
        self.state == 2
    }

    /// Get delegate if set (COption discriminator == 1)
    pub fn delegate(&self) -> Option<&Pubkey> {
        if u32::from(self.delegate_option_prefix) == 1 {
            Some(&self.delegate)
        } else {
            None
        }
    }

    /// Get is_native value if set (COption discriminator == 1)
    pub fn is_native_value(&self) -> Option<u64> {
        if u32::from(self.is_native_option_prefix) == 1 {
            Some(u64::from(self.is_native))
        } else {
            None
        }
    }

    /// Get close_authority if set (COption discriminator == 1)
    pub fn close_authority(&self) -> Option<&Pubkey> {
        if u32::from(self.close_authority_option_prefix) == 1 {
            Some(&self.close_authority)
        } else {
            None
        }
    }

    /// Get decimals if set (option prefix == 1)
    pub fn decimals(&self) -> Option<u8> {
        if self.decimal_option_prefix == 1 {
            Some(self.decimals)
        } else {
            None
        }
    }
}

// Checked methods on CTokenZeroCopy
impl CToken {
    /// Zero-copy deserialization with initialization and account_type check.
    /// Returns an error if:
    /// - Account is uninitialized (byte 108 == 0)
    /// - Account type is not ACCOUNT_TYPE_TOKEN_ACCOUNT (byte 165 != 2)
    ///   Allows both Initialized (1) and Frozen (2) states.
    #[profile]
    pub fn zero_copy_at_checked(
        bytes: &[u8],
    ) -> Result<(ZCToken<'_>, &[u8]), crate::error::CTokenError> {
        // Check minimum size
        if bytes.len() < BASE_TOKEN_ACCOUNT_SIZE as usize {
            msg!(
                "zero_copy_at_checked bytes.len() < BASE_TOKEN_ACCOUNT_SIZE {}",
                bytes.len()
            );
            return Err(crate::error::CTokenError::InvalidAccountData);
        }

        let (ctoken, remaining) = CToken::zero_copy_at(bytes)?;

        if !ctoken.is_initialized() {
            return Err(crate::error::CTokenError::InvalidAccountState);
        }
        if !ctoken.is_ctoken_account() {
            return Err(crate::error::CTokenError::InvalidAccountType);
        }

        Ok((ctoken, remaining))
    }

    /// Mutable zero-copy deserialization with initialization and account_type check.
    /// Returns an error if:
    /// - Account is uninitialized (state == 0)
    /// - Account type is not ACCOUNT_TYPE_TOKEN_ACCOUNT
    #[profile]
    pub fn zero_copy_at_mut_checked(
        bytes: &mut [u8],
    ) -> Result<(ZCTokenMut<'_>, &mut [u8]), crate::error::CTokenError> {
        // Check minimum size
        if bytes.len() < BASE_TOKEN_ACCOUNT_SIZE as usize {
            msg!(
                "zero_copy_at_checked bytes.len() < BASE_TOKEN_ACCOUNT_SIZE {}",
                bytes.len()
            );
            return Err(crate::error::CTokenError::InvalidAccountData);
        }

        let (ctoken, remaining) = CToken::zero_copy_at_mut(bytes)?;

        if !ctoken.is_initialized() {
            return Err(crate::error::CTokenError::InvalidAccountState);
        }
        if !ctoken.is_ctoken_account() {
            return Err(crate::error::CTokenError::InvalidAccountType);
        }

        Ok((ctoken, remaining))
    }
}

#[cfg(feature = "test-only")]
impl PartialEq<CToken> for ZCToken<'_> {
    fn eq(&self, other: &CToken) -> bool {
        // Compare basic fields
        if self.mint.to_bytes() != other.mint.to_bytes()
            || self.owner.to_bytes() != other.owner.to_bytes()
            || u64::from(self.amount) != other.amount
            || self.state != other.state as u8
            || u64::from(self.delegated_amount) != other.delegated_amount
        {
            return false;
        }

        // Compare delegate
        match (self.delegate(), &other.delegate) {
            (Some(zc_delegate), Some(regular_delegate)) => {
                if zc_delegate.to_bytes() != regular_delegate.to_bytes() {
                    return false;
                }
            }
            (None, None) => {}
            _ => return false,
        }

        // Compare is_native
        match (self.is_native_value(), &other.is_native) {
            (Some(zc_native), Some(regular_native)) => {
                if zc_native != *regular_native {
                    return false;
                }
            }
            (None, None) => {}
            _ => return false,
        }

        // Compare close_authority
        match (self.close_authority(), &other.close_authority) {
            (Some(zc_close), Some(regular_close)) => {
                if zc_close.to_bytes() != regular_close.to_bytes() {
                    return false;
                }
            }
            (None, None) => {}
            _ => return false,
        }

        // Compare decimals
        match (self.decimals(), &other.decimals) {
            (Some(zc_decimals), Some(regular_decimals)) => {
                if zc_decimals != *regular_decimals {
                    return false;
                }
            }
            (None, None) => {}
            _ => return false,
        }

        // Compare compression_only
        if self.compression_only() != other.compression_only {
            return false;
        }

        // Compare compression fields
        if u16::from(self.compression.config_account_version)
            != other.compression.config_account_version
        {
            return false;
        }
        if self.compression.compress_to_pubkey != other.compression.compress_to_pubkey {
            return false;
        }
        if self.compression.account_version != other.compression.account_version {
            return false;
        }
        if u64::from(self.compression.last_claimed_slot) != other.compression.last_claimed_slot {
            return false;
        }
        if u32::from(self.compression.lamports_per_write) != other.compression.lamports_per_write {
            return false;
        }
        if self.compression.compression_authority != other.compression.compression_authority {
            return false;
        }
        if self.compression.rent_sponsor != other.compression.rent_sponsor {
            return false;
        }
        // Compare rent_config fields
        if u16::from(self.compression.rent_config.base_rent)
            != other.compression.rent_config.base_rent
        {
            return false;
        }
        if u16::from(self.compression.rent_config.compression_cost)
            != other.compression.rent_config.compression_cost
        {
            return false;
        }
        if self.compression.rent_config.lamports_per_byte_per_epoch
            != other.compression.rent_config.lamports_per_byte_per_epoch
        {
            return false;
        }
        if self.compression.rent_config.max_funded_epochs
            != other.compression.rent_config.max_funded_epochs
        {
            return false;
        }
        if u16::from(self.compression.rent_config.max_top_up)
            != other.compression.rent_config.max_top_up
        {
            return false;
        }

        // Compare extensions
        match (&self.extensions, &other.extensions) {
            (Some(zc_extensions), Some(regular_extensions)) => {
                if zc_extensions.len() != regular_extensions.len() {
                    return false;
                }
                for (zc_ext, regular_ext) in zc_extensions.iter().zip(regular_extensions.iter()) {
                    match (zc_ext, regular_ext) {
                        (
                            ZExtensionStruct::TokenMetadata(zc_tm),
                            crate::state::extensions::ExtensionStruct::TokenMetadata(regular_tm),
                        ) => {
                            if zc_tm.mint.to_bytes() != regular_tm.mint.to_bytes()
                                || zc_tm.name != regular_tm.name.as_slice()
                                || zc_tm.symbol != regular_tm.symbol.as_slice()
                                || zc_tm.uri != regular_tm.uri.as_slice()
                            {
                                return false;
                            }
                            if zc_tm.update_authority != regular_tm.update_authority {
                                return false;
                            }
                            if zc_tm.additional_metadata.len()
                                != regular_tm.additional_metadata.len()
                            {
                                return false;
                            }
                            for (zc_meta, regular_meta) in zc_tm
                                .additional_metadata
                                .iter()
                                .zip(regular_tm.additional_metadata.iter())
                            {
                                if zc_meta.key != regular_meta.key.as_slice()
                                    || zc_meta.value != regular_meta.value.as_slice()
                                {
                                    return false;
                                }
                            }
                        }
                        (
                            ZExtensionStruct::PausableAccount(_),
                            crate::state::extensions::ExtensionStruct::PausableAccount(_),
                        ) => {
                            // Marker extension with no data, just matching discriminant is enough
                        }
                        (
                            ZExtensionStruct::PermanentDelegateAccount(_),
                            crate::state::extensions::ExtensionStruct::PermanentDelegateAccount(_),
                        ) => {
                            // Marker extension with no data
                        }
                        (
                            ZExtensionStruct::TransferFeeAccount(zc_tfa),
                            crate::state::extensions::ExtensionStruct::TransferFeeAccount(
                                regular_tfa,
                            ),
                        ) => {
                            if u64::from(zc_tfa.withheld_amount) != regular_tfa.withheld_amount {
                                return false;
                            }
                        }
                        (
                            ZExtensionStruct::TransferHookAccount(zc_tha),
                            crate::state::extensions::ExtensionStruct::TransferHookAccount(
                                regular_tha,
                            ),
                        ) => {
                            if zc_tha.transferring != regular_tha.transferring {
                                return false;
                            }
                        }
                        (
                            ZExtensionStruct::Compressible(zc_comp),
                            crate::state::extensions::ExtensionStruct::Compressible(regular_comp),
                        ) => {
                            if (zc_comp.compression_only != 0) != regular_comp.compression_only
                                || zc_comp.decimals != regular_comp.decimals
                                || zc_comp.has_decimals != regular_comp.has_decimals
                            {
                                return false;
                            }
                            // Compare nested CompressionInfo
                            if u16::from(zc_comp.info.config_account_version)
                                != regular_comp.info.config_account_version
                                || zc_comp.info.compress_to_pubkey
                                    != regular_comp.info.compress_to_pubkey
                                || zc_comp.info.account_version != regular_comp.info.account_version
                                || u32::from(zc_comp.info.lamports_per_write)
                                    != regular_comp.info.lamports_per_write
                                || zc_comp.info.compression_authority
                                    != regular_comp.info.compression_authority
                                || zc_comp.info.rent_sponsor != regular_comp.info.rent_sponsor
                                || u64::from(zc_comp.info.last_claimed_slot)
                                    != regular_comp.info.last_claimed_slot
                            {
                                return false;
                            }
                        }
                        // Unknown or unhandled extension types should panic to surface bugs early
                        (zc_ext, regular_ext) => {
                            panic!(
                                "Unknown extension type comparison: ZCToken extension {:?} vs CToken extension {:?}",
                                std::mem::discriminant(zc_ext),
                                std::mem::discriminant(regular_ext)
                            );
                        }
                    }
                }
            }
            (None, None) => {}
            _ => return false,
        }

        true
    }
}

#[cfg(feature = "test-only")]
impl PartialEq<ZCToken<'_>> for CToken {
    fn eq(&self, other: &ZCToken<'_>) -> bool {
        other.eq(self)
    }
}
