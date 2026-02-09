//! Unified account interfaces for hot/cold account handling.
//!
//! Core type: `AccountInterface` - Generic account (PDAs, mints, ATAs).
//! Consumers parse `account.data` (SPL layout) for hot or cold.
//!
//! All interfaces use standard Solana/SPL types:
//! - `solana_account::Account` for raw account data
//! - `spl_token_2022_interface::pod::PodAccount` for parsed token data

use solana_account::Account;
use solana_pubkey::Pubkey;
use spl_pod::{bytemuck::pod_bytes_of, primitives::PodU64};
use spl_token_2022_interface::{
    pod::{PodAccount, PodCOption},
    state::AccountState,
};
use thiserror::Error;

use crate::indexer::{CompressedAccount, CompressedTokenAccount, TreeInfo};

/// Context for cold accounts.
///
/// Three variants based on data structure:
/// - `Account` - Generic PDA
/// - `Token` - Token account
/// - `Mint` - Compressed mint
#[derive(Clone, Debug, PartialEq)]
pub enum ColdContext {
    /// Generic PDA
    Account(CompressedAccount),
    /// Token account
    Token(CompressedTokenAccount),
    /// Compressed mint
    Mint(CompressedAccount),
}

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

/// Unified account interface for PDAs, mints, and tokens.
///
/// Uses standard `solana_account::Account` for raw data.
/// For hot accounts: actual on-chain bytes.
/// For cold accounts: synthetic bytes from cold data.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AccountInterface {
    /// The account's public key.
    pub key: Pubkey,
    /// Standard Solana Account (lamports, data, owner, executable, rent_epoch).
    pub account: Account,
    /// Cold context (only present when cold).
    pub cold: Option<ColdContext>,
}

impl AccountInterface {
    /// Create a hot (on-chain) account interface.
    pub fn hot(key: Pubkey, account: Account) -> Self {
        Self {
            key,
            account,
            cold: None,
        }
    }

    /// Create a cold account interface for a PDA/mint.
    pub fn cold(key: Pubkey, compressed: CompressedAccount, owner: Pubkey) -> Self {
        let data = compressed
            .data
            .as_ref()
            .map(|d| {
                let mut buf = d.discriminator.to_vec();
                buf.extend_from_slice(&d.data);
                buf
            })
            .unwrap_or_default();

        Self {
            key,
            account: Account {
                lamports: compressed.lamports,
                data,
                owner,
                executable: false,
                rent_epoch: 0,
            },
            cold: Some(ColdContext::Account(compressed)),
        }
    }

    /// Create a cold account interface for a token account.
    pub fn cold_token(
        key: Pubkey,
        compressed: CompressedTokenAccount,
        wallet_owner: Pubkey,
    ) -> Self {
        use light_token::compat::AccountState as LightAccountState;

        let token = &compressed.token;
        let parsed = PodAccount {
            mint: token.mint,
            owner: wallet_owner,
            amount: PodU64::from(token.amount),
            delegate: match token.delegate {
                Some(pk) => PodCOption::some(pk),
                None => PodCOption::none(),
            },
            state: match token.state {
                LightAccountState::Frozen => AccountState::Frozen as u8,
                _ => AccountState::Initialized as u8,
            },
            is_native: PodCOption::none(),
            delegated_amount: PodU64::from(0u64),
            close_authority: PodCOption::none(),
        };
        let data = pod_bytes_of(&parsed).to_vec();

        Self {
            key,
            account: Account {
                lamports: compressed.account.lamports,
                data,
                owner: light_token::instruction::LIGHT_TOKEN_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
            cold: Some(ColdContext::Token(compressed)),
        }
    }

    /// Whether this account is cold.
    #[inline]
    pub fn is_cold(&self) -> bool {
        self.cold.is_some()
    }

    /// Whether this account is hot.
    #[inline]
    pub fn is_hot(&self) -> bool {
        self.cold.is_none()
    }

    /// Get data bytes.
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.account.data
    }

    /// Get the account hash if cold.
    pub fn hash(&self) -> Option<[u8; 32]> {
        match &self.cold {
            Some(ColdContext::Account(c)) => Some(c.hash),
            Some(ColdContext::Token(c)) => Some(c.account.hash),
            Some(ColdContext::Mint(c)) => Some(c.hash),
            None => None,
        }
    }

    /// Get tree info if cold.
    pub fn tree_info(&self) -> Option<&TreeInfo> {
        match &self.cold {
            Some(ColdContext::Account(c)) => Some(&c.tree_info),
            Some(ColdContext::Token(c)) => Some(&c.account.tree_info),
            Some(ColdContext::Mint(c)) => Some(&c.tree_info),
            None => None,
        }
    }

    /// Get leaf index if cold.
    pub fn leaf_index(&self) -> Option<u32> {
        match &self.cold {
            Some(ColdContext::Account(c)) => Some(c.leaf_index),
            Some(ColdContext::Token(c)) => Some(c.account.leaf_index),
            Some(ColdContext::Mint(c)) => Some(c.leaf_index),
            None => None,
        }
    }

    /// Get as CompressedAccount if cold account or mint type.
    pub fn as_compressed_account(&self) -> Option<&CompressedAccount> {
        match &self.cold {
            Some(ColdContext::Account(c)) => Some(c),
            Some(ColdContext::Mint(c)) => Some(c),
            _ => None,
        }
    }

    /// Get as CompressedTokenAccount if cold token type.
    pub fn as_compressed_token(&self) -> Option<&CompressedTokenAccount> {
        match &self.cold {
            Some(ColdContext::Token(c)) => Some(c),
            _ => None,
        }
    }

    /// Try to parse as Mint. Returns None if not a mint or parse fails.
    pub fn as_mint(&self) -> Option<light_token_interface::state::Mint> {
        match &self.cold {
            Some(ColdContext::Mint(ca)) | Some(ColdContext::Account(ca)) => {
                let data = ca.data.as_ref()?;
                borsh::BorshDeserialize::deserialize(&mut data.data.as_slice()).ok()
            }
            _ => None,
        }
    }

    /// Get mint signer if this is a cold mint.
    pub fn mint_signer(&self) -> Option<[u8; 32]> {
        self.as_mint().map(|m| m.metadata.mint_signer)
    }

    /// Get mint compressed address if this is a cold mint.
    pub fn mint_compressed_address(&self) -> Option<[u8; 32]> {
        self.as_mint().map(|m| m.metadata.compressed_address())
    }
}
