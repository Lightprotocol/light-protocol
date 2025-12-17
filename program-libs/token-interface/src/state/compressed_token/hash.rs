use borsh::BorshSerialize;
use light_compressed_account::hash_to_bn254_field_size_be;
use light_hasher::{errors::HasherError, sha256::Sha256BE, Hasher, Poseidon};
use light_program_profiler::profile;

use super::TokenData;
use crate::{state::compressed_token::CompressedTokenAccountState, NATIVE_MINT};

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
        self.mint == NATIVE_MINT
    }

    pub fn hash_with_hashed_values(
        hashed_mint: &[u8; 32],
        hashed_owner: &[u8; 32],
        amount_bytes: &[u8; 32],
        hashed_delegate: &Option<&[u8; 32]>,
    ) -> Result<[u8; 32], HasherError> {
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
    ) -> Result<[u8; 32], HasherError> {
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
    ) -> Result<[u8; 32], HasherError> {
        let mut hash_inputs = vec![mint.as_slice(), owner.as_slice(), amount_bytes];
        if let Some(hashed_delegate) = hashed_delegate {
            hash_inputs.push(hashed_delegate.as_slice());
        }
        let mut state_bytes = [0u8; 32];
        if FROZEN_INPUTS {
            state_bytes[31] = CompressedTokenAccountState::Frozen as u8;
            hash_inputs.push(&state_bytes[..]);
        }
        Poseidon::hashv(hash_inputs.as_slice())
    }
}

impl TokenData {
    /// TokenDataVersion 3
    /// CompressedAccount Discriminator [0,0,0,0,0,0,0,4]
    #[profile]
    #[inline(always)]
    pub fn hash_sha_flat(&self) -> Result<[u8; 32], HasherError> {
        let bytes = self.try_to_vec().map_err(|_| HasherError::BorshError)?;
        Sha256BE::hash(bytes.as_slice())
    }

    /// Hashes token data of token accounts.
    ///
    /// Note, hashing changed for token account data in batched Merkle trees.
    /// For hashing of token account data stored in concurrent Merkle trees use hash_v1().
    /// TokenDataVersion 2
    /// CompressedAccount Discriminator [0,0,0,0,0,0,0,3]
    pub fn hash_v2(&self) -> Result<[u8; 32], HasherError> {
        self._hash::<true>()
    }

    /// Hashes token data of token accounts stored in concurrent Merkle trees.
    /// TokenDataVersion 1
    /// CompressedAccount Discriminator [2,0,0,0,0,0,0,0]
    ///
    pub fn hash_v1(&self) -> Result<[u8; 32], HasherError> {
        self._hash::<false>()
    }

    fn _hash<const BATCHED: bool>(&self) -> Result<[u8; 32], HasherError> {
        let hashed_mint = hash_to_bn254_field_size_be(self.mint.to_bytes().as_slice());
        let hashed_owner = hash_to_bn254_field_size_be(self.owner.to_bytes().as_slice());
        let mut amount_bytes = [0u8; 32];
        if BATCHED {
            amount_bytes[24..].copy_from_slice(self.amount.to_be_bytes().as_slice());
        } else {
            amount_bytes[24..].copy_from_slice(self.amount.to_le_bytes().as_slice());
        }
        let hashed_delegate;
        let hashed_delegate_option = if let Some(delegate) = self.delegate {
            hashed_delegate = hash_to_bn254_field_size_be(delegate.to_bytes().as_slice());
            Some(&hashed_delegate)
        } else {
            None
        };
        if self.state != CompressedTokenAccountState::Initialized as u8 {
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
