// This file stores constants which do not have to be configured.

use light_macros::pubkey;

use crate::Pubkey;

pub const DEFAULT_BATCH_ADDRESS_TREE_HEIGHT: u32 = 40;

pub const DEFAULT_BATCH_STATE_TREE_HEIGHT: u32 = 32;

pub const DEFAULT_NUM_BATCHES: u64 = 2;

pub const TEST_DEFAULT_BATCH_SIZE: u64 = 50;

pub const TEST_DEFAULT_ZKP_BATCH_SIZE: u64 = 10;

pub const DEFAULT_BATCH_SIZE: u64 = 50000;

pub const DEFAULT_ZKP_BATCH_SIZE: u64 = 500;

pub const DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE: u64 = 20 * 1024 + 8;

pub const ADDRESS_TREE_INIT_ROOT_40: [u8; 32] = [
    28, 65, 107, 255, 208, 234, 51, 3, 131, 95, 62, 130, 202, 177, 176, 26, 216, 81, 64, 184, 200,
    25, 95, 124, 248, 129, 44, 109, 229, 146, 106, 76,
];

pub const ACCOUNT_COMPRESSION_PROGRAM_ID: Pubkey =
    pubkey!("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq");

pub const BATCHED_STATE_TREE_TYPE: u64 = 3;
pub const BATCHED_ADDRESS_TREE_TYPE: u64 = 4;
pub const OUTPUT_QUEUE_TYPE: u64 = 5;
pub const NUM_BATCHES: usize = 2;
