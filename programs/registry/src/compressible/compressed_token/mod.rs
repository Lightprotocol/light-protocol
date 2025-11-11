pub mod accounts;
pub mod compress_and_close;

pub use accounts::Transfer2CpiAccounts;
use anchor_lang::pubkey;
pub use compress_and_close::{
    compress_and_close_ctoken_accounts_with_indices, CompressAndCloseIndices,
};
use solana_pubkey::Pubkey;

// Program ID for light-compressed-token
pub const COMPRESSED_TOKEN_PROGRAM_ID: Pubkey =
    pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

// Light System Program ID
pub const LIGHT_SYSTEM_PROGRAM_ID: Pubkey = pubkey!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");

// Account Compression Program ID
pub const ACCOUNT_COMPRESSION_PROGRAM_ID: Pubkey =
    pubkey!("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq");

// Account Compression Authority PDA
pub const ACCOUNT_COMPRESSION_AUTHORITY_PDA: Pubkey =
    pubkey!("HwXnGK3tPkkVY6P439H2p68AxpeuWXd5PcrAxFpbmfbA");

// Registered Program PDA
pub const REGISTERED_PROGRAM_PDA: Pubkey = pubkey!("35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh");
