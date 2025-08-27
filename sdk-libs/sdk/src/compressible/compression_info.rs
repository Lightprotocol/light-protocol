use std::borrow::Cow;

use solana_clock::Clock;
use solana_sysvar::Sysvar;

use crate::{AnchorDeserialize, AnchorSerialize};

/// Trait for compressible accounts.
pub trait HasCompressionInfo {
    fn compression_info(&self) -> &CompressionInfo;
    fn compression_info_mut(&mut self) -> &mut CompressionInfo;
    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo>;
    fn set_compression_info_none(&mut self);
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

    /// Checks if the account can be compressed based on the compression delay constant.
    pub fn can_compress(&self, compression_delay: u64) -> Result<bool, crate::ProgramError> {
        let current_slot = Clock::get()?.slot;
        Ok(current_slot >= self.last_written_slot + compression_delay)
    }

    /// Gets the number of slots remaining before compression is allowed
    pub fn slots_until_compressible(
        &self,
        compression_delay: u64,
    ) -> Result<u64, crate::ProgramError> {
        let current_slot = Clock::get()?.slot;
        Ok((self.last_written_slot + compression_delay).saturating_sub(current_slot))
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
