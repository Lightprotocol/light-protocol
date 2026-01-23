//! Anchor wrapper for Light Protocol mint accounts.
//!
//! Provides `AccountLoader<'info, T>` - a type-safe wrapper for Light Protocol accounts that
//! provides zero-copy access. Named `AccountLoader` for API familiarity with Anchor users.
//!
//! # Key Insight: Why This Works with Anchor
//!
//! Anchor's codegen for ALL field types calls `Accounts::try_accounts()`. Our `AccountLoader`
//! implements this trait with no-op validation, allowing the account to be uninitialized
//! before CPI. Validation happens lazily when `load()` is called.
//!
//! # Usage
//!
//! Use `AccountLoader<'info, Mint>` in Anchor accounts structs:
//!
//! ```ignore
//! use light_token_interface::state::mint::{AccountLoader, Mint};
//!
//! #[derive(Accounts, LightAccounts)]  // Both derives work together!
//! #[instruction(params: CreateMintParams)]
//! pub struct CreateMint<'info> {
//!     #[account(mut)]
//!     #[light_account(init, mint::signer = mint_signer, ...)]
//!     pub mint: AccountLoader<'info, Mint>,
//! }
//!
//! pub fn handler(ctx: Context<CreateMint>) -> Result<()> {
//!     // After CPI completes, access mint data via zero-copy:
//!     let mint_data = ctx.accounts.mint.load()?;
//!     msg!("Decimals: {}", mint_data.decimals);
//!     Ok(())
//! }
//! ```
//!
//! # Zero-Copy Pattern
//!
//! `load()` returns a `ZMint<'info>` zero-copy view that reads directly from account
//! data without allocation. `load_mut()` returns a `ZMintMut<'info>` that can write
//! directly to account data. Since zero-copy writes directly, `AccountsExit::exit()`
//! is a no-op.

use std::{marker::PhantomData, ops::Deref};

use anchor_lang::prelude::*;

use super::{Mint, ZMint, ZMintMut, IS_INITIALIZED_OFFSET};
use crate::{TokenError, LIGHT_TOKEN_PROGRAM_ID};

/// Marker trait for types that can be loaded via AccountLoader.
///
/// This trait marks types that have zero-copy serialization support
/// and can be accessed through the AccountLoader pattern.
pub trait LightZeroCopy {}

impl LightZeroCopy for Mint {}

/// Zero-copy account loader for Light Protocol accounts.
///
/// Named `AccountLoader` for API familiarity with Anchor users.
/// Unlike Anchor's AccountLoader, this performs NO validation in `try_accounts`
/// - validation happens lazily when `load()` is called.
///
/// # Type Parameter
///
/// - `T`: The account type to load (e.g., `Mint`). Must implement `LightZeroCopy`.
///
/// # Anchor Integration
///
/// `AccountLoader` implements all required Anchor traits, allowing it to be used
/// in `#[derive(Accounts)]` structs. During account deserialization
/// (`try_accounts`), no validation is performed - this allows the account to
/// be uninitialized before CPI. Validation happens when `load()` is called.
///
/// # Zero-Copy Behavior
///
/// Unlike borsh deserialization which allocates memory, zero-copy views read
/// and write directly from/to account data. This means:
/// - No memory allocation overhead
/// - Writes via `load_mut()` are immediately reflected in account data
/// - `AccountsExit::exit()` is a no-op since there's nothing to serialize
#[derive(Debug, Clone)]
pub struct AccountLoader<'info, T> {
    info: AccountInfo<'info>,
    _phantom: PhantomData<T>,
}

impl<'info, T> AccountLoader<'info, T> {
    /// Creates a new `AccountLoader` wrapper from an `AccountInfo`.
    ///
    /// This does not perform any validation - the account may be uninitialized.
    /// Validation occurs when `load()` is called.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // In an Anchor instruction handler, after CPI:
    /// let loader = AccountLoader::<Mint>::new(ctx.accounts.mint.to_account_info());
    /// let mint_data = loader.load()?;
    /// assert!(mint_data.is_initialized());
    /// ```
    pub fn new(info: AccountInfo<'info>) -> Self {
        Self {
            info,
            _phantom: PhantomData,
        }
    }

    /// Returns a clone of the underlying `AccountInfo`.
    ///
    /// This is required for macro-generated code that calls `.to_account_info()`
    /// on account fields in `#[derive(LightAccounts)]` structs.
    pub fn to_account_info(&self) -> AccountInfo<'info> {
        self.info.clone()
    }

    /// Returns a reference to the account's public key.
    pub fn key(&self) -> &Pubkey {
        self.info.key
    }
}

// =============================================================================
// Mint-specific methods
// =============================================================================

impl<'info> AccountLoader<'info, Mint> {
    /// Loads and validates the mint data, returning an immutable zero-copy view.
    ///
    /// This method:
    /// 1. Validates the account is owned by the Light Token Program
    /// 2. Creates a zero-copy view into the account data
    /// 3. Validates the mint is initialized and account type is correct
    ///
    /// Each call creates a fresh view - there is no caching.
    ///
    /// # Errors
    ///
    /// Returns `TokenError::InvalidMintOwner` if the account owner is not the Light Token Program.
    /// Returns `TokenError::MintNotInitialized` if the mint is not initialized.
    /// Returns `TokenError::InvalidAccountType` if the account type is not a mint.
    /// Returns `TokenError::MintBorrowFailed` if the account data cannot be borrowed.
    /// Returns `TokenError::MintDeserializationFailed` if zero-copy parsing fails.
    pub fn load(&self) -> std::result::Result<ZMint<'info>, TokenError> {
        // Validate owner
        if self.info.owner != &LIGHT_TOKEN_PROGRAM_ID.into() {
            return Err(TokenError::InvalidMintOwner);
        }

        let data = self
            .info
            .try_borrow_data()
            .map_err(|_| TokenError::MintBorrowFailed)?;

        // Extend lifetime - safe because account data lives for transaction duration.
        // This matches the pattern used in Token::from_account_info_checked.
        let data_slice: &'info [u8] =
            unsafe { core::slice::from_raw_parts(data.as_ptr(), data.len()) };

        let (mint, _) = Mint::zero_copy_at_checked(data_slice)?;
        Ok(mint)
    }

    /// Loads and validates the mint data, returning a mutable zero-copy view.
    ///
    /// This method behaves like `load()` but returns a mutable view,
    /// allowing modifications to the mint data. Since zero-copy writes directly
    /// to account data, changes are immediately persisted.
    ///
    /// # Errors
    ///
    /// Same as `load()`.
    pub fn load_mut(&self) -> std::result::Result<ZMintMut<'info>, TokenError> {
        // Validate owner
        if self.info.owner != &LIGHT_TOKEN_PROGRAM_ID.into() {
            return Err(TokenError::InvalidMintOwner);
        }

        let mut data = self
            .info
            .try_borrow_mut_data()
            .map_err(|_| TokenError::MintBorrowFailed)?;

        // Extend lifetime - safe because account data lives for transaction duration.
        let data_slice: &'info mut [u8] =
            unsafe { core::slice::from_raw_parts_mut(data.as_mut_ptr(), data.len()) };

        let (mint, _) = Mint::zero_copy_at_mut_checked(data_slice)?;
        Ok(mint)
    }

    /// Returns true if the mint account appears to be initialized.
    ///
    /// This performs a quick check without fully parsing the account.
    /// It checks:
    /// 1. Account is owned by the Light Token Program
    /// 2. Account has sufficient data length
    /// 3. The is_initialized byte is non-zero
    pub fn is_initialized(&self) -> bool {
        // Check owner
        if self.info.owner != &LIGHT_TOKEN_PROGRAM_ID.into() {
            return false;
        }

        let data = match self.info.try_borrow_data() {
            Ok(d) => d,
            Err(_) => return false,
        };

        if data.len() <= IS_INITIALIZED_OFFSET {
            return false;
        }

        data[IS_INITIALIZED_OFFSET] != 0
    }
}

// =============================================================================
// Deref, AsRef, and ToAccountInfo implementations
// =============================================================================

impl<'info, T> Deref for AccountLoader<'info, T> {
    type Target = AccountInfo<'info>;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl<'info, T> AsRef<AccountInfo<'info>> for AccountLoader<'info, T> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.info
    }
}

// =============================================================================
// Anchor trait implementations
// =============================================================================

impl<'info, T, B> Accounts<'info, B> for AccountLoader<'info, T> {
    fn try_accounts(
        _program_id: &Pubkey,
        accounts: &mut &'info [AccountInfo<'info>],
        _ix_data: &[u8],
        _bumps: &mut B,
        _reallocs: &mut std::collections::BTreeSet<Pubkey>,
    ) -> Result<Self> {
        // NO validation - just grab AccountInfo
        // This allows the account to be uninitialized before CPI
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = accounts[0].clone();
        *accounts = &accounts[1..];
        Ok(AccountLoader::new(account))
    }
}

impl<'info, T> AccountsExit<'info> for AccountLoader<'info, T> {
    fn exit(&self, _program_id: &Pubkey) -> Result<()> {
        // No-op: zero-copy writes directly to account data
        Ok(())
    }
}

impl<T> ToAccountMetas for AccountLoader<'_, T> {
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.info.is_signer);
        if self.info.is_writable {
            vec![AccountMeta::new(*self.info.key, is_signer)]
        } else {
            vec![AccountMeta::new_readonly(*self.info.key, is_signer)]
        }
    }
}

impl<'info, T> ToAccountInfos<'info> for AccountLoader<'info, T> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.info.clone()]
    }
}

impl<T> anchor_lang::Key for AccountLoader<'_, T> {
    fn key(&self) -> Pubkey {
        *self.info.key
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell as StdRefCell, rc::Rc};

    use solana_pubkey::Pubkey as SolanaPubkey;

    use super::*;

    /// Helper to create a mock AccountInfo for testing
    fn create_mock_account_info<'a>(
        key: &'a SolanaPubkey,
        owner: &'a SolanaPubkey,
        lamports: &'a mut u64,
        data: &'a mut [u8],
        is_writable: bool,
        is_signer: bool,
    ) -> AccountInfo<'a> {
        AccountInfo {
            key,
            lamports: Rc::new(StdRefCell::new(lamports)),
            data: Rc::new(StdRefCell::new(data)),
            owner,
            rent_epoch: 0,
            is_signer,
            is_writable,
            executable: false,
        }
    }

    #[test]
    fn test_account_loader_new() {
        let key = SolanaPubkey::new_unique();
        let owner = SolanaPubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID);
        let mut lamports = 1_000_000u64;
        let mut data = vec![0u8; 256];

        let info = create_mock_account_info(&key, &owner, &mut lamports, &mut data, true, false);

        let loader: AccountLoader<'_, Mint> = AccountLoader::new(info);
        assert_eq!(*loader.key(), key);
    }

    #[test]
    fn test_deref_provides_account_info_access() {
        let key = SolanaPubkey::new_unique();
        let owner = SolanaPubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID);
        let mut lamports = 1_000_000u64;
        let mut data = vec![0u8; 256];

        let info = create_mock_account_info(&key, &owner, &mut lamports, &mut data, true, false);

        let loader: AccountLoader<'_, Mint> = AccountLoader::new(info);

        // Deref should provide access to AccountInfo fields
        assert!(loader.is_writable);
        assert!(!loader.is_signer);
    }

    #[test]
    fn test_load_fails_for_wrong_owner() {
        let key = SolanaPubkey::new_unique();
        let wrong_owner = SolanaPubkey::new_unique(); // Not Light Token Program
        let mut lamports = 1_000_000u64;
        let mut data = vec![0u8; 256];

        let info =
            create_mock_account_info(&key, &wrong_owner, &mut lamports, &mut data, true, false);

        let loader: AccountLoader<'_, Mint> = AccountLoader::new(info);

        let result = loader.load();
        assert!(matches!(result, Err(TokenError::InvalidMintOwner)));
    }

    #[test]
    fn test_load_fails_for_uninitialized() {
        let key = SolanaPubkey::new_unique();
        let owner = SolanaPubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID);
        let mut lamports = 1_000_000u64;
        // Create data with is_initialized = 0 (uninitialized)
        let mut data = vec![0u8; 256];

        let info = create_mock_account_info(&key, &owner, &mut lamports, &mut data, true, false);

        let loader: AccountLoader<'_, Mint> = AccountLoader::new(info);

        let result = loader.load();
        // Will fail during validation
        assert!(result.is_err());
    }

    #[test]
    fn test_to_account_metas_writable() {
        let key = SolanaPubkey::new_unique();
        let owner = SolanaPubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID);
        let mut lamports = 1_000_000u64;
        let mut data = vec![0u8; 256];

        let info = create_mock_account_info(&key, &owner, &mut lamports, &mut data, true, false);

        let loader: AccountLoader<'_, Mint> = AccountLoader::new(info);

        let metas = loader.to_account_metas(None);
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].pubkey, key);
        assert!(metas[0].is_writable);
        assert!(!metas[0].is_signer);
    }

    #[test]
    fn test_to_account_metas_readonly() {
        let key = SolanaPubkey::new_unique();
        let owner = SolanaPubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID);
        let mut lamports = 1_000_000u64;
        let mut data = vec![0u8; 256];

        let info = create_mock_account_info(&key, &owner, &mut lamports, &mut data, false, false);

        let loader: AccountLoader<'_, Mint> = AccountLoader::new(info);

        let metas = loader.to_account_metas(None);
        assert_eq!(metas.len(), 1);
        assert!(!metas[0].is_writable);
    }

    #[test]
    fn test_key_trait() {
        let key = SolanaPubkey::new_unique();
        let owner = SolanaPubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID);
        let mut lamports = 1_000_000u64;
        let mut data = vec![0u8; 256];

        let info = create_mock_account_info(&key, &owner, &mut lamports, &mut data, true, false);

        let loader: AccountLoader<'_, Mint> = AccountLoader::new(info);
        assert_eq!(anchor_lang::Key::key(&loader), key);
    }

    #[test]
    fn test_is_initialized_false_for_wrong_owner() {
        let key = SolanaPubkey::new_unique();
        let wrong_owner = SolanaPubkey::new_unique();
        let mut lamports = 1_000_000u64;
        let mut data = vec![0u8; 256];
        // Set is_initialized byte to 1
        data[IS_INITIALIZED_OFFSET] = 1;

        let info =
            create_mock_account_info(&key, &wrong_owner, &mut lamports, &mut data, true, false);

        let loader: AccountLoader<'_, Mint> = AccountLoader::new(info);
        assert!(!loader.is_initialized());
    }

    #[test]
    fn test_is_initialized_false_for_zero_byte() {
        let key = SolanaPubkey::new_unique();
        let owner = SolanaPubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID);
        let mut lamports = 1_000_000u64;
        let mut data = vec![0u8; 256];
        // is_initialized byte is 0

        let info = create_mock_account_info(&key, &owner, &mut lamports, &mut data, true, false);

        let loader: AccountLoader<'_, Mint> = AccountLoader::new(info);
        assert!(!loader.is_initialized());
    }

    #[test]
    fn test_is_initialized_true() {
        let key = SolanaPubkey::new_unique();
        let owner = SolanaPubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID);
        let mut lamports = 1_000_000u64;
        let mut data = vec![0u8; 256];
        // Set is_initialized byte to 1
        data[IS_INITIALIZED_OFFSET] = 1;

        let info = create_mock_account_info(&key, &owner, &mut lamports, &mut data, true, false);

        let loader: AccountLoader<'_, Mint> = AccountLoader::new(info);
        assert!(loader.is_initialized());
    }
}
