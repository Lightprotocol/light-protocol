//! Unified account interface for hot/cold account handling.
//!
//! Single type: `AccountInterface` - works for PDAs, mints, ATAs.
//! For hot accounts: real on-chain data.
//! For cold accounts: synthetic data from Photon + compressed account metadata.

use borsh::BorshDeserialize;
use light_sdk_types::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR;
use solana_account::Account;
use solana_pubkey::Pubkey;

use crate::indexer::{CompressedAccount, CompressedTokenAccount, TreeInfo};

/// C_TOKEN_DISCRIMINATOR_V2: batched Merkle trees.
const C_TOKEN_V2: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 3];
/// C_TOKEN_DISCRIMINATOR_V3: SHA256 flat hash with TLV extensions.
const C_TOKEN_V3: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 4];

/// Unified account interface for PDAs, mints, and tokens.
///
/// `account` contains usable data bytes in both hot and cold cases:
/// - Hot: actual on-chain bytes
/// - Cold: synthetic bytes from Photon (SPL layout for tokens, disc+data for PDAs)
///
/// `cold` contains the raw compressed account(s) when cold, needed for proof generation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AccountInterface {
    /// The account's public key.
    pub key: Pubkey,
    /// Standard Solana Account (lamports, data, owner, executable, rent_epoch).
    pub account: Account,
    /// Compressed accounts when cold (None = hot).
    pub cold: Option<Vec<CompressedAccount>>,
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

    /// Create a cold account interface from compressed accounts and synthetic account data.
    pub fn cold(key: Pubkey, account: Account, compressed: Vec<CompressedAccount>) -> Self {
        Self {
            key,
            account,
            cold: Some(compressed),
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

    /// Get data bytes (works for both hot and cold).
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.account.data
    }

    /// Get the primary compressed account (first in the cold vec).
    pub fn compressed(&self) -> Option<&CompressedAccount> {
        self.cold.as_ref().and_then(|v| v.first())
    }

    /// Get all compressed accounts.
    pub fn compressed_accounts(&self) -> Option<&[CompressedAccount]> {
        self.cold.as_deref()
    }

    /// Get the account hash if cold.
    pub fn hash(&self) -> Option<[u8; 32]> {
        self.compressed().map(|c| c.hash)
    }

    /// Get tree info if cold.
    pub fn tree_info(&self) -> Option<&TreeInfo> {
        self.compressed().map(|c| &c.tree_info)
    }

    /// Get leaf index if cold.
    pub fn leaf_index(&self) -> Option<u32> {
        self.compressed().map(|c| c.leaf_index)
    }

    /// Parse as CompressedTokenAccount if the primary compressed account is a token.
    ///
    /// Token detection: owner == LIGHT_TOKEN_PROGRAM_ID and c_token discriminator.
    /// Token data is borsh-deserialized from the compressed account data.
    pub fn as_compressed_token(&self) -> Option<CompressedTokenAccount> {
        let compressed = self.compressed()?;
        let data = compressed.data.as_ref()?;

        if compressed.owner != light_token::instruction::LIGHT_TOKEN_PROGRAM_ID {
            return None;
        }
        if !is_c_token_discriminator(&data.discriminator) {
            return None;
        }

        let token = light_token::compat::TokenData::deserialize(&mut data.data.as_slice()).ok()?;
        Some(CompressedTokenAccount {
            token,
            account: compressed.clone(),
        })
    }

    /// Try to parse as Mint. Returns None if not a mint or parse fails.
    pub fn as_mint(&self) -> Option<light_token_interface::state::Mint> {
        let compressed = self.compressed()?;
        let data = compressed.data.as_ref()?;
        BorshDeserialize::deserialize(&mut data.data.as_slice()).ok()
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

/// Check if a discriminator is a c_token discriminator (V1, V2, or V3).
fn is_c_token_discriminator(disc: &[u8; 8]) -> bool {
    *disc == TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR || *disc == C_TOKEN_V2 || *disc == C_TOKEN_V3
}
