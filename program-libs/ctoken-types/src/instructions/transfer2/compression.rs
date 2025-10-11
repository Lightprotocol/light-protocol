use std::fmt::Debug;

use light_zero_copy::{
    errors::ZeroCopyError, traits::ZeroCopyAtMut, ZeroCopy, ZeroCopyMut, ZeroCopyNew,
};
use zerocopy::Ref;

use crate::{AnchorDeserialize, AnchorSerialize, CTokenError};

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

impl<'a> ZeroCopyAtMut<'a> for CompressionMode {
    type ZeroCopyAtMut = Ref<&'a mut [u8], u8>;
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        let (mode, bytes) = zerocopy::Ref::<&mut [u8], u8>::from_prefix(bytes)?;

        Ok((mode, bytes))
    }
}

impl<'a> ZeroCopyNew<'a> for CompressionMode {
    type ZeroCopyConfig = ();
    type Output = Ref<&'a mut [u8], u8>;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        Ok(1) // CompressionMode enum size is always 1 byte
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
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
    /// rent_sponsor_index for CompressAndClose
    pub pool_account_index: u8, // This account is not necessary to decompress ctokens because there are no token pools
    /// pool index for spl token Compression/Decompression
    /// compressed account index for CompressAndClose
    pub pool_index: u8, // This account is not necessary to decompress ctokens because there are no token pools
    pub bump: u8, // This account is not necessary to decompress ctokens because there are no token pools
}

impl ZCompression<'_> {
    pub fn get_rent_sponsor_index(&self) -> Result<u8, CTokenError> {
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
    pub fn compress_and_close_ctoken(
        amount: u64,
        mint: u8,
        source: u8,
        authority: u8,
        rent_sponsor_index: u8,
        compressed_account_index: u8,
        destination_index: u8,
    ) -> Self {
        Compression {
            amount, // the full balance of the ctoken account to be compressed
            mode: CompressionMode::CompressAndClose,
            mint,
            source_or_recipient: source,
            authority,
            pool_account_index: rent_sponsor_index,
            pool_index: compressed_account_index,
            bump: destination_index,
        }
    }

    pub fn compress_spl(
        amount: u64,
        mint: u8,
        source: u8,
        authority: u8,
        pool_account_index: u8,
        pool_index: u8,
        bump: u8,
    ) -> Self {
        Compression {
            amount,
            mode: CompressionMode::Compress,
            mint,
            source_or_recipient: source,
            authority,
            pool_account_index,
            pool_index,
            bump,
        }
    }
    pub fn compress_ctoken(amount: u64, mint: u8, source: u8, authority: u8) -> Self {
        Compression {
            amount,
            mode: CompressionMode::Compress,
            mint,
            source_or_recipient: source,
            authority,
            pool_account_index: 0,
            pool_index: 0,
            bump: 0,
        }
    }

    pub fn decompress_spl(
        amount: u64,
        mint: u8,
        recipient: u8,
        pool_account_index: u8,
        pool_index: u8,
        bump: u8,
    ) -> Self {
        Compression {
            amount,
            mode: CompressionMode::Decompress,
            mint,
            source_or_recipient: recipient,
            authority: 0,
            pool_account_index,
            pool_index,
            bump,
        }
    }

    pub fn decompress_ctoken(amount: u64, mint: u8, recipient: u8) -> Self {
        Compression {
            amount,
            mode: CompressionMode::Decompress,
            mint,
            source_or_recipient: recipient,
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
                // Compress: add to balance (tokens are being added to spl token pool)
                current_balance
                    .checked_add((*self.amount).into())
                    .ok_or(CTokenError::ArithmeticOverflow)
            }
            ZCompressionMode::Decompress => {
                // Decompress: subtract from balance (tokens are being removed from spl token pool)
                current_balance
                    .checked_sub((*self.amount).into())
                    .ok_or(CTokenError::CompressInsufficientFunds)
            }
        }?;
        Ok(new_balance)
    }
}
