use std::vec;

use anchor_lang::{
    prelude::borsh, solana_program::pubkey::Pubkey, AnchorDeserialize, AnchorSerialize,
};
use light_hasher::{errors::HasherError, DataHasher};
use light_utils::hash_to_bn254_field_size_be;

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
    pub fn hash_with_hashed_values<H: light_hasher::Hasher>(
        hashed_mint: &[u8; 32],
        hashed_owner: &[u8; 32],
        amount_bytes: &[u8; 8],
        hashed_delegate: &Option<&[u8; 32]>,
    ) -> std::result::Result<[u8; 32], HasherError> {
        Self::hash_inputs_with_hashed_values::<H, false>(
            hashed_mint,
            hashed_owner,
            amount_bytes,
            hashed_delegate,
        )
    }

    pub fn hash_frozen_with_hashed_values<H: light_hasher::Hasher>(
        hashed_mint: &[u8; 32],
        hashed_owner: &[u8; 32],
        amount_bytes: &[u8; 8],
        hashed_delegate: &Option<&[u8; 32]>,
    ) -> std::result::Result<[u8; 32], HasherError> {
        Self::hash_inputs_with_hashed_values::<H, true>(
            hashed_mint,
            hashed_owner,
            amount_bytes,
            hashed_delegate,
        )
    }

    /// We should not hash pubkeys multiple times. For all we can assume mints
    /// are equal. For all input compressed accounts we assume owners are
    /// equal.
    pub fn hash_inputs_with_hashed_values<H: light_hasher::Hasher, const FROZEN_INPUTS: bool>(
        mint: &[u8; 32],
        owner: &[u8; 32],
        amount_bytes: &[u8; 8],
        hashed_delegate: &Option<&[u8; 32]>,
    ) -> std::result::Result<[u8; 32], HasherError> {
        let mut hash_inputs = vec![mint.as_slice(), owner.as_slice(), amount_bytes.as_slice()];
        if let Some(hashed_delegate) = hashed_delegate {
            hash_inputs.push(hashed_delegate.as_slice());
        }
        let state_bytes = [AccountState::Frozen as u8];
        if FROZEN_INPUTS {
            hash_inputs.push(&state_bytes[..]);
        }
        H::hashv(hash_inputs.as_slice())
    }
}

impl DataHasher for TokenData {
    fn hash<H: light_hasher::Hasher>(&self) -> std::result::Result<[u8; 32], HasherError> {
        let hashed_mint = hash_to_bn254_field_size_be(self.mint.to_bytes().as_slice())
            .unwrap()
            .0;
        let hashed_owner = hash_to_bn254_field_size_be(self.owner.to_bytes().as_slice())
            .unwrap()
            .0;
        let amount_bytes = self.amount.to_le_bytes();
        let hashed_delegate;
        let hashed_delegate_option = if let Some(delegate) = self.delegate {
            hashed_delegate = hash_to_bn254_field_size_be(delegate.to_bytes().as_slice())
                .unwrap()
                .0;
            Some(&hashed_delegate)
        } else {
            None
        };
        if self.state != AccountState::Initialized {
            Self::hash_inputs_with_hashed_values::<H, true>(
                &hashed_mint,
                &hashed_owner,
                &amount_bytes,
                &hashed_delegate_option,
            )
        } else {
            Self::hash_inputs_with_hashed_values::<H, false>(
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
    use super::*;
    use light_hasher::{Keccak, Poseidon};
    use rand::Rng;

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
        let hashed_token_data = token_data.hash::<Poseidon>().unwrap();
        let hashed_mint = hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice())
            .unwrap()
            .0;
        let hashed_owner = hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice())
            .unwrap()
            .0;
        let hashed_delegate =
            hash_to_bn254_field_size_be(token_data.delegate.unwrap().to_bytes().as_slice())
                .unwrap()
                .0;
        let hashed_token_data_with_hashed_values =
            TokenData::hash_inputs_with_hashed_values::<Poseidon, false>(
                &hashed_mint,
                &hashed_owner,
                &token_data.amount.to_le_bytes(),
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
        let hashed_token_data = token_data.hash::<Poseidon>().unwrap();
        let hashed_mint = hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice())
            .unwrap()
            .0;
        let hashed_owner = hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice())
            .unwrap()
            .0;
        let hashed_token_data_with_hashed_values = TokenData::hash_with_hashed_values::<Poseidon>(
            &hashed_mint,
            &hashed_owner,
            &token_data.amount.to_le_bytes(),
            &None,
        )
        .unwrap();
        assert_eq!(hashed_token_data, hashed_token_data_with_hashed_values);
    }

    fn equivalency_of_hash_functions_rnd_iters<H: light_hasher::Hasher, const ITERS: usize>() {
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
            let hashed_token_data = token_data.hash::<H>().unwrap();
            let hashed_mint = hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice())
                .unwrap()
                .0;
            let hashed_owner = hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice())
                .unwrap()
                .0;
            let hashed_delegate =
                hash_to_bn254_field_size_be(token_data.delegate.unwrap().to_bytes().as_slice())
                    .unwrap()
                    .0;
            let hashed_token_data_with_hashed_values = TokenData::hash_with_hashed_values::<H>(
                &hashed_mint,
                &hashed_owner,
                &token_data.amount.to_le_bytes(),
                &Some(&hashed_delegate),
            )
            .unwrap();
            assert_eq!(hashed_token_data, hashed_token_data_with_hashed_values);

            let token_data = TokenData {
                mint: Pubkey::new_unique(),
                owner: Pubkey::new_unique(),
                amount: rng.gen(),
                delegate: None,
                state: AccountState::Initialized,
                tlv: None,
            };
            let hashed_token_data = token_data.hash::<H>().unwrap();
            let hashed_mint = hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice())
                .unwrap()
                .0;
            let hashed_owner = hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice())
                .unwrap()
                .0;
            let hashed_token_data_with_hashed_values: [u8; 32] =
                TokenData::hash_with_hashed_values::<H>(
                    &hashed_mint,
                    &hashed_owner,
                    &token_data.amount.to_le_bytes(),
                    &None,
                )
                .unwrap();
            assert_eq!(hashed_token_data, hashed_token_data_with_hashed_values);
        }
    }

    #[test]
    fn equivalency_of_hash_functions_iters_poseidon() {
        equivalency_of_hash_functions_rnd_iters::<Poseidon, 10_000>();
    }

    #[test]
    fn equivalency_of_hash_functions_iters_keccak() {
        equivalency_of_hash_functions_rnd_iters::<Keccak, 100_000>();
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
        let hashed_mint = hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice())
            .unwrap()
            .0;
        let hashed_owner = hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice())
            .unwrap()
            .0;
        let hashed_delegate =
            hash_to_bn254_field_size_be(token_data.delegate.unwrap().to_bytes().as_slice())
                .unwrap()
                .0;
        let hash = TokenData::hash_with_hashed_values::<Poseidon>(
            &hashed_mint,
            &hashed_owner,
            &token_data.amount.to_le_bytes(),
            &Some(&hashed_delegate),
        )
        .unwrap();
        let other_hash = token_data.hash::<Poseidon>().unwrap();
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
        let hashed_mint = hash_to_bn254_field_size_be(token_data.mint.to_bytes().as_slice())
            .unwrap()
            .0;
        let hashed_owner = hash_to_bn254_field_size_be(token_data.owner.to_bytes().as_slice())
            .unwrap()
            .0;
        let hash = TokenData::hash_with_hashed_values::<Poseidon>(
            &hashed_mint,
            &hashed_owner,
            &token_data.amount.to_le_bytes(),
            &None,
        )
        .unwrap();
        vec_previous_hashes.push(hash);
        // different mint
        let hashed_mint_2 = hash_to_bn254_field_size_be(Pubkey::new_unique().to_bytes().as_slice())
            .unwrap()
            .0;
        let hash2 = TokenData::hash_with_hashed_values::<Poseidon>(
            &hashed_mint_2,
            &hashed_owner,
            &token_data.amount.to_le_bytes(),
            &None,
        )
        .unwrap();
        assert_to_previous_hashes(hash2, &mut vec_previous_hashes);

        // different owner
        let hashed_owner_2 =
            hash_to_bn254_field_size_be(Pubkey::new_unique().to_bytes().as_slice())
                .unwrap()
                .0;
        let hash3 = TokenData::hash_with_hashed_values::<Poseidon>(
            &hashed_mint,
            &hashed_owner_2,
            &token_data.amount.to_le_bytes(),
            &None,
        )
        .unwrap();
        assert_to_previous_hashes(hash3, &mut vec_previous_hashes);

        // different amount
        let different_amount: u64 = 101;
        let hash4 = TokenData::hash_with_hashed_values::<Poseidon>(
            &hashed_mint,
            &hashed_owner,
            &different_amount.to_le_bytes(),
            &None,
        )
        .unwrap();
        assert_to_previous_hashes(hash4, &mut vec_previous_hashes);

        // different delegate
        let delegate = Some(Pubkey::new_unique());
        let hashed_delegate = hash_to_bn254_field_size_be(delegate.unwrap().to_bytes().as_slice())
            .unwrap()
            .0;
        let hash7 = TokenData::hash_with_hashed_values::<Poseidon>(
            &hashed_mint,
            &hashed_owner,
            &token_data.amount.to_le_bytes(),
            &Some(&hashed_delegate),
        )
        .unwrap();

        assert_to_previous_hashes(hash7, &mut vec_previous_hashes);
        // different account state
        let mut token_data = token_data;
        token_data.state = AccountState::Frozen;
        let hash9 = token_data.hash::<Poseidon>().unwrap();
        assert_to_previous_hashes(hash9, &mut vec_previous_hashes);
        // different account state with delegate
        let mut token_data = token_data;
        token_data.delegate = delegate;
        let hash10 = token_data.hash::<Poseidon>().unwrap();
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
