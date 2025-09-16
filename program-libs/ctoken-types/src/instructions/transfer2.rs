use std::fmt::Debug;

use light_compressed_account::{
    compressed_account::PackedMerkleContext,
    instruction_data::{compressed_proof::CompressedProof, cpi_context::CompressedCpiContext},
};
use light_zero_copy::{ZeroCopy, ZeroCopyMut, ZeroCopyNew};
use zerocopy::Ref;

use crate::{AnchorDeserialize, AnchorSerialize, CTokenError};

#[repr(C)]
#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    PartialEq,
    AnchorSerialize,
    AnchorDeserialize,
    ZeroCopy,
    ZeroCopyMut,
)]
pub struct MultiInputTokenDataWithContext {
    pub owner: u8,
    pub amount: u64,
    pub has_delegate: bool, // Optional delegate is set
    pub delegate: u8,
    pub mint: u8,
    pub version: u8,
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
}

#[repr(C)]
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    AnchorSerialize,
    AnchorDeserialize,
    ZeroCopy,
    ZeroCopyMut,
)]
pub struct MultiTokenTransferOutputData {
    pub owner: u8,
    pub amount: u64,
    pub has_delegate: bool, // Optional delegate is set
    pub delegate: u8,
    pub mint: u8,
    pub version: u8,
    pub merkle_tree: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
#[repr(C)]
pub enum CompressionMode {
    Compress,
    Decompress,
    /// Compresses ctoken account and closes it
    /// Signer must be owner or rent authority, if rent authority ctoken account must be compressible
    /// Not implemented for spl token accounts.
    CompressAndClose,
}

pub const COMPRESS: u8 = 0u8;
pub const DECOMPRESS: u8 = 1u8;
pub const COMPRESS_AND_CLOSE: u8 = 2u8;

impl<'a> light_zero_copy::traits::ZeroCopyAtMut<'a> for CompressionMode {
    type ZeroCopyAtMut = Ref<&'a mut [u8], u8>;
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (mode, bytes) = zerocopy::Ref::<&mut [u8], u8>::from_prefix(bytes)?;

        Ok((mode, bytes))
    }
}

impl<'a> ZeroCopyNew<'a> for CompressionMode {
    type ZeroCopyConfig = ();
    type Output = Ref<&'a mut [u8], u8>;

    fn byte_len(
        _config: &Self::ZeroCopyConfig,
    ) -> Result<usize, light_zero_copy::errors::ZeroCopyError> {
        Ok(1) // CompressionMode enum size is always 1 byte
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (mode, remaining_bytes) = zerocopy::Ref::<&mut [u8], u8>::from_prefix(bytes)?;

        Ok((mode, remaining_bytes))
    }
}

#[repr(C)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
pub struct Compression {
    pub mode: CompressionMode,
    pub amount: u64,
    pub mint: u8,
    pub source_or_recipient: u8,
    pub authority: u8, // Index of owner or delegate account
    /// pool account index for spl token Compression/Decompression
    /// rent_recipient_index for CompressAndClose
    pub pool_account_index: u8, // This account is not necessary to decompress ctokens because there are no token pools
    /// pool index for spl token Compression/Decompression
    /// compressed account index for CompressAndClose
    pub pool_index: u8, // This account is not necessary to decompress ctokens because there are no token pools
    pub bump: u8, // This account is not necessary to decompress ctokens because there are no token pools
}

impl ZCompression<'_> {
    pub fn get_rent_recipient_index(&self) -> Result<u8, CTokenError> {
        match self.mode {
            ZCompressionMode::CompressAndClose => Ok(self.pool_account_index),
            _ => Err(CTokenError::InvalidCompressionMode),
        }
    }
    pub fn get_compressed_token_account_index(&self) -> Result<u8, CTokenError> {
        match self.mode {
            ZCompressionMode::CompressAndClose => Ok(self.pool_index),
            _ => Err(CTokenError::InvalidCompressionMode),
        }
    }
    pub fn get_destination_index(&self) -> Result<u8, CTokenError> {
        match self.mode {
            ZCompressionMode::CompressAndClose => Ok(self.bump),
            _ => Err(CTokenError::InvalidCompressionMode),
        }
    }
}

impl Compression {
    pub fn compress_and_close(
        amount: u64,
        mint: u8,
        source_or_recipient: u8,
        authority: u8,
        rent_recipient_index: u8,
        compressed_account_index: u8,
        destination_index: u8,
    ) -> Self {
        Compression {
            amount, // the full balance of the ctoken account to be compressed
            mode: CompressionMode::CompressAndClose,
            mint,
            source_or_recipient,
            authority,
            pool_account_index: rent_recipient_index,
            pool_index: compressed_account_index,
            bump: destination_index,
        }
    }
    pub fn compress(amount: u64, mint: u8, source_or_recipient: u8, authority: u8) -> Self {
        Compression {
            amount,
            mode: CompressionMode::Compress,
            mint,
            source_or_recipient,
            authority,
            pool_account_index: 0,
            pool_index: 0,
            bump: 0,
        }
    }
    pub fn compress_spl(
        amount: u64,
        mint: u8,
        source_or_recipient: u8,
        authority: u8,
        pool_account_index: u8,
        pool_index: u8,
        bump: u8,
    ) -> Self {
        Compression {
            amount,
            mode: CompressionMode::Compress,
            mint,
            source_or_recipient,
            authority,
            pool_account_index,
            pool_index,
            bump,
        }
    }
    pub fn compress_ctoken(amount: u64, mint: u8, source_or_recipient: u8, authority: u8) -> Self {
        Compression {
            amount,
            mode: CompressionMode::Compress,
            mint,
            source_or_recipient,
            authority,
            pool_account_index: 0,
            pool_index: 0,
            bump: 0,
        }
    }
    pub fn decompress(amount: u64, mint: u8, source_or_recipient: u8) -> Self {
        Compression {
            amount,
            mode: CompressionMode::Decompress,
            mint,
            source_or_recipient,
            authority: 0,
            pool_account_index: 0,
            pool_index: 0,
            bump: 0,
        }
    }
    pub fn decompress_spl(
        amount: u64,
        mint: u8,
        source_or_recipient: u8,
        pool_account_index: u8,
        pool_index: u8,
        bump: u8,
    ) -> Self {
        Compression {
            amount,
            mode: CompressionMode::Decompress,
            mint,
            source_or_recipient,
            authority: 0,
            pool_account_index,
            pool_index,
            bump,
        }
    }

    pub fn decompress_ctoken(amount: u64, mint: u8, source_or_recipient: u8) -> Self {
        Compression {
            amount,
            mode: CompressionMode::Decompress,
            mint,
            source_or_recipient,
            authority: 0,
            pool_account_index: 0,
            pool_index: 0,
            bump: 0,
        }
    }
}

impl ZCompressionMut<'_> {
    pub fn mode(&self) -> Result<CompressionMode, CTokenError> {
        match *self.mode {
            COMPRESS => Ok(CompressionMode::Compress),
            DECOMPRESS => Ok(CompressionMode::Decompress),
            COMPRESS_AND_CLOSE => Ok(CompressionMode::CompressAndClose),
            _ => Err(CTokenError::InvalidCompressionMode),
        }
    }
}

impl ZCompression<'_> {
    pub fn new_balance_compressed_account(&self, current_balance: u64) -> Result<u64, CTokenError> {
        let new_balance = match self.mode {
            ZCompressionMode::Compress | ZCompressionMode::CompressAndClose => {
                // Compress: add to balance (tokens are being added to compressed pool)
                current_balance
                    .checked_add((*self.amount).into())
                    .ok_or(CTokenError::ArithmeticOverflow)
            }
            ZCompressionMode::Decompress => {
                // Decompress: subtract from balance (tokens are being removed from compressed pool)
                current_balance
                    .checked_sub((*self.amount).into())
                    .ok_or(CTokenError::CompressInsufficientFunds)
            }
        }?;
        Ok(new_balance)
    }

    pub fn new_balance_solana_account(&self, current_balance: u64) -> Result<u64, CTokenError> {
        let new_balance = match self.mode {
            ZCompressionMode::Compress | ZCompressionMode::CompressAndClose => {
                // Compress: add to balance (tokens are being added to compressed pool)
                current_balance
                    .checked_sub((*self.amount).into())
                    .ok_or(CTokenError::InsufficientSupply)
            }
            ZCompressionMode::Decompress => {
                // Decompress: subtract from balance (tokens are being removed from compressed pool)
                current_balance
                    .checked_add((*self.amount).into())
                    .ok_or(CTokenError::ArithmeticOverflow)
            }
        }?;
        Ok(new_balance)
    }
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut)]
pub struct CompressedTokenInstructionDataTransfer2 {
    pub with_transaction_hash: bool,
    pub with_lamports_change_account_merkle_tree_index: bool,
    // Set zero if unused
    pub lamports_change_account_merkle_tree_index: u8,
    pub lamports_change_account_owner_index: u8,
    pub proof: Option<CompressedProof>,
    pub in_token_data: Vec<MultiInputTokenDataWithContext>,
    pub out_token_data: Vec<MultiTokenTransferOutputData>,
    // put accounts with lamports first, stop adding values after TODO: only access by get to prevent oob errors
    pub in_lamports: Option<Vec<u64>>,
    pub out_lamports: Option<Vec<u64>>,
    pub in_tlv: Option<Vec<Vec<u8>>>,
    pub out_tlv: Option<Vec<Vec<u8>>>,
    pub compressions: Option<Vec<Compression>>,
    pub cpi_context: Option<CompressedCpiContext>,
}
