pub fn decode_hash(hash: &str) -> [u8; 32] {
    let bytes = bs58::decode(hash).into_vec().unwrap();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}
