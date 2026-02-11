//! LightProgramInterface trait and supporting types for client-side cold account handling.
//!
//! Core types:
//! - `ColdContext` - Cold data context (Account or Token)
//! - `PdaSpec` - Spec for PDA loading with typed variant
//! - `AccountSpec` - Unified spec enum for load instruction building
//! - `LightProgramInterface` - Trait for program SDKs

use std::fmt::Debug;

use light_account::Pack;
use solana_pubkey::Pubkey;

use super::{AccountInterface, TokenAccountInterface};
use crate::indexer::{CompressedAccount, CompressedTokenAccount};

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

    /// Get the compressed account if cold (handles both Account and Token cold contexts).
    #[must_use]
    pub fn compressed(&self) -> Option<&CompressedAccount> {
        match &self.interface.cold {
            Some(ColdContext::Account(c)) => Some(c),
            Some(ColdContext::Token(c)) => Some(&c.account),
            Some(ColdContext::Mint(c)) => Some(c),
            None => None,
        }
    }

    /// Get the compressed token account if this is a cold token PDA.
    #[must_use]
    pub fn compressed_token(&self) -> Option<&CompressedTokenAccount> {
        self.interface.as_compressed_token()
    }

    /// Whether this spec is for a token PDA (cold context is Token variant).
    #[must_use]
    pub fn is_token_pda(&self) -> bool {
        self.interface.as_compressed_token().is_some()
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

/// Trait for program SDKs to produce load specs for cold accounts.
///
/// Implementors hold parsed program state (e.g., pool config, vault addresses,
/// seed values). The trait provides two methods:
/// - `instruction_accounts`: which pubkeys does this instruction reference?
/// - `load_specs`: given cold AccountInterfaces, build AccountSpec with variants.
///
/// The caller handles construction, caching, and cold detection.
/// The trait only maps cold accounts to their variants for `create_load_instructions`.
pub trait LightProgramInterface: Sized {
    /// The program's account variant enum (macro-generated, carries PDA seeds).
    type Variant: Pack<solana_instruction::AccountMeta> + Clone + Debug;

    /// Program-specific instruction enum.
    type Instruction;

    /// The program ID.
    fn program_id() -> Pubkey;

    /// Which compressible account pubkeys does this instruction reference?
    /// Used by callers to check which accounts might need loading.
    #[must_use]
    fn instruction_accounts(&self, ix: &Self::Instruction) -> Vec<Pubkey>;

    /// Build AccountSpec for cold accounts.
    /// Matches each AccountInterface by pubkey, constructs the variant (seeds)
    /// from internal parsed state, wraps in PdaSpec/AccountSpec.
    /// Only called on the cold path.
    fn load_specs(
        &self,
        cold_accounts: &[AccountInterface],
    ) -> Result<Vec<AccountSpec<Self::Variant>>, Box<dyn std::error::Error>>;
}

/// Extract 8-byte discriminator from account data.
#[inline]
#[must_use]
pub fn discriminator(data: &[u8]) -> Option<[u8; 8]> {
    data.get(..8).and_then(|s| s.try_into().ok())
}

/// Check if account data matches a discriminator.
#[inline]
#[must_use]
pub fn matches_discriminator(data: &[u8], disc: &[u8; 8]) -> bool {
    discriminator(data) == Some(*disc)
}
