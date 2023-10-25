use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use js_sys::{Array, Uint8Array};
use light_poseidon::{Poseidon, PoseidonHasher};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

// #[wasm_bindgen]
// pub fn hash(_priv_key: &str, _commitment: &str, _merkle_path: u32) -> Uint8Array {
//   let mut poseidon = Poseidon::<Fr>::new_circom(3).unwrap();
//   let hash = poseidon.hash_bytes_be(&[&[1u8; 32], &[2u8; 32], &[3u8; 32]]).unwrap().to_vec();
//   let js_arr = Uint8Array::from(&hash[..]);
//   return js_arr;
// }

#[wasm_bindgen]
pub fn hash(input: Array) -> Result<Uint8Array, JsValue> {
    let input: Vec<String> = input.iter().map(|x| x.as_string().unwrap()).collect();

    let mut poseidon = Poseidon::<Fr>::new_circom(input.len()).map_err(|e| e.to_string())?;

    let bytes: Result<Vec<Vec<u8>>, _> = input
        .iter()
        .map(|x| hex::decode(x).map_err(|e| e.to_string()))
        .collect();
    let bytes = bytes.map_err(|e| JsValue::from_str(&e))?;

    let inputs: Vec<Fr> = bytes
        .iter()
        .map(|x| Fr::from_be_bytes_mod_order(x))
        .collect();

    let hash = poseidon.hash(&inputs).unwrap();
    let hash_bytes = hash.into_bigint().to_bytes_be();
    let js_arr = Uint8Array::from(&hash_bytes[..]);
    Ok(js_arr)
}
