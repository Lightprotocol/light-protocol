//! LightAccount trait definition for compressible account data structs.

use anchor_lang::prelude::*;
use light_hasher::DataHasher;
use solana_program_error::ProgramError;

use crate::{
    account::compression_info::CompressionInfo,
    instruction::PackedAccounts,
    program::config::LightConfig,
    light_account_checks::{packed_accounts::ProgramPackedAccounts, AccountInfoTrait},
};

pub enum AccountType {
    Pda,
    PdaZeroCopy,
    Token,
    Ata,
    Mint,
}

/// Trait for compressible account data structs.
///
/// Supertraits:
/// - `Discriminator` from light-account-checks for the 8-byte discriminator
/// - `DataHasher` from light-hasher for Merkle tree hashing
pub trait LightAccount:
    Sized
    + Clone
    + AnchorSerialize
    + AnchorDeserialize
    + crate::light_account_checks::discriminator::Discriminator
    + DataHasher
{
    const ACCOUNT_TYPE: AccountType;
    /// Packed version (Pubkeys -> u8 indices)
    type Packed: AnchorSerialize + AnchorDeserialize;

    /// Compile-time size for space allocation
    const INIT_SPACE: usize;

    /// Get compression info reference
    fn compression_info(&self) -> &CompressionInfo;

    /// Get mutable compression info reference
    fn compression_info_mut(&mut self) -> &mut CompressionInfo;

    /// Set compression info to decompressed state (used at decompression)
    fn set_decompressed(&mut self, config: &LightConfig, current_slot: u64);

    /// Convert to packed form (Pubkeys -> indices)
    fn pack(
        &self,
        accounts: &mut PackedAccounts,
    ) -> std::result::Result<Self::Packed, ProgramError>;

    /// Convert from packed form (indices -> Pubkeys)
    fn unpack<A: AccountInfoTrait>(
        packed: &Self::Packed,
        accounts: &ProgramPackedAccounts<A>,
    ) -> std::result::Result<Self, ProgramError>;
}
