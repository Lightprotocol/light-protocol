use light_macros::pubkey_array;

/// ID of the account-compression program.
pub const ACCOUNT_COMPRESSION_PROGRAM_ID: [u8; 32] =
    pubkey_array!("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq");
/// ID of the light-system program.
pub const LIGHT_SYSTEM_PROGRAM_ID: [u8; 32] =
    pubkey_array!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");
/// ID of the light-registry program.
pub const LIGHT_REGISTRY_PROGRAM_ID: [u8; 32] =
    pubkey_array!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX");
#[deprecated(since = "0.9.0", note = "Use LIGHT_SYSTEM_PROGRAM_ID instead")]
pub const SYSTEM_PROGRAM_ID: [u8; 32] = LIGHT_SYSTEM_PROGRAM_ID;
pub const REGISTERED_PROGRAM_PDA: [u8; 32] =
    pubkey_array!("35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh");
pub const ACCOUNT_COMPRESSION_AUTHORITY_PDA: [u8; 32] =
    pubkey_array!("HwXnGK3tPkkVY6P439H2p68AxpeuWXd5PcrAxFpbmfbA");
/// Seed of the CPI authority.
pub const CPI_AUTHORITY_PDA_SEED: &[u8] = b"cpi_authority";

pub const CREATE_CPI_CONTEXT_ACCOUNT: [u8; 8] = [233, 112, 71, 66, 121, 33, 178, 188];

pub const ADDRESS_MERKLE_TREE_ACCOUNT_DISCRIMINATOR: [u8; 8] = [11, 161, 175, 9, 212, 229, 73, 73];
pub const STATE_MERKLE_TREE_ACCOUNT_DISCRIMINATOR: [u8; 8] = [172, 43, 172, 186, 29, 73, 219, 84];
pub const QUEUE_ACCOUNT_DISCRIMINATOR: [u8; 8] = [164, 200, 108, 62, 87, 63, 123, 65];
pub const INSERT_INTO_QUEUES_INSTRUCTION_DISCRIMINATOR: [u8; 8] =
    [180, 143, 159, 153, 35, 46, 248, 163];
