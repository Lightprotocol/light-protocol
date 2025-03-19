use std::vec;

use anchor_lang::{
    prelude::borsh, solana_program::pubkey::Pubkey, AnchorDeserialize, AnchorSerialize,
};
use light_compressed_account::hash_to_bn254_field_size_be;
use light_hasher::{errors::HasherError, Hasher, Poseidon};

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum AccountState {
    Initialized,
    Frozen,
}

#[derive(Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TokenData {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: Option<Pubkey>,
    /// The account's state
    pub state: AccountState,
    /// Placeholder for TokenExtension tlv data (unimplemented)
    pub tlv: Option<Vec<u8>>,
}

/// Hashing schema: H(mint, owner, amount, delegate, delegated_amount,
/// is_native, state)
///
/// delegate, delegated_amount, is_native and state have dynamic positions.
/// Always hash mint, owner and amount If delegate hash delegate and
/// delegated_amount together. If is native hash is_native else is omitted.
/// If frozen hash AccountState::Frozen else is omitted.
///
/// Security: to prevent the possibility that different fields with the same
/// value to result in the same hash we add a prefix to the delegated amount, is
/// native and state fields. This way we can have a dynamic hashing schema and
/// hash only used values.
impl TokenData {
    /// Only the spl representation of native tokens (wrapped SOL) is
    /// compressed.
    /// The sol value is stored in the token pool account.
    /// The sol value in the compressed account is independent from
    /// the wrapped sol amount.
    pub fn is_native(&self) -> bool {
        self.mint == spl_token::native_mint::id()
    }
    pub fn hash_with_hashed_values(
        hashed_mint: &[u8; 32],
        hashed_owner: &[u8; 32],
        amount_bytes: &[u8; 32],
        hashed_delegate: &Option<&[u8; 32]>,
    ) -> std::result::Result<[u8; 32], HasherError> {
        Self::hash_inputs_with_hashed_values::<false>(
            hashed_mint,
            hashed_owner,
            amount_bytes,
            hashed_delegate,
        )
    }

    pub fn hash_frozen_with_hashed_values(
        hashed_mint: &[u8; 32],
        hashed_owner: &[u8; 32],
        amount_bytes: &[u8; 32],
        hashed_delegate: &Option<&[u8; 32]>,
    ) -> std::result::Result<[u8; 32], HasherError> {
        Self::hash_inputs_with_hashed_values::<true>(
            hashed_mint,
            hashed_owner,
            amount_bytes,
            hashed_delegate,
        )
    }

    /// We should not hash pubkeys multiple times. For all we can assume mints
    /// are equal. For all input compressed accounts we assume owners are
    /// equal.
    pub fn hash_inputs_with_hashed_values<const FROZEN_INPUTS: bool>(
        mint: &[u8; 32],
        owner: &[u8; 32],
        amount_bytes: &[u8],
        hashed_delegate: &Option<&[u8; 32]>,
    ) -> std::result::Result<[u8; 32], HasherError> {
        let mut hash_inputs = vec![mint.as_slice(), owner.as_slice(), amount_bytes];
        if let Some(hashed_delegate) = hashed_delegate {
            hash_inputs.push(hashed_delegate.as_slice());
        }
        let mut state_bytes = [0u8; 32];
        if FROZEN_INPUTS {
            state_bytes[31] = AccountState::Frozen as u8;
            hash_inputs.push(&state_bytes[..]);
        }
        Poseidon::hashv(hash_inputs.as_slice())
    }
}

impl TokenData {
    /// Hashes token data of token accounts.
    ///
    /// Note, hashing changed for token account data in batched Merkle trees.
    /// For hashing of token account data stored in concurrent Merkle trees use hash_legacy().
    pub fn hash(&self) -> std::result::Result<[u8; 32], HasherError> {
        self._hash::<true>()
    }

    /// Hashes token data of token accounts stored in concurrent Merkle trees.
    pub fn hash_legacy(&self) -> std::result::Result<[u8; 32], HasherError> {
        self._hash::<false>()
    }

    fn _hash<const BATCHED: bool>(&self) -> std::result::Result<[u8; 32], HasherError> {
        let hashed_mint = hash_to_bn254_field_size_be(self.mint.to_bytes().as_slice())?;
        let hashed_owner = hash_to_bn254_field_size_be(self.owner.to_bytes().as_slice())?;
        let mut amount_bytes = [0u8; 32];
        if BATCHED {
            amount_bytes[24..].copy_from_slice(self.amount.to_be_bytes().as_slice());
        } else {
            amount_bytes[24..].copy_from_slice(self.amount.to_le_bytes().as_slice());
        }
        let hashed_delegate;
        let hashed_delegate_option = if let Some(delegate) = self.delegate {
            hashed_delegate = hash_to_bn254_field_size_be(delegate.to_bytes().as_slice())?;
            Some(&hashed_delegate)
        } else {
            None
        };
        if self.state != AccountState::Initialized {
            Self::hash_inputs_with_hashed_values::<true>(
                &hashed_mint,
                &hashed_owner,
                &amount_bytes,
                &hashed_delegate_option,
            )
        } else {
            Self::hash_inputs_with_hashed_values::<false>(
                &hashed_mint,
                &hashed_owner,
                &amount_bytes,
                &hashed_delegate_option,
            )
        }
    }
}

#[cfg(test)]
pub mod test {

    use num_bigint::BigUint;
    use rand::Rng;

    use super::*;

    #[test]
    fn equivalency_of_hash_functions() {
        let token_data = TokenData {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: 100,
            delegate: Some(Pubkey::new_unique()),
            state: AccountState::Initialized,
            tlv: None,
        };
        let hashed_token_data = token_data.hash_legacy().unwrap();
        let hashed_mint =
            hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice()).unwrap();
        let hashed_owner =
            hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice()).unwrap();
        let hashed_delegate =
            hash_to_bn254_field_size_be(token_data.delegate.unwrap().to_bytes().as_slice())
                .unwrap();
        let mut amount_bytes = [0u8; 32];
        amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
        let hashed_token_data_with_hashed_values =
            TokenData::hash_inputs_with_hashed_values::<false>(
                &hashed_mint,
                &hashed_owner,
                &amount_bytes,
                &Some(&hashed_delegate),
            )
            .unwrap();
        assert_eq!(hashed_token_data, hashed_token_data_with_hashed_values);

        let token_data = TokenData {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: 101,
            delegate: None,
            state: AccountState::Initialized,
            tlv: None,
        };
        let hashed_token_data = token_data.hash_legacy().unwrap();
        let hashed_mint =
            hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice()).unwrap();
        let hashed_owner =
            hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice()).unwrap();
        let mut amount_bytes = [0u8; 32];
        amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
        let hashed_token_data_with_hashed_values =
            TokenData::hash_with_hashed_values(&hashed_mint, &hashed_owner, &amount_bytes, &None)
                .unwrap();
        assert_eq!(hashed_token_data, hashed_token_data_with_hashed_values);
    }

    impl TokenData {
        fn legacy_hash(&self) -> std::result::Result<[u8; 32], HasherError> {
            let hashed_mint = hash_to_bn254_field_size_be(self.mint.to_bytes().as_slice())?;
            let hashed_owner = hash_to_bn254_field_size_be(self.owner.to_bytes().as_slice())?;
            let amount_bytes = self.amount.to_le_bytes();
            let hashed_delegate;
            let hashed_delegate_option = if let Some(delegate) = self.delegate {
                hashed_delegate = hash_to_bn254_field_size_be(delegate.to_bytes().as_slice())?;
                Some(&hashed_delegate)
            } else {
                None
            };
            if self.state != AccountState::Initialized {
                Self::hash_inputs_with_hashed_values::<true>(
                    &hashed_mint,
                    &hashed_owner,
                    &amount_bytes,
                    &hashed_delegate_option,
                )
            } else {
                Self::hash_inputs_with_hashed_values::<false>(
                    &hashed_mint,
                    &hashed_owner,
                    &amount_bytes,
                    &hashed_delegate_option,
                )
            }
        }
    }
    fn equivalency_of_hash_functions_rnd_iters<const ITERS: usize>() {
        let mut rng = rand::thread_rng();

        for _ in 0..ITERS {
            let token_data = TokenData {
                mint: Pubkey::new_unique(),
                owner: Pubkey::new_unique(),
                amount: rng.gen(),
                delegate: Some(Pubkey::new_unique()),
                state: AccountState::Initialized,
                tlv: None,
            };
            let hashed_token_data = token_data.hash_legacy().unwrap();
            let hashed_mint =
                hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice()).unwrap();
            let hashed_owner =
                hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice()).unwrap();
            let hashed_delegate =
                hash_to_bn254_field_size_be(token_data.delegate.unwrap().to_bytes().as_slice())
                    .unwrap();
            let mut amount_bytes = [0u8; 32];
            amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
            let hashed_token_data_with_hashed_values = TokenData::hash_with_hashed_values(
                &hashed_mint,
                &hashed_owner,
                &amount_bytes,
                &Some(&hashed_delegate),
            )
            .unwrap();
            assert_eq!(hashed_token_data, hashed_token_data_with_hashed_values);
            let legacy_hash = token_data.legacy_hash().unwrap();
            assert_eq!(hashed_token_data, legacy_hash);

            let token_data = TokenData {
                mint: Pubkey::new_unique(),
                owner: Pubkey::new_unique(),
                amount: rng.gen(),
                delegate: None,
                state: AccountState::Initialized,
                tlv: None,
            };
            let hashed_token_data = token_data.hash_legacy().unwrap();
            let hashed_mint =
                hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice()).unwrap();
            let hashed_owner =
                hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice()).unwrap();
            let mut amount_bytes = [0u8; 32];
            amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
            let hashed_token_data_with_hashed_values: [u8; 32] =
                TokenData::hash_with_hashed_values(
                    &hashed_mint,
                    &hashed_owner,
                    &amount_bytes,
                    &None,
                )
                .unwrap();
            assert_eq!(hashed_token_data, hashed_token_data_with_hashed_values);
            let legacy_hash = token_data.legacy_hash().unwrap();
            assert_eq!(hashed_token_data, legacy_hash);
        }
    }

    #[test]
    fn equivalency_of_hash_functions_iters_poseidon() {
        equivalency_of_hash_functions_rnd_iters::<10_000>();
    }

    #[test]
    fn test_circuit_equivalence() {
        // Convert hex strings to Pubkeys
        let mint_pubkey = Pubkey::new_from_array([
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        let owner_pubkey = Pubkey::new_from_array([
            0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        let delegate_pubkey = Pubkey::new_from_array([
            0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);

        let token_data = TokenData {
            mint: mint_pubkey,
            owner: owner_pubkey,
            amount: 1000000u64,
            delegate: Some(delegate_pubkey),
            state: AccountState::Initialized, // Using Frozen state to match our circuit test
            tlv: None,
        };

        // Calculate the hash with the Rust code
        let rust_hash = token_data.hash().unwrap();

        let circuit_hash_str =
            "12698830169693734517877055378728747723888091986541703429186543307137690361131";
        use std::str::FromStr;
        let circuit_hash = BigUint::from_str(circuit_hash_str).unwrap().to_bytes_be();
        let rust_hash_string = BigUint::from_bytes_be(rust_hash.as_slice()).to_string();
        println!("Circuit hash string: {}", circuit_hash_str);
        println!("rust_hash_string {}", rust_hash_string);
        assert_eq!(rust_hash.to_vec(), circuit_hash);
    }

    #[test]
    fn test_frozen_equivalence() {
        let token_data = TokenData {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: 100,
            delegate: Some(Pubkey::new_unique()),
            state: AccountState::Initialized,
            tlv: None,
        };
        let hashed_mint =
            hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice()).unwrap();
        let hashed_owner =
            hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice()).unwrap();
        let hashed_delegate =
            hash_to_bn254_field_size_be(token_data.delegate.unwrap().to_bytes().as_slice())
                .unwrap();
        let mut amount_bytes = [0u8; 32];
        amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
        let hash = TokenData::hash_with_hashed_values(
            &hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &Some(&hashed_delegate),
        )
        .unwrap();
        let other_hash = token_data.hash_legacy().unwrap();
        assert_eq!(hash, other_hash);
    }

    #[test]
    fn failing_tests_hashing() {
        let mut vec_previous_hashes = Vec::new();
        let token_data = TokenData {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: 100,
            delegate: None,
            state: AccountState::Initialized,
            tlv: None,
        };
        let hashed_mint =
            hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice()).unwrap();
        let hashed_owner =
            hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice()).unwrap();
        let mut amount_bytes = [0u8; 32];
        amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
        let hash =
            TokenData::hash_with_hashed_values(&hashed_mint, &hashed_owner, &amount_bytes, &None)
                .unwrap();
        vec_previous_hashes.push(hash);
        // different mint
        let hashed_mint_2 =
            hash_to_bn254_field_size_be(Pubkey::new_unique().to_bytes().as_slice()).unwrap();
        let mut amount_bytes = [0u8; 32];
        amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
        let hash2 =
            TokenData::hash_with_hashed_values(&hashed_mint_2, &hashed_owner, &amount_bytes, &None)
                .unwrap();
        assert_to_previous_hashes(hash2, &mut vec_previous_hashes);

        // different owner
        let hashed_owner_2 =
            hash_to_bn254_field_size_be(Pubkey::new_unique().to_bytes().as_slice()).unwrap();
        let mut amount_bytes = [0u8; 32];
        amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
        let hash3 =
            TokenData::hash_with_hashed_values(&hashed_mint, &hashed_owner_2, &amount_bytes, &None)
                .unwrap();
        assert_to_previous_hashes(hash3, &mut vec_previous_hashes);

        // different amount
        let different_amount: u64 = 101;
        let mut different_amount_bytes = [0u8; 32];
        different_amount_bytes[24..].copy_from_slice(different_amount.to_le_bytes().as_slice());
        let hash4 = TokenData::hash_with_hashed_values(
            &hashed_mint,
            &hashed_owner,
            &different_amount_bytes,
            &None,
        )
        .unwrap();
        assert_to_previous_hashes(hash4, &mut vec_previous_hashes);

        // different delegate
        let delegate = Pubkey::new_unique();
        let hashed_delegate = hash_to_bn254_field_size_be(delegate.to_bytes().as_slice()).unwrap();
        let mut amount_bytes = [0u8; 32];
        amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
        let hash7 = TokenData::hash_with_hashed_values(
            &hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &Some(&hashed_delegate),
        )
        .unwrap();

        assert_to_previous_hashes(hash7, &mut vec_previous_hashes);
        // different account state
        let mut token_data = token_data;
        token_data.state = AccountState::Frozen;
        let hash9 = token_data.hash_legacy().unwrap();
        assert_to_previous_hashes(hash9, &mut vec_previous_hashes);
        // different account state with delegate
        token_data.delegate = Some(delegate);
        let hash10 = token_data.hash_legacy().unwrap();
        assert_to_previous_hashes(hash10, &mut vec_previous_hashes);
    }

    fn assert_to_previous_hashes(hash: [u8; 32], previous_hashes: &mut Vec<[u8; 32]>) {
        for previous_hash in previous_hashes.iter() {
            assert_ne!(hash, *previous_hash);
        }
        println!("len previous hashes: {}", previous_hashes.len());
        previous_hashes.push(hash);
    }
}
