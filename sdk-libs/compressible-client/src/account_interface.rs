//! Unified account interfaces for hot/cold account handling.
//!
//! Mirrors TypeScript SDK patterns:
//! - `AccountInfoInterface` - Generic compressible account (PDAs)
//! - `TokenAccountInterface` - Token accounts (SPL/T22/ctoken)
//! - `AtaInterface` - Associated token accounts
//!
//! All interfaces use standard Solana/SPL types:
//! - `solana_account::Account` for raw account data
//! - `spl_token_2022::state::Account` for parsed token data

use light_client::indexer::{CompressedAccount, CompressedTokenAccount, TreeInfo};
use light_token_interface::state::ExtensionStruct;
use solana_account::Account;
use solana_pubkey::Pubkey;
use spl_token_2022::state::Account as SplTokenAccount;
use thiserror::Error;

/// Error type for account interface operations.
#[derive(Debug, Error)]
pub enum AccountInterfaceError {
    #[error("Account not found")]
    NotFound,

    #[error("Invalid account data")]
    InvalidData,

    #[error("Parse error: {0}")]
    ParseError(String),
}

// ============================================================================
// Decompression Contexts
// ============================================================================

/// Context for decompressing a cold PDA account.
#[derive(Debug, Clone)]
pub struct PdaLoadContext {
    /// Full compressed account from indexer.
    pub compressed: CompressedAccount,
}

impl PdaLoadContext {
    /// Get the compressed account hash (for validity proof).
    #[inline]
    pub fn hash(&self) -> [u8; 32] {
        self.compressed.hash
    }

    /// Get tree info (for proof and instruction building).
    #[inline]
    pub fn tree_info(&self) -> &TreeInfo {
        &self.compressed.tree_info
    }

    /// Get leaf index.
    #[inline]
    pub fn leaf_index(&self) -> u32 {
        self.compressed.leaf_index
    }
}

/// Context for decompressing a cold token account (ATA or other).
#[derive(Debug, Clone)]
pub struct TokenLoadContext {
    /// Full compressed token account from indexer.
    pub compressed: CompressedTokenAccount,
    /// Wallet owner (signer for decompression).
    pub wallet_owner: Pubkey,
    /// Token mint.
    pub mint: Pubkey,
    /// ATA derivation bump (if ATA).
    pub bump: u8,
}

impl TokenLoadContext {
    /// Get the compressed account hash (for validity proof).
    #[inline]
    pub fn hash(&self) -> [u8; 32] {
        self.compressed.account.hash
    }

    /// Get tree info (for proof and instruction building).
    #[inline]
    pub fn tree_info(&self) -> &TreeInfo {
        &self.compressed.account.tree_info
    }

    /// Get leaf index.
    #[inline]
    pub fn leaf_index(&self) -> u32 {
        self.compressed.account.leaf_index
    }
}

// ============================================================================
// AccountInfoInterface - Generic compressible accounts (PDAs)
// ============================================================================

/// Generic account interface for compressible accounts (PDAs).
///
/// Uses standard `solana_account::Account` for raw data.
/// For hot accounts: actual on-chain bytes.
/// For cold accounts: synthetic bytes from compressed data.
#[derive(Debug, Clone)]
pub struct AccountInfoInterface {
    /// The account pubkey.
    pub pubkey: Pubkey,
    /// Raw Solana Account - always present.
    pub account: Account,
    /// Whether this account is compressed (needs decompression).
    pub is_cold: bool,
    /// Load context (only if cold).
    pub load_context: Option<PdaLoadContext>,
}

impl AccountInfoInterface {
    /// Create a hot (on-chain) account interface.
    pub fn hot(pubkey: Pubkey, account: Account) -> Self {
        Self {
            pubkey,
            account,
            is_cold: false,
            load_context: None,
        }
    }

    /// Create a cold (compressed) account interface.
    pub fn cold(pubkey: Pubkey, compressed: CompressedAccount, owner: Pubkey) -> Self {
        // Synthesize Account from compressed data
        let data = compressed
            .data
            .as_ref()
            .map(|d| {
                let mut buf = d.discriminator.to_vec();
                buf.extend_from_slice(&d.data);
                buf
            })
            .unwrap_or_default();

        let account = Account {
            lamports: compressed.lamports,
            data,
            owner,
            executable: false,
            rent_epoch: 0,
        };

        Self {
            pubkey,
            account,
            is_cold: true,
            load_context: Some(PdaLoadContext { compressed }),
        }
    }

    /// Get the compressed account hash if cold (for validity proof).
    pub fn hash(&self) -> Option<[u8; 32]> {
        self.load_context.as_ref().map(|ctx| ctx.hash())
    }

    /// Get the raw account data bytes.
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.account.data
    }
}

// ============================================================================
// TokenAccountInterface - Token accounts (SPL/T22/ctoken)
// ============================================================================

/// Token account interface with both raw and parsed data.
///
/// Uses standard types:
/// - `solana_account::Account` for raw bytes
/// - `spl_token_2022::state::Account` for parsed token data
#[derive(Debug, Clone)]
pub struct TokenAccountInterface {
    /// The token account pubkey.
    pub pubkey: Pubkey,
    /// Raw Solana Account - always present.
    pub account: Account,
    /// Parsed SPL Token Account - standard type.
    pub parsed: SplTokenAccount,
    /// Whether this account is compressed (needs decompression).
    pub is_cold: bool,
    /// Load context (only if cold).
    pub load_context: Option<TokenLoadContext>,
    /// Optional TLV extension data (compressed token extensions).
    pub extensions: Option<Vec<ExtensionStruct>>,
}

impl TokenAccountInterface {
    /// Create a hot (on-chain) token account interface.
    pub fn hot(pubkey: Pubkey, account: Account) -> Result<Self, AccountInterfaceError> {
        use solana_program::program_pack::Pack;

        if account.data.len() < SplTokenAccount::LEN {
            return Err(AccountInterfaceError::InvalidData);
        }

        let parsed = SplTokenAccount::unpack(&account.data[..SplTokenAccount::LEN])
            .map_err(|e| AccountInterfaceError::ParseError(e.to_string()))?;

        Ok(Self {
            pubkey,
            account,
            parsed,
            is_cold: false,
            load_context: None,
            extensions: None, // Hot accounts don't have compressed extensions
        })
    }

    /// Create a cold (compressed) token account interface.
    pub fn cold(
        pubkey: Pubkey,
        compressed: CompressedTokenAccount,
        wallet_owner: Pubkey,
        mint: Pubkey,
        bump: u8,
        program_owner: Pubkey,
    ) -> Self {
        use light_token_sdk::compat::AccountState;
        use solana_program::program_pack::Pack;

        let token = &compressed.token;

        // Create SPL Token Account from TokenData
        let parsed = SplTokenAccount {
            mint: token.mint,
            owner: token.owner,
            amount: token.amount,
            delegate: token.delegate.into(),
            state: match token.state {
                AccountState::Frozen => spl_token_2022::state::AccountState::Frozen,
                _ => spl_token_2022::state::AccountState::Initialized,
            },
            is_native: solana_program::program_option::COption::None,
            delegated_amount: 0,
            close_authority: solana_program::program_option::COption::None,
        };

        // Pack into synthetic Account bytes (165 bytes SPL Token Account format)
        let mut data = vec![0u8; SplTokenAccount::LEN];
        SplTokenAccount::pack(parsed, &mut data).expect("pack should never fail");

        // Store extensions separately (not appended to data - they're compressed-specific)
        let extensions = token.tlv.clone();

        let account = Account {
            lamports: compressed.account.lamports,
            data,
            owner: program_owner,
            executable: false,
            rent_epoch: 0,
        };

        Self {
            pubkey,
            account,
            parsed,
            is_cold: true,
            load_context: Some(TokenLoadContext {
                compressed,
                wallet_owner,
                mint,
                bump,
            }),
            extensions,
        }
    }

    /// Convenience: get amount.
    #[inline]
    pub fn amount(&self) -> u64 {
        self.parsed.amount
    }

    /// Convenience: get delegate.
    #[inline]
    pub fn delegate(&self) -> Option<Pubkey> {
        self.parsed.delegate.into()
    }

    /// Convenience: get mint.
    #[inline]
    pub fn mint(&self) -> Pubkey {
        self.parsed.mint
    }

    /// Convenience: get owner.
    #[inline]
    pub fn owner(&self) -> Pubkey {
        self.parsed.owner
    }

    /// Convenience: check if frozen.
    #[inline]
    pub fn is_frozen(&self) -> bool {
        self.parsed.state == spl_token_2022::state::AccountState::Frozen
    }

    /// Get the compressed account hash if cold (for validity proof).
    pub fn hash(&self) -> Option<[u8; 32]> {
        self.load_context.as_ref().map(|ctx| ctx.hash())
    }
}

// ============================================================================
// AtaInterface - Associated Token Accounts
// ============================================================================

/// Associated token account interface.
///
/// Wraps `TokenAccountInterface` with ATA-specific marker.
/// The owner and mint are available via `parsed.owner` and `parsed.mint`.
#[derive(Debug, Clone)]
pub struct AtaInterface {
    /// Inner token account interface.
    pub inner: TokenAccountInterface,
}

impl AtaInterface {
    /// Create from TokenAccountInterface.
    pub fn new(inner: TokenAccountInterface) -> Self {
        Self { inner }
    }

    /// The ATA pubkey.
    #[inline]
    pub fn pubkey(&self) -> Pubkey {
        self.inner.pubkey
    }

    /// Raw Solana Account.
    #[inline]
    pub fn account(&self) -> &Account {
        &self.inner.account
    }

    /// Parsed SPL Token Account.
    #[inline]
    pub fn parsed(&self) -> &SplTokenAccount {
        &self.inner.parsed
    }

    /// Whether compressed.
    #[inline]
    pub fn is_cold(&self) -> bool {
        self.inner.is_cold
    }

    /// Load context for decompression.
    #[inline]
    pub fn load_context(&self) -> Option<&TokenLoadContext> {
        self.inner.load_context.as_ref()
    }

    /// Amount.
    #[inline]
    pub fn amount(&self) -> u64 {
        self.inner.amount()
    }

    /// Mint.
    #[inline]
    pub fn mint(&self) -> Pubkey {
        self.inner.mint()
    }

    /// Owner (wallet that owns this ATA).
    #[inline]
    pub fn owner(&self) -> Pubkey {
        self.inner.owner()
    }

    /// Hash for validity proof.
    pub fn hash(&self) -> Option<[u8; 32]> {
        self.inner.hash()
    }
}

impl std::ops::Deref for AtaInterface {
    type Target = TokenAccountInterface;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
