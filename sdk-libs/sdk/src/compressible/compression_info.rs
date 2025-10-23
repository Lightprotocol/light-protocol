use std::borrow::Cow;

use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use solana_account_info::AccountInfo;
use solana_clock::Clock;
use solana_sysvar::Sysvar;

use crate::{instruction::PackedAccounts, AnchorDeserialize, AnchorSerialize};

/// Replace 32-byte Pubkeys with 1-byte indices to save space.
/// If your type has no Pubkeys, just return self.
pub trait Pack {
    type Packed: AnchorSerialize + Clone + std::fmt::Debug;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed;
}

/// Convert indices back to Pubkeys using remaining_accounts.
pub trait Unpack {
    type Unpacked;

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

pub trait HasCompressionInfo {
    fn compression_info(&self) -> &CompressionInfo;
    fn compression_info_mut(&mut self) -> &mut CompressionInfo;
    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo>;
    fn set_compression_info_none(&mut self);
}

/// Account space when compressed.
pub trait CompressedInitSpace {
    const COMPRESSED_INIT_SPACE: usize;
}

/// Override what gets stored when compressing. Return Self or a different type.
pub trait CompressAs {
    type Output: crate::AnchorSerialize
        + crate::AnchorDeserialize
        + crate::LightDiscriminator
        + crate::account::Size
        + HasCompressionInfo
        + Default
        + Clone;

    /// Return data to store. compression_info must be None.
    fn compress_as(&self) -> Cow<'_, Self::Output>;
}

/// Last write slot and compression state.
#[derive(Debug, Clone, Default, AnchorSerialize, AnchorDeserialize)]
pub struct CompressionInfo {
    pub last_written_slot: u64,
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
    pub fn new_decompressed() -> Result<Self, crate::ProgramError> {
        Ok(Self {
            last_written_slot: Clock::get()?.slot,
            state: CompressionState::Decompressed,
        })
    }

    pub fn bump_last_written_slot(&mut self) -> Result<(), crate::ProgramError> {
        self.last_written_slot = Clock::get()?.slot;
        Ok(())
    }

    pub fn set_last_written_slot(&mut self, slot: u64) {
        self.last_written_slot = slot;
    }

    pub fn last_written_slot(&self) -> u64 {
        self.last_written_slot
    }

    pub fn set_compressed(&mut self) {
        self.state = CompressionState::Compressed;
    }

    pub fn is_compressed(&self) -> bool {
        self.state == CompressionState::Compressed
    }
}

/// Space calculation without anchor (like anchor_lang::Space but standalone).
pub trait Space {
    const INIT_SPACE: usize;
}

impl Space for CompressionInfo {
    const INIT_SPACE: usize = 8 + 1; // u64 + state enum (u8)
}

#[cfg(feature = "anchor")]
impl anchor_lang::Space for CompressionInfo {
    const INIT_SPACE: usize = <Self as Space>::INIT_SPACE;
}

/// Compressed account data used when decompressing.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CompressedAccountData<T> {
    pub meta: CompressedAccountMetaNoLamportsNoAddress,
    pub data: T,
}
