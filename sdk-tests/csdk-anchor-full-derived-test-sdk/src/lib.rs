//! Client SDK for the AMM test program.
//!
//! Implements the `CompressibleProgram` trait to provide a Jupiter-style
//! interface for clients to build decompression instructions.
//!
//! # Usage
//!
//! ```ignore
//! use csdk_anchor_full_derived_test_sdk::{AmmSdk, AmmOperation};
//! use light_compressible_client::{AccountInterfaceExt, KeyedAccountInterface};
//!
//! // 1. Fetch pool state interface
//! let pool_interface = rpc.get_account_info_interface(&pool_pubkey, &program_id).await?;
//! let keyed = KeyedAccountInterface::from_pda_interface(pool_interface);
//!
//! // 2. Create SDK from pool state
//! let mut sdk = AmmSdk::from_keyed_accounts(&[keyed])?;
//!
//! // 3. Get accounts needed for Deposit
//! let to_fetch = sdk.get_accounts_to_update_typed(&AmmOperation::Deposit);
//!
//! // 4. Fetch all accounts (unified method)
//! let keyed_accounts = rpc.get_multiple_account_interfaces(&to_fetch).await?;
//! sdk.update(&keyed_accounts)?;
//!
//! // 5. Get specs for decompression
//! let specs = sdk.get_specs_for_operation(&AmmOperation::Deposit);
//! ```

use std::collections::HashMap;

use anchor_lang::AnchorDeserialize;
use light_compressible_client::{
    AccountToFetch, AllSpecs, AtaSpec, CompressibleProgram, KeyedAccountInterface, MintSpec,
    ProgramOwnedSpec,
};
use light_sdk::LightDiscriminator;
use solana_pubkey::Pubkey;

// Import types from the program crate
use csdk_anchor_full_derived_test::amm_test::{
    ObservationState, PoolState, AUTH_SEED, POOL_LP_MINT_SIGNER_SEED,
};
use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::{
    ObservationStateSeeds, PoolStateSeeds, RentFreeAccountVariant, TokenAccountVariant,
};

/// Program ID for the AMM test program.
pub const PROGRAM_ID: Pubkey = csdk_anchor_full_derived_test::ID;

// =============================================================================
// OPERATION ENUM
// =============================================================================

/// AMM operations that may require loading cold accounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmmOperation {
    /// Swap tokens - requires vaults
    Swap,
    /// Deposit liquidity - requires vaults + LP mint
    Deposit,
    /// Withdraw liquidity - requires vaults + LP mint
    Withdraw,
}

// =============================================================================
// ERROR TYPE
// =============================================================================

/// Errors that can occur in AMM SDK operations.
#[derive(Debug, Clone)]
pub enum AmmSdkError {
    /// Failed to parse account data
    ParseError(String),
    /// Unknown account discriminator
    UnknownDiscriminator([u8; 8]),
    /// Missing required field
    MissingField(&'static str),
    /// Pool state not yet parsed
    PoolStateNotParsed,
}

impl std::fmt::Display for AmmSdkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::UnknownDiscriminator(disc) => write!(f, "Unknown discriminator: {:?}", disc),
            Self::MissingField(field) => write!(f, "Missing field: {}", field),
            Self::PoolStateNotParsed => write!(f, "Pool state must be parsed first"),
        }
    }
}

impl std::error::Error for AmmSdkError {}

// =============================================================================
// AMM SDK
// =============================================================================

/// Client SDK for the AMM program.
///
/// Caches parsed account data and specs for building decompression instructions.
/// Initialize from pool state, then update with additional accounts as needed.
#[derive(Debug, Default)]
pub struct AmmSdk {
    // === EXTRACTED FROM POOLSTATE ===
    pool_state_pubkey: Option<Pubkey>,
    amm_config: Option<Pubkey>,
    token_0_mint: Option<Pubkey>,
    token_1_mint: Option<Pubkey>,
    token_0_vault: Option<Pubkey>,
    token_1_vault: Option<Pubkey>,
    lp_mint: Option<Pubkey>,
    observation_key: Option<Pubkey>,

    // === DERIVED PDAS ===
    authority: Option<Pubkey>,
    lp_mint_signer: Option<Pubkey>,

    // === SPECS CACHE ===
    program_owned_specs: HashMap<Pubkey, ProgramOwnedSpec<RentFreeAccountVariant>>,
    ata_specs: HashMap<Pubkey, AtaSpec>,
    mint_specs: HashMap<Pubkey, MintSpec>,
}

impl AmmSdk {
    /// Create a new empty SDK instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the pool state pubkey if parsed.
    pub fn pool_state_pubkey(&self) -> Option<Pubkey> {
        self.pool_state_pubkey
    }

    /// Get the LP mint pubkey if available.
    pub fn lp_mint(&self) -> Option<Pubkey> {
        self.lp_mint
    }

    /// Get the LP mint signer pubkey if derived.
    pub fn lp_mint_signer(&self) -> Option<Pubkey> {
        self.lp_mint_signer
    }

    /// Parse PoolState and extract all relevant pubkeys.
    fn parse_pool_state(&mut self, account: &KeyedAccountInterface) -> Result<(), AmmSdkError> {
        // Deserialize PoolState
        let pool = PoolState::deserialize(&mut &account.data[8..])
            .map_err(|e| AmmSdkError::ParseError(e.to_string()))?;

        // Store pool pubkey
        self.pool_state_pubkey = Some(account.pubkey);

        // Extract all pubkeys directly from PoolState fields
        self.amm_config = Some(pool.amm_config);
        self.token_0_mint = Some(pool.token_0_mint);
        self.token_1_mint = Some(pool.token_1_mint);
        self.token_0_vault = Some(pool.token_0_vault);
        self.token_1_vault = Some(pool.token_1_vault);
        self.lp_mint = Some(pool.lp_mint);
        self.observation_key = Some(pool.observation_key);

        // Derive authority PDA
        let (authority, _) = Pubkey::find_program_address(&[AUTH_SEED.as_bytes()], &PROGRAM_ID);
        self.authority = Some(authority);

        // Derive lp_mint_signer PDA
        let (lp_mint_signer, _) = Pubkey::find_program_address(
            &[POOL_LP_MINT_SIGNER_SEED, account.pubkey.as_ref()],
            &PROGRAM_ID,
        );
        self.lp_mint_signer = Some(lp_mint_signer);

        // Build PoolState spec with seed values
        let variant = RentFreeAccountVariant::PoolState {
            data: pool,
            amm_config: self.amm_config.unwrap(),
            token_0_mint: self.token_0_mint.unwrap(),
            token_1_mint: self.token_1_mint.unwrap(),
        };

        let spec = if account.is_cold {
            let context = account
                .pda_context()
                .ok_or(AmmSdkError::MissingField("pda_context"))?
                .clone();
            ProgramOwnedSpec::cold(account.pubkey, variant, context)
        } else {
            ProgramOwnedSpec::hot(account.pubkey, variant)
        };

        self.program_owned_specs.insert(account.pubkey, spec);

        Ok(())
    }

    /// Parse ObservationState and build spec.
    fn parse_observation_state(
        &mut self,
        account: &KeyedAccountInterface,
    ) -> Result<(), AmmSdkError> {
        let pool_state = self
            .pool_state_pubkey
            .ok_or(AmmSdkError::PoolStateNotParsed)?;

        let observation = ObservationState::deserialize(&mut &account.data[8..])
            .map_err(|e| AmmSdkError::ParseError(e.to_string()))?;

        let variant = RentFreeAccountVariant::ObservationState {
            data: observation,
            pool_state,
        };

        let spec = if account.is_cold {
            let context = account
                .pda_context()
                .ok_or(AmmSdkError::MissingField("pda_context"))?
                .clone();
            ProgramOwnedSpec::cold(account.pubkey, variant, context)
        } else {
            ProgramOwnedSpec::hot(account.pubkey, variant)
        };

        self.program_owned_specs.insert(account.pubkey, spec);

        Ok(())
    }

    /// Parse token vault and build spec.
    fn parse_token_vault(
        &mut self,
        account: &KeyedAccountInterface,
        is_vault_0: bool,
    ) -> Result<(), AmmSdkError> {
        use light_token_sdk::compat::TokenData;

        let pool_state = self
            .pool_state_pubkey
            .ok_or(AmmSdkError::PoolStateNotParsed)?;

        // Parse TokenData from compressed account data
        let token_data = TokenData::deserialize(&mut &account.data[..])
            .map_err(|e| AmmSdkError::ParseError(e.to_string()))?;

        let variant = if is_vault_0 {
            let token_0_mint = self
                .token_0_mint
                .ok_or(AmmSdkError::MissingField("token_0_mint"))?;
            RentFreeAccountVariant::CTokenData(light_token_sdk::compat::CTokenData {
                variant: TokenAccountVariant::Token0Vault {
                    pool_state,
                    token_0_mint,
                },
                token_data,
            })
        } else {
            let token_1_mint = self
                .token_1_mint
                .ok_or(AmmSdkError::MissingField("token_1_mint"))?;
            RentFreeAccountVariant::CTokenData(light_token_sdk::compat::CTokenData {
                variant: TokenAccountVariant::Token1Vault {
                    pool_state,
                    token_1_mint,
                },
                token_data,
            })
        };

        let spec = if account.is_cold {
            let context = account
                .pda_context()
                .ok_or(AmmSdkError::MissingField("pda_context"))?
                .clone();
            ProgramOwnedSpec::cold(account.pubkey, variant, context)
        } else {
            ProgramOwnedSpec::hot(account.pubkey, variant)
        };

        self.program_owned_specs.insert(account.pubkey, spec);

        Ok(())
    }

    /// Parse an account based on its discriminator or known pubkey.
    fn parse_account(&mut self, account: &KeyedAccountInterface) -> Result<(), AmmSdkError> {
        // Check if this is a known vault by pubkey
        if Some(account.pubkey) == self.token_0_vault {
            return self.parse_token_vault(account, true);
        }
        if Some(account.pubkey) == self.token_1_vault {
            return self.parse_token_vault(account, false);
        }

        // Try to identify by discriminator
        if account.data.len() >= 8 {
            let disc: [u8; 8] = account.data[..8].try_into().unwrap();

            if disc == PoolState::LIGHT_DISCRIMINATOR {
                return self.parse_pool_state(account);
            }
            if disc == ObservationState::LIGHT_DISCRIMINATOR {
                return self.parse_observation_state(account);
            }
        }

        // Unknown account - skip silently (might be LP mint or other)
        Ok(())
    }

    /// Derive the compressed address for the LP mint.
    pub fn derive_lp_mint_compressed_address(&self, address_tree: &Pubkey) -> Option<[u8; 32]> {
        self.lp_mint_signer.map(|signer| {
            light_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address(
                &signer,
                address_tree,
            )
        })
    }
}

// =============================================================================
// COMPRESSIBLE PROGRAM TRAIT IMPLEMENTATION
// =============================================================================

impl CompressibleProgram for AmmSdk {
    type Variant = RentFreeAccountVariant;
    type Operation = AmmOperation;
    type Error = AmmSdkError;

    fn from_keyed_accounts(
        accounts: &[KeyedAccountInterface],
    ) -> std::result::Result<Self, Self::Error> {
        let mut sdk = Self::new();

        for account in accounts {
            // Try to parse as pool state first (our root account)
            if account.data.len() >= 8 {
                let disc: [u8; 8] = account.data[..8].try_into().unwrap();
                if disc == PoolState::LIGHT_DISCRIMINATOR {
                    sdk.parse_pool_state(account)?;
                } else {
                    sdk.parse_account(account)?;
                }
            }
        }

        Ok(sdk)
    }

    fn get_accounts_to_update(&self, op: &Self::Operation) -> Vec<Pubkey> {
        match op {
            AmmOperation::Swap => {
                // Swap needs: vaults
                vec![self.token_0_vault, self.token_1_vault]
                    .into_iter()
                    .flatten()
                    .collect()
            }
            AmmOperation::Deposit | AmmOperation::Withdraw => {
                // Deposit/Withdraw needs: vaults + observation + lp_mint
                vec![
                    self.token_0_vault,
                    self.token_1_vault,
                    self.observation_key,
                    self.lp_mint,
                ]
                .into_iter()
                .flatten()
                .collect()
            }
        }
    }

    fn update(
        &mut self,
        accounts: &[KeyedAccountInterface],
    ) -> std::result::Result<(), Self::Error> {
        for account in accounts {
            self.parse_account(account)?;
        }
        Ok(())
    }

    fn get_all_specs(&self) -> AllSpecs<Self::Variant> {
        AllSpecs {
            program_owned: self.program_owned_specs.values().cloned().collect(),
            atas: self.ata_specs.values().cloned().collect(),
            mints: self.mint_specs.values().cloned().collect(),
        }
    }

    fn get_specs_for_operation(&self, op: &Self::Operation) -> AllSpecs<Self::Variant> {
        let keys: Vec<Pubkey> = match op {
            AmmOperation::Swap => {
                vec![
                    self.pool_state_pubkey,
                    self.token_0_vault,
                    self.token_1_vault,
                ]
            }
            AmmOperation::Deposit | AmmOperation::Withdraw => {
                vec![
                    self.pool_state_pubkey,
                    self.token_0_vault,
                    self.token_1_vault,
                    self.observation_key,
                ]
            }
        }
        .into_iter()
        .flatten()
        .collect();

        let program_owned = keys
            .iter()
            .filter_map(|k| self.program_owned_specs.get(k).cloned())
            .collect();

        // For Deposit/Withdraw, include LP mint spec if available
        let mints = match op {
            AmmOperation::Deposit | AmmOperation::Withdraw => self
                .lp_mint
                .and_then(|m| self.mint_specs.get(&m).cloned())
                .into_iter()
                .collect(),
            _ => Vec::new(),
        };

        AllSpecs {
            program_owned,
            atas: self.ata_specs.values().cloned().collect(),
            mints,
        }
    }
}

// =============================================================================
// ACCOUNT FETCH HELPERS
// =============================================================================

impl AmmSdk {
    /// Get accounts to update with fetch descriptors.
    ///
    /// Returns `AccountToFetch` descriptors that can be passed directly to
    /// `rpc.get_multiple_account_interfaces()`. No type switching needed by caller.
    pub fn get_accounts_to_update_typed(&self, op: &AmmOperation) -> Vec<AccountToFetch> {
        let mut accounts = Vec::new();

        // Pool state is a PDA
        if let Some(address) = self.pool_state_pubkey {
            accounts.push(AccountToFetch::pda(address, PROGRAM_ID));
        }

        // Vaults are token accounts
        if let Some(address) = self.token_0_vault {
            accounts.push(AccountToFetch::token(address));
        }
        if let Some(address) = self.token_1_vault {
            accounts.push(AccountToFetch::token(address));
        }

        // Observation is a PDA, needed for Deposit/Withdraw
        if matches!(op, AmmOperation::Deposit | AmmOperation::Withdraw) {
            if let Some(address) = self.observation_key {
                accounts.push(AccountToFetch::pda(address, PROGRAM_ID));
            }
        }

        // LP mint is needed for Deposit/Withdraw
        if matches!(op, AmmOperation::Deposit | AmmOperation::Withdraw) {
            if let Some(signer) = self.lp_mint_signer {
                accounts.push(AccountToFetch::mint(signer));
            }
        }

        accounts
    }

    /// Get the program ID for this AMM.
    pub fn program_id(&self) -> Pubkey {
        PROGRAM_ID
    }
}

// =============================================================================
// HELPER FUNCTIONS FOR SEED CONSTRUCTION
// =============================================================================

impl AmmSdk {
    /// Create PoolStateSeeds from cached values.
    ///
    /// Useful when manually building `RentFreeDecompressAccount` without the trait.
    pub fn pool_state_seeds(&self) -> Result<PoolStateSeeds, AmmSdkError> {
        Ok(PoolStateSeeds {
            amm_config: self
                .amm_config
                .ok_or(AmmSdkError::MissingField("amm_config"))?,
            token_0_mint: self
                .token_0_mint
                .ok_or(AmmSdkError::MissingField("token_0_mint"))?,
            token_1_mint: self
                .token_1_mint
                .ok_or(AmmSdkError::MissingField("token_1_mint"))?,
        })
    }

    /// Create ObservationStateSeeds from cached values.
    pub fn observation_state_seeds(&self) -> Result<ObservationStateSeeds, AmmSdkError> {
        Ok(ObservationStateSeeds {
            pool_state: self
                .pool_state_pubkey
                .ok_or(AmmSdkError::PoolStateNotParsed)?,
        })
    }

    /// Create Token0Vault variant from cached values.
    pub fn token_0_vault_variant(&self) -> Result<TokenAccountVariant, AmmSdkError> {
        Ok(TokenAccountVariant::Token0Vault {
            pool_state: self
                .pool_state_pubkey
                .ok_or(AmmSdkError::PoolStateNotParsed)?,
            token_0_mint: self
                .token_0_mint
                .ok_or(AmmSdkError::MissingField("token_0_mint"))?,
        })
    }

    /// Create Token1Vault variant from cached values.
    pub fn token_1_vault_variant(&self) -> Result<TokenAccountVariant, AmmSdkError> {
        Ok(TokenAccountVariant::Token1Vault {
            pool_state: self
                .pool_state_pubkey
                .ok_or(AmmSdkError::PoolStateNotParsed)?,
            token_1_mint: self
                .token_1_mint
                .ok_or(AmmSdkError::MissingField("token_1_mint"))?,
        })
    }
}
