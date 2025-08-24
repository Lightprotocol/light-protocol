use std::fmt::Debug;

use light_compressed_account::{
    compressed_account::PackedMerkleContext,
    instruction_data::{compressed_proof::CompressedProof, cpi_context::CompressedCpiContext},
};
use light_zero_copy::{ZeroCopy, ZeroCopyMut, ZeroCopyNew};
use spl_pod::solana_msg::msg;
use zerocopy::Ref;

use crate::{AnchorDeserialize, AnchorSerialize, CTokenError};
// TODO: move to token data
#[repr(u8)]
pub enum TokenAccountVersion {
    V1 = 1u8,
    V2 = 2u8,
}

impl TokenAccountVersion {
    pub fn discriminator(&self) -> [u8; 8] {
        match self {
            TokenAccountVersion::V1 => [2, 0, 0, 0, 0, 0, 0, 0], // 2 le
            TokenAccountVersion::V2 => [0, 0, 0, 0, 0, 0, 0, 3], // 3 be
        }
    }

    /// Serializes amount to bytes using version-specific endianness
    /// V1: little-endian, V2: big-endian
    pub fn serialize_amount_bytes(&self, amount: u64) -> [u8; 32] {
        let mut amount_bytes = [0u8; 32];
        match self {
            TokenAccountVersion::V1 => {
                amount_bytes[24..].copy_from_slice(&amount.to_le_bytes());
            }
            TokenAccountVersion::V2 => {
                amount_bytes[24..].copy_from_slice(&amount.to_be_bytes());
            }
        }
        amount_bytes
    }
}

impl TryFrom<u8> for TokenAccountVersion {
    type Error = crate::CTokenError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(TokenAccountVersion::V1),
            2 => Ok(TokenAccountVersion::V2),
            _ => Err(crate::CTokenError::InvalidTokenDataVersion),
        }
    }
}

#[repr(C)]
#[derive(
    Debug, Clone, Default, PartialEq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
pub struct MultiInputTokenDataWithContext {
    pub amount: u64,
    pub merkle_context: PackedMerkleContext,
    pub root_index: u16,
    // From remaining accounts.
    pub mint: u8,
    pub owner: u8,
    pub with_delegate: bool,
    // Only used if with_delegate is true, we could also use 255 to indicate no delegate
    pub delegate: u8,
    pub version: u8,
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
    pub merkle_tree: u8,
    pub delegate: u8, // TODO: check whether we need delegate is set
    pub mint: u8,
    pub version: u8,
}
// TODO: allow repr(u8) in zero copy derive macro
#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum CompressionMode {
    Compress = COMPRESS,
    Decompress = DECOMPRESS,
    // CompressFull = COMPRESS_FULL, // Ignores the amount, we keep the amount for efficient zero copy
    //CompressAndClose = COMPRESS_AND_CLOSE, // Compresses the token and closes the account
}

pub const COMPRESS: u8 = 0u8;
pub const DECOMPRESS: u8 = 1u8;
//pub const COMPRESS_FULL: u8 = 2u8;
//pub const COMPRESS_AND_CLOSE: u8 = 3u8;

impl<'a> light_zero_copy::traits::ZeroCopyAt<'a> for CompressionMode {
    type ZeroCopyAt = CompressionMode;
    fn zero_copy_at(
        bytes: &'a [u8],
    ) -> Result<(Self::ZeroCopyAt, &'a [u8]), light_zero_copy::errors::ZeroCopyError> {
        let (mode, bytes) = bytes.split_at(1);
        let enm = match mode[0] {
            COMPRESS => Ok(CompressionMode::Compress),
            DECOMPRESS => Ok(CompressionMode::Decompress),
            // COMPRESS_FULL => Ok(CompressionMode::CompressFull),
            // COMPRESS_AND_CLOSE => Ok(CompressionMode::CompressAndClose),
            // TODO: add enum error
            _ => Err(light_zero_copy::errors::ZeroCopyError::IterFromOutOfBounds),
        }?;
        Ok((enm, bytes))
    }
}

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
        Ok(1) // CompressionMode is always 1 byte
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
    pub authority: u8,          // Index of owner or delegate account
    pub pool_account_index: u8, // This account is not necessary to decompress ctokens because there are no token pools
    pub pool_index: u8, // This account is not necessary to decompress ctokens because there are no token pools
    pub bump: u8, // This account is not necessary to decompress ctokens because there are no token pools
}

impl Compression {
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
            // COMPRESS_FULL => Ok(CompressionMode::CompressFull),
            // COMPRESS_AND_CLOSE => Ok(CompressionMode::CompressAndClose),
            _ => Err(CTokenError::InvalidCompressionMode),
        }
    }
}

impl ZCompression<'_> {
    pub fn new_balance_compressed_account(&self, current_balance: u64) -> Result<u64, CTokenError> {
        let new_balance = match self.mode {
            CompressionMode::Compress => {
                // Compress: add to balance (tokens are being added to compressed pool)
                current_balance
                    .checked_add((*self.amount).into())
                    .ok_or(CTokenError::ArithmeticOverflow)
            }
            CompressionMode::Decompress => {
                // Decompress: subtract from balance (tokens are being removed from compressed pool)
                current_balance
                    .checked_sub((*self.amount).into())
                    .ok_or(CTokenError::CompressInsufficientFunds)
            } //   CompressionMode::CompressFull => {
              //       // CompressFull: add entire amount to compressed pool (amount will be set to actual balance in preprocessing)
              //       current_balance
              //            .checked_add((*self.amount).into())
              //            .ok_or(CTokenError::ArithmeticOverflow)
              //    }
              // CompressionMode::CompressAndClose => {
              //      // CompressAndClose: add entire amount to compressed pool (amount will be set to actual balance in preprocessing)
              //     current_balance
              //          .checked_add((*self.amount).into())
              //          .ok_or(CTokenError::ArithmeticOverflow)
              //  }
        }?;
        Ok(new_balance)
    }

    pub fn new_balance_solana_account(&self, current_balance: u64) -> Result<u64, CTokenError> {
        let new_balance = match self.mode {
            CompressionMode::Compress => {
                // Compress: add to balance (tokens are being added to compressed pool)
                current_balance
                    .checked_sub((*self.amount).into())
                    .ok_or(CTokenError::InsufficientSupply)
            }
            CompressionMode::Decompress => {
                // Decompress: subtract from balance (tokens are being removed from compressed pool)
                current_balance
                    .checked_add((*self.amount).into())
                    .ok_or(CTokenError::ArithmeticOverflow)
            } //     CompressionMode::CompressFull => {
              //        // CompressFull: subtract entire amount from solana account (amount will be set to actual balance in preprocessing)
              //        current_balance
              ////            .checked_sub((*self.amount).into())
              //             .ok_or(CTokenError::ArithmeticOverflow)
              //     }
              //    CompressionMode::CompressAndClose => {
              //       // CompressAndClose: subtract entire amount from solana account (amount will be set to actual balance in preprocessing)
              //         current_balance
              //             .checked_sub((*self.amount).into())
              //             .ok_or(CTokenError::ArithmeticOverflow)
              //       }
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
    // TODO: put accounts with lamports first, stop adding values after TODO: only access by get to prevent oob errors
    pub out_lamports: Option<Vec<u64>>,
    // TODO:  put accounts with tlv first, stop adding values after TODO: only access by get to prevent oob errors
    pub in_tlv: Option<Vec<Vec<u8>>>,
    pub out_tlv: Option<Vec<Vec<u8>>>,
    pub compressions: Option<Vec<Compression>>,
    pub cpi_context: Option<CompressedCpiContext>,
}

/// Validate instruction data consistency (lamports and TLV checks)
pub fn validate_instruction_data(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
) -> Result<(), crate::CTokenError> {
    if let Some(ref in_lamports) = inputs.in_lamports {
        if in_lamports.len() != inputs.in_token_data.len() {
            msg!(
                "in_lamports {} != inputs in_token_data {}",
                in_lamports.len(),
                inputs.in_token_data.len()
            );
            return Err(CTokenError::InputAccountsLamportsLengthMismatch);
        }
    }
    if let Some(ref out_lamports) = inputs.out_lamports {
        if out_lamports.len() != inputs.out_token_data.len() {
            msg!(
                "outlamports {} != inputs out_token_data {}",
                out_lamports.len(),
                inputs.out_token_data.len()
            );
            return Err(CTokenError::OutputAccountsLamportsLengthMismatch);
        }
    }
    if inputs.in_tlv.is_some() {
        return Err(CTokenError::CompressedTokenAccountTlvUnimplemented);
    }
    if inputs.out_tlv.is_some() {
        return Err(CTokenError::CompressedTokenAccountTlvUnimplemented);
    }
    Ok(())
}
