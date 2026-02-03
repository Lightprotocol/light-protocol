//! D2 Test: compress_as attribute absent
//!
//! Exercises the code path where no #[compress_as] attribute is present.
//! All fields use self.field directly for compression.

use anchor_lang::prelude::*;
use light_account::{CompressionInfo, LightAccount, LightDiscriminator};

/// A struct without any compress_as attribute.
/// All fields are compressed as-is using self.field.
#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct NoCompressAsRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
    pub counter: u64,
    pub flag: bool,
}
