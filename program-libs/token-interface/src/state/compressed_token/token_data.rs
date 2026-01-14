use light_compressed_account::Pubkey;
use light_program_profiler::profile;
use light_zero_copy::{num_trait::ZeroCopyNumTrait, ZeroCopy, ZeroCopyMut};

use crate::{
    instructions::extensions::ZExtensionInstructionData,
    state::extensions::{ExtensionStruct, ZExtensionStructMut},
    AnchorDeserialize, AnchorSerialize, TokenError,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum CompressedTokenAccountState {
    //Uninitialized, is always initialized.
    Initialized = 0,
    Frozen = 1,
}

impl TryFrom<u8> for CompressedTokenAccountState {
    type Error = TokenError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CompressedTokenAccountState::Initialized),
            1 => Ok(CompressedTokenAccountState::Frozen),
            _ => Err(TokenError::InvalidAccountState),
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
    /// Extensions for the compressed token account
    pub tlv: Option<Vec<ExtensionStruct>>,
}

impl TokenData {
    pub fn state(&self) -> Result<CompressedTokenAccountState, TokenError> {
        CompressedTokenAccountState::try_from(self.state)
    }
}

// Implementation for zero-copy mutable TokenData
impl<'a> ZTokenDataMut<'a> {
    /// Set all fields of the TokenData struct at once.
    /// All data must be allocated before calling this function.
    #[inline]
    #[profile]
    pub fn set(
        &mut self,
        mint: Pubkey,
        owner: Pubkey,
        amount: impl ZeroCopyNumTrait,
        delegate: Option<Pubkey>,
        state: CompressedTokenAccountState,
        tlv_data: Option<&[ZExtensionInstructionData<'_>]>,
    ) -> Result<(), TokenError> {
        self.mint = mint;
        self.owner = owner;
        self.amount.set(amount.into());
        if let Some(z_delegate) = self.delegate.as_deref_mut() {
            *z_delegate = delegate.ok_or(TokenError::InstructionDataExpectedDelegate)?;
        }
        if self.delegate.is_none() && delegate.is_some() {
            return Err(TokenError::ZeroCopyExpectedDelegate);
        }

        *self.state = state as u8;

        // Set TLV extension values (space was pre-allocated via new_zero_copy)
        match (self.tlv.as_mut(), tlv_data) {
            (Some(tlv_vec), Some(exts)) => {
                if tlv_vec.len() != 1 || exts.len() != 1 {
                    return Err(TokenError::TlvExtensionLengthMismatch);
                }
                for (tlv_ext, instruction_ext) in tlv_vec.iter_mut().zip(exts.iter()) {
                    match (tlv_ext, instruction_ext) {
                        (
                            ZExtensionStructMut::CompressedOnly(compressed_only),
                            ZExtensionInstructionData::CompressedOnly(data),
                        ) => {
                            compressed_only.delegated_amount = data.delegated_amount;
                            compressed_only.withheld_transfer_fee = data.withheld_transfer_fee;
                            compressed_only.is_ata = if data.is_ata() { 1 } else { 0 };
                        }
                        _ => return Err(TokenError::UnsupportedTlvExtensionType),
                    }
                }
            }
            (Some(_), None) | (None, Some(_)) => {
                return Err(TokenError::TlvExtensionLengthMismatch);
            }
            (None, None) => {}
        }

        Ok(())
    }
}
