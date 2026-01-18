//! CompressibleProgram trait and supporting types for client-side SDK patterns.
//!
//! This module provides a trait-based approach for programs to expose their
//! compressible account structure to clients. Inspired by Jupiter AMM interface.
//!
//! # Usage Pattern
//!
//! 1. Program implements `CompressibleProgram` trait in a separate SDK module
//! 2. Client fetches root accounts (e.g., PoolState) via indexer
//! 3. Client creates SDK instance via `from_keyed_accounts([pool])`
//! 4. Client queries what accounts need updating via `get_accounts_to_update(op)`
//! 5. Client fetches those accounts and calls `update(accounts)`
//! 6. Client gets specs via `get_specs_for_operation(op)`
//! 7. Client passes specs to `build_load_instructions()` for decompression
//!
//! # Example
//!
//! ```ignore
//! // 1. Fetch root state
//! let pool_interface = rpc.get_account_info_interface(&pool_pubkey).await?;
//! let keyed = KeyedAccountInterface::from_pda_interface(pool_interface);
//!
//! // 2. Create SDK from root
//! let mut sdk = AmmSdk::from_keyed_accounts(&[keyed])?;
//!
//! // 3. Query what accounts to fetch for Deposit operation
//! let needed = sdk.get_accounts_to_update(&AmmOperation::Deposit);
//!
//! // 4. Fetch and update
//! let interfaces = fetch_keyed_interfaces(&needed).await?;
//! sdk.update(&interfaces)?;
//!
//! // 5. Get specs for building decompress instructions
//! let specs = sdk.get_specs_for_operation(&AmmOperation::Deposit);
//! let ixs = build_load_instructions_from_specs(&specs, ...).await?;
//! ```

use crate::{
    AccountInfoInterface, PdaDecompressionContext, TokenAccountInterface, TokenLoadContext,
};
use light_sdk::compressible::Pack;
use solana_pubkey::Pubkey;
use std::fmt::Debug;

// =============================================================================
// ACCOUNT TO FETCH
// =============================================================================

/// Account descriptor for fetching. Contains all info needed to call the right
/// indexer endpoint. Pass to `get_multiple_account_interfaces()`.
#[derive(Debug, Clone)]
pub enum AccountToFetch {
    /// PDA account - uses `get_account_info_interface(address, program_id)`
    Pda { address: Pubkey, program_id: Pubkey },
    /// Token account (program-owned or ATA) - uses `get_token_account_interface(address)`
    /// The address is the owner of the compressed token.
    Token { address: Pubkey },
    /// Light mint - uses `get_mint_interface(signer)`
    Mint { signer: Pubkey },
}

impl AccountToFetch {
    /// Create a PDA fetch descriptor.
    pub fn pda(address: Pubkey, program_id: Pubkey) -> Self {
        Self::Pda {
            address,
            program_id,
        }
    }

    /// Create a token account fetch descriptor.
    pub fn token(address: Pubkey) -> Self {
        Self::Token { address }
    }

    /// Create a mint fetch descriptor.
    pub fn mint(signer: Pubkey) -> Self {
        Self::Mint { signer }
    }

    /// Get the primary pubkey for this account.
    pub fn pubkey(&self) -> Pubkey {
        match self {
            Self::Pda { address, .. } => *address,
            Self::Token { address } => *address,
            Self::Mint { signer } => *signer,
        }
    }
}

// =============================================================================
// KEYED ACCOUNT INTERFACE
// =============================================================================

/// Account interface with explicit pubkey.
///
/// Wraps `AccountInterface` variants with their pubkey for SDK usage.
/// Programs extract seed values and state from these when building specs.
#[derive(Clone, Debug)]
pub struct KeyedAccountInterface {
    /// The account's public key (PDA address or token account address)
    pub pubkey: Pubkey,
    /// Whether the account is compressed (cold) or on-chain (hot)
    pub is_cold: bool,
    /// Raw account data bytes (synthesized from compressed or actual on-chain)
    pub data: Vec<u8>,
    /// Context for decompression (only present when is_cold)
    pub cold_context: Option<ColdContext>,
}

/// Context needed for decompression, unified for different account types.
#[derive(Clone, Debug)]
pub enum ColdContext {
    /// PDA account decompression context
    Pda(PdaDecompressionContext),
    /// Token account decompression context
    Token(TokenLoadContext),
    /// Mint decompression context
    Mint {
        signer: Pubkey,
        compressed_address: [u8; 32],
        compressed: light_client::indexer::CompressedAccount,
        mint_data: light_token_interface::state::Mint,
    },
}

impl KeyedAccountInterface {
    /// Create from PDA interface (AccountInfoInterface).
    pub fn from_pda_interface(interface: AccountInfoInterface) -> Self {
        Self {
            pubkey: interface.pubkey,
            is_cold: interface.is_cold,
            data: interface.account.data.clone(),
            cold_context: interface.load_context.map(|ctx| {
                ColdContext::Pda(crate::PdaDecompressionContext {
                    compressed_account: ctx.compressed,
                })
            }),
        }
    }

    /// Create from token account interface (TokenAccountInterface).
    pub fn from_token_interface(interface: TokenAccountInterface) -> Self {
        Self {
            pubkey: interface.pubkey,
            is_cold: interface.is_cold,
            data: interface.account.data.clone(),
            cold_context: interface.load_context.map(ColdContext::Token),
        }
    }

    /// Create from mint interface (MintInterface).
    pub fn from_mint_interface(interface: crate::MintInterface) -> Self {
        match interface.state {
            crate::MintState::Hot { account } => Self {
                pubkey: interface.cmint,
                is_cold: false,
                data: account.data,
                cold_context: None,
            },
            crate::MintState::Cold {
                compressed,
                mint_data,
            } => {
                // Serialize mint data for the data field
                use borsh::BorshSerialize;
                let data = mint_data.try_to_vec().unwrap_or_default();
                Self {
                    pubkey: interface.cmint,
                    is_cold: true,
                    data,
                    cold_context: Some(ColdContext::Mint {
                        signer: interface.signer,
                        compressed_address: interface.compressed_address,
                        compressed,
                        mint_data,
                    }),
                }
            }
            crate::MintState::None => Self {
                pubkey: interface.cmint,
                is_cold: false,
                data: vec![],
                cold_context: None,
            },
        }
    }

    /// Create a hot (on-chain) keyed interface.
    pub fn hot(pubkey: Pubkey, data: Vec<u8>) -> Self {
        Self {
            pubkey,
            is_cold: false,
            data,
            cold_context: None,
        }
    }

    /// Create a cold (compressed) keyed interface for PDA.
    pub fn cold_pda(
        pubkey: Pubkey,
        data: Vec<u8>,
        compressed_account: light_client::indexer::CompressedAccount,
    ) -> Self {
        Self {
            pubkey,
            is_cold: true,
            data,
            cold_context: Some(ColdContext::Pda(crate::PdaDecompressionContext {
                compressed_account,
            })),
        }
    }

    /// Get the compressed account hash if cold PDA.
    pub fn pda_hash(&self) -> Option<[u8; 32]> {
        match &self.cold_context {
            Some(ColdContext::Pda(ctx)) => Some(ctx.compressed_account.hash),
            _ => None,
        }
    }

    /// Get the compressed account hash if cold token.
    pub fn token_hash(&self) -> Option<[u8; 32]> {
        match &self.cold_context {
            Some(ColdContext::Token(ctx)) => Some(ctx.compressed.account.hash),
            _ => None,
        }
    }

    /// Get PDA decompression context if available.
    pub fn pda_context(&self) -> Option<&PdaDecompressionContext> {
        match &self.cold_context {
            Some(ColdContext::Pda(ctx)) => Some(ctx),
            _ => None,
        }
    }

    /// Get token decompression context if available.
    pub fn token_context(&self) -> Option<&TokenLoadContext> {
        match &self.cold_context {
            Some(ColdContext::Token(ctx)) => Some(ctx),
            _ => None,
        }
    }
}

// =============================================================================
// SPEC TYPES
// =============================================================================

/// Specification for a program-owned account (PDA or program-owned token).
///
/// Contains all information needed to build decompression instructions:
/// - The variant with seed values filled in
/// - Cold context for proof fetching
#[derive(Clone, Debug)]
pub struct ProgramOwnedSpec<V> {
    /// The account's public key
    pub address: Pubkey,
    /// The typed variant with all seed values populated
    pub variant: V,
    /// Whether this account is compressed
    pub is_cold: bool,
    /// Decompression context (hash, tree info, etc.) - only if cold
    pub cold_context: Option<PdaDecompressionContext>,
}

impl<V> ProgramOwnedSpec<V> {
    /// Create a new spec for a hot account.
    pub fn hot(address: Pubkey, variant: V) -> Self {
        Self {
            address,
            variant,
            is_cold: false,
            cold_context: None,
        }
    }

    /// Create a new spec for a cold account.
    pub fn cold(address: Pubkey, variant: V, context: PdaDecompressionContext) -> Self {
        Self {
            address,
            variant,
            is_cold: true,
            cold_context: Some(context),
        }
    }

    /// Get the compressed account hash if cold.
    pub fn hash(&self) -> Option<[u8; 32]> {
        self.cold_context
            .as_ref()
            .map(|c| c.compressed_account.hash)
    }
}

/// Specification for an Associated Token Account.
///
/// ATAs are decompressed differently (create ATA + transfer2) so they
/// have their own spec type with wallet owner and mint info.
#[derive(Clone, Debug)]
pub struct AtaSpec {
    /// The ATA's public key
    pub address: Pubkey,
    /// The wallet that owns this ATA
    pub wallet_owner: Pubkey,
    /// The token mint
    pub mint: Pubkey,
    /// Whether this ATA is compressed
    pub is_cold: bool,
    /// Token load context - only if cold
    pub load_context: Option<TokenLoadContext>,
}

impl AtaSpec {
    /// Create a new spec for a hot ATA.
    pub fn hot(address: Pubkey, wallet_owner: Pubkey, mint: Pubkey) -> Self {
        Self {
            address,
            wallet_owner,
            mint,
            is_cold: false,
            load_context: None,
        }
    }

    /// Create a new spec for a cold ATA.
    pub fn cold(
        address: Pubkey,
        wallet_owner: Pubkey,
        mint: Pubkey,
        load_context: TokenLoadContext,
    ) -> Self {
        Self {
            address,
            wallet_owner,
            mint,
            is_cold: true,
            load_context: Some(load_context),
        }
    }

    /// Get the compressed account hash if cold.
    pub fn hash(&self) -> Option<[u8; 32]> {
        self.load_context.as_ref().map(|c| c.hash())
    }
}

/// Specification for a Light Mint.
///
/// Mints are decompressed via DecompressMint instruction.
/// For cold mints, stores the compressed account and parsed mint data
/// needed to build decompression instructions.
#[derive(Clone, Debug)]
pub struct MintSpec {
    /// The on-chain mint address (derived from mint_signer)
    pub cmint: Pubkey,
    /// The mint signer PDA used to derive the mint address
    pub mint_signer: Pubkey,
    /// The compressed address of this mint
    pub compressed_address: [u8; 32],
    /// Whether this mint is compressed
    pub is_cold: bool,
    /// Compressed account - only if cold
    pub compressed: Option<light_client::indexer::CompressedAccount>,
    /// Parsed mint data - only if cold
    pub mint_data: Option<light_token_interface::state::Mint>,
}

impl MintSpec {
    /// Create a new spec for a hot mint.
    pub fn hot(cmint: Pubkey, mint_signer: Pubkey, compressed_address: [u8; 32]) -> Self {
        Self {
            cmint,
            mint_signer,
            compressed_address,
            is_cold: false,
            compressed: None,
            mint_data: None,
        }
    }

    /// Create a new spec for a cold mint.
    pub fn cold(
        cmint: Pubkey,
        mint_signer: Pubkey,
        compressed_address: [u8; 32],
        compressed: light_client::indexer::CompressedAccount,
        mint_data: light_token_interface::state::Mint,
    ) -> Self {
        Self {
            cmint,
            mint_signer,
            compressed_address,
            is_cold: true,
            compressed: Some(compressed),
            mint_data: Some(mint_data),
        }
    }

    /// Get the compressed account hash if cold.
    pub fn hash(&self) -> Option<[u8; 32]> {
        self.compressed.as_ref().map(|c| c.hash)
    }
}

/// Collection of all specs for a program's compressible accounts.
///
/// Grouped by account type for building appropriate decompression instructions.
#[derive(Clone, Debug, Default)]
pub struct AllSpecs<V> {
    /// Program-owned accounts (PDAs + program-owned token accounts)
    /// These are decompressed via `decompress_accounts_idempotent`
    pub program_owned: Vec<ProgramOwnedSpec<V>>,
    /// Associated token accounts (user ATAs)
    /// These are decompressed via create_ata + transfer2
    pub atas: Vec<AtaSpec>,
    /// Light mints
    /// These are decompressed via DecompressMint
    pub mints: Vec<MintSpec>,
}

impl<V> AllSpecs<V> {
    /// Create empty specs.
    pub fn new() -> Self {
        Self {
            program_owned: Vec::new(),
            atas: Vec::new(),
            mints: Vec::new(),
        }
    }

    /// Check if all specs are hot (no decompression needed).
    pub fn all_hot(&self) -> bool {
        self.program_owned.iter().all(|s| !s.is_cold)
            && self.atas.iter().all(|s| !s.is_cold)
            && self.mints.iter().all(|s| !s.is_cold)
    }

    /// Check if any specs are cold (decompression needed).
    pub fn has_cold(&self) -> bool {
        !self.all_hot()
    }

    /// Get only cold program-owned specs.
    pub fn cold_program_owned(&self) -> Vec<&ProgramOwnedSpec<V>> {
        self.program_owned.iter().filter(|s| s.is_cold).collect()
    }

    /// Get only cold ATA specs.
    pub fn cold_atas(&self) -> Vec<&AtaSpec> {
        self.atas.iter().filter(|s| s.is_cold).collect()
    }

    /// Get only cold mint specs.
    pub fn cold_mints(&self) -> Vec<&MintSpec> {
        self.mints.iter().filter(|s| s.is_cold).collect()
    }
}

// =============================================================================
// COMPRESSIBLE PROGRAM TRAIT
// =============================================================================

/// Trait for programs to expose their compressible account structure to clients.
///
/// Programs implement this trait in a SDK module that clients can import.
/// The SDK handles:
/// - Parsing root state accounts to extract related account pubkeys
/// - Caching account specs internally
/// - Providing filtered specs for specific operations
///
/// # Type Parameters
///
/// - `Variant`: The program's `RentFreeAccountVariant` enum (implements Pack)
/// - `Operation`: Program-specific operation enum (e.g., Swap, Deposit, Withdraw)
/// - `Error`: Program-specific error type
///
/// # Implementation Notes
///
/// - `from_keyed_accounts`: Should accept root accounts (e.g., PoolState) and extract
///   all related pubkeys from their fields. Initialize internal caches.
///
/// - `get_accounts_to_update`: Return pubkeys that need to be fetched for an operation.
///   These are typically derived from root state fields.
///
/// - `update`: Parse fetched accounts, build variants with seed values, cache specs.
///   Should be idempotent - updating with same accounts shouldn't change state.
///
/// - `get_specs_for_operation`: Return specs filtered for the operation.
///   Swap might need vaults only, Deposit might also need LP mint, etc.
pub trait CompressibleProgram: Sized {
    /// The program's compressed account variant enum.
    /// Must implement Pack for instruction serialization.
    type Variant: Pack + Clone + Debug;

    /// Program-specific operation enum.
    /// Used to filter which accounts are needed.
    type Operation;

    /// Error type for SDK operations.
    type Error: std::error::Error;

    /// Construct SDK from canonical root account(s).
    ///
    /// Parses the root state (e.g., PoolState), extracts seed context
    /// (all pubkeys stored in the state), and initializes internal caches.
    ///
    /// # Arguments
    /// * `accounts` - Root account interfaces (e.g., just the pool state)
    ///
    /// # Returns
    /// Initialized SDK instance or error if parsing fails.
    fn from_keyed_accounts(accounts: &[KeyedAccountInterface]) -> Result<Self, Self::Error>;

    /// Returns pubkeys of accounts needed for an operation.
    ///
    /// After calling this, client should fetch these accounts and pass
    /// them to `update()` to fill the specs cache.
    ///
    /// # Arguments
    /// * `op` - The operation to get accounts for
    ///
    /// # Returns
    /// List of pubkeys to fetch. May include accounts already cached
    /// (client can filter based on freshness requirements).
    fn get_accounts_to_update(&self, op: &Self::Operation) -> Vec<Pubkey>;

    /// Update internal cache with fetched account data.
    ///
    /// Parses each account, builds the appropriate variant with seed values,
    /// and caches the spec. Should be idempotent.
    ///
    /// # Arguments
    /// * `accounts` - Fetched account interfaces
    ///
    /// # Returns
    /// Ok(()) on success, error if parsing fails.
    fn update(&mut self, accounts: &[KeyedAccountInterface]) -> Result<(), Self::Error>;

    /// Get all cached specs (for simple clients who fetch everything).
    ///
    /// Returns all specs regardless of operation. Useful for clients
    /// that pre-fetch all related accounts.
    fn get_all_specs(&self) -> AllSpecs<Self::Variant>;

    /// Get specs filtered for a specific operation.
    ///
    /// Returns only the specs relevant to the operation.
    /// E.g., Swap might return vaults only, Deposit might include LP mint.
    ///
    /// # Arguments
    /// * `op` - The operation to get specs for
    fn get_specs_for_operation(&self, op: &Self::Operation) -> AllSpecs<Self::Variant>;
}
