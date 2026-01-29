//! Client SDK for the AMM test program.
//!
//! Implements the `LightProgramInterface` trait to provide a Jupiter-style
//! interface for clients to build decompression instructions.

use std::collections::HashMap;

use anchor_lang::AnchorDeserialize;
use csdk_anchor_full_derived_test::{
    amm_test::{ObservationState, PoolState, AUTH_SEED, POOL_LP_MINT_SIGNER_SEED},
    csdk_anchor_full_derived_test::{
        LightAccountVariant, ObservationStateSeeds, ObservationStateVariant, PoolStateSeeds,
        PoolStateVariant, Token0VaultSeeds, Token1VaultSeeds,
    },
};
use light_client::interface::{
    matches_discriminator, AccountInterface, AccountSpec, AccountToFetch, ColdContext,
    LightProgramInterface, PdaSpec,
};
use light_sdk::LightDiscriminator;
use solana_pubkey::Pubkey;

/// Program ID for the AMM test program.
pub const PROGRAM_ID: Pubkey = csdk_anchor_full_derived_test::ID;

/// Map of account pubkeys to program-owned specs.
pub type PdaSpecMap = HashMap<Pubkey, PdaSpec<LightAccountVariant>, ahash::RandomState>;

/// Map of account pubkeys to mint interfaces.
pub type MintInterfaceMap = HashMap<Pubkey, AccountInterface, ahash::RandomState>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountKind {
    Pda,
    Token,
    Mint,
}

#[derive(Debug, Clone, Copy)]
pub struct AccountRequirement {
    pub pubkey: Option<Pubkey>,
    pub kind: AccountKind,
}

impl AccountRequirement {
    fn new(pubkey: Option<Pubkey>, kind: AccountKind) -> Self {
        Self { pubkey, kind }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmmInstruction {
    Swap,
    Deposit,
    Withdraw,
}

#[derive(Debug, Clone)]
pub enum AmmSdkError {
    ParseError(String),
    UnknownDiscriminator([u8; 8]),
    MissingField(&'static str),
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

#[derive(Debug)]
pub struct AmmSdk {
    pool_state_pubkey: Option<Pubkey>,
    amm_config: Option<Pubkey>,
    token_0_mint: Option<Pubkey>,
    token_1_mint: Option<Pubkey>,
    token_0_vault: Option<Pubkey>,
    token_1_vault: Option<Pubkey>,
    lp_mint: Option<Pubkey>,
    observation_key: Option<Pubkey>,
    authority: Option<Pubkey>,
    lp_mint_signer: Option<Pubkey>,
    program_owned_specs: PdaSpecMap,
    mint_specs: MintInterfaceMap,
}

impl Default for AmmSdk {
    fn default() -> Self {
        Self::new()
    }
}

impl AmmSdk {
    pub fn new() -> Self {
        Self {
            pool_state_pubkey: None,
            amm_config: None,
            token_0_mint: None,
            token_1_mint: None,
            token_0_vault: None,
            token_1_vault: None,
            lp_mint: None,
            observation_key: None,
            authority: None,
            lp_mint_signer: None,
            program_owned_specs: HashMap::with_hasher(ahash::RandomState::new()),
            mint_specs: HashMap::with_hasher(ahash::RandomState::new()),
        }
    }

    pub fn pool_state_pubkey(&self) -> Option<Pubkey> {
        self.pool_state_pubkey
    }

    pub fn lp_mint(&self) -> Option<Pubkey> {
        self.lp_mint
    }

    pub fn lp_mint_signer(&self) -> Option<Pubkey> {
        self.lp_mint_signer
    }

    fn parse_pool_state(&mut self, account: &AccountInterface) -> Result<(), AmmSdkError> {
        let pool = PoolState::deserialize(&mut &account.data()[8..])
            .map_err(|e| AmmSdkError::ParseError(e.to_string()))?;

        self.pool_state_pubkey = Some(account.key);

        self.amm_config = Some(pool.amm_config);
        self.token_0_mint = Some(pool.token_0_mint);
        self.token_1_mint = Some(pool.token_1_mint);
        self.token_0_vault = Some(pool.token_0_vault);
        self.token_1_vault = Some(pool.token_1_vault);
        self.lp_mint = Some(pool.lp_mint);
        self.observation_key = Some(pool.observation_key);

        let (authority, _) = Pubkey::find_program_address(&[AUTH_SEED.as_bytes()], &PROGRAM_ID);
        self.authority = Some(authority);

        let (lp_mint_signer, _) = Pubkey::find_program_address(
            &[POOL_LP_MINT_SIGNER_SEED, account.key.as_ref()],
            &PROGRAM_ID,
        );
        self.lp_mint_signer = Some(lp_mint_signer);

        let variant = LightAccountVariant::PoolState(PoolStateVariant {
            seeds: PoolStateSeeds {
                amm_config: self.amm_config.unwrap(),
                token_0_mint: self.token_0_mint.unwrap(),
                token_1_mint: self.token_1_mint.unwrap(),
            },
            data: pool,
        });

        let spec = PdaSpec::new(account.clone(), variant, PROGRAM_ID);
        self.program_owned_specs.insert(account.key, spec);

        Ok(())
    }

    fn parse_observation_state(&mut self, account: &AccountInterface) -> Result<(), AmmSdkError> {
        let pool_state = self
            .pool_state_pubkey
            .ok_or(AmmSdkError::PoolStateNotParsed)?;

        let observation = ObservationState::deserialize(&mut &account.data()[8..])
            .map_err(|e| AmmSdkError::ParseError(e.to_string()))?;

        let variant = LightAccountVariant::ObservationState(ObservationStateVariant {
            seeds: ObservationStateSeeds { pool_state },
            data: observation,
        });

        let spec = PdaSpec::new(account.clone(), variant, PROGRAM_ID);
        self.program_owned_specs.insert(account.key, spec);

        Ok(())
    }

    fn parse_token_vault(
        &mut self,
        account: &AccountInterface,
        is_vault_0: bool,
    ) -> Result<(), AmmSdkError> {
        use light_sdk::interface::token::{Token, TokenDataWithSeeds};

        let pool_state = self
            .pool_state_pubkey
            .ok_or(AmmSdkError::PoolStateNotParsed)?;

        let token: Token = Token::deserialize(&mut &account.data()[..])
            .map_err(|e| AmmSdkError::ParseError(e.to_string()))?;

        let variant = if is_vault_0 {
            let token_0_mint = self
                .token_0_mint
                .ok_or(AmmSdkError::MissingField("token_0_mint"))?;
            LightAccountVariant::Token0Vault(TokenDataWithSeeds {
                seeds: Token0VaultSeeds {
                    pool_state,
                    token_0_mint,
                },
                token_data: token,
            })
        } else {
            let token_1_mint = self
                .token_1_mint
                .ok_or(AmmSdkError::MissingField("token_1_mint"))?;
            LightAccountVariant::Token1Vault(TokenDataWithSeeds {
                seeds: Token1VaultSeeds {
                    pool_state,
                    token_1_mint,
                },
                token_data: token,
            })
        };

        // For token vaults, convert ColdContext::Token to ColdContext::Account
        // because they're decompressed as PDAs, not as token accounts
        let interface = if account.is_cold() {
            let compressed_account = match &account.cold {
                Some(ColdContext::Token(ct)) => ct.account.clone(),
                Some(ColdContext::Account(ca)) => ca.clone(),
                None => return Err(AmmSdkError::MissingField("cold_context")),
            };
            AccountInterface {
                key: account.key,
                account: account.account.clone(), // Keep original owner (SPL Token)
                cold: Some(ColdContext::Account(compressed_account)),
            }
        } else {
            account.clone()
        };

        // Decompression goes to PROGRAM_ID (AMM), not interface.account.owner (SPL/Light Token)
        let spec = PdaSpec::new(interface, variant, PROGRAM_ID);
        self.program_owned_specs.insert(account.key, spec);

        Ok(())
    }

    fn parse_account(&mut self, account: &AccountInterface) -> Result<(), AmmSdkError> {
        if Some(account.key) == self.token_0_vault {
            return self.parse_token_vault(account, true);
        }
        if Some(account.key) == self.token_1_vault {
            return self.parse_token_vault(account, false);
        }

        if matches_discriminator(account.data(), &PoolState::LIGHT_DISCRIMINATOR) {
            return self.parse_pool_state(account);
        }
        if matches_discriminator(account.data(), &ObservationState::LIGHT_DISCRIMINATOR) {
            return self.parse_observation_state(account);
        }

        // Check if this is an LP mint by matching the signer
        if let Some(lp_mint_signer) = self.lp_mint_signer {
            if let Some(mint_signer) = account.mint_signer() {
                if Pubkey::new_from_array(mint_signer) == lp_mint_signer {
                    return self.parse_mint(account);
                }
            }
        }

        Ok(())
    }

    fn parse_mint(&mut self, account: &AccountInterface) -> Result<(), AmmSdkError> {
        // Store AccountInterface directly - mints are just accounts with special data
        self.mint_specs.insert(account.key, account.clone());
        Ok(())
    }

    pub fn derive_lp_mint_compressed_address(&self, address_tree: &Pubkey) -> Option<[u8; 32]> {
        self.lp_mint_signer.map(|signer| {
            light_compressed_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address(
                &signer,
                address_tree,
            )
        })
    }

    fn account_requirements(&self, ix: &AmmInstruction) -> Vec<AccountRequirement> {
        match ix {
            AmmInstruction::Swap => {
                vec![
                    AccountRequirement::new(self.pool_state_pubkey, AccountKind::Pda),
                    AccountRequirement::new(self.token_0_vault, AccountKind::Token),
                    AccountRequirement::new(self.token_1_vault, AccountKind::Token),
                    AccountRequirement::new(self.observation_key, AccountKind::Pda),
                ]
            }
            AmmInstruction::Deposit | AmmInstruction::Withdraw => {
                vec![
                    AccountRequirement::new(self.pool_state_pubkey, AccountKind::Pda),
                    AccountRequirement::new(self.token_0_vault, AccountKind::Token),
                    AccountRequirement::new(self.token_1_vault, AccountKind::Token),
                    AccountRequirement::new(self.observation_key, AccountKind::Pda),
                    AccountRequirement::new(self.lp_mint, AccountKind::Mint),
                ]
            }
        }
    }
}

impl LightProgramInterface for AmmSdk {
    type Variant = LightAccountVariant;
    type Instruction = AmmInstruction;
    type Error = AmmSdkError;

    fn program_id(&self) -> Pubkey {
        PROGRAM_ID
    }

    fn from_keyed_accounts(accounts: &[AccountInterface]) -> Result<Self, Self::Error> {
        let mut sdk = Self::new();

        for account in accounts {
            // Parse pool_state first (needed for other accounts), then remaining
            if matches_discriminator(account.data(), &PoolState::LIGHT_DISCRIMINATOR) {
                sdk.parse_pool_state(account)?;
            } else {
                sdk.parse_account(account)?;
            }
        }

        Ok(sdk)
    }

    fn get_accounts_to_update(&self, ix: &Self::Instruction) -> Vec<AccountToFetch> {
        self.account_requirements(ix)
            .into_iter()
            .filter_map(|req| {
                req.pubkey.map(|pubkey| match req.kind {
                    AccountKind::Pda => AccountToFetch::pda(pubkey, PROGRAM_ID),
                    AccountKind::Token => AccountToFetch::token(pubkey),
                    AccountKind::Mint => AccountToFetch::mint(pubkey),
                })
            })
            .collect()
    }

    fn update(&mut self, accounts: &[AccountInterface]) -> Result<(), Self::Error> {
        for account in accounts {
            self.parse_account(account)?;
        }
        Ok(())
    }

    fn get_all_specs(&self) -> Vec<AccountSpec<Self::Variant>> {
        let mut specs = Vec::new();
        specs.extend(
            self.program_owned_specs
                .values()
                .cloned()
                .map(AccountSpec::Pda),
        );
        specs.extend(self.mint_specs.values().cloned().map(AccountSpec::Mint));
        specs
    }

    fn get_specs_for_instruction(&self, ix: &Self::Instruction) -> Vec<AccountSpec<Self::Variant>> {
        let requirements = self.account_requirements(ix);
        let mut specs = Vec::new();

        for req in &requirements {
            match req.kind {
                AccountKind::Pda | AccountKind::Token => {
                    if let Some(pubkey) = req.pubkey {
                        if let Some(spec) = self.program_owned_specs.get(&pubkey) {
                            specs.push(AccountSpec::Pda(spec.clone()));
                        }
                    }
                }
                AccountKind::Mint => {
                    if let Some(mint_pubkey) = req.pubkey {
                        if let Some(spec) = self.mint_specs.get(&mint_pubkey) {
                            specs.push(AccountSpec::Mint(spec.clone()));
                        }
                    }
                }
            }
        }

        specs
    }
}

impl AmmSdk {
    pub fn program_id(&self) -> Pubkey {
        PROGRAM_ID
    }

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

    pub fn observation_state_seeds(&self) -> Result<ObservationStateSeeds, AmmSdkError> {
        Ok(ObservationStateSeeds {
            pool_state: self
                .pool_state_pubkey
                .ok_or(AmmSdkError::PoolStateNotParsed)?,
        })
    }

    pub fn token_0_vault_seeds(&self) -> Result<Token0VaultSeeds, AmmSdkError> {
        Ok(Token0VaultSeeds {
            pool_state: self
                .pool_state_pubkey
                .ok_or(AmmSdkError::PoolStateNotParsed)?,
            token_0_mint: self
                .token_0_mint
                .ok_or(AmmSdkError::MissingField("token_0_mint"))?,
        })
    }

    pub fn token_1_vault_seeds(&self) -> Result<Token1VaultSeeds, AmmSdkError> {
        Ok(Token1VaultSeeds {
            pool_state: self
                .pool_state_pubkey
                .ok_or(AmmSdkError::PoolStateNotParsed)?,
            token_1_mint: self
                .token_1_mint
                .ok_or(AmmSdkError::MissingField("token_1_mint"))?,
        })
    }
}
