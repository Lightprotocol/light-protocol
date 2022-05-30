use crate::ENCRYPTED_UTXOS_LENGTH;

const ROOT_CHECK: u8 = 15;
pub const TWO_LEAVES_PDA_SIZE: u64 = 106 + ENCRYPTED_UTXOS_LENGTH as u64;
//instruction order
pub const IX_ORDER: [u8; 76] = [
    ROOT_CHECK, 34, 14, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2,
    25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25,
    0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 16,
    //perform last checks and transfer requested amount
    241,
];
