use std::borrow::Cow;

use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use solana_account_info::AccountInfo;
use solana_clock::Clock;
use solana_sysvar::Sysvar;

use crate::{instruction::PackedAccounts, AnchorDeserialize, AnchorSerialize};

/// Trait for types that can be packed for compression.
///
/// Packing is a space optimization technique where 32-byte `Pubkey` fields are replaced
/// with 1-byte indices that reference positions in a `remaining_accounts` array.
/// This significantly reduces instruction data size.
///
/// For types without Pubkeys, implement identity packing (return self).
pub trait Pack {
    /// The packed version of this type
    type Packed: AnchorSerialize + Clone + std::fmt::Debug;

    /// Pack this type, replacing Pubkeys with indices
    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed;
}

/// Trait for types that can be unpacked from their compressed form.
///
/// This is used on-chain to convert packed instruction data back to the original types.
/// The unpacking resolves u8 indices back to Pubkeys using the remaining_accounts array.
///
/// For identity-packed types, unpack returns a clone of self.
pub trait Unpack {
    /// The unpacked version of this type
    type Unpacked;

    /// Unpack this type, resolving indices to Pubkeys from remaining_accounts
    fn unpack(
        &self,
        remaining_accounts: &[AccountInfo],
    ) -> Result<Self::Unpacked, crate::ProgramError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum AccountState {
    Initialized,
    Frozen,
}

/// Trait for compressible accounts.
pub trait HasCompressionInfo {
    fn compression_info(&self) -> &CompressionInfo;
    fn compression_info_mut(&mut self) -> &mut CompressionInfo;
    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo>;
    fn set_compression_info_none(&mut self);
}

/// Compile-time constant for the compressed INIT_SPACE of an
/// account type, considering #[compress_as(... = None)] overrides.
pub trait CompressedInitSpace {
    const COMPRESSED_INIT_SPACE: usize;
}

/// Trait for accounts that want to customize how their state gets compressed,
/// instead of just copying the current onchain state.
pub trait CompressAs {
    /// The type that will be stored in the compressed state.
    /// Can be `Self` or a different type entirely for maximum flexibility.
    type Output: crate::AnchorSerialize
        + crate::AnchorDeserialize
        + crate::LightDiscriminator
        + crate::account::Size
        + HasCompressionInfo
        + Default
        + Clone;

    /// Returns the data that should be stored in the compressed state. This
    /// allows developers to reset some fields while keeping others, or even
    /// return a completely different type during compression.
    ///
    /// compression_info must ALWAYS be None in the returned data. This
    /// eliminates the need for mutation after calling compress_as().
    ///
    /// Uses Cow (Clone on Write) for performance - typically returns owned data
    /// since compression_info must be None (different from onchain state).
    ///
    /// # Example - Default.
    /// ```rust
    /// impl CompressAs for UserRecord {
    ///     type Output = Self;
    ///     
    ///     fn compress_as(&self) -> Cow<'_, Self::Output> {
    ///         Cow::Owned(Self {
    ///             compression_info: None,     // ALWAYS None
    ///             owner: self.owner,
    ///             name: self.name.clone(),
    ///             score: self.score,
    ///         })
    ///     }
    /// }
    /// ```
    ///
    /// # Example - Custom Compression (reset some values)
    /// ```rust
    /// impl CompressAs for Oracle {
    ///     type Output = Self;
    ///     
    ///     fn compress_as(&self) -> Cow<'_, Self::Output> {
    ///         Cow::Owned(Self {
    ///             compression_info: None,     // ALWAYS None
    ///             initialized: false,         // set false
    ///             observation_index: 0,       // set 0
    ///             pool_id: self.pool_id,      // default
    ///             observations: None,         // set None
    ///             padding: self.padding,
    ///         })
    ///     }
    /// }
    /// ```
    ///
    /// # Example - Different Type
    /// ```rust
    /// impl CompressAs for LargeGameState {
    ///     type Output = CompactGameState;
    ///     
    ///     fn compress_as(&self) -> Cow<'_, Self::Output> {
    ///         Cow::Owned(CompactGameState {
    ///             compression_info: None,     // ALWAYS None
    ///             player_id: self.player_id,
    ///             level: self.level,
    ///             // Skip large arrays, temporary state, etc.
    ///         })
    ///     }
    /// }
    /// ```
    fn compress_as(&self) -> Cow<'_, Self::Output>;
}

/// Information for compressible accounts that tracks when the account was last
/// written
#[derive(Debug, Clone, Default, AnchorSerialize, AnchorDeserialize)]
pub struct CompressionInfo {
    /// The slot when this account was last written/decompressed
    pub last_written_slot: u64,
    /// 0 not inited, 1 decompressed, 2 compressed
    pub state: CompressionState,
}

#[derive(Debug, Clone, Default, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub enum CompressionState {
    #[default]
    Uninitialized,
    Decompressed,
    Compressed,
}

// TODO: move to proper rent_func.
impl CompressionInfo {
    /// Creates new compression info with the current slot and sets state to
    /// decompressed.
    pub fn new_decompressed() -> Result<Self, crate::ProgramError> {
        Ok(Self {
            last_written_slot: Clock::get()?.slot,
            state: CompressionState::Decompressed,
        })
    }

    /// Updates the last written slot to the current slot
    pub fn bump_last_written_slot(&mut self) -> Result<(), crate::ProgramError> {
        self.last_written_slot = Clock::get()?.slot;
        Ok(())
    }

    /// Sets the last written slot to a specific value
    pub fn set_last_written_slot(&mut self, slot: u64) {
        self.last_written_slot = slot;
    }

    /// Gets the last written slot
    pub fn last_written_slot(&self) -> u64 {
        self.last_written_slot
    }

    /// Set compressed
    pub fn set_compressed(&mut self) {
        self.state = CompressionState::Compressed;
    }

    /// Check if the account is compressed
    pub fn is_compressed(&self) -> bool {
        self.state == CompressionState::Compressed
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::Space for CompressionInfo {
    const INIT_SPACE: usize = 8 + 1; // u64 + state enum
}

/// Generic compressed account data structure for decompress operations
/// This is generic over the account variant type, allowing programs to use their specific enums
///
/// # Type Parameters
/// * `T` - The program-specific compressed account variant enum (e.g., CompressedAccountVariant)
///
/// # Fields
/// * `meta` - The compressed account metadata containing tree info, address, and output index
/// * `data` - The program-specific account variant enum
/// * `seeds` - The PDA seeds (without bump) used to derive the PDA address
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CompressedAccountData<T> {
    pub meta: CompressedAccountMetaNoLamportsNoAddress,
    /// Program-specific account variant enum
    pub data: T,
}
