use light_compressed_account::Pubkey;
use light_zero_copy::errors::ZeroCopyError;

use crate::{state::ExtensionStruct, AnchorDeserialize, AnchorSerialize, TokenError};

/// AccountType discriminator value for token accounts (at byte 165)
pub const ACCOUNT_TYPE_TOKEN_ACCOUNT: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum AccountState {
    Uninitialized = 0,
    Initialized = 1,
    Frozen = 2,
}

impl TryFrom<u8> for AccountState {
    type Error = TokenError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AccountState::Uninitialized),
            1 => Ok(AccountState::Initialized),
            2 => Ok(AccountState::Frozen),
            _ => Err(TokenError::InvalidAccountState),
        }
    }
}

/// Ctoken account structure (same as SPL Token Account but with extensions).
/// Ctokens are solana accounts, compressed tokens are stored
/// as TokenData that is optimized for compressed accounts.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Token {
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
    /// Account type discriminator (at byte 165 when extensions present).
    /// For valid Token accounts this is ACCOUNT_TYPE_TOKEN_ACCOUNT (2).
    pub account_type: u8,
    /// Extensions for the token account (including compressible config)
    pub extensions: Option<Vec<ExtensionStruct>>,
}

impl Token {
    /// Extract amount directly from account data slice using hardcoded offset
    /// Token layout: mint (32 bytes) + owner (32 bytes) + amount (8 bytes)
    pub fn amount_from_slice(data: &[u8]) -> Result<u64, ZeroCopyError> {
        const AMOUNT_OFFSET: usize = 64; // 32 (mint) + 32 (owner)

        check_token_account(data)?;

        #[inline(always)]
        fn check_token_account(bytes: &[u8]) -> Result<(), ZeroCopyError> {
            if bytes.len() == 165 || (bytes.len() > 165 && bytes[165] == ACCOUNT_TYPE_TOKEN_ACCOUNT)
            {
                Ok(())
            } else {
                Err(ZeroCopyError::InvalidConversion)
            }
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

    /// Returns the account type discriminator
    #[inline(always)]
    pub fn account_type(&self) -> u8 {
        self.account_type
    }

    /// Checks if account_type matches Token discriminator value
    #[inline(always)]
    pub fn is_token_account(&self) -> bool {
        self.account_type == ACCOUNT_TYPE_TOKEN_ACCOUNT
    }
}
