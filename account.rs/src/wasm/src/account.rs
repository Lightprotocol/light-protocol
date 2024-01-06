use aes_gcm_siv::{
    aead::{Aead, NewAead},
    Aes256GcmSiv, Key, Nonce,
};
use crypto_box::{aead::generic_array::GenericArray, Box, PublicKey, SecretKey};
use light_poseidon::PoseidonError;
use thiserror::Error;
use wasm_bindgen::prelude::wasm_bindgen;

extern crate console_error_panic_hook;
use js_sys::Error as JsError;
use num_bigint::BigUint;
use wasm_bindgen::JsValue;

use crate::{
    hash::{
        blake2::{blake2, blake2_string},
        poseidon::poseidon_hash,
    },
    utils::{vec_to_key, vec_to_string},
};

const UTXO_PREFIX_LENGTH: usize = 4;

const SECRET_KEY: [u8; 32] = [
    155, 249, 234, 55, 8, 49, 0, 14, 84, 72, 10, 224, 21, 139, 87, 102, 115, 88, 217, 72, 137, 38,
    0, 179, 93, 202, 220, 31, 143, 79, 247, 200,
];

#[derive(Error, Debug)]
pub enum AccountError {
    #[error("invalid seed")]
    InvalidSeed,

    #[error("invalid key size")]
    InvalidKeySize,

    #[error("poseidon error `{0}`")]
    Poseidon(PoseidonError),
    #[error("aes encryption error `{0}`")]
    AesGcmSiv(aes_gcm_siv::aead::Error),

    #[error("can't decrypt nacl: `{0}`")]
    NaclDecryptionError(crypto_box::aead::Error),

    #[error("can't encrypt nacl: `{0}`")]
    NaclEncryptionError(crypto_box::aead::Error),

    #[error("account error `{0}`")]
    Generic(String),
}

impl From<AccountError> for JsValue {
    fn from(error: AccountError) -> Self {
        JsError::new(&error.to_string()).into()
    }
}

const ACCOUNT_HASH_LENGTH: usize = 32;

#[wasm_bindgen]
pub struct Account {
    solana_public_key: Option<[u8; 32]>,
    private_key: [u8; 32],
    public_key: [u8; 32],

    encryption_private_key: [u8; 32],
    encryption_public_key: [u8; 32],

    aes_secret: [u8; 32],
    hashing_secret: Option<[u8; 32]>,
    burner_seed: Option<[u8; 32]>,

    prefix_counter: u32,
}

#[wasm_bindgen]
impl Account {
    #[wasm_bindgen]
    pub fn new(seed: &str) -> Result<Account, AccountError> {
        console_error_panic_hook::set_once();

        let private_key = vec_to_key(&Account::generate_shielded_private_key(seed)?)?;
        let public_key = vec_to_key(&Account::generate_shielded_public_key(
            private_key.to_vec(),
        )?)?;

        let encryption_private_key = vec_to_key(&Account::create_encryption_private_key(seed)?)?;
        let encryption_public_key = vec_to_key(&Account::create_encryption_public_key(
            encryption_private_key.to_vec(),
        )?)?;

        let aes_secret = vec_to_key(&Account::generate_secret(seed, "aes"))?;
        let hashing_secret = vec_to_key(&Account::generate_secret(seed, "hashing"))?;

        Ok(Account {
            solana_public_key: None,
            private_key,
            public_key,

            encryption_public_key,
            encryption_private_key,

            aes_secret,
            hashing_secret: Some(hashing_secret),

            burner_seed: None,

            prefix_counter: 0,
        })
    }

    #[wasm_bindgen]
    pub fn burner(seed: &str, index: &str) -> Result<Account, AccountError> {
        let input = format!("{}burnerSeed{}", seed, index);

        let burner_seed = blake2_string(input, ACCOUNT_HASH_LENGTH);
        let burner_arr = vec_to_key(&burner_seed)?;
        let burner_arr_string = vec_to_string(&burner_seed);
        let burner_seed_string = bs58::encode(burner_seed).into_string();

        let mut account = Self::new(&burner_seed_string)?;
        account.burner_seed = Some(burner_arr);
        account.aes_secret = vec_to_key(&Account::generate_secret(&burner_arr_string, "aes"))?;
        account.hashing_secret = Some(vec_to_key(&Account::generate_secret(
            &burner_arr_string,
            "hashing",
        ))?);
        Ok(account)
    }

    #[wasm_bindgen(js_name = createFromBurnerSeed)]
    pub fn from_burner_seed(burner_seed: &str) -> Result<Account, AccountError> {
        let burner_seed_vec = bs58::decode(burner_seed)
            .into_vec()
            .map_err(|_| AccountError::InvalidSeed)?;
        let burner_arr = vec_to_key(&burner_seed_vec)?;
        let burner_arr_string = vec_to_string(&burner_seed_vec);

        let mut account = Self::new(burner_seed)?;
        account.burner_seed = Some(burner_arr);
        account.aes_secret = vec_to_key(&Account::generate_secret(&burner_arr_string, "aes"))?;
        account.hashing_secret = Some(vec_to_key(&Account::generate_secret(
            &burner_arr_string,
            "hashing",
        ))?);
        Ok(account)
    }

    #[wasm_bindgen(js_name = fromPrivateKey)]
    pub fn from_private_key(
        private_key: Vec<u8>,
        encryption_private_key: Vec<u8>,
        aes_secret: Vec<u8>,
    ) -> Result<Account, AccountError> {
        let private_key_arr: [u8; 32] = vec_to_key(&private_key)?;
        let public_key = vec_to_key(&Account::generate_shielded_public_key(private_key)?)?;

        let encryption_public_key =
            Account::create_encryption_public_key(encryption_private_key.to_vec())?;

        Ok(Account {
            solana_public_key: None,

            private_key: private_key_arr,
            public_key,

            encryption_public_key: vec_to_key(&encryption_public_key)?,
            encryption_private_key: vec_to_key(&encryption_private_key)?,

            aes_secret: vec_to_key(&aes_secret)?,
            hashing_secret: None,

            burner_seed: None,

            prefix_counter: 0,
        })
    }

    #[wasm_bindgen(js_name = fromPublicKey)]
    pub fn from_public_key(
        public_key: Vec<u8>,
        encryption_public_key: Option<Vec<u8>>,
    ) -> Result<Account, AccountError> {
        let public_key = vec_to_key(&public_key)?;
        let encryption_public_key: [u8; 32] = match encryption_public_key {
            Some(encryption_public_key) => vec_to_key(&encryption_public_key)?,
            None => Default::default(),
        };

        Ok(Account {
            solana_public_key: None,
            private_key: Default::default(),
            public_key,
            encryption_public_key,
            encryption_private_key: Default::default(),
            aes_secret: Default::default(),
            hashing_secret: None,
            burner_seed: None,
            prefix_counter: 0,
        })
    }

    #[wasm_bindgen(js_name = fromAesSecret)]
    pub fn from_aes_secret(aes_secret: Vec<u8>) -> Result<Account, AccountError> {
        Ok(Account {
            solana_public_key: None,

            private_key: Default::default(),
            public_key: Default::default(),

            encryption_public_key: Default::default(),
            encryption_private_key: Default::default(),

            aes_secret: vec_to_key(&aes_secret)?,

            hashing_secret: None,
            burner_seed: None,

            prefix_counter: 0,
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
        let secret_key_bytes: [u8; ACCOUNT_HASH_LENGTH] = vec_to_key(&blake_hash)?;
        let secret_key = SecretKey::from(secret_key_bytes);
        Ok(secret_key.as_bytes().to_vec())
    }

    #[wasm_bindgen(js_name = createEncryptionPublicKey)]
    pub fn create_encryption_public_key(
        encryption_private_key: Vec<u8>,
    ) -> Result<Vec<u8>, AccountError> {
        let encryption_private_key_array: [u8; 32] = vec_to_key(&encryption_private_key)?;
        let secret_key = SecretKey::from(encryption_private_key_array);
        Ok(secret_key.public_key().as_bytes().to_vec())
    }

    #[wasm_bindgen(js_name = generateSecret)]
    pub fn generate_secret(seed: &str, domain: &str) -> Vec<u8> {
        let input = format!("{}{}", seed, domain);
        blake2_string(input, ACCOUNT_HASH_LENGTH)
    }

    #[wasm_bindgen(js_name = sign)]
    pub fn sign(&self, commitment: String, merkle_path: u32) -> Result<Vec<u8>, AccountError> {
        let commitment_bn = BigUint::parse_bytes(commitment.as_bytes(), 10)
            .ok_or_else(|| AccountError::Generic("Error parsing string to BigUint".to_string()))?;
        let commitment_bytes = commitment_bn.to_bytes_be();
        let merkle_tree_bytes = merkle_path.to_be_bytes();
        let inputs = vec![
            self.private_key.as_slice(),
            commitment_bytes.as_slice(),
            merkle_tree_bytes.as_slice(),
        ];
        poseidon_hash(inputs.as_slice()).map_err(AccountError::Poseidon)
    }

    #[wasm_bindgen(js_name = encryptAesUtxo)]
    pub fn encrypt_aes_utxo(
        &self,
        message: Vec<u8>,
        merkle_tree_pda_public_key: Vec<u8>,
        commitment: Vec<u8>,
    ) -> Result<Vec<u8>, AccountError> {
        let iv12 = commitment[0..12].to_vec();
        let aes_utxo_viewing_key =
            self.get_aes_utxo_viewing_key(merkle_tree_pda_public_key, commitment)?;
        let key = Key::from_slice(&aes_utxo_viewing_key);
        let cipher = Aes256GcmSiv::new(key);
        let nonce = Nonce::from_slice(&iv12);
        let ciphertext = cipher
            .encrypt(nonce, message.as_ref())
            .map_err(AccountError::AesGcmSiv);
        ciphertext
    }

    #[wasm_bindgen(js_name = decryptAesUtxo)]
    pub fn decrypt_aes_utxo(
        &self,
        encrypted_message: Vec<u8>,
        merkle_tree_pda_public_key: Vec<u8>,
        commitment: Vec<u8>,
    ) -> Result<Vec<u8>, AccountError> {
        let iv12 = commitment[0..12].to_vec();
        let aes_utxo_viewing_key =
            self.get_aes_utxo_viewing_key(merkle_tree_pda_public_key, commitment)?;
        let key = Key::from_slice(&aes_utxo_viewing_key);
        let cipher = Aes256GcmSiv::new(key);
        let nonce = Nonce::from_slice(&iv12);
        let ciphertext = cipher
            .decrypt(nonce, encrypted_message.as_ref())
            .map_err(AccountError::AesGcmSiv);

        ciphertext
    }

    #[wasm_bindgen(js_name = encryptAes)]
    pub fn encrypt_aes(&self, message: Vec<u8>, iv12: Vec<u8>) -> Result<Vec<u8>, AccountError> {
        let key = Key::from_slice(&self.aes_secret);
        let cipher = Aes256GcmSiv::new(key);
        let nonce = Nonce::from_slice(&iv12);
        let ciphertext = cipher
            .encrypt(nonce, message.as_ref())
            .map_err(AccountError::AesGcmSiv)?;
        let mut result = iv12;
        result.extend(ciphertext);
        Ok(result)
    }

    #[wasm_bindgen(js_name = decryptAes)]
    pub fn decrypt_aes(&self, encrypted: Vec<u8>) -> Result<Vec<u8>, AccountError> {
        let iv12 = encrypted[0..12].to_vec();
        let cipher_text = encrypted[12..].to_vec();
        self.decrypt_aes_internal(cipher_text, iv12)
    }

    fn decrypt_aes_internal(
        &self,
        cipher_text: Vec<u8>,
        iv12: Vec<u8>,
    ) -> Result<Vec<u8>, AccountError> {
        let key = Key::from_slice(&self.aes_secret);
        let cipher = Aes256GcmSiv::new(key);
        let nonce = Nonce::from_slice(&iv12);
        cipher
            .decrypt(nonce, cipher_text.as_ref())
            .map_err(AccountError::AesGcmSiv)
    }

    #[wasm_bindgen(js_name = decryptNaclUtxo)]
    pub fn decrypt_nacl_utxo(
        &self,
        ciphertext: Vec<u8>,
        commitment: Vec<u8>,
    ) -> Result<Vec<u8>, AccountError> {
        if commitment.len() != 32 {
            return Err(AccountError::Generic(
                "Commitment hash must be 32 bytes".to_string(),
            ));
        }

        let nonce = GenericArray::from_slice(&commitment[0..24]);
        let signer_pub_key = SecretKey::from(SECRET_KEY).public_key();
        let private_key = SecretKey::from(self.encryption_private_key);
        let nacl_box = Box::new(&signer_pub_key, &private_key);
        let decrypted_message = nacl_box
            .decrypt(nonce, &ciphertext[..])
            .map_err(AccountError::NaclDecryptionError)?;

        Ok(decrypted_message)
    }

    /// Encrypts UTXO bytes to a public key using a nonce and a standardized secret for HMAC.
    ///
    /// # Arguments
    /// * `public_key` - The public key to encrypt to.
    /// * `bytes_message` - The message to be encrypted.
    /// * `commitment` - The commitment used to generate the nonce.
    ///
    /// # Returns
    /// The encrypted `Uint8Array`.
    #[wasm_bindgen(js_name = encryptNaclUtxo)]
    pub fn encrypt_nacl_utxo(
        public_key: Vec<u8>,
        message: Vec<u8>,
        commitment: Vec<u8>,
    ) -> Result<Vec<u8>, AccountError> {
        if public_key.len() != 32 {
            return Err(AccountError::InvalidKeySize);
        }
        if commitment.len() != 32 {
            return Err(AccountError::Generic(
                "Commitment hash must be 32 bytes".to_string(),
            ));
        }
        if message.is_empty() {
            return Err(AccountError::Generic("Message can't be empty".to_string()));
        }

        let pub_key_array: [u8; 32] = vec_to_key(&public_key)?;
        let pub_key = PublicKey::from(pub_key_array);
        let nonce = GenericArray::from_slice(&commitment[0..24]);

        // CONSTANT_SECRET_AUTHKEY is used to minimize the number of bytes sent to the blockchain.
        // This results in poly135 being useless since the CONSTANT_SECRET_AUTHKEY is public.
        // However, ciphertext integrity is guaranteed since a hash of the ciphertext is included in a zero-knowledge proof.
        let private_key = SecretKey::from(SECRET_KEY);

        let nacl_box = Box::new(&pub_key, &private_key);

        let encrypted_message = nacl_box
            .encrypt(nonce, &message[..])
            .map_err(AccountError::NaclEncryptionError)?;

        Ok(encrypted_message)
    }

    #[wasm_bindgen(js_name = getUtxoPrefixViewingKey)]
    pub fn get_utxo_prefix_viewing_key(&self, salt: &str) -> Result<Vec<u8>, AccountError> {
        let hashing_secret_string = self.get_hashing_secret_string()?;
        let secret = Account::generate_secret(&hashing_secret_string, salt);
        Ok(secret)
    }

    #[wasm_bindgen(js_name = generateUtxoPrefixHash)]
    pub fn generate_utxo_prefix_hash(
        &self,
        merkle_tree_public_key: Vec<u8>,
        prefix_counter: u32,
    ) -> Result<Vec<u8>, AccountError> {
        let merkle_tree_pubkey_array = vec_to_key(&merkle_tree_public_key)?;
        let hash =
            self.generate_utxo_prefix_hash_internal(&merkle_tree_pubkey_array, prefix_counter)?;
        Ok(hash.to_vec())
    }

    fn generate_utxo_prefix_hash_internal(
        &self,
        merkle_tree_public_key: &[u8; 32],
        prefix_counter: u32,
    ) -> Result<[u8; UTXO_PREFIX_LENGTH], AccountError> {
        let mut input = self.get_utxo_prefix_viewing_key("hashing")?;
        input.extend(merkle_tree_public_key.to_vec());

        let prefix_counter_vec = prefix_counter.to_be_bytes().to_vec();
        let padded_prefix_counter: Vec<u8> = if prefix_counter_vec.len() < 32 {
            let mut padded = vec![0; 32 - prefix_counter_vec.len()];
            padded.extend_from_slice(&prefix_counter_vec);
            padded
        } else {
            prefix_counter_vec
        };

        input.extend(padded_prefix_counter);
        let hash_vec = blake2(&input, UTXO_PREFIX_LENGTH);
        let hash: [u8; UTXO_PREFIX_LENGTH] = hash_vec
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
        let hash = self.generate_utxo_prefix_hash(merkle_tree_public_key, self.prefix_counter)?;
        self.prefix_counter += 1;
        Ok(hash)
    }

    #[wasm_bindgen(js_name = getAesUtxoViewingKey)]
    pub fn get_aes_utxo_viewing_key(
        &self,
        merkle_tree_pda_public_key: Vec<u8>,
        commitment: Vec<u8>,
    ) -> Result<Vec<u8>, AccountError> {
        let merkle_tree_pubkey_bs58 = bs58::encode(merkle_tree_pda_public_key).into_string();
        let salt = bs58::encode(commitment).into_string();
        let aes_salt = format!("{}{}", merkle_tree_pubkey_bs58, salt);
        let aes_secret_string = vec_to_string(&self.aes_secret);
        let secret = Account::generate_secret(&aes_secret_string, &aes_salt);
        Ok(secret)
    }

    #[wasm_bindgen(js_name = getPrivateKey)]
    pub fn get_private_key(&self) -> Vec<u8> {
        self.private_key.to_vec()
    }

    #[wasm_bindgen(js_name = getPublicKey)]
    pub fn get_public_key(&self) -> Vec<u8> {
        self.public_key.to_vec()
    }

    fn get_hashing_secret_string(&self) -> Result<String, AccountError> {
        let hashing_secret = self.hashing_secret.as_ref().ok_or(AccountError::Generic(
            "hashing_secret is undefined".to_string(),
        ))?;
        let hashing_secret_string: String = vec_to_string(hashing_secret);
        Ok(hashing_secret_string)
    }

    #[wasm_bindgen(js_name = getCombinedPublicKey)]
    pub fn get_combined_public_key(&self) -> Result<String, AccountError> {
        let combined_keys_iter = self.public_key.iter().chain(&self.encryption_public_key);

        let combined_public_keys: [u8; 64] = combined_keys_iter
            .cloned()
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| {
                AccountError::Generic(
                    "get_combined_public_key: error converting to a 64-byte array".to_string(),
                )
            })?;

        Ok(bs58::encode(combined_public_keys).into_string())
    }

    #[wasm_bindgen(js_name = getEncryptionPrivateKey)]

    pub fn get_encryption_private_key(&self) -> Vec<u8> {
        self.encryption_private_key.to_vec()
    }

    #[wasm_bindgen(js_name = getEncryptionPublicKey)]
    pub fn get_encryption_public_key(&self) -> Vec<u8> {
        self.encryption_public_key.to_vec()
    }

    #[wasm_bindgen(js_name = getAesSecret)]
    pub fn get_aes_secret(&self) -> Vec<u8> {
        self.aes_secret.to_vec()
    }

    #[wasm_bindgen(js_name = getSolanaPublicKey)]
    pub fn get_solana_public_key(&self) -> Option<Vec<u8>> {
        Some(self.solana_public_key?.to_vec())
    }

    #[wasm_bindgen(js_name = setSolanaPublicKey)]
    pub fn set_solana_public_key(&mut self, solana_public_key: Option<Vec<u8>>) {
        match solana_public_key {
            Some(solana_public_key) => {
                self.solana_public_key = vec_to_key(&solana_public_key).ok();
            }
            None => {
                self.solana_public_key = None;
            }
        }
    }

    #[wasm_bindgen(js_name = getPrefixCounter)]
    pub fn get_prefix_counter(&self) -> u32 {
        self.prefix_counter
    }

    #[wasm_bindgen(js_name = setPrefixCounter)]
    pub fn set_prefix_counter(&mut self, prefix_counter: u32) {
        self.prefix_counter = prefix_counter;
    }

    #[wasm_bindgen(js_name = getBurnerSeed)]
    pub fn get_burner_seed(&self) -> Option<Vec<u8>> {
        self.burner_seed.map(|seed| seed.to_vec())
    }

    #[wasm_bindgen(js_name = setBurnerSeed)]
    pub fn set_burner_seed(&mut self, burner_seed: Vec<u8>) {
        self.burner_seed = vec_to_key(&burner_seed).ok();
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
