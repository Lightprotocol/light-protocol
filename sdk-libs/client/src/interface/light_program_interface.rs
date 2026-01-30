//! LightProgramInterface trait and supporting types for client-side SDK patterns.
//!
//! Core types:
//! - `ColdContext` - Cold data context (Account or Token)
//! - `ColdAccountSpec` - Lean cold spec without redundant Account struct
//! - `PdaSpec` - Spec for PDA loading with typed variant
//! - `AccountSpec` - Unified spec enum for load instruction building
//! - `LightProgramInterface` - Base trait for program SDKs with per-instruction granularity
//! - `LightAmmInterface` - Extension trait for swap-focused AMM SDKs

use std::fmt::Debug;

use light_sdk::interface::Pack;
use light_token::instruction::derive_token_ata;
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

/// Lean cold account spec - NO redundant `Account` struct.
///
/// This is the internal storage format for cold accounts in SDKs.
/// Unlike `AccountSpec` which wraps `AccountInterface`, this only stores
/// what's actually needed for building load instructions.
#[derive(Clone, Debug)]
pub enum ColdAccountSpec<V> {
    /// Program-owned PDA - needs Variant for pack()
    Pda {
        key: Pubkey,
        compressed: CompressedAccount,
        variant: V,
        program_id: Pubkey,
    },
    /// Program-owned token account (vault)
    Token {
        key: Pubkey,
        compressed: CompressedTokenAccount,
    },
    /// Compressed mint
    Mint {
        key: Pubkey,
        compressed: CompressedAccount,
    },
}

impl<V> ColdAccountSpec<V> {
    /// Create a PDA spec from AccountInterface + variant.
    pub fn from_pda_interface(
        interface: &AccountInterface,
        variant: V,
        program_id: Pubkey,
    ) -> Option<Self> {
        let compressed = interface.as_compressed_account()?.clone();
        Some(Self::Pda {
            key: interface.key,
            compressed,
            variant,
            program_id,
        })
    }

    /// Create a Token spec from AccountInterface.
    pub fn from_token_interface(interface: &AccountInterface) -> Option<Self> {
        let compressed = interface.as_compressed_token()?.clone();
        Some(Self::Token {
            key: interface.key,
            compressed,
        })
    }

    /// Create a Mint spec from AccountInterface.
    pub fn from_mint_interface(interface: &AccountInterface) -> Option<Self> {
        let compressed = interface.as_compressed_account()?.clone();
        Some(Self::Mint {
            key: interface.key,
            compressed,
        })
    }

    /// Get the account's public key.
    #[must_use]
    pub fn key(&self) -> Pubkey {
        match self {
            Self::Pda { key, .. } => *key,
            Self::Token { key, .. } => *key,
            Self::Mint { key, .. } => *key,
        }
    }

    /// Get the account hash (for proof fetching).
    #[must_use]
    pub fn hash(&self) -> [u8; 32] {
        match self {
            Self::Pda { compressed, .. } => compressed.hash,
            Self::Token { compressed, .. } => compressed.account.hash,
            Self::Mint { compressed, .. } => compressed.hash,
        }
    }

    /// Get the compressed account (for PDAs/mints).
    #[must_use]
    pub fn compressed_account(&self) -> Option<&CompressedAccount> {
        match self {
            Self::Pda { compressed, .. } | Self::Mint { compressed, .. } => Some(compressed),
            Self::Token { .. } => None,
        }
    }

    /// Get the compressed token account (for tokens).
    #[must_use]
    pub fn compressed_token(&self) -> Option<&CompressedTokenAccount> {
        match self {
            Self::Token { compressed, .. } => Some(compressed),
            _ => None,
        }
    }
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

/// Base trait for programs with compressible accounts.
///
/// Provides per-instruction granularity for account discovery and cold spec retrieval.
/// Programs implement this trait to enable clients to:
/// 1. Fetch accounts needed for specific instructions
/// 2. Determine which accounts are cold and need loading
/// 3. Build load instructions for cold accounts
pub trait LightProgramInterface: Sized {
    /// The program's interface account variant enum.
    /// Generated by `#[light_program]` macro, contains parsed data + seed values.
    type Variant: Pack + Clone + Debug;

    /// Program-specific instruction kind enum (e.g., `Swap`, `Deposit`, `Withdraw`).
    type InstructionKind: Copy + Debug;

    /// Error type for SDK operations.
    type Error: std::error::Error;

    /// The program ID.
    #[must_use]
    fn program_id(&self) -> Pubkey;

    /// Construct SDK from keyed account interfaces.
    fn from_keyed_accounts(accounts: &[AccountInterface]) -> Result<Self, Self::Error>;

    /// Returns ALL compressible account pubkeys (accounts that could be cold).
    /// Used for initial fetch and caching.
    #[must_use]
    fn get_compressible_accounts(&self) -> Vec<Pubkey>;

    // TODO: Replace AccountToFetch with just Pubkey once Photon can determine type from pubkey alone.
    /// Returns accounts needed for a specific instruction.
    #[must_use]
    fn get_accounts_for_instruction(&self, kind: Self::InstructionKind) -> Vec<AccountToFetch>;

    /// Returns compressible accounts needed for a specific instruction.
    /// This is a subset of `get_accounts_for_instruction()`.
    #[must_use]
    fn get_compressible_accounts_for_instruction(
        &self,
        kind: Self::InstructionKind,
    ) -> Vec<Pubkey>;

    /// Update internal cache from account interfaces.
    /// Works uniformly for hot/cold accounts.
    /// Named `update_with_interfaces` to avoid collision with Jupiter's `Amm::update`.
    fn update_with_interfaces(&mut self, accounts: &[AccountInterface]) -> Result<(), Self::Error>;

    /// Get all cached specs (hot and cold).
    #[must_use]
    fn get_all_specs(&self) -> Vec<AccountSpec<Self::Variant>>;

    /// Get specs for a specific instruction.
    #[must_use]
    fn get_specs_for_instruction(
        &self,
        kind: Self::InstructionKind,
    ) -> Vec<AccountSpec<Self::Variant>>;

    /// Get lean cold specs for a specific instruction.
    /// Returns `ColdAccountSpec` which has NO redundant `Account` data.
    #[must_use]
    fn get_cold_specs_for_instruction(
        &self,
        kind: Self::InstructionKind,
    ) -> Vec<ColdAccountSpec<Self::Variant>>;

    /// Check if any accounts for this instruction are cold.
    #[must_use]
    fn has_cold_accounts_for_instruction(&self, kind: Self::InstructionKind) -> bool {
        !self.get_cold_specs_for_instruction(kind).is_empty()
    }

    /// Check if any cached accounts are cold.
    #[must_use]
    fn has_any_cold_accounts(&self) -> bool {
        any_cold(&self.get_all_specs())
    }
}

/// Extension trait for AMM SDKs that support swap operations.
///
/// This is a convenience layer over `LightProgramInterface` that pins
/// the instruction kind to "Swap". Aggregators use this trait.
pub trait LightAmmInterface: LightProgramInterface {
    /// The swap instruction kind for this AMM.
    fn swap_instruction_kind(&self) -> Self::InstructionKind;

    /// Get accounts needed for swap.
    /// Equivalent to `get_accounts_for_instruction(swap_instruction_kind())`.
    #[must_use]
    fn get_swap_accounts(&self) -> Vec<AccountToFetch> {
        self.get_accounts_for_instruction(self.swap_instruction_kind())
    }

    /// Get compressible accounts needed for swap.
    #[must_use]
    fn get_compressible_swap_accounts(&self) -> Vec<Pubkey> {
        self.get_compressible_accounts_for_instruction(self.swap_instruction_kind())
    }

    /// Get specs for swap instruction.
    #[must_use]
    fn get_swap_specs(&self) -> Vec<AccountSpec<Self::Variant>> {
        self.get_specs_for_instruction(self.swap_instruction_kind())
    }

    /// Get lean cold specs for swap accounts only.
    /// This is what aggregators call before building load instructions.
    #[must_use]
    fn get_cold_swap_specs(&self) -> Vec<ColdAccountSpec<Self::Variant>> {
        self.get_cold_specs_for_instruction(self.swap_instruction_kind())
    }

    /// Check if swap requires loading cold accounts.
    #[must_use]
    fn swap_needs_loading(&self) -> bool {
        self.has_cold_accounts_for_instruction(self.swap_instruction_kind())
    }
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
