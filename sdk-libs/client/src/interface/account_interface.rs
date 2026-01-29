//! Unified account interfaces for hot/cold account handling.
//!
//! Core types:
//! - `AccountInterface` - Generic account (PDAs, mints)
//! - `TokenAccountInterface` - Token accounts (ATAs, program-owned vaults)
//!
//! All interfaces use standard Solana/SPL types:
//! - `solana_account::Account` for raw account data
//! - `spl_token_2022_interface::pod::PodAccount` for parsed token data

use light_token::instruction::derive_token_ata;
use light_token_interface::state::ExtensionStruct;
use solana_account::Account;
use solana_pubkey::Pubkey;
use spl_pod::{
    bytemuck::{pod_bytes_of, pod_from_bytes, pod_get_packed_len},
    primitives::PodU64,
};
use spl_token_2022_interface::{
    pod::{PodAccount, PodCOption},
    state::AccountState,
};
use thiserror::Error;

use super::ColdContext;
use crate::indexer::{CompressedAccount, CompressedTokenAccount, TreeInfo};

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
            Some(ColdContext::Mint(ca)) => {
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

    /// Get mint address if this is a cold mint.
    pub fn mint_compressed_address(&self) -> Option<[u8; 32]> {
        self.as_mint().map(|m| m.metadata.compressed_address())
    }
}

/// Token account interface with both raw and parsed data.
///
/// Uses standard types:
/// - `solana_account::Account` for raw bytes
/// - `spl_token_2022_interface::pod::PodAccount` for parsed token data
///
/// For ATAs: `parsed.owner` is the wallet owner (set from fetch params).
/// For program-owned: `parsed.owner` is the PDA.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TokenAccountInterface {
    /// The token account's public key.
    pub key: Pubkey,
    /// Standard Solana Account (lamports, data, owner, executable, rent_epoch).
    pub account: Account,
    /// Parsed SPL Token Account (POD format).
    pub parsed: PodAccount,
    /// Cold context (only present when cold).
    pub cold: Option<ColdContext>,
    /// Optional TLV extension data.
    pub extensions: Option<Vec<ExtensionStruct>>,
}

impl TokenAccountInterface {
    /// Create a hot (on-chain) token account interface.
    pub fn hot(key: Pubkey, account: Account) -> Result<Self, AccountInterfaceError> {
        let pod_len = pod_get_packed_len::<PodAccount>();
        if account.data.len() < pod_len {
            return Err(AccountInterfaceError::InvalidData);
        }

        let parsed: &PodAccount = pod_from_bytes(&account.data[..pod_len])
            .map_err(|e| AccountInterfaceError::ParseError(e.to_string()))?;

        Ok(Self {
            key,
            parsed: *parsed,
            account,
            cold: None,
            extensions: None,
        })
    }

    /// Create a cold token account interface.
    ///
    /// # Arguments
    /// * `key` - The token account address
    /// * `compressed` - The cold token account from indexer
    /// * `owner_override` - For ATAs, pass the wallet owner. For program-owned, pass the PDA.
    /// * `program_owner` - The program that owns this account (usually LIGHT_TOKEN_PROGRAM_ID)
    pub fn cold(
        key: Pubkey,
        compressed: CompressedTokenAccount,
        owner_override: Pubkey,
        program_owner: Pubkey,
    ) -> Self {
        use light_token::compat::AccountState as LightAccountState;

        let token = &compressed.token;

        let parsed = PodAccount {
            mint: token.mint,
            owner: owner_override,
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

        let extensions = token.tlv.clone();

        let account = Account {
            lamports: compressed.account.lamports,
            data,
            owner: program_owner,
            executable: false,
            rent_epoch: 0,
        };

        Self {
            key,
            account,
            parsed,
            cold: Some(ColdContext::Token(compressed)),
            extensions,
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

    /// Get the CompressedTokenAccount if cold.
    pub fn compressed(&self) -> Option<&CompressedTokenAccount> {
        match &self.cold {
            Some(ColdContext::Token(c)) => Some(c),
            _ => None,
        }
    }

    /// Get amount.
    #[inline]
    pub fn amount(&self) -> u64 {
        u64::from(self.parsed.amount)
    }

    /// Get delegate.
    #[inline]
    pub fn delegate(&self) -> Option<Pubkey> {
        if self.parsed.delegate.is_some() {
            Some(self.parsed.delegate.value)
        } else {
            None
        }
    }

    /// Get mint.
    #[inline]
    pub fn mint(&self) -> Pubkey {
        self.parsed.mint
    }

    /// Get owner (wallet for ATAs, PDA for program-owned).
    #[inline]
    pub fn owner(&self) -> Pubkey {
        self.parsed.owner
    }

    /// Check if frozen.
    #[inline]
    pub fn is_frozen(&self) -> bool {
        self.parsed.state == AccountState::Frozen as u8
    }

    /// Get the account hash if cold.
    #[inline]
    pub fn hash(&self) -> Option<[u8; 32]> {
        self.compressed().map(|c| c.account.hash)
    }

    /// Get tree info if cold.
    #[inline]
    pub fn tree_info(&self) -> Option<&TreeInfo> {
        self.compressed().map(|c| &c.account.tree_info)
    }

    /// Get leaf index if cold.
    #[inline]
    pub fn leaf_index(&self) -> Option<u32> {
        self.compressed().map(|c| c.account.leaf_index)
    }

    /// Get ATA bump if this is an ATA. Returns None if not a valid ATA derivation.
    pub fn ata_bump(&self) -> Option<u8> {
        let (derived_ata, bump) = derive_token_ata(&self.parsed.owner, &self.parsed.mint);
        (derived_ata == self.key).then_some(bump)
    }

    /// Check if this token account is an ATA (derivation matches).
    pub fn is_ata(&self) -> bool {
        self.ata_bump().is_some()
    }
}

impl From<TokenAccountInterface> for AccountInterface {
    fn from(tai: TokenAccountInterface) -> Self {
        Self {
            key: tai.key,
            account: tai.account,
            cold: tai.cold,
        }
    }
}
