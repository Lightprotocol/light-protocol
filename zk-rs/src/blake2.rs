use blake2b_simd::Params;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = blake2str)]
pub fn blake2_string(input: String, hash_length: usize) -> Vec<u8> {
    Params::new()
        .hash_length(hash_length)
        .hash(input.as_bytes())
        .as_bytes()
        .to_vec()
}

#[wasm_bindgen(js_name = blake2)]
pub fn blake2(input: &[u8], hash_length: usize) -> Vec<u8> {
    Params::new()
        .hash_length(hash_length)
        .hash(input)
        .as_bytes()
        .to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blake_4() {
        let input: [u8; 64] = [
            8, 11, 255, 174, 253, 221, 253, 111, 32, 197, 22, 38, 135, 201, 120, 114, 203, 112, 85,
            63, 101, 26, 5, 118, 231, 206, 220, 12, 10, 137, 200, 136, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        ];
        let hash = blake2(&input, 4);

        let expected_output: [u8; 4] = [55, 154, 4, 63];
        assert_eq!(hash.as_slice(), &expected_output);
    }
}
