use light_compressed_account::{hash_to_bn254_field_size_be, Pubkey};
use light_hasher::{errors::HasherError, Hasher, Poseidon};
use light_zero_copy::{num_trait::ZeroCopyNumTrait, ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize, CTokenError, NATIVE_MINT};

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum AccountState {
    //Uninitialized,
    Initialized,
    Frozen,
}

impl TryFrom<u8> for AccountState {
    type Error = CTokenError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            //0 => Ok(AccountState::Uninitialized), TODO: check with main that we don't create breaking changes for v1 token data.
            0 => Ok(AccountState::Initialized),
            1 => Ok(AccountState::Frozen),
            _ => Err(CTokenError::InvalidAccountState),
        }
    }
}

#[derive(
    Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, Clone, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
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
    pub state: u8,
    /// Placeholder for TokenExtension tlv data (unimplemented)
    pub tlv: Option<Vec<u8>>,
}

impl TokenData {
    pub fn state(&self) -> Result<AccountState, CTokenError> {
        AccountState::try_from(self.state)
    }
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
    pub fn hash(&self) -> Result<[u8; 32], HasherError> {
        self._hash::<true>()
    }
    // TODO: rename to v1
    // TODO: add hard coded v1 compat test
    /// Hashes token data of token accounts stored in concurrent Merkle trees.
    pub fn hash_legacy(&self) -> Result<[u8; 32], HasherError> {
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
        if self.state != AccountState::Initialized as u8 {
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

// Implementation for zero-copy mutable TokenData
impl ZTokenDataMut<'_> {
    /// Set all fields of the TokenData struct at once
    #[inline]
    pub fn set(
        &mut self,
        mint: Pubkey,
        owner: Pubkey,
        amount: impl ZeroCopyNumTrait,
        delegate: Option<Pubkey>,
        state: AccountState,
    ) -> Result<(), CTokenError> {
        self.mint = mint;
        self.owner = owner;
        self.amount.set(amount.into());
        if let Some(z_delegate) = self.delegate.as_deref_mut() {
            *z_delegate = delegate.ok_or(CTokenError::InstructionDataExpectedDelegate)?;
        }
        if self.delegate.is_none() && delegate.is_some() {
            return Err(CTokenError::ZeroCopyExpectedDelegate);
        }

        *self.state = state as u8;

        if self.tlv.is_some() {
            return Err(CTokenError::TokenDataTlvUnimplemented);
        }
        Ok(())
    }
}
