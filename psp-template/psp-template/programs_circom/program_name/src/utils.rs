const CHUNK_SIZE: usize = 32;
pub fn change_endianness<const SIZE: usize>(bytes: &[u8; SIZE]) -> [u8; SIZE] {
    let mut arr = [0u8; SIZE];
    for (i, b) in bytes.chunks(CHUNK_SIZE).enumerate() {
        for (j, byte) in b.iter().rev().enumerate() {
            arr[i * CHUNK_SIZE + j] = *byte;
        }
    }
    arr
}
