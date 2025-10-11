use light_compressed_account::Pubkey;
use light_program_profiler::profile;
use light_zero_copy::{num_trait::ZeroCopyNumTrait, ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize, CTokenError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum CompressedTokenAccountState {
    //Uninitialized, is always initialized.
    Initialized = 0,
    Frozen = 1,
}

impl TryFrom<u8> for CompressedTokenAccountState {
    type Error = CTokenError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CompressedTokenAccountState::Initialized),
            1 => Ok(CompressedTokenAccountState::Frozen),
            _ => Err(CTokenError::InvalidAccountState),
        }
    }
}

/// TokenData of Compressed Tokens.
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
    pub fn state(&self) -> Result<CompressedTokenAccountState, CTokenError> {
        CompressedTokenAccountState::try_from(self.state)
    }
}

// Implementation for zero-copy mutable TokenData
impl ZTokenDataMut<'_> {
    /// Set all fields of the TokenData struct at once
    #[inline]
    #[profile]
    pub fn set(
        &mut self,
        mint: Pubkey,
        owner: Pubkey,
        amount: impl ZeroCopyNumTrait,
        delegate: Option<Pubkey>,
        state: CompressedTokenAccountState,
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
