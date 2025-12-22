use core::ops::{Deref, DerefMut};

use aligned_sized::aligned_sized;
use light_compressed_account::Pubkey;
use light_compressible::compression_info::CompressionInfo;
use light_program_profiler::profile;
use light_zero_copy::{
    traits::{ZeroCopyAt, ZeroCopyAtMut},
    ZeroCopy, ZeroCopyMut, ZeroCopyNew,
};

use crate::{
    state::{
        CToken, ExtensionStruct, ExtensionStructConfig, ZExtensionStruct, ZExtensionStructMut,
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

/// Zero-copy view of CToken with base and optional extensions
#[derive(Debug)]
pub struct ZCToken<'a> {
    pub base: ZCTokenZeroCopyMeta<'a>,
    pub extensions: Option<Vec<ZExtensionStruct<'a>>>,
}

/// Mutable zero-copy view of CToken with base and optional extensions
#[derive(Debug)]
pub struct ZCTokenMut<'a> {
    pub base: ZCTokenZeroCopyMetaMut<'a>,
    pub extensions: Option<Vec<ZExtensionStructMut<'a>>>,
}

/// Configuration for creating a new CToken via ZeroCopyNew
#[derive(Debug, Clone, PartialEq)]
pub struct CompressedTokenConfig {
    /// The mint pubkey
    pub mint: Pubkey,
    /// The owner pubkey
    pub owner: Pubkey,
    /// Account state: 1=Initialized, 2=Frozen
    pub state: u8,
    /// Whether account is compression-only (cannot decompress)
    pub compression_only: bool,
    /// Extensions to include in the account
    pub extensions: Option<Vec<ExtensionStructConfig>>,
}

impl<'a> ZeroCopyNew<'a> for CToken {
    type ZeroCopyConfig = CompressedTokenConfig;
    type Output = ZCTokenMut<'a>;

    fn byte_len(
        config: &Self::ZeroCopyConfig,
    ) -> Result<usize, light_zero_copy::errors::ZeroCopyError> {
        let mut size = BASE_TOKEN_ACCOUNT_SIZE as usize;
        if let Some(extensions) = &config.extensions {
            if !extensions.is_empty() {
                size += 4; // Vec length prefix
                for ext in extensions {
                    size += ExtensionStruct::byte_len(ext)?;
                }
            }
        }
        Ok(size)
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        // Use derived new_zero_copy for base struct
        let base_config = CTokenZeroCopyMetaConfig {
            compression: light_compressible::compression_info::CompressionInfoConfig {
                rent_config: (),
            },
        };
        let (mut base, mut remaining) =
            <CTokenZeroCopyMeta as ZeroCopyNew<'a>>::new_zero_copy(bytes, base_config)?;

        // Set base token account fields from config
        base.mint = config.mint;
        base.owner = config.owner;
        base.state = config.state;
        base.account_type = ACCOUNT_TYPE_TOKEN_ACCOUNT;
        base.compression_only = config.compression_only as u8;

        // Write extensions using ExtensionStruct::new_zero_copy
        if let Some(extensions) = config.extensions {
            if !extensions.is_empty() {
                *base.has_extensions = 1u8;

                // Write Vec length prefix (4 bytes, little-endian u32)
                remaining[..4].copy_from_slice(&(extensions.len() as u32).to_le_bytes());
                remaining = &mut remaining[4..];

                // Write each extension
                for ext_config in extensions {
                    let (_, rest) = ExtensionStruct::new_zero_copy(remaining, ext_config)?;
                    remaining = rest;
                }
            }
        }

        Ok((
            ZCTokenMut {
                base,
                extensions: None, // Extensions are written directly, not tracked as Vec
            },
            remaining,
        ))
    }
}

impl<'a> ZeroCopyAt<'a> for CToken {
    type ZeroCopyAt = ZCToken<'a>;

    #[inline(always)]
    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(Self::ZeroCopyAt, &'a [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (base, bytes) = <CTokenZeroCopyMeta as ZeroCopyAt<'a>>::zero_copy_at(bytes)?;
        // has_extensions already consumed the Option discriminator byte
        if base.has_extensions() {
            let (extensions, bytes) =
                <Vec<ExtensionStruct> as ZeroCopyAt<'a>>::zero_copy_at(bytes)?;
            Ok((
                ZCToken {
                    base,
                    extensions: Some(extensions),
                },
                bytes,
            ))
        } else {
            Ok((
                ZCToken {
                    base,
                    extensions: None,
                },
                bytes,
            ))
        }
    }
}

impl<'a> ZeroCopyAtMut<'a> for CToken {
    type ZeroCopyAtMut = ZCTokenMut<'a>;

    #[inline(always)]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (base, bytes) = <CTokenZeroCopyMeta as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)?;
        // has_extensions already consumed the Option discriminator byte
        if base.has_extensions() {
            let (extensions, bytes) =
                <Vec<ExtensionStruct> as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)?;
            Ok((
                ZCTokenMut {
                    base,
                    extensions: Some(extensions),
                },
                bytes,
            ))
        } else {
            Ok((
                ZCTokenMut {
                    base,
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
        &self.base
    }
}

impl<'a> Deref for ZCTokenMut<'a> {
    type Target = ZCTokenZeroCopyMetaMut<'a>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<'a> DerefMut for ZCTokenMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

// Getters on ZCTokenZeroCopyMeta (immutable)
impl ZCTokenZeroCopyMeta<'_> {
    /// Checks if account_type matches CToken discriminator value
    #[inline(always)]
    pub fn is_ctoken_account(&self) -> bool {
        self.account_type == ACCOUNT_TYPE_TOKEN_ACCOUNT
    }

    /// Checks if account is initialized (state == 1)
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.state == 1
    }

    /// Checks if account is frozen (state == 2)
    #[inline(always)]
    pub fn is_frozen(&self) -> bool {
        self.state == 2
    }

    /// Get delegate if set (COption discriminator == 1)
    #[inline(always)]
    pub fn delegate(&self) -> Option<&Pubkey> {
        if u32::from(self.delegate_option_prefix) == 1 {
            Some(&self.delegate)
        } else {
            None
        }
    }

    /// Get is_native value if set (COption discriminator == 1)
    #[inline(always)]
    pub fn is_native_value(&self) -> Option<u64> {
        if u32::from(self.is_native_option_prefix) == 1 {
            Some(u64::from(self.is_native))
        } else {
            None
        }
    }

    /// Get close_authority if set (COption discriminator == 1)
    #[inline(always)]
    pub fn close_authority(&self) -> Option<&Pubkey> {
        if u32::from(self.close_authority_option_prefix) == 1 {
            Some(&self.close_authority)
        } else {
            None
        }
    }

    /// Get decimals if set (option prefix == 1)
    #[inline(always)]
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

    /// Checks if account is initialized (state == 1)
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.state == 1
    }

    /// Checks if account is frozen (state == 2)
    #[inline(always)]
    pub fn is_frozen(&self) -> bool {
        self.state == 2
    }

    /// Get delegate if set (COption discriminator == 1)
    #[inline(always)]
    pub fn delegate(&self) -> Option<&Pubkey> {
        if u32::from(self.delegate_option_prefix) == 1 {
            Some(&self.delegate)
        } else {
            None
        }
    }

    /// Get is_native value if set (COption discriminator == 1)
    #[inline(always)]
    pub fn is_native_value(&self) -> Option<u64> {
        if u32::from(self.is_native_option_prefix) == 1 {
            Some(u64::from(self.is_native))
        } else {
            None
        }
    }

    /// Get close_authority if set (COption discriminator == 1)
    #[inline(always)]
    pub fn close_authority(&self) -> Option<&Pubkey> {
        if u32::from(self.close_authority_option_prefix) == 1 {
            Some(&self.close_authority)
        } else {
            None
        }
    }

    /// Get decimals if set (option prefix == 1)
    #[inline(always)]
    pub fn decimals(&self) -> Option<u8> {
        if self.decimal_option_prefix == 1 {
            Some(self.decimals)
        } else {
            None
        }
    }

    /// Set decimals value
    #[inline(always)]
    pub fn set_decimals(&mut self, decimals: u8) {
        self.decimal_option_prefix = 1;
        self.decimals = decimals;
    }

    /// Set delegate (Some to set, None to clear)
    #[inline(always)]
    pub fn set_delegate(&mut self, delegate: Option<Pubkey>) -> Result<(), crate::CTokenError> {
        match delegate {
            Some(pubkey) => {
                self.delegate_option_prefix.set(1);
                self.delegate = pubkey;
            }
            None => {
                self.delegate_option_prefix.set(0);
                // Clear delegate bytes
                self.delegate = Pubkey::default();
            }
        }
        Ok(())
    }

    /// Set account as frozen (state = 2)
    #[inline(always)]
    pub fn set_frozen(&mut self) {
        self.state = 2;
    }

    /// Set account as initialized/unfrozen (state = 1)
    #[inline(always)]
    pub fn set_initialized(&mut self) {
        self.state = 1;
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
    #[inline(always)]
    pub fn zero_copy_at_checked(
        bytes: &[u8],
    ) -> Result<(ZCToken<'_>, &[u8]), crate::error::CTokenError> {
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
    #[inline(always)]
    pub fn zero_copy_at_mut_checked(
        bytes: &mut [u8],
    ) -> Result<(ZCTokenMut<'_>, &mut [u8]), crate::error::CTokenError> {
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
                            ZExtensionStruct::CompressedOnly(zc_co),
                            crate::state::extensions::ExtensionStruct::CompressedOnly(regular_co),
                        ) => {
                            if u64::from(zc_co.delegated_amount) != regular_co.delegated_amount
                                || u64::from(zc_co.withheld_transfer_fee)
                                    != regular_co.withheld_transfer_fee
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
