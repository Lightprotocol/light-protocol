// 1 in little endian (for compressed mint accounts)
pub const COMPRESSED_MINT_DISCRIMINATOR: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];
// 2 in little endian
pub const TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR: [u8; 8] = [2, 0, 0, 0, 0, 0, 0, 0];
// 3 in big endian (for V2 token accounts in batched trees)
pub const TOKEN_COMPRESSED_ACCOUNT_V2_DISCRIMINATOR: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 3];
pub const TOKEN_COMPRESSED_ACCOUNT_V3_DISCRIMINATOR: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 4];
pub const BUMP_CPI_AUTHORITY: u8 = 254;
pub const NOT_FROZEN: bool = false;
pub const POOL_SEED: &[u8] = b"pool";

/// Maximum number of pool accounts that can be created for each mint.
pub const NUM_MAX_POOL_ACCOUNTS: u8 = 5;
