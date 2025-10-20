use light_macros::pubkey_array;

/// ID of the account-compression program.
pub const ACCOUNT_COMPRESSION_PROGRAM_ID: [u8; 32] =
    pubkey_array!("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq");
/// ID of the light-system program.
pub const LIGHT_SYSTEM_PROGRAM_ID: [u8; 32] =
    pubkey_array!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");
pub const REGISTERED_PROGRAM_PDA: [u8; 32] =
    pubkey_array!("35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh");
pub const ACCOUNT_COMPRESSION_AUTHORITY_PDA: [u8; 32] =
    pubkey_array!("HwXnGK3tPkkVY6P439H2p68AxpeuWXd5PcrAxFpbmfbA");

/// ID of the light-compressed-token program.
pub const C_TOKEN_PROGRAM_ID: [u8; 32] =
    pubkey_array!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

/// Seed of the CPI authority.
pub const CPI_AUTHORITY_PDA_SEED: &[u8] = b"cpi_authority";
pub const NOOP_PROGRAM_ID: [u8; 32] = pubkey_array!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

pub const STATE_MERKLE_TREE_HEIGHT: usize = 26;
pub const STATE_MERKLE_TREE_CHANGELOG: usize = 1400;
pub const STATE_MERKLE_TREE_ROOTS: usize = 2400;
pub const STATE_MERKLE_TREE_CANOPY_DEPTH: usize = 10;

pub const ADDRESS_MERKLE_TREE_HEIGHT: usize = 26;
pub const ADDRESS_MERKLE_TREE_CHANGELOG: usize = 1400;
pub const ADDRESS_MERKLE_TREE_ROOTS: usize = 2400;
pub const ADDRESS_MERKLE_TREE_CANOPY_DEPTH: usize = 10;
pub const ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG: usize = 1400;

pub const TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR: [u8; 8] = [2, 0, 0, 0, 0, 0, 0, 0];

pub const ADDRESS_TREE_V1: [u8; 32] = pubkey_array!("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2");
pub const ADDRESS_QUEUE_V1: [u8; 32] = pubkey_array!("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F");
pub const CPI_CONTEXT_ACCOUNT_1_DISCRIMINATOR: [u8; 8] = [22, 20, 149, 218, 74, 204, 128, 166];
pub const CPI_CONTEXT_ACCOUNT_2_DISCRIMINATOR: [u8; 8] = [34, 184, 183, 14, 100, 80, 183, 124];

pub const SOL_POOL_PDA: [u8; 32] = pubkey_array!("CHK57ywWSDncAoRu1F8QgwYJeXuAJyyBYT4LixLXvMZ1");

pub const ADDRESS_TREE_V2: [u8; 32] = pubkey_array!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx");
