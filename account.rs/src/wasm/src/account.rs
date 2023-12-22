use std::ops::Add;

use aes_gcm_siv::{
    aead::{Aead, NewAead},
    Aes256GcmSiv, Key, Nonce,
};
use crypto_box::SecretKey;
use light_poseidon::PoseidonError;
use num_bigint::BigUint;
use wasm_bindgen::prelude::wasm_bindgen;

extern crate console_error_panic_hook;
use js_sys::Error as JsError;
use wasm_bindgen::JsValue;

use crate::hash::{
    blake2::{blake2, blake2_string},
    poseidon::poseidon_hash,
};

pub const UTXO_PREFIX_LENGTH: usize = 4;

#[derive(Debug)]
pub enum AccountError {
    Poseidon(PoseidonError),
    AesGcmSiv(aes_gcm_siv::aead::Error),
    Generic(String),
}

impl From<PoseidonError> for AccountError {
    fn from(error: PoseidonError) -> Self {
        AccountError::Poseidon(error)
    }
}

impl From<aes_gcm_siv::aead::Error> for AccountError {
    fn from(error: aes_gcm_siv::aead::Error) -> Self {
        AccountError::AesGcmSiv(error)
    }
}

impl From<AccountError> for JsValue {
    fn from(error: AccountError) -> Self {
        let error_message = match error {
            AccountError::Poseidon(e) => format!("{}", e),
            AccountError::AesGcmSiv(e) => format!("{}", e),
            AccountError::Generic(e) => e,
        };
        JsError::new(&error_message).into()
    }
}
const ACCOUNT_HASH_LENGTH: usize = 32;

#[wasm_bindgen]
pub struct Account {
    // solana_public_key: String,
    private_key: [u8; 32],
    public_key: [u8; 32],

    encryption_private_key: [u8; 32],
    encryption_public_key: [u8; 32],

    aes_secret: [u8; 32],
    hashing_secret: Option<[u8; 32]>,
    burner_seed: Option<String>,

    prefix_counter: BigUint,
}

#[wasm_bindgen]
impl Account {
    fn vec_to_key(vec: Vec<u8>) -> Result<[u8; 32], AccountError> {
        vec.try_into()
            .map_err(|_| AccountError::Generic(String::from("Expected a Vec of length 32")))
    }

    fn key_to_vec(key: [u8; 32]) -> Vec<u8> {
        key.to_vec()
    }

    #[wasm_bindgen]
    pub fn new(seed: &str) -> Result<Account, AccountError> {
        console_error_panic_hook::set_once();

        let private_key = Account::vec_to_key(Account::generate_shielded_private_key(seed)?)?;
        let public_key =
            Account::vec_to_key(Account::generate_shielded_public_key(private_key.to_vec())?)?;

        let encryption_private_key =
            Account::vec_to_key(Account::create_encryption_private_key(seed)?)?;
        let encryption_public_key = Account::vec_to_key(Account::create_encryption_public_key(
            encryption_private_key.to_vec(),
        )?)?;

        let aes_secret = Account::vec_to_key(Account::generate_secret(seed, "aes"))?;
        let hashing_secret = Account::vec_to_key(Account::generate_secret(seed, "hashing"))?;

        Ok(Account {
            // solana_public_key,
            private_key,
            public_key,

            encryption_public_key,
            encryption_private_key,

            aes_secret,
            hashing_secret: Some(hashing_secret),

            burner_seed: None,

            prefix_counter: BigUint::from(0u32),
        })
    }

    #[wasm_bindgen]
    pub fn burner(seed: &str, index: &str) -> Result<Account, AccountError> {
        let input = format!("{}burnerSeed{}", seed, index);
        let burner_seed = blake2_string(input, ACCOUNT_HASH_LENGTH);
        let burner_seed_string = bs58::encode(burner_seed).into_string();
        let mut account = Self::new(&burner_seed_string)?;
        account.burner_seed = Some(burner_seed_string);
        Ok(account)
    }

    #[wasm_bindgen(js_name = fromPrivateKey)]
    pub fn from_private_key(
        private_key: Vec<u8>,
        encryption_private_key: Vec<u8>,
        aes_secret: Vec<u8>,
    ) -> Result<Account, AccountError> {
        let private_key_arr: [u8; 32] = private_key.clone().try_into().map_err(|_| {
            AccountError::Generic(String::from(
                "Can't generate shielded private key: expected a vec of length 32",
            ))
        })?;

        let public_key =
            Account::vec_to_key(Account::generate_shielded_public_key(private_key.to_vec())?)?;
        let encryption_public_key =
            Account::create_encryption_public_key(encryption_private_key.to_vec())?;

        Ok(Account {
            // solana_public_key,
            private_key: private_key_arr,
            public_key,

            encryption_public_key: Account::vec_to_key(encryption_public_key)?,
            encryption_private_key: Account::vec_to_key(encryption_private_key)?,

            aes_secret: Account::vec_to_key(aes_secret)?,
            hashing_secret: None,

            burner_seed: None,

            prefix_counter: BigUint::from(0u32),
        })
    }

    #[wasm_bindgen(js_name = generateShieldedPrivateKey)]
    pub fn generate_shielded_private_key(seed: &str) -> Result<Vec<u8>, AccountError> {
        let private_key_seed = format!("{}shielded", seed);
        let hash = BigUint::from_bytes_be(
            &blake2_string(private_key_seed, ACCOUNT_HASH_LENGTH)[1..ACCOUNT_HASH_LENGTH],
        )
        .to_bytes_be();

        poseidon_hash(vec![hash.as_slice()].as_slice()).map_err(AccountError::Poseidon)
    }

    #[wasm_bindgen(js_name = generateShieldedPublicKey)]
    pub fn generate_shielded_public_key(private_key: Vec<u8>) -> Result<Vec<u8>, AccountError> {
        poseidon_hash(vec![private_key.as_slice()].as_slice()).map_err(AccountError::Poseidon)
    }

    #[wasm_bindgen(js_name = createEncryptionPrivateKey)]
    pub fn create_encryption_private_key(seed: &str) -> Result<Vec<u8>, AccountError> {
        let encryption_seed = format!("{}encryption", seed);
        let blake_hash = blake2_string(encryption_seed, ACCOUNT_HASH_LENGTH);
        let secret_key_bytes: [u8; ACCOUNT_HASH_LENGTH] = blake_hash
            .as_slice()
            .try_into()
            .map_err(|_| AccountError::Generic(String::from("Expected a Vec of length 32")))?;
        let secret_key = SecretKey::from(secret_key_bytes);
        Ok(secret_key.as_bytes().to_vec())
    }

    #[wasm_bindgen(js_name = createEncryptionPublicKey)]
    pub fn create_encryption_public_key(
        encryption_private_key: Vec<u8>,
    ) -> Result<Vec<u8>, AccountError> {
        let encryption_private_key_array: [u8; 32] = encryption_private_key
            .try_into()
            .map_err(|_| AccountError::Generic(String::from("Expected a Vec of length 32")))?;

        let secret_key = SecretKey::from(encryption_private_key_array);
        Ok(secret_key.public_key().as_bytes().to_vec())
    }

    #[wasm_bindgen(js_name = generateSecret)]
    pub fn generate_secret(seed: &str, domain: &str) -> Vec<u8> {
        let input = format!("{}{}", seed, domain);
        blake2_string(input, ACCOUNT_HASH_LENGTH)
    }

    pub fn sign(&self, commitment: String, merkle_path: u32) -> Result<Vec<u8>, AccountError> {
        poseidon_hash(&[
            self.private_key.as_slice(),
            commitment.into_bytes().as_slice(),
            merkle_path.to_be_bytes().as_slice(),
        ])
        .map_err(AccountError::Poseidon)
    }

    #[wasm_bindgen(js_name = encryptAes)]
    pub fn encrypt_aes(&self, message: Vec<u8>, iv16: Vec<u8>) -> Vec<u8> {
        let key = Key::from_slice(&self.aes_secret);
        let cipher = Aes256GcmSiv::new(key);
        let nonce = Nonce::from_slice(&iv16);
        let ciphertext = cipher
            .encrypt(nonce, message.as_ref())
            .expect("encryption failure!");

        let mut result = iv16;
        result.extend(ciphertext);
        result
    }

    #[wasm_bindgen(js_name = decryptAes)]
    pub fn decrypt_aes(&self, encrypted: Vec<u8>) -> Result<Vec<u8>, AccountError> {
        let iv16 = encrypted[0..16].to_vec();
        let cipher_text = encrypted[16..].to_vec();
        self.decrypt_aes_internal(cipher_text, iv16)
    }

    fn decrypt_aes_internal(
        &self,
        cipher_text: Vec<u8>,
        iv16: Vec<u8>,
    ) -> Result<Vec<u8>, AccountError> {
        let key = Key::from_slice(&self.aes_secret);
        let cipher = Aes256GcmSiv::new(key);
        let nonce = Nonce::from_slice(&iv16);
        cipher
            .decrypt(nonce, cipher_text.as_ref())
            .map_err(AccountError::AesGcmSiv)
    }

    #[wasm_bindgen(js_name = getUtxoPrefixViewingKey)]
    pub fn get_utxo_prefix_viewing_key(&self, salt: &str) -> Result<Vec<u8>, AccountError> {
        if let Some(hashing_secret) = &self.hashing_secret {
            let input_string = String::from_utf8(hashing_secret.to_vec()).map_err(|_| {
                AccountError::Generic(String::from("Cannot convert hashing_secret to String"))
            })?;
            let secret = Account::generate_secret(&input_string, salt);
            Ok(secret)
        } else {
            Err(AccountError::Generic("hashing_secret is empty".to_string()))
        }
    }

    fn generate_utxo_prefix_hash(
        &self,
        merkle_tree_public_key: &[u8; 32],
        prefix_counter: BigUint,
    ) -> Result<[u8; UTXO_PREFIX_LENGTH], AccountError> {
        let mut input = self.get_utxo_prefix_viewing_key("hashing")?;
        input.extend(merkle_tree_public_key.to_vec());
        input.extend(prefix_counter.to_bytes_be());
        let hash: [u8; UTXO_PREFIX_LENGTH] = blake2(&input, UTXO_PREFIX_LENGTH)
            .as_slice()
            .try_into()
            .map_err(|_| AccountError::Generic(String::from("Expected a Vec of length 32")))?;
        Ok(hash)
    }

    #[wasm_bindgen(js_name = generateLatestUtxoPrefixHash)]
    pub fn generate_latest_utxo_prefix_hash(
        &mut self,
        merkle_tree_public_key: Vec<u8>,
    ) -> Result<Vec<u8>, AccountError> {
        let merkle_tree_pubkey_array: Result<[u8; 32], AccountError> =
            merkle_tree_public_key.as_slice().try_into().map_err(|_| {
                AccountError::Generic(String::from(
                    "Expected a merkle_tree_public_key of length 32",
                ))
            });

        let hash = self
            .generate_utxo_prefix_hash(&merkle_tree_pubkey_array?, self.prefix_counter.clone())?
            .to_vec();
        self.prefix_counter = self.prefix_counter.clone().add(BigUint::from(1u32));
        Ok(hash)
    }

    #[wasm_bindgen(js_name = getAesUtxoViewingKey)]
    pub fn get_aes_utxo_viewing_key(
        &self,
        merkle_tree_pda_public_key: Vec<u8>,
        salt: &str,
    ) -> Result<Vec<u8>, AccountError> {
        let encoded_merkle_tree_pda_public_key =
            bs58::encode(merkle_tree_pda_public_key).into_string();
        let aes_salt = format!("{}{}", encoded_merkle_tree_pda_public_key, salt);

        let aes_secret_string = String::from_utf8(self.aes_secret.to_vec())
            .map_err(|_| AccountError::Generic(String::from("aes_secret is invalid")))?;
        Ok(Account::generate_secret(&aes_secret_string, &aes_salt))
    }

    #[wasm_bindgen(js_name = getPrivateKey)]
    pub fn get_private_key(&self) -> Vec<u8> {
        self.private_key.to_vec()
    }

    #[wasm_bindgen(js_name = getPublicKey)]

    pub fn get_public_key(&self) -> Vec<u8> {
        Account::key_to_vec(self.public_key)
    }

    #[wasm_bindgen(js_name = getEncryptionPrivateKey)]

    pub fn get_encryption_private_key(&self) -> Vec<u8> {
        Account::key_to_vec(self.encryption_private_key)
    }

    #[wasm_bindgen(js_name = getEncryptionPublicKey)]
    pub fn get_encryption_public_key(&self) -> Vec<u8> {
        Account::key_to_vec(self.encryption_public_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::poseidon::poseidon_hash;

    #[test]
    fn poseidon_test_uneq() {
        let x1 = vec![1u8; 30];
        let y1 = vec![2u8; 30];
        let hash1 = poseidon_hash(vec![x1.as_slice(), y1.as_slice()].as_slice());

        let x2 = vec![1u8; 29];
        let y2 = vec![2u8; 31];
        let hash2 = poseidon_hash(vec![x2.as_slice(), y2.as_slice()].as_slice());

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn poseidon_test_eq() {
        let x1 = vec![1u8; 30];
        let y1 = vec![2u8; 30];
        let hash1 = poseidon_hash(vec![x1.as_slice(), y1.as_slice()].as_slice());

        let x2 = vec![1u8; 30];
        let y2 = vec![2u8; 30];
        let hash2 = poseidon_hash(vec![x2.as_slice(), y2.as_slice()].as_slice());

        assert_eq!(hash1, hash2);
    }

    fn assert_account_equality(a0: &Account, a1: &Account) {
        assert_eq!(a0.private_key, a1.private_key);
        assert_eq!(a0.public_key, a1.public_key);
        assert_eq!(a0.aes_secret, a1.aes_secret);
        assert_eq!(a0.encryption_private_key, a1.encryption_private_key);
        assert_eq!(a0.encryption_public_key, a1.encryption_public_key);
    }

    fn assert_encryption_public_key(account: &Account) {
        let ref_encryption_public_key = [
            28, 117, 135, 209, 179, 252, 123, 65, 26, 50, 142, 83, 93, 91, 228, 148, 120, 141, 141,
            161, 182, 86, 131, 115, 63, 11, 196, 172, 184, 158, 34, 5,
        ];
        assert_eq!(account.encryption_public_key, ref_encryption_public_key);
    }

    fn assert_private_key(account: &Account) {
        let ref_private_key = [
            12, 178, 194, 170, 213, 212, 132, 232, 179, 103, 6, 153, 111, 52, 44, 234, 78, 104,
            176, 170, 110, 206, 90, 113, 224, 128, 9, 115, 125, 158, 32, 112,
        ];
        assert_eq!(account.private_key, ref_private_key);
    }

    fn assert_public_key(account: &Account) {
        let ref_pubkey = [
            15, 74, 51, 27, 176, 196, 75, 247, 216, 187, 125, 105, 158, 48, 160, 112, 27, 208, 157,
            98, 114, 144, 100, 250, 125, 48, 160, 133, 16, 199, 14, 243,
        ];
        assert_eq!(account.public_key, ref_pubkey);
    }

    #[test]
    fn seed_init_test() {
        let seed32 = bs58::encode(vec![2u8; 32]).into_string();
        println!("seed32: {:?}", seed32);
        let a0 = Account::new(&seed32).unwrap();
        let a1 = Account::new(&seed32).unwrap();
        assert_account_equality(&a0, &a1);
        assert_encryption_public_key(&a0);
        assert_private_key(&a0);
        assert_public_key(&a0);
    }
}
