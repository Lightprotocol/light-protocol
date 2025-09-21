use light_compressed_account::Pubkey;
use light_zero_copy::errors::ZeroCopyError;

use crate::{state::ExtensionStruct, AnchorDeserialize, AnchorSerialize, CTokenError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum AccountState {
    Uninitialized = 0,
    Initialized = 1,
    Frozen = 2,
}

impl TryFrom<u8> for AccountState {
    type Error = CTokenError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AccountState::Uninitialized),
            1 => Ok(AccountState::Initialized),
            2 => Ok(AccountState::Frozen),
            _ => Err(CTokenError::InvalidAccountState),
        }
    }
}

/// Ctoken account structure (same as SPL Token Account but with extensions).
/// Ctokens are solana accounts, compressed tokens are stored
/// as TokenData that is optimized for compressed accounts.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct CToken {
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
    /// If `is_some`, this is a native token, and the value logs the rent-exempt
    /// reserve. An Account is required to be rent-exempt, so the value is
    /// used by the Processor to ensure that wrapped SOL accounts do not
    /// drop below this threshold.
    pub is_native: Option<u64>,
    /// The amount delegated
    pub delegated_amount: u64,
    /// Optional authority to close the account.
    pub close_authority: Option<Pubkey>,
    /// Extensions for the token account (including compressible config)
    pub extensions: Option<Vec<ExtensionStruct>>,
}

impl CToken {
    /// Extract amount directly from account data slice using hardcoded offset
    /// CToken layout: mint (32 bytes) + owner (32 bytes) + amount (8 bytes)
    pub fn amount_from_slice(data: &[u8]) -> Result<u64, ZeroCopyError> {
        const AMOUNT_OFFSET: usize = 64; // 32 (mint) + 32 (owner)

        if data.len() < AMOUNT_OFFSET + 8 {
            return Err(ZeroCopyError::Size);
        }

        let amount_bytes = &data[AMOUNT_OFFSET..AMOUNT_OFFSET + 8];
        let amount = u64::from_le_bytes(amount_bytes.try_into().map_err(|_| ZeroCopyError::Size)?);

        Ok(amount)
    }

    /// Extract amount from an AccountInfo
    #[cfg(feature = "solana")]
    pub fn amount_from_account_info(
        account_info: &solana_account_info::AccountInfo,
    ) -> Result<u64, ZeroCopyError> {
        let data = account_info
            .try_borrow_data()
            .map_err(|_| ZeroCopyError::Size)?;
        Self::amount_from_slice(&data)
    }

    /// Checks if account is frozen
    pub fn is_frozen(&self) -> bool {
        self.state == AccountState::Frozen
    }

    /// Checks if account is native
    pub fn is_native(&self) -> bool {
        self.is_native.is_some()
    }

    /// Checks if account is initialized
    pub fn is_initialized(&self) -> bool {
        self.state == AccountState::Initialized
    }
}
