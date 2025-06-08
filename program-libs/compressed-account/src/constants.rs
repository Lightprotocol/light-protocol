use light_macros::pubkey_array;

pub const ACCOUNT_COMPRESSION_PROGRAM_ID: [u8; 32] =
    pubkey_array!("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq");
pub const SYSTEM_PROGRAM_ID: [u8; 32] =
    pubkey_array!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");
pub const REGISTERED_PROGRAM_PDA: [u8; 32] =
    pubkey_array!("35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh");
pub const CREATE_CPI_CONTEXT_ACCOUNT: [u8; 8] = [233, 112, 71, 66, 121, 33, 178, 188];

pub const ADDRESS_MERKLE_TREE_ACCOUNT_DISCRIMINATOR: [u8; 8] = [11, 161, 175, 9, 212, 229, 73, 73];
pub const STATE_MERKLE_TREE_ACCOUNT_DISCRIMINATOR: [u8; 8] = [172, 43, 172, 186, 29, 73, 219, 84];
pub const QUEUE_ACCOUNT_DISCRIMINATOR: [u8; 8] = [164, 200, 108, 62, 87, 63, 123, 65];
pub const INSERT_INTO_QUEUES_INSTRUCTION_DISCRIMINATOR: [u8; 8] =
    [180, 143, 159, 153, 35, 46, 248, 163];
