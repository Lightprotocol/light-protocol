use core::ops::{Deref, DerefMut};

use aligned_sized::aligned_sized;
use light_compressed_account::Pubkey;
use light_program_profiler::profile;
use light_zero_copy::{
    traits::{ZeroCopyAt, ZeroCopyAtMut},
    ZeroCopy, ZeroCopyMut, ZeroCopyNew,
};

use crate::{
    state::{
        ExtensionStruct, ExtensionStructConfig, Token, ZExtensionStruct, ZExtensionStructMut,
        ACCOUNT_TYPE_TOKEN_ACCOUNT,
    },
    AnchorDeserialize, AnchorSerialize,
};

/// SPL Token Account base size (165 bytes)
pub const BASE_TOKEN_ACCOUNT_SIZE: u64 = TokenZeroCopyMeta::LEN as u64;

/// SPL-compatible Token zero copy struct (165 bytes).
/// Uses derive macros to generate TokenZeroCopyMeta<'a> and TokenZeroCopyMetaMut<'a>.
/// Note: account_type byte at position 165 is handled separately in ZeroCopyAt/ZeroCopyAtMut implementations.
#[derive(
    Debug, PartialEq, Eq, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
#[aligned_sized]
struct TokenZeroCopyMeta {
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
    // End of SPL Token Account compatible layout (165 bytes)
}

/// Zero-copy view of Token with base and optional extensions
#[derive(Debug)]
pub struct ZToken<'a> {
    pub base: ZTokenZeroCopyMeta<'a>,
    /// Account type byte read from position 165 (immutable)
    account_type: u8,
    pub extensions: Option<Vec<ZExtensionStruct<'a>>>,
}

/// Mutable zero-copy view of Token with base and optional extensions
#[derive(Debug)]
pub struct ZTokenMut<'a> {
    pub base: ZTokenZeroCopyMetaMut<'a>,
    /// Account type byte read from position 165 (immutable even for mut)
    account_type: u8,
    pub extensions: Option<Vec<ZExtensionStructMut<'a>>>,
}

/// Configuration for creating a new Token via ZeroCopyNew
#[derive(Debug, Clone, PartialEq)]
pub struct TokenConfig {
    /// The mint pubkey
    pub mint: Pubkey,
    /// The owner pubkey
    pub owner: Pubkey,
    /// Account state: 1=Initialized, 2=Frozen
    pub state: u8,
    /// Extensions to include in the account (should include Compressible extension for compressible accounts)
    pub extensions: Option<Vec<ExtensionStructConfig>>,
}

impl<'a> ZeroCopyNew<'a> for Token {
    type ZeroCopyConfig = TokenConfig;
    type Output = ZTokenMut<'a>;

    fn byte_len(
        config: &Self::ZeroCopyConfig,
    ) -> Result<usize, light_zero_copy::errors::ZeroCopyError> {
        let mut size = BASE_TOKEN_ACCOUNT_SIZE as usize;
        if let Some(extensions) = &config.extensions {
            if !extensions.is_empty() {
                size += 1; // account_type byte at position 165
                size += 1; // Option discriminator for extensions (1 = Some)
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
        // Check that the account is not already initialized (state byte at offset 108)
        const STATE_OFFSET: usize = 108;
        if bytes.len() > STATE_OFFSET && bytes[STATE_OFFSET] != 0 {
            return Err(light_zero_copy::errors::ZeroCopyError::MemoryNotZeroed);
        }
        // Use derived new_zero_copy for base struct (config type is () for fixed-size struct)
        let (mut base, mut remaining) =
            <TokenZeroCopyMeta as ZeroCopyNew<'a>>::new_zero_copy(bytes, ())?;

        // Set base token account fields from config
        base.mint = config.mint;
        base.owner = config.owner;
        base.state = config.state;

        // Write extensions using ExtensionStruct::new_zero_copy
        let (account_type, extensions) = if let Some(ref extensions_config) = config.extensions {
            if extensions_config.is_empty() {
                return Err(light_zero_copy::errors::ZeroCopyError::InvalidEnumValue);
            }
            // Check buffer has enough space for header: account_type (1) + Option (1) + Vec len (4)
            if remaining.len() < 6 {
                return Err(
                    light_zero_copy::errors::ZeroCopyError::InsufficientMemoryAllocated(
                        remaining.len(),
                        6,
                    ),
                );
            }

            // Split remaining: header (6 bytes) and extension data
            let (header, ext_data) = remaining.split_at_mut(6);
            // Write account_type byte at position 165
            header[0] = ACCOUNT_TYPE_TOKEN_ACCOUNT;
            // Write Option discriminator (1 = Some)
            header[1] = 1;
            // Write Vec length prefix (4 bytes, little-endian u32)
            header[2..6].copy_from_slice(&(extensions_config.len() as u32).to_le_bytes());

            // Write each extension and collect mutable references
            let mut parsed_extensions = Vec::with_capacity(extensions_config.len());
            let mut write_remaining = ext_data;

            for ext_config in extensions_config {
                let (ext, rest) =
                    ExtensionStruct::new_zero_copy(write_remaining, ext_config.clone())?;
                parsed_extensions.push(ext);
                write_remaining = rest;
            }
            // Update remaining to point past all written data
            remaining = write_remaining;
            (ACCOUNT_TYPE_TOKEN_ACCOUNT, Some(parsed_extensions))
        } else {
            (ACCOUNT_TYPE_TOKEN_ACCOUNT, None)
        };
        if !remaining.is_empty() {
            return Err(light_zero_copy::errors::ZeroCopyError::Size);
        }
        Ok((
            ZTokenMut {
                base,
                account_type,
                extensions,
            },
            remaining,
        ))
    }
}

impl<'a> ZeroCopyAt<'a> for Token {
    type ZeroCopyAt = ZToken<'a>;

    #[inline(always)]
    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(Self::ZeroCopyAt, &'a [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (base, bytes) = <TokenZeroCopyMeta as ZeroCopyAt<'a>>::zero_copy_at(bytes)?;

        // Check if there are extensions by looking at account_type byte at position 165
        if !bytes.is_empty() {
            let account_type = bytes[0];
            // Skip account_type byte
            let bytes = &bytes[1..];

            // Read extensions using Option<Vec<ExtensionStruct>>
            let (extensions, bytes) =
                <Option<Vec<ExtensionStruct>> as ZeroCopyAt<'a>>::zero_copy_at(bytes)?;
            Ok((
                ZToken {
                    base,
                    account_type,
                    extensions,
                },
                bytes,
            ))
        } else {
            // No extensions - account_type defaults to TOKEN_ACCOUNT type
            Ok((
                ZToken {
                    base,
                    account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
                    extensions: None,
                },
                bytes,
            ))
        }
    }
}

impl<'a> ZeroCopyAtMut<'a> for Token {
    type ZeroCopyAtMut = ZTokenMut<'a>;

    #[inline(always)]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (base, bytes) = <TokenZeroCopyMeta as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)?;

        // Check if there are extensions by looking at account_type byte at position 165
        if !bytes.is_empty() {
            let account_type = bytes[0];
            // Skip account_type byte
            let bytes = &mut bytes[1..];

            // Read extensions using Option<Vec<ExtensionStruct>>
            let (extensions, bytes) =
                <Option<Vec<ExtensionStruct>> as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)?;
            Ok((
                ZTokenMut {
                    base,
                    account_type,
                    extensions,
                },
                bytes,
            ))
        } else {
            // No extensions - account_type defaults to TOKEN_ACCOUNT type
            Ok((
                ZTokenMut {
                    base,
                    account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
                    extensions: None,
                },
                bytes,
            ))
        }
    }
}

// Deref implementations for field access
impl<'a> Deref for ZToken<'a> {
    type Target = ZTokenZeroCopyMeta<'a>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<'a> Deref for ZTokenMut<'a> {
    type Target = ZTokenZeroCopyMetaMut<'a>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<'a> DerefMut for ZTokenMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

// Getters on ZToken (immutable view)
impl<'a> ZToken<'a> {
    /// Returns the account_type byte read from position 165
    #[inline(always)]
    pub fn account_type(&self) -> u8 {
        self.account_type
    }

    /// Checks if account_type matches Token discriminator value
    #[inline(always)]
    pub fn is_token_account(&self) -> bool {
        self.account_type == ACCOUNT_TYPE_TOKEN_ACCOUNT
    }

    /// Returns a reference to the Compressible extension if it exists
    #[inline(always)]
    pub fn get_compressible_extension(
        &self,
    ) -> Option<&crate::state::extensions::ZCompressibleExtension<'a>> {
        self.extensions.as_ref().and_then(|exts| {
            exts.iter().find_map(|ext| match ext {
                ZExtensionStruct::Compressible(comp) => Some(comp),
                _ => None,
            })
        })
    }
}

// Getters on ZTokenMut (account_type is still immutable)
impl<'a> ZTokenMut<'a> {
    /// Returns the account_type byte read from position 165
    #[inline(always)]
    pub fn account_type(&self) -> u8 {
        self.account_type
    }

    /// Checks if account_type matches Token discriminator value
    #[inline(always)]
    pub fn is_token_account(&self) -> bool {
        self.account_type == ACCOUNT_TYPE_TOKEN_ACCOUNT
    }

    /// Returns a mutable reference to the Compressible extension if it exists
    #[inline(always)]
    pub fn get_compressible_extension_mut(
        &mut self,
    ) -> Option<&mut crate::state::extensions::ZCompressibleExtensionMut<'a>> {
        self.extensions.as_mut().and_then(|exts| {
            exts.iter_mut().find_map(|ext| match ext {
                ZExtensionStructMut::Compressible(comp) => Some(comp),
                _ => None,
            })
        })
    }

    /// Returns an immutable reference to the Compressible extension if it exists
    #[inline(always)]
    pub fn get_compressible_extension(
        &self,
    ) -> Option<&crate::state::extensions::ZCompressibleExtensionMut<'a>> {
        self.extensions.as_ref().and_then(|exts| {
            exts.iter().find_map(|ext| match ext {
                ZExtensionStructMut::Compressible(comp) => Some(comp),
                _ => None,
            })
        })
    }
}

// Getters on ZTokenZeroCopyMeta (immutable)
impl ZTokenZeroCopyMeta<'_> {
    /// Checks if account is uninitialized (state == 0)
    #[inline(always)]
    pub fn is_uninitialized(&self) -> bool {
        self.state == 0
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
}

// Getters on ZTokenZeroCopyMetaMut (mutable)
impl ZTokenZeroCopyMetaMut<'_> {
    /// Checks if account is uninitialized (state == 0)
    #[inline(always)]
    pub fn is_uninitialized(&self) -> bool {
        self.state == 0
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

    /// Set delegate (Some to set, None to clear)
    #[inline(always)]
    pub fn set_delegate(&mut self, delegate: Option<Pubkey>) -> Result<(), crate::TokenError> {
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

// Checked methods on TokenZeroCopy
impl Token {
    /// Zero-copy deserialization with initialization and account_type check.
    /// Returns an error if:
    /// - Account is uninitialized (byte 108 == 0)
    /// - Account type is not ACCOUNT_TYPE_TOKEN_ACCOUNT (byte 165 != 2)
    ///   Allows both Initialized (1) and Frozen (2) states.
    #[profile]
    #[inline(always)]
    pub fn zero_copy_at_checked(
        bytes: &[u8],
    ) -> Result<(ZToken<'_>, &[u8]), crate::error::TokenError> {
        let (token, remaining) = Token::zero_copy_at(bytes)?;

        if !token.is_initialized() {
            return Err(crate::error::TokenError::InvalidAccountState);
        }
        if !token.is_token_account() {
            return Err(crate::error::TokenError::InvalidAccountType);
        }

        Ok((token, remaining))
    }

    /// Mutable zero-copy deserialization with initialization and account_type check.
    /// Returns an error if:
    /// - Account is uninitialized (state == 0)
    /// - Account type is not ACCOUNT_TYPE_TOKEN_ACCOUNT
    #[profile]
    #[inline(always)]
    pub fn zero_copy_at_mut_checked(
        bytes: &mut [u8],
    ) -> Result<(ZTokenMut<'_>, &mut [u8]), crate::error::TokenError> {
        let (token, remaining) = Token::zero_copy_at_mut(bytes)?;

        if !token.is_initialized() {
            return Err(crate::error::TokenError::InvalidAccountState);
        }
        if !token.is_token_account() {
            return Err(crate::error::TokenError::InvalidAccountType);
        }

        Ok((token, remaining))
    }

    /// Deserialize a Token from account info with validation using zero-copy.
    ///
    /// Checks:
    /// 1. Account is owned by the CTOKEN program
    /// 2. Account is initialized (state != 0)
    /// 3. Account type is ACCOUNT_TYPE_TOKEN_ACCOUNT (byte 165 == 2)
    /// 4. No trailing bytes after the Token structure
    ///
    /// Safety: The returned ZToken references the account data which is valid
    /// for the duration of the transaction. The caller must ensure the account
    /// is not modified through other means while this reference exists.
    #[inline(always)]
    pub fn from_account_info_checked<'a>(
        account_info: &pinocchio::account_info::AccountInfo,
    ) -> Result<ZToken<'a>, crate::error::TokenError> {
        // 1. Check program ownership
        if !account_info.is_owned_by(&crate::LIGHT_TOKEN_PROGRAM_ID) {
            return Err(crate::error::TokenError::InvalidTokenOwner);
        }

        let data = account_info
            .try_borrow_data()
            .map_err(|_| crate::error::TokenError::BorrowFailed)?;

        // Extend lifetime to 'a - safe because account data lives for transaction duration
        let data_slice: &'a [u8] =
            unsafe { core::slice::from_raw_parts(data.as_ptr(), data.len()) };

        let (token, remaining) = Token::zero_copy_at_checked(data_slice)?;

        // 4. Check no trailing bytes
        if !remaining.is_empty() {
            return Err(crate::error::TokenError::InvalidAccountData);
        }

        Ok(token)
    }

    /// Mutable version of from_account_info_checked.
    /// Deserialize a Token from account info with validation using zero-copy.
    ///
    /// Checks:
    /// 1. Account is owned by the CTOKEN program
    /// 2. Account is initialized (state != 0)
    /// 3. Account type is ACCOUNT_TYPE_TOKEN_ACCOUNT (byte 165 == 2)
    /// 4. No trailing bytes after the Token structure
    #[inline(always)]
    pub fn from_account_info_mut_checked<'a>(
        account_info: &pinocchio::account_info::AccountInfo,
    ) -> Result<ZTokenMut<'a>, crate::error::TokenError> {
        // 1. Check program ownership
        if !account_info.is_owned_by(&crate::LIGHT_TOKEN_PROGRAM_ID) {
            return Err(crate::error::TokenError::InvalidTokenOwner);
        }

        let mut data = account_info
            .try_borrow_mut_data()
            .map_err(|_| crate::error::TokenError::BorrowFailed)?;

        // Extend lifetime to 'a - safe because account data lives for transaction duration
        let data_slice: &'a mut [u8] =
            unsafe { core::slice::from_raw_parts_mut(data.as_mut_ptr(), data.len()) };

        let (token, remaining) = Token::zero_copy_at_mut_checked(data_slice)?;

        // 4. Check no trailing bytes
        if !remaining.is_empty() {
            return Err(crate::error::TokenError::InvalidAccountData);
        }

        Ok(token)
    }
}

#[cfg(feature = "test-only")]
impl PartialEq<Token> for ZToken<'_> {
    fn eq(&self, other: &Token) -> bool {
        // Compare basic fields
        if self.mint.to_bytes() != other.mint.to_bytes()
            || self.owner.to_bytes() != other.owner.to_bytes()
            || u64::from(self.amount) != other.amount
            || self.state != other.state as u8
            || u64::from(self.delegated_amount) != other.delegated_amount
            || self.account_type != other.account_type
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
                        (
                            ZExtensionStruct::Compressible(zc_comp),
                            crate::state::extensions::ExtensionStruct::Compressible(regular_comp),
                        ) => {
                            // Compare decimals
                            let zc_decimals = if zc_comp.decimals_option == 1 {
                                Some(zc_comp.decimals)
                            } else {
                                None
                            };
                            if zc_decimals != regular_comp.decimals() {
                                return false;
                            }
                            // Compare compression_only (zero-copy has u8, regular has bool)
                            if (zc_comp.compression_only != 0) != regular_comp.compression_only {
                                return false;
                            }
                            // Compare CompressionInfo fields
                            let zc_info = &zc_comp.info;
                            let regular_info = &regular_comp.info;
                            if u16::from(zc_info.config_account_version)
                                != regular_info.config_account_version
                            {
                                return false;
                            }
                            if zc_info.compress_to_pubkey != regular_info.compress_to_pubkey {
                                return false;
                            }
                            if zc_info.account_version != regular_info.account_version {
                                return false;
                            }
                            if u64::from(zc_info.last_claimed_slot)
                                != regular_info.last_claimed_slot
                            {
                                return false;
                            }
                            if u32::from(zc_info.lamports_per_write)
                                != regular_info.lamports_per_write
                            {
                                return false;
                            }
                            if zc_info.compression_authority != regular_info.compression_authority {
                                return false;
                            }
                            if zc_info.rent_sponsor != regular_info.rent_sponsor {
                                return false;
                            }
                            // Compare rent_config fields
                            if u16::from(zc_info.rent_config.base_rent)
                                != regular_info.rent_config.base_rent
                            {
                                return false;
                            }
                            if u16::from(zc_info.rent_config.compression_cost)
                                != regular_info.rent_config.compression_cost
                            {
                                return false;
                            }
                            if zc_info.rent_config.lamports_per_byte_per_epoch
                                != regular_info.rent_config.lamports_per_byte_per_epoch
                            {
                                return false;
                            }
                            if zc_info.rent_config.max_funded_epochs
                                != regular_info.rent_config.max_funded_epochs
                            {
                                return false;
                            }
                            if u16::from(zc_info.rent_config.max_top_up)
                                != regular_info.rent_config.max_top_up
                            {
                                return false;
                            }
                        }
                        // Unknown or unhandled extension types should panic to surface bugs early
                        (zc_ext, regular_ext) => {
                            panic!(
                                "Unknown extension type comparison: ZToken extension {:?} vs Token extension {:?}",
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
impl PartialEq<ZToken<'_>> for Token {
    fn eq(&self, other: &ZToken<'_>) -> bool {
        other.eq(self)
    }
}
