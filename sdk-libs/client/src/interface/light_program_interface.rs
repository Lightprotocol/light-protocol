//! LightProgramInterface trait and supporting types for client-side SDK patterns.
//!
//! Core types:
//! - `ColdContext` - Cold data context (Account or Token)
//! - `PdaSpec` - Spec for PDA loading with typed variant
//! - `AccountSpec` - Unified spec enum for load instruction building
//! - `LightProgramInterface` - Trait for program SDKs

use std::fmt::Debug;

use light_sdk::interface::Pack;
use light_token_sdk::token::derive_token_ata;
use solana_pubkey::Pubkey;

use super::{AccountInterface, TokenAccountInterface};
use crate::indexer::{CompressedAccount, CompressedTokenAccount};

/// Account descriptor for fetching. Routes to the correct indexer endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AccountToFetch {
    /// PDA account - uses `get_account_interface(address, program_id)`
    Pda { address: Pubkey, program_id: Pubkey },
    /// Token account (program-owned) - uses `get_token_account_interface(address)`
    Token { address: Pubkey },
    /// ATA - uses `get_ata_interface(wallet_owner, mint)`
    Ata { wallet_owner: Pubkey, mint: Pubkey },
    /// Light mint - uses `get_mint_interface(address)`
    Mint { address: Pubkey },
}

impl AccountToFetch {
    pub fn pda(address: Pubkey, program_id: Pubkey) -> Self {
        Self::Pda {
            address,
            program_id,
        }
    }

    pub fn token(address: Pubkey) -> Self {
        Self::Token { address }
    }

    pub fn ata(wallet_owner: Pubkey, mint: Pubkey) -> Self {
        Self::Ata { wallet_owner, mint }
    }

    pub fn mint(address: Pubkey) -> Self {
        Self::Mint { address }
    }

    #[must_use]
    pub fn pubkey(&self) -> Pubkey {
        match self {
            Self::Pda { address, .. } => *address,
            Self::Token { address } => *address,
            Self::Ata { wallet_owner, mint } => derive_token_ata(wallet_owner, mint).0,
            Self::Mint { address } => *address,
        }
    }
}

/// Context for cold accounts.
///
/// Two variants based on data structure, not account type:
/// - `Account` - PDA
/// - `Token` - Token account
#[derive(Clone, Debug)]
pub enum ColdContext {
    /// PDA
    Account(CompressedAccount),
    /// Token account
    Token(CompressedTokenAccount),
}

/// Specification for a program-owned PDA with typed variant.
///
/// Embeds `AccountInterface` for account data and adds `variant` for typed variant.
#[derive(Clone, Debug)]
pub struct PdaSpec<V> {
    /// The account interface.
    pub interface: AccountInterface,
    /// The typed variant with all seed values populated.
    pub variant: V,
    /// The program owner to call for loading the account.
    pub program_id: Pubkey,
}

impl<V> PdaSpec<V> {
    /// Create a new PdaSpec from an interface, variant, and program owner.
    #[must_use]
    pub fn new(interface: AccountInterface, variant: V, program_id: Pubkey) -> Self {
        Self {
            interface,
            variant,
            program_id,
        }
    }

    /// The account's public key.
    #[inline]
    #[must_use]
    pub fn address(&self) -> Pubkey {
        self.interface.key
    }

    /// The program owner to call for loading the account.
    #[inline]
    #[must_use]
    pub fn program_id(&self) -> Pubkey {
        self.program_id
    }

    /// Whether this account is cold and must be loaded.
    #[inline]
    #[must_use]
    pub fn is_cold(&self) -> bool {
        self.interface.is_cold()
    }

    /// Whether this account is hot and will not be loaded.
    #[inline]
    #[must_use]
    pub fn is_hot(&self) -> bool {
        self.interface.is_hot()
    }

    /// Get the compressed account if cold.
    #[must_use]
    pub fn compressed(&self) -> Option<&CompressedAccount> {
        self.interface.as_compressed_account()
    }

    /// Get the cold account hash.
    #[must_use]
    pub fn hash(&self) -> Option<[u8; 32]> {
        self.interface.hash()
    }

    /// Get account data bytes.
    #[inline]
    #[must_use]
    pub fn data(&self) -> &[u8] {
        self.interface.data()
    }
}

/// Account specification for loading cold accounts.
#[derive(Clone, Debug)]
pub enum AccountSpec<V> {
    /// Program-owned PDA with typed variant.
    Pda(PdaSpec<V>),
    /// Associated token account
    Ata(TokenAccountInterface),
    /// Light token mint
    Mint(AccountInterface),
}

impl<V> AccountSpec<V> {
    #[inline]
    #[must_use]
    pub fn is_cold(&self) -> bool {
        match self {
            Self::Pda(s) => s.is_cold(),
            Self::Ata(s) => s.is_cold(),
            Self::Mint(s) => s.is_cold(),
        }
    }

    #[inline]
    #[must_use]
    pub fn is_hot(&self) -> bool {
        !self.is_cold()
    }

    #[must_use]
    pub fn pubkey(&self) -> Pubkey {
        match self {
            Self::Pda(s) => s.address(),
            Self::Ata(s) => s.key,
            Self::Mint(s) => s.key,
        }
    }
}

impl<V> From<PdaSpec<V>> for AccountSpec<V> {
    fn from(spec: PdaSpec<V>) -> Self {
        Self::Pda(spec)
    }
}

impl From<TokenAccountInterface> for AccountSpec<()> {
    fn from(interface: TokenAccountInterface) -> Self {
        Self::Ata(interface)
    }
}

impl From<AccountInterface> for AccountSpec<()> {
    fn from(interface: AccountInterface) -> Self {
        Self::Mint(interface)
    }
}

/// Check if any specs in the slice are cold.
#[inline]
#[must_use]
pub fn any_cold<V>(specs: &[AccountSpec<V>]) -> bool {
    specs.iter().any(|s| s.is_cold())
}

/// Check if all specs in the slice are hot.
#[inline]
#[must_use]
pub fn all_hot<V>(specs: &[AccountSpec<V>]) -> bool {
    specs.iter().all(|s| s.is_hot())
}

/// Trait for programs to give clients a unified API to load cold program accounts.
pub trait LightProgramInterface: Sized {
    /// The program's interface account variant enum.
    type Variant: Pack + Clone + Debug;

    /// Program-specific instruction enum.
    type Instruction;

    /// Error type for SDK operations.
    type Error: std::error::Error;

    /// The program ID.
    #[must_use]
    fn program_id(&self) -> Pubkey;

    /// Construct SDK from root account(s).
    fn from_keyed_accounts(accounts: &[AccountInterface]) -> Result<Self, Self::Error>;

    /// Returns pubkeys of accounts needed for an instruction.
    #[must_use]
    fn get_accounts_to_update(&self, ix: &Self::Instruction) -> Vec<AccountToFetch>;

    /// Update internal cache with fetched account data.
    fn update(&mut self, accounts: &[AccountInterface]) -> Result<(), Self::Error>;

    /// Get all cached specs.
    #[must_use]
    fn get_all_specs(&self) -> Vec<AccountSpec<Self::Variant>>;

    /// Get specs filtered for a specific instruction.
    #[must_use]
    fn get_specs_for_instruction(&self, ix: &Self::Instruction) -> Vec<AccountSpec<Self::Variant>>;
}
