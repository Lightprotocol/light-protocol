use ark_bn254::Fr;
use crypto_box::SecretKey;
use num_bigint::BigUint;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{
    blake2::{blake2_hash, blake2_hash_str},
    poseidon::poseidon_hash,
};

#[wasm_bindgen]
pub struct Account {
    // solana_public_key: Vec<u8>,
    // seed: Vec<u8>,
    public_key: Vec<u8>,
    private_key: Vec<u8>,
    encryption_public_key: Vec<u8>,
    encryption_private_key: Vec<u8>,
    aes_secret: String,
    hashing_secret: String,
}

#[wasm_bindgen]
impl Account {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: String) -> Self {
        // if seed.len() < 32 {
        //     return Err(AccountError::new(
        //         AccountErrorCode::InvalidSeedSize,
        //         "constructor",
        //         "seed too short length less than 32",
        //     ));
        // }
        let encryption_private_key = Account::get_encryption_private_key(seed.clone());
        let encryption_public_key =
            Account::get_encryption_public_key(encryption_private_key.to_vec());
        let private_key = Account::generate_shielded_private_key(seed.clone());
        let public_key = Account::generate_shielded_public_key(private_key.to_vec());
        let aes_secret = Account::generate_secret(seed.clone(), "aes".to_string());
        let hashing_secret = Account::generate_secret(seed.clone(), "hashing".to_string());

        Account {
            private_key,
            public_key,
            encryption_public_key,
            encryption_private_key,
            aes_secret,
            hashing_secret,
        }
    }

    fn generate_shielded_public_key(private_key: Vec<u8>) -> Vec<u8> {
        /*
        return new BN(poseidon.F.toString(poseidon([privateKey])));
        */
        let hash = poseidon_hash(vec![private_key]);
        hash.to_vec()
    }

    fn generate_shielded_private_key(seed: String) -> Vec<u8> {
        let private_key_seed = format!("{}shielded", seed);
        let blake_hash = blake2_hash(private_key_seed);
        let hash = poseidon_hash(vec![blake_hash]);
        let bn = BigUint::from_bytes_be(&hash).to_string();
        hash
    }

    fn get_encryption_private_key(seed: String) -> Vec<u8> {
        /*
        const encSeed = seed + "encryption";
        const encryptionPrivateKey = blake2b
            .create(b2params)
            .update(encSeed)
            .digest();
        return nacl.box.keyPair.fromSecretKey(encryptionPrivateKey);
        */

        let encryption_seed = format!("{}encryption", seed);
        let blake_hash = blake2_hash(encryption_seed);
        let secret_key_bytes: [u8; 32] = blake_hash.as_slice().try_into().unwrap();
        let secret_key = SecretKey::from(secret_key_bytes);
        secret_key.as_bytes().to_vec()
    }

    fn get_encryption_public_key(encryption_private_key: Vec<u8>) -> Vec<u8> {
        let secret_key_bytes: [u8; 32] = encryption_private_key.try_into().unwrap();
        let secret_key = SecretKey::from(secret_key_bytes);
        let public_key_bytes = secret_key.public_key().as_bytes().clone();
        public_key_bytes.to_vec()
    }

    pub fn get_public_key(&self) -> String {
        /*const concatPublicKey = new Uint8Array([
          ...this.pubkey.toArray("be", 32),
          ...this.encryptionKeypair.publicKey,
        ]);
        return bs58.encode(concatPublicKey);
        */

        let concat_public_key = [
            self.public_key.as_slice(),
            self.encryption_public_key.as_slice(),
        ]
        .concat();
        bs58::encode(concat_public_key).into_string()
    }

    fn get_utxo_prefix_viewing_key(&self, salt: String) -> String {
        Account::generate_secret(self.hashing_secret.clone(), salt)
    }

    fn generate_utxo_prefix_hash(&self, commitment_hash: String) -> Vec<u8> {
        /*
              const input = Uint8Array.from([
            ...this.getUtxoPrefixViewingKey("hashing"),
            ...commitmentHash,
          ]);

          return blake2b.create({ dkLen }).update(input).digest();
        */
        let salt = "hashing".to_string();
        let viewing_key = self.get_utxo_prefix_viewing_key(salt);
        let input = format!("{}{}", viewing_key, commitment_hash);
        let blake_hash = blake2_hash(input);
        blake_hash
    }

    fn get_aes_utxo_viewing_key(
        &self,
        merkle_tree_pda_public_key: Vec<u8>,
        salt: String,
    ) -> String {
        /*
        return Account.generateSecret(
          b2params.dkLen,
          this.aesSecret?.toString(),
          merkleTreePdaPublicKey.toBase58() + salt,
        );
        */
        let encoded_merkle_tree_pda_public_key =
            bs58::encode(merkle_tree_pda_public_key).into_string();
        let aes_salt = format!("{}{}", encoded_merkle_tree_pda_public_key, salt);
        Account::generate_secret(self.aes_secret.clone(), aes_salt)
    }

    fn generate_secret(seed: String, domain: String) -> String {
        let input = format!("{}{}", seed, domain);
        blake2_hash_str(input)
    }

    fn sign(&self, commitment: String, merkle_path: u32) -> Vec<u8> {
        let inputs: Vec<Vec<u8>> = vec![
            self.private_key.to_vec(),
            commitment.into_bytes(),
            merkle_path.to_be_bytes().to_vec(),
        ];
        poseidon_hash(inputs)
    }

    fn encrypt_aes(&self, message: Vec<u8>, iv16: Vec<u8>) -> Vec<u8> {
        /*

        /// encrypt(msg: Uint8Array, key: Uint8Array, iv: Uint8Array, mode?: string, pkcs7PaddingEnabled?: boolean): Promise<Uint8Array>;

         const ciphertext = await encrypt(
          messageBytes,
          this.aesSecret,
          iv16,
          "aes-256-cbc",
          true,
        );
        return new Uint8Array([...iv16, ...ciphertext]);
        */

        // let cipher = Aes256GcmSiv::new(GenericArray::from_slice(&self.aes_secret.as_bytes()));
        // let nonce = GenericArray::clone_from_slice(&iv16);
        // let ciphertext = cipher.encrypt(nonce, message.as_ref()).expect("Encryption failure!");
        // let mut result = iv16;
        // result.extend(ciphertext);
        // result

        unimplemented!("enc_aes")
    }

    pub fn decrypt_aes(&self, encrypted: Vec<u8>) -> Vec<u8> {
        let iv16 = encrypted[0..16].to_vec();
        let cipher_text = encrypted[16..].to_vec();
        return self.decrypt_aes_internal(cipher_text, iv16);
    }

    fn decrypt_aes_internal(&self, cipher_text: Vec<u8>, iv16: Vec<u8>) -> Vec<u8> {
        //decrypt(cipher_text, secretKey, iv16, "aes-256-cbc", true),

        // // let key = Aes256GcmSiv::generate_key(&mut OsRng);
        // let key = GenericArray::from_slice([0u8; 32].as_ref());
        // let cipher = Aes256GcmSiv::new(&key);
        // let nonce = Nonce::from_slice(&iv16);
        // // let nonce = GenericArray::from_slice(iv16.as_ref());
        // let decrypted_message = cipher.decrypt(nonce, cipher_text.as_ref())
        //     .expect("decryption failure!");
        //
        // decrypted_message
        unimplemented!("dec_aes")
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use num_bigint::BigUint;

    use super::*;
    use crate::poseidon::poseidon_hash;

    #[test]
    fn poseidon_test_uneq() {
        let x1 = vec![1u8; 30];
        let y1 = vec![2u8; 30];
        let hash1 = poseidon_hash(vec![x1, y1]);

        let x2 = vec![1u8; 29];
        let y2 = vec![2u8; 31];
        let hash2 = poseidon_hash(vec![x2, y2]);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn poseidon_test_eq() {
        let x1 = vec![1u8; 30];
        let y1 = vec![2u8; 30];
        let hash1 = poseidon_hash(vec![x1, y1]);

        let x2 = vec![1u8; 30];
        let y2 = vec![2u8; 30];
        let hash2 = poseidon_hash(vec![x2, y2]);

        assert_eq!(hash1, hash2);
    }


    /*
    const seed32 = bs58.encode(new Uint8Array(32).fill(1));
    const keypairReferenceAccount = {
      pubkey:
        "4872110042567103538494256021660200303799952171296478346021664068592360300729",
      eddsaSignature:
        "149,4,55,200,119,181,112,89,28,114,19,62,250,125,9,166,167,0,255,21,231,177,123,126,100,125,212,10,93,27,186,172,107,200,130,11,182,98,146,73,73,248,205,73,73,217,201,196,85,249,115,198,152,225,175,160,254,131,131,146,148,73,211,1",
    };
*/

    fn assert_account_equality(a0: &Account, a1: &Account) {
        assert_eq!(a0.private_key, a1.private_key);
        assert_eq!(a0.public_key, a1.public_key);
        assert_eq!(a0.aes_secret, a1.aes_secret);
        assert_eq!(a0.encryption_private_key, a1.encryption_private_key);
        assert_eq!(a0.encryption_public_key, a1.encryption_public_key);
    }

    fn assert_encryption_public_key(account: &Account) {
        let ref_encryption_public_key = vec![
            246, 239, 160, 64, 108, 202, 122, 119, 186, 218, 229, 31, 22, 26, 16, 217, 91, 100,
            166, 215, 150, 23, 31, 160, 171, 11, 70, 146, 121, 162, 63, 118,
        ];
        assert_eq!(account.encryption_public_key, ref_encryption_public_key);
    }

    fn assert_private_key(account: &Account) {
        let ref_private_key =
            "1150625612398812446861023352466533113402693634687987105896815329603238601802";

        //10549614312533267481475691431782247653443118790544059346879074363637081087224 -- keypair
        //8005258175950153822746760972612266673018285206748118268998514552503031523041 -- seed
        let ref_privkey = BigUint::from_str(ref_private_key).unwrap().to_bytes_be();
        assert_eq!(account.private_key, ref_privkey);
    }

    fn assert_public_key(account: &Account) {
        let ref_pubkey_str =
            "9068318595592656704518224523825369341156075926474609439138522814574969615219";

        //4872110042567103538494256021660200303799952171296478346021664068592360300729 -- keypair
        //6377640866559980556624371737408417701494249873246144458744315242624363752533 -- seed
        let ref_pubkey = BigUint::from_str(ref_pubkey_str).unwrap().to_bytes_be();
        assert_eq!(account.public_key, ref_pubkey);
    }

    fn assert_eddsa_sig(account: &Account) {
        let ref_edds_sig: [u8; 64] = [149,4,55,200,119,181,112,89,28,114,19,62,250,125,9,166,167,0,255,21,231,177,123,126,100,125,212,10,93,27,186,172,107,200,130,11,182,98,146,73,73,248,205,73,73,217,201,196,85,249,115,198,152,225,175,160,254,131,131,146,148,73,211,1];
        let ref_eddsa = BigUint::from_bytes_be(&ref_edds_sig);
    }

    #[test]
    fn seed_init_test() {
        let seed32 = bs58::encode(vec![1u8; 32]).into_string();
        println!("seed32: {:?}", seed32);
        let a0 = Account::new(seed32.clone());
        let a1 = Account::new(seed32);
        assert_account_equality(&a0, &a1);

        assert_encryption_public_key(&a0);
        assert_private_key(&a0);
        assert_public_key(&a0);
        assert_eddsa_sig(&a0);
    }
    /*

    const referenceAccount = {
      privkey:
        "8005258175950153822746760972612266673018285206748118268998514552503031523041",
      pubkey:
        "6377640866559980556624371737408417701494249873246144458744315242624363752533",
      eddsaSignature:
        "49,171,181,231,94,94,233,87,62,92,132,207,160,18,252,199,169,46,131,38,9,250,202,156,232,7,147,10,62,115,216,21,224,99,163,86,218,224,115,91,107,158,231,171,120,83,79,35,221,119,92,43,69,148,166,215,39,96,194,102,65,19,238,1",
    };
    await compareAccountToReference(k0, referenceAccount);
    */
}
