//! Program IDs and constants for Light Protocol.
//!
//! Ported from `light-token-types/src/constants.rs` and `light-compressed-token-sdk/src/constants.rs`.

use solana_pubkey::Pubkey;

/// Light Compressed Token Program ID
pub const LIGHT_TOKEN_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

/// Light System Program ID
pub const LIGHT_SYSTEM_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");

/// Account Compression Program ID
pub const ACCOUNT_COMPRESSION_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq");

/// Account Compression Authority PDA
pub const ACCOUNT_COMPRESSION_AUTHORITY_PDA: Pubkey =
    Pubkey::from_str_const("HwXnGK3tPkkVY6P439H2p68AxpeuWXd5PcrAxFpbmfbA");

/// Noop Program ID (used for logging)
pub const NOOP_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

/// CPI Authority PDA for the Light Token Program
pub const CPI_AUTHORITY_PDA: Pubkey =
    Pubkey::from_str_const("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy");

/// SPL Token Program ID
pub const SPL_TOKEN_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// SPL Token 2022 Program ID
pub const SPL_TOKEN_2022_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// Default compressible config PDA (V1)
pub const LIGHT_TOKEN_CONFIG: Pubkey =
    Pubkey::from_str_const("ACXg8a7VaqecBWrSbdu73W4Pg9gsqXJ3EXAqkHyhvVXg");

/// Default rent sponsor PDA (V1)
pub const RENT_SPONSOR_V1: Pubkey =
    Pubkey::from_str_const("r18WwUxfG8kQ69bQPAB2jV6zGNKy3GosFGctjQoV4ti");

/// CPI Authority PDA seed
pub const CPI_AUTHORITY_PDA_SEED: &[u8] = b"cpi_authority";

/// CPI Authority bump
pub const BUMP_CPI_AUTHORITY: u8 = 254;

/// Pool seed for SPL token pool accounts
pub const POOL_SEED: &[u8] = b"pool";

/// Transfer2 instruction discriminator
pub const TRANSFER2_DISCRIMINATOR: u8 = 101;

/// Default max top-up (u16::MAX = no limit)
pub const DEFAULT_MAX_TOP_UP: u16 = u16::MAX;

/// Wrapped SOL mint
pub const WSOL_MINT: Pubkey = Pubkey::from_str_const("So11111111111111111111111111111111111111112");

/// System program ID
pub const SYSTEM_PROGRAM_ID: Pubkey = Pubkey::from_str_const("11111111111111111111111111111111");

/// Registered program PDA (from registry)
pub const REGISTERED_PROGRAM_PDA: Pubkey =
    Pubkey::from_str_const("35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh");

/// Light Token mainnet LUT address
pub const LIGHT_LUT_MAINNET: Pubkey =
    Pubkey::from_str_const("9NYFyEqPeWQHiS8Jv4VjZcjKBMPRCJ3KbEbaBcy4Mza");

/// Light Token devnet LUT address (currently same as mainnet)
pub const LIGHT_LUT_DEVNET: Pubkey =
    Pubkey::from_str_const("9NYFyEqPeWQHiS8Jv4VjZcjKBMPRCJ3KbEbaBcy4Mza");
