use light_macros::pubkey_array;

// Program ID for light-compressed-token
pub const PROGRAM_ID: [u8; 32] = pubkey_array!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

// SPL Token Program ID
pub const SPL_TOKEN_PROGRAM_ID: [u8; 32] =
    pubkey_array!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

// SPL Token 2022 Program ID
pub const SPL_TOKEN_2022_PROGRAM_ID: [u8; 32] =
    pubkey_array!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

// Light System Program ID
pub const LIGHT_SYSTEM_PROGRAM_ID: [u8; 32] =
    pubkey_array!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");

// Account Compression Program ID
pub const ACCOUNT_COMPRESSION_PROGRAM_ID: [u8; 32] =
    pubkey_array!("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq");

// Account Compression Program ID
pub const ACCOUNT_COMPRESSION_AUTHORITY_PDA: [u8; 32] =
    pubkey_array!("HwXnGK3tPkkVY6P439H2p68AxpeuWXd5PcrAxFpbmfbA");

// Noop Program ID
pub const NOOP_PROGRAM_ID: [u8; 32] = pubkey_array!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

// CPI Authority PDA seed
pub const CPI_AUTHORITY_PDA_SEED: &[u8] = b"cpi_authority";

pub const CPI_AUTHORITY_PDA: [u8; 32] =
    pubkey_array!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");

// 2 in little endian
pub const TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR: [u8; 8] = [2, 0, 0, 0, 0, 0, 0, 0];
pub const BUMP_CPI_AUTHORITY: u8 = 254;
pub const NOT_FROZEN: bool = false;
pub const POOL_SEED: &[u8] = b"pool";

/// Maximum number of pool accounts that can be created for each mint.
pub const NUM_MAX_POOL_ACCOUNTS: u8 = 5;
pub const MINT_TO: [u8; 8] = [241, 34, 48, 186, 37, 179, 123, 192];
pub const TRANSFER: [u8; 8] = [163, 52, 200, 231, 140, 3, 69, 186];
pub const BATCH_COMPRESS: [u8; 8] = [65, 206, 101, 37, 147, 42, 221, 144];
pub const APPROVE: [u8; 8] = [69, 74, 217, 36, 115, 117, 97, 76];
pub const REVOKE: [u8; 8] = [170, 23, 31, 34, 133, 173, 93, 242];
pub const FREEZE: [u8; 8] = [255, 91, 207, 84, 251, 194, 254, 63];
pub const THAW: [u8; 8] = [226, 249, 34, 57, 189, 21, 177, 101];
pub const CREATE_TOKEN_POOL: [u8; 8] = [23, 169, 27, 122, 147, 169, 209, 152];
pub const CREATE_ADDITIONAL_TOKEN_POOL: [u8; 8] = [114, 143, 210, 73, 96, 115, 1, 228];
pub const TRANSFER2: u8 = 101;
