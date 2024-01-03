use crate::account::AccountError;

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

pub fn vec_to_key(vec: &[u8]) -> Result<[u8; 32], AccountError> {
    vec.try_into()
        .map_err(|_| AccountError::Generic(String::from("Expected a Vec of length 32")))
}

pub fn key_to_vec(key: [u8; 32]) -> Vec<u8> {
    key.to_vec()
}

pub fn vec_to_string(vec: &[u8]) -> String {
    vec.iter()
        .map(|&value| value.to_string())
        .collect::<Vec<String>>()
        .join(",")
}
