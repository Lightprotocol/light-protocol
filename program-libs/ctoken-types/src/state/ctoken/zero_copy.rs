use std::ops::{Deref, DerefMut};

use light_compressed_account::Pubkey;
use light_program_profiler::profile;
use light_zero_copy::{
    errors::ZeroCopyError,
    traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew},
};
use spl_pod::solana_msg::msg;

use crate::{
    state::{
        CToken, CompressionInfoConfig, ExtensionStruct, ExtensionStructConfig, ZExtensionStruct,
        ZExtensionStructMut,
    },
    AnchorDeserialize, AnchorSerialize,
};

#[derive(Debug, PartialEq, Eq, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CTokenMeta {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: Option<Pubkey>,
    /// The account's state
    pub state: u8,
    /// If `is_some`, this is a native token, and the value logs the rent-exempt
    /// reserve. An Account is required to be rent-exempt, so the value is
    /// used by the Processor to ensure that wrapped SOL accounts do not
    /// drop below this threshold.
    pub is_native: Option<u64>,
    /// The amount delegated
    pub delegated_amount: u64,
    /// Optional authority to close the account.
    pub close_authority: Option<Pubkey>,
}

// Note: spl zero-copy compatibility is implemented in fn zero_copy_at
#[derive(Debug, PartialEq, Clone)]
pub struct ZCTokenMeta<'a> {
    pub mint: <Pubkey as ZeroCopyAt<'a>>::ZeroCopyAt,
    pub owner: <Pubkey as ZeroCopyAt<'a>>::ZeroCopyAt,
    pub amount: zerocopy::Ref<&'a [u8], zerocopy::little_endian::U64>,
    pub delegate: Option<<Pubkey as ZeroCopyAt<'a>>::ZeroCopyAt>,
    pub state: u8,
    pub is_native: Option<zerocopy::Ref<&'a [u8], zerocopy::little_endian::U64>>,
    pub delegated_amount: zerocopy::Ref<&'a [u8], zerocopy::little_endian::U64>,
    pub close_authority: Option<<Pubkey as ZeroCopyAt<'a>>::ZeroCopyAt>,
}

#[derive(Debug, PartialEq)]
pub struct ZCompressedTokenMetaMut<'a> {
    pub mint: <Pubkey as ZeroCopyAtMut<'a>>::ZeroCopyAtMut,
    pub owner: <Pubkey as ZeroCopyAtMut<'a>>::ZeroCopyAtMut,
    pub amount: zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U64>,
    // 4 option bytes (spl compat) + 32 pubkey bytes
    delegate_option: zerocopy::Ref<&'a mut [u8], [u8; 36]>,
    pub delegate: Option<<Pubkey as ZeroCopyAtMut<'a>>::ZeroCopyAtMut>,
    pub state: zerocopy::Ref<&'a mut [u8], u8>,
    // 4 option bytes (spl compat) + 8 u64 bytes
    is_native_option: zerocopy::Ref<&'a mut [u8], [u8; 12]>,
    pub is_native: Option<zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U64>>,
    pub delegated_amount: zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U64>,
    // 4 option bytes (spl compat) + 32 pubkey bytes
    close_authority_option: zerocopy::Ref<&'a mut [u8], [u8; 36]>,
    pub close_authority: Option<<Pubkey as ZeroCopyAtMut<'a>>::ZeroCopyAtMut>,
}

impl<'a> ZeroCopyAt<'a> for CTokenMeta {
    type ZeroCopyAt = ZCTokenMeta<'a>;

    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        use zerocopy::{
            little_endian::{U32 as ZU32, U64 as ZU64},
            Ref,
        };

        if bytes.len() < 165 {
            // SPL Token Account size
            return Err(ZeroCopyError::Size);
        }

        let (mint, bytes) = Pubkey::zero_copy_at(bytes)?;

        // owner: 32 bytes
        let (owner, bytes) = Pubkey::zero_copy_at(bytes)?;

        // amount: 8 bytes
        let (amount, bytes) = Ref::<&[u8], ZU64>::from_prefix(bytes)?;

        // delegate: 36 bytes (4 byte COption + 32 byte pubkey)
        let (delegate_option, bytes) = Ref::<&[u8], ZU32>::from_prefix(bytes)?;
        let (delegate_pubkey, bytes) = Pubkey::zero_copy_at(bytes)?;
        let delegate = if u32::from(*delegate_option) == 1 {
            Some(delegate_pubkey)
        } else {
            None
        };

        // state: 1 byte
        let (state, bytes) = u8::zero_copy_at(bytes)?;

        // is_native: 12 bytes (4 byte COption + 8 byte u64)
        let (native_option, bytes) = Ref::<&[u8], ZU32>::from_prefix(bytes)?;
        let (native_value, bytes) = Ref::<&[u8], ZU64>::from_prefix(bytes)?;
        let is_native = if u32::from(*native_option) == 1 {
            Some(native_value)
        } else {
            None
        };

        // delegated_amount: 8 bytes
        let (delegated_amount, bytes) = Ref::<&[u8], ZU64>::from_prefix(bytes)?;

        // close_authority: 36 bytes (4 byte COption + 32 byte pubkey)
        let (close_option, bytes) = Ref::<&[u8], ZU32>::from_prefix(bytes)?;
        let (close_pubkey, bytes) = Pubkey::zero_copy_at(bytes)?;
        let close_authority = if u32::from(*close_option) == 1 {
            Some(close_pubkey)
        } else {
            None
        };

        let meta = ZCTokenMeta {
            mint,
            owner,
            amount,
            delegate,
            state,
            is_native,
            delegated_amount,
            close_authority,
        };

        Ok((meta, bytes))
    }
}

impl<'a> ZeroCopyAtMut<'a> for CTokenMeta {
    type ZeroCopyAtMut = ZCompressedTokenMetaMut<'a>;

    #[profile]
    #[inline(always)]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        use zerocopy::{little_endian::U64 as ZU64, Ref};

        if bytes.len() < 165 {
            return Err(ZeroCopyError::Size);
        }

        let (mint, bytes) = Pubkey::zero_copy_at_mut(bytes)?;
        let (owner, bytes) = Pubkey::zero_copy_at_mut(bytes)?;
        let (amount, bytes) = Ref::<&mut [u8], ZU64>::from_prefix(bytes)?;

        let (mut delegate_option, bytes) = Ref::<&mut [u8], [u8; 36]>::from_prefix(bytes)?;
        let pubkey_bytes =
            unsafe { std::slice::from_raw_parts_mut(delegate_option.as_mut_ptr().add(4), 32) };
        let (delegate_pubkey, _) = Pubkey::zero_copy_at_mut(pubkey_bytes)?;
        let delegate = if delegate_option[0] == 1 {
            Some(delegate_pubkey)
        } else {
            None
        };

        // state: 1 byte
        let (state, bytes) = Ref::<&mut [u8], u8>::from_prefix(bytes)?;

        // is_native: 12 bytes (4 byte COption + 8 byte u64)
        let (mut is_native_option, bytes) = Ref::<&mut [u8], [u8; 12]>::from_prefix(bytes)?;
        let value_bytes =
            unsafe { std::slice::from_raw_parts_mut(is_native_option.as_mut_ptr().add(4), 8) };
        let (native_value, _) = Ref::<&mut [u8], ZU64>::from_prefix(value_bytes)?;
        let is_native = if is_native_option[0] == 1 {
            Some(native_value)
        } else {
            None
        };

        // delegated_amount: 8 bytes
        let (delegated_amount, bytes) = Ref::<&mut [u8], ZU64>::from_prefix(bytes)?;

        // close_authority: 36 bytes (4 byte COption + 32 byte pubkey)
        let (mut close_authority_option, bytes) = Ref::<&mut [u8], [u8; 36]>::from_prefix(bytes)?;
        let pubkey_bytes = unsafe {
            std::slice::from_raw_parts_mut(close_authority_option.as_mut_ptr().add(4), 32)
        };
        let (close_pubkey, _) = Pubkey::zero_copy_at_mut(pubkey_bytes)?;
        let close_authority = if close_authority_option[0] == 1 {
            Some(close_pubkey)
        } else {
            None
        };

        let meta = ZCompressedTokenMetaMut {
            mint,
            owner,
            amount,
            delegate_option,
            delegate,
            state,
            is_native_option,
            is_native,
            delegated_amount,
            close_authority_option,
            close_authority,
        };

        Ok((meta, bytes))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ZCToken<'a> {
    __meta: ZCTokenMeta<'a>,
    /// Extensions for the token account (including compressible config)
    pub extensions: Option<Vec<ZExtensionStruct<'a>>>,
}

impl<'a> Deref for ZCToken<'a> {
    type Target = <CTokenMeta as ZeroCopyAt<'a>>::ZeroCopyAt;

    fn deref(&self) -> &Self::Target {
        &self.__meta
    }
}

impl PartialEq<CToken> for ZCToken<'_> {
    fn eq(&self, other: &CToken) -> bool {
        // Compare basic fields
        if self.mint.to_bytes() != other.mint.to_bytes()
            || self.owner.to_bytes() != other.owner.to_bytes()
            || u64::from(*self.amount) != other.amount
            || self.state != other.state as u8
            || u64::from(*self.delegated_amount) != other.delegated_amount
        {
            return false;
        }

        // Compare delegate
        match (&self.delegate, &other.delegate) {
            (Some(zc_delegate), Some(regular_delegate)) => {
                if zc_delegate.to_bytes() != regular_delegate.to_bytes() {
                    return false;
                }
            }
            (None, None) => {}
            _ => return false,
        }

        // Compare is_native
        match (&self.is_native, &other.is_native) {
            (Some(zc_native), Some(regular_native)) => {
                if u64::from(**zc_native) != *regular_native {
                    return false;
                }
            }
            (None, None) => {}
            _ => return false,
        }

        // Compare close_authority
        match (&self.close_authority, &other.close_authority) {
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
                            crate::state::extensions::ZExtensionStruct::Compressible(zc_comp),
                            crate::state::extensions::ExtensionStruct::Compressible(regular_comp),
                        ) => {
                            // Compare config_account_version
                            if zc_comp.config_account_version != regular_comp.config_account_version
                            {
                                return false;
                            }

                            // Compare last_claimed_slot
                            if u64::from(zc_comp.last_claimed_slot)
                                != regular_comp.last_claimed_slot
                            {
                                return false;
                            }

                            // Compare rent_config fields
                            if u16::from(zc_comp.rent_config.base_rent)
                                != regular_comp.rent_config.base_rent
                            {
                                return false;
                            }
                            if u16::from(zc_comp.rent_config.compression_cost)
                                != regular_comp.rent_config.compression_cost
                            {
                                return false;
                            }
                            if zc_comp.rent_config.lamports_per_byte_per_epoch
                                != regular_comp.rent_config.lamports_per_byte_per_epoch
                            {
                                return false;
                            }
                            if zc_comp.rent_config.max_funded_epochs
                                != regular_comp.rent_config.max_funded_epochs
                            {
                                return false;
                            }
                            // Compare compression_authority ([u8; 32])
                            if zc_comp.compression_authority != regular_comp.compression_authority {
                                return false;
                            }

                            // Compare rent_sponsor ([u8; 32])
                            if zc_comp.rent_sponsor != regular_comp.rent_sponsor {
                                return false;
                            }

                            // Compare lamports_per_write (u32)
                            if u32::from(zc_comp.lamports_per_write)
                                != regular_comp.lamports_per_write
                            {
                                return false;
                            }
                        }
                        (
                            crate::state::extensions::ZExtensionStruct::TokenMetadata(zc_tm),
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
                        _ => return false, // Different extension types
                    }
                }
            }
            (None, None) => {}
            _ => return false,
        }

        true
    }
}

impl PartialEq<ZCToken<'_>> for CToken {
    fn eq(&self, other: &ZCToken<'_>) -> bool {
        other.eq(self)
    }
}

#[derive(Debug)]
pub struct ZCompressedTokenMut<'a> {
    __meta: <CTokenMeta as ZeroCopyAtMut<'a>>::ZeroCopyAtMut,
    /// Extensions for the token account (including compressible config)
    pub extensions: Option<Vec<ZExtensionStructMut<'a>>>,
}
impl<'a> Deref for ZCompressedTokenMut<'a> {
    type Target = <CTokenMeta as ZeroCopyAtMut<'a>>::ZeroCopyAtMut;

    fn deref(&self) -> &Self::Target {
        &self.__meta
    }
}

impl DerefMut for ZCompressedTokenMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.__meta
    }
}

impl<'a> ZeroCopyAt<'a> for CToken {
    type ZeroCopyAt = ZCToken<'a>;

    #[profile]
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        let (__meta, bytes) = <CTokenMeta as ZeroCopyAt<'a>>::zero_copy_at(bytes)?;
        let (extensions, bytes) = if !bytes.is_empty() {
            // Check if first byte is AccountType::Account (value 2) for SPL Token 2022 compatibility
            let extension_start = if bytes.first() == Some(&2) {
                // Skip AccountType::Account byte at position 165
                &bytes[1..]
            } else {
                return Err(ZeroCopyError::Size);
            };

            let (extensions, remaining_bytes) =
                <Option<Vec<ExtensionStruct>> as ZeroCopyAt<'a>>::zero_copy_at(extension_start)?;
            (extensions, remaining_bytes)
        } else {
            (None, bytes)
        };
        Ok((ZCToken { __meta, extensions }, bytes))
    }
}

impl CToken {
    /// Zero-copy deserialization with initialization check.
    /// Returns an error if the account is not initialized (byte 108 must be 1).
    #[profile]
    pub fn zero_copy_at_checked(
        bytes: &[u8],
    ) -> Result<(ZCToken<'_>, &[u8]), crate::error::CTokenError> {
        // Check minimum size for state field at byte 108
        if bytes.len() < 109 {
            return Err(crate::error::CTokenError::InvalidAccountData);
        }

        // Verify account is initialized (state byte at offset 108 must be 1)
        if bytes[108] != 1 {
            return Err(crate::error::CTokenError::InvalidAccountState);
        }

        // Proceed with normal deserialization
        Ok(CToken::zero_copy_at(bytes)?)
    }

    /// Mutable zero-copy deserialization with initialization check.
    /// Returns an error if the account is not initialized (byte 108 must be 1).
    #[profile]
    pub fn zero_copy_at_mut_checked(
        bytes: &mut [u8],
    ) -> Result<(ZCompressedTokenMut<'_>, &mut [u8]), crate::error::CTokenError> {
        // Check minimum size for state field at byte 108
        if bytes.len() < 109 {
            return Err(crate::error::CTokenError::InvalidAccountData);
        }

        // Verify account is initialized (state byte at offset 108 must be 1)
        if bytes[108] != 1 {
            return Err(crate::error::CTokenError::InvalidAccountState);
        }

        Ok(CToken::zero_copy_at_mut(bytes)?)
    }
}

impl<'a> ZeroCopyAtMut<'a> for CToken {
    type ZeroCopyAtMut = ZCompressedTokenMut<'a>;

    #[profile]
    #[inline(always)]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        let (__meta, bytes) = <CTokenMeta as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)?;
        let (extensions, bytes) = if !bytes.is_empty() {
            // Check if first byte is AccountType::Account (value 2) for SPL Token 2022 compatibility
            let extension_start = if bytes.first() == Some(&2) {
                // Skip AccountType::Account byte at position 165
                &mut bytes[1..]
            } else {
                return Err(ZeroCopyError::Size);
            };

            let (extensions, remaining_bytes) = <Option<Vec<ExtensionStruct>> as ZeroCopyAtMut<
                'a,
            >>::zero_copy_at_mut(extension_start)?;
            (extensions, remaining_bytes)
        } else {
            (None, bytes)
        };
        Ok((ZCompressedTokenMut { __meta, extensions }, bytes))
    }
}

impl ZCompressedTokenMetaMut<'_> {
    /// Set the delegate field by updating both the COption discriminator and value
    pub fn set_delegate(&mut self, delegate: Option<Pubkey>) -> Result<(), ZeroCopyError> {
        match (&mut self.delegate, delegate) {
            (Some(delegate), Some(new)) => {
                **delegate = new;
            }
            (Some(delegate), None) => {
                // Set discriminator to 0 (None)
                self.delegate_option[0] = 0;
                **delegate = Pubkey::default();
            }
            (None, Some(new)) => {
                self.delegate_option[0] = 1;
                let pubkey_bytes = unsafe {
                    std::slice::from_raw_parts_mut(self.delegate_option.as_mut_ptr().add(4), 32)
                };
                let (mut delegate_pubkey, _) = Pubkey::zero_copy_at_mut(pubkey_bytes)?;
                *delegate_pubkey = new;
                self.delegate = Some(delegate_pubkey);
            }
            (None, None) => {}
        }
        Ok(())
    }

    /// Set the is_native field by updating both the COption discriminator and value
    pub fn set_is_native(&mut self, is_native: Option<u64>) -> Result<(), ZeroCopyError> {
        match (&mut self.is_native, is_native) {
            (Some(native_value), Some(new)) => {
                **native_value = new.into();
            }
            (Some(native_value), None) => {
                // Set discriminator to 0 (None)
                self.is_native_option[0] = 0;
                **native_value = 0u64.into();
                self.is_native = None;
            }
            (None, Some(new)) => {
                self.is_native_option[0] = 1;
                let value_bytes = unsafe {
                    std::slice::from_raw_parts_mut(self.is_native_option.as_mut_ptr().add(4), 8)
                };
                let (mut native_value, _) =
                    zerocopy::Ref::<&mut [u8], zerocopy::little_endian::U64>::from_prefix(
                        value_bytes,
                    )?;
                *native_value = new.into();
                self.is_native = Some(native_value);
            }
            (None, None) => {}
        }
        Ok(())
    }

    /// Set the close_authority field by updating both the COption discriminator and value
    pub fn set_close_authority(
        &mut self,
        close_authority: Option<Pubkey>,
    ) -> Result<(), ZeroCopyError> {
        match (&mut self.close_authority, close_authority) {
            (Some(authority), Some(new)) => {
                **authority = new;
            }
            (Some(authority), None) => {
                // Set discriminator to 0 (None)
                self.close_authority_option[0] = 0;
                **authority = Pubkey::default();
                self.close_authority = None;
            }
            (None, Some(new)) => {
                self.close_authority_option[0] = 1;
                let pubkey_bytes = unsafe {
                    std::slice::from_raw_parts_mut(
                        self.close_authority_option.as_mut_ptr().add(4),
                        32,
                    )
                };
                let (mut close_authority_pubkey, _) = Pubkey::zero_copy_at_mut(pubkey_bytes)?;
                *close_authority_pubkey = new;
                self.close_authority = Some(close_authority_pubkey);
            }
            (None, None) => {}
        }
        Ok(())
    }
}

// Configuration for initializing a compressed token
#[derive(Debug, Clone)]
pub struct CompressedTokenConfig {
    pub delegate: bool,
    pub is_native: bool,
    pub close_authority: bool,
    pub extensions: Vec<ExtensionStructConfig>,
}

impl CompressedTokenConfig {
    pub fn new(delegate: bool, is_native: bool, close_authority: bool) -> Self {
        Self {
            delegate,
            is_native,
            close_authority,
            extensions: vec![],
        }
    }
    pub fn new_compressible(delegate: bool, is_native: bool, close_authority: bool) -> Self {
        Self {
            delegate,
            is_native,
            close_authority,
            extensions: vec![ExtensionStructConfig::Compressible(CompressionInfoConfig {
                rent_config: (),
            })],
        }
    }
}

impl<'a> ZeroCopyNew<'a> for CToken {
    type ZeroCopyConfig = CompressedTokenConfig;
    type Output = ZCompressedTokenMut<'a>;

    fn byte_len(config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        // mint: 32 bytes
        // owner: 32 bytes
        // amount: 8 bytes
        // delegate: 4 bytes discriminator + 32 bytes pubkey
        // state: 1 byte
        // is_native: 4 bytes discriminator + 8 bytes u64
        // delegated_amount: 8 bytes
        // close_authority: 4 bytes discriminator + 32 bytes pubkey
        // Total: 165 bytes (SPL Token Account size)
        let mut len = 165;
        // Add AccountType byte for SPL Token 2022 compatibility (always present if we have extensions)
        if !config.extensions.is_empty() {
            len += 1; // AccountType::Account byte at position 165
            len += 1; // Option discriminant for extensions (Some = 1)
            len += <Vec<ExtensionStruct> as ZeroCopyNew<'a>>::byte_len(&config.extensions)?;
        }
        Ok(len)
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        if bytes.len() < Self::byte_len(&config)? {
            msg!("CToken new_zero_copy Insufficient buffer size");
            return Err(ZeroCopyError::ArraySize(
                bytes.len(),
                Self::byte_len(&config)?,
            ));
        }
        if bytes[108] != 0 {
            msg!("Account already initialized");
            return Err(ZeroCopyError::MemoryNotZeroed);
        }
        // Set the state to Initialized (1) at offset 108 (32 mint + 32 owner + 8 amount + 36 delegate)
        bytes[108] = 1; // AccountState::Initialized

        // Set discriminator bytes based on config
        // delegate discriminator at offset 72 (32 mint + 32 owner + 8 amount)
        bytes[72] = if config.delegate { 1 } else { 0 };

        // is_native discriminator at offset 109 (72 + 36 delegate + 1 state)
        bytes[109] = if config.is_native { 1 } else { 0 };

        // close_authority discriminator at offset 129 (109 + 12 is_native + 8 delegated_amount)
        bytes[129] = if config.close_authority { 1 } else { 0 };

        // Initialize extensions if present
        if !config.extensions.is_empty() {
            // Set AccountType::Account byte at position 165 for SPL Token 2022 compatibility
            bytes[165] = 2; // AccountType::Account = 2

            // Set Option discriminant for extensions (Some = 1) at position 166
            bytes[166] = 1;

            // Extensions Vec starts after the Option discriminant (167 bytes)
            let extension_bytes = &mut bytes[167..];

            // Write Vec length (4 bytes little-endian)
            let len = config.extensions.len() as u32;
            extension_bytes[0..4].copy_from_slice(&len.to_le_bytes());

            // Initialize each extension
            let mut current_bytes = &mut extension_bytes[4..];
            for extension_config in &config.extensions {
                let (_, remaining_bytes) = <ExtensionStruct as ZeroCopyNew<'_>>::new_zero_copy(
                    current_bytes,
                    extension_config.clone(),
                )?;
                current_bytes = remaining_bytes;
            }
        }
        CToken::zero_copy_at_mut(bytes)
    }
}
