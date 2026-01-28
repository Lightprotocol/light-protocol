//! Client SDK for the AMM test program.
//!
//! Implements `LightProgramInterface` and `LightAmmInterface` traits to provide
//! a Jupiter-style interface for clients to build decompression instructions.

use std::collections::HashMap;

use anchor_lang::AnchorDeserialize;
use csdk_anchor_full_derived_test::{
    amm_test::{ObservationState, PoolState, AUTH_SEED, POOL_LP_MINT_SIGNER_SEED},
    csdk_anchor_full_derived_test::{
        LightAccountVariant, ObservationStateSeeds, PoolStateSeeds, TokenAccountVariant,
    },
};
use light_client::interface::{
    matches_discriminator, AccountInterface, AccountSpec, AccountToFetch, ColdAccountSpec,
    ColdContext, CreateAccountsProofInput, LightAmmInterface, LightProgramInterface, PdaSpec,
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
    /// Whether this account is compressible (can go cold).
    pub compressible: bool,
}

impl AccountRequirement {
    fn new(pubkey: Option<Pubkey>, kind: AccountKind, compressible: bool) -> Self {
        Self {
            pubkey,
            kind,
            compressible,
        }
    }
}

/// Instruction kinds for the AMM program.
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

#[derive(Debug, Clone)]
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

        let variant = LightAccountVariant::PoolState {
            data: pool,
            amm_config: self.amm_config.unwrap(),
            token_0_mint: self.token_0_mint.unwrap(),
            token_1_mint: self.token_1_mint.unwrap(),
        };

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

        let variant = LightAccountVariant::ObservationState {
            data: observation,
            pool_state,
        };

        let spec = PdaSpec::new(account.clone(), variant, PROGRAM_ID);
        self.program_owned_specs.insert(account.key, spec);

        Ok(())
    }

    fn parse_token_vault(
        &mut self,
        account: &AccountInterface,
        is_vault_0: bool,
    ) -> Result<(), AmmSdkError> {
        use light_token::compat::TokenData;
        use light_token_interface::state::Token;

        let pool_state = self
            .pool_state_pubkey
            .ok_or(AmmSdkError::PoolStateNotParsed)?;

        // Hot accounts use SPL-compatible Token layout, cold use compressed TokenData
        let token_data = if account.is_hot() {
            // On-chain accounts use Token struct (SPL-compatible layout)
            let token = Token::deserialize(&mut &account.data()[..])
                .map_err(|e| AmmSdkError::ParseError(format!("Token deser error: {} data_len={}", e, account.data().len())))?;
            // Convert Token to compressed TokenData format
            TokenData {
                mint: solana_pubkey::Pubkey::new_from_array(token.mint.to_bytes()),
                owner: solana_pubkey::Pubkey::new_from_array(token.owner.to_bytes()),
                amount: token.amount,
                delegate: token
                    .delegate
                    .map(|d| solana_pubkey::Pubkey::new_from_array(d.to_bytes())),
                state: match token.state {
                    light_token_interface::state::AccountState::Initialized => {
                        light_token::compat::AccountState::Initialized
                    }
                    _ => light_token::compat::AccountState::Frozen,
                },
                tlv: None,
            }
        } else {
            // Compressed accounts use TokenData format directly
            TokenData::deserialize(&mut &account.data()[..])
                .map_err(|e| AmmSdkError::ParseError(e.to_string()))?
        };

        let variant = if is_vault_0 {
            let token_0_mint = self
                .token_0_mint
                .ok_or(AmmSdkError::MissingField("token_0_mint"))?;
            LightAccountVariant::CTokenData(light_token::compat::CTokenData {
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
            LightAccountVariant::CTokenData(light_token::compat::CTokenData {
                variant: TokenAccountVariant::Token1Vault {
                    pool_state,
                    token_1_mint,
                },
                token_data,
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

    fn account_requirements(&self, kind: AmmInstruction) -> Vec<AccountRequirement> {
        match kind {
            AmmInstruction::Swap => {
                vec![
                    AccountRequirement::new(self.pool_state_pubkey, AccountKind::Pda, true),
                    AccountRequirement::new(self.token_0_vault, AccountKind::Token, true),
                    AccountRequirement::new(self.token_1_vault, AccountKind::Token, true),
                    AccountRequirement::new(self.observation_key, AccountKind::Pda, true),
                ]
            }
            AmmInstruction::Deposit | AmmInstruction::Withdraw => {
                vec![
                    AccountRequirement::new(self.pool_state_pubkey, AccountKind::Pda, true),
                    AccountRequirement::new(self.token_0_vault, AccountKind::Token, true),
                    AccountRequirement::new(self.token_1_vault, AccountKind::Token, true),
                    AccountRequirement::new(self.observation_key, AccountKind::Pda, true),
                    AccountRequirement::new(self.lp_mint, AccountKind::Mint, true),
                ]
            }
        }
    }

    /// Get all compressible account pubkeys.
    fn all_compressible_accounts(&self) -> Vec<Pubkey> {
        [
            self.pool_state_pubkey,
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

impl LightProgramInterface for AmmSdk {
    type Variant = LightAccountVariant;
    type InstructionKind = AmmInstruction;
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

    fn get_compressible_accounts(&self) -> Vec<Pubkey> {
        self.all_compressible_accounts()
    }

    fn get_accounts_for_instruction(&self, kind: Self::InstructionKind) -> Vec<AccountToFetch> {
        self.account_requirements(kind)
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

    fn get_compressible_accounts_for_instruction(
        &self,
        kind: Self::InstructionKind,
    ) -> Vec<Pubkey> {
        self.account_requirements(kind)
            .into_iter()
            .filter_map(|req| if req.compressible { req.pubkey } else { None })
            .collect()
    }

    fn update_with_interfaces(&mut self, accounts: &[AccountInterface]) -> Result<(), Self::Error> {
        for account in accounts {
            // Handle decompression: if account was cold but now hot, remove from specs
            if account.is_hot() {
                // Remove stale cold entry if account is now hot
                if self
                    .program_owned_specs
                    .get(&account.key)
                    .map_or(false, |s| s.is_cold())
                {
                    self.program_owned_specs.remove(&account.key);
                }
                if self
                    .mint_specs
                    .get(&account.key)
                    .map_or(false, |s| s.is_cold())
                {
                    self.mint_specs.remove(&account.key);
                }
            }
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

    fn get_specs_for_instruction(
        &self,
        kind: Self::InstructionKind,
    ) -> Vec<AccountSpec<Self::Variant>> {
        let requirements = self.account_requirements(kind);
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

    fn get_cold_specs_for_instruction(
        &self,
        kind: Self::InstructionKind,
    ) -> Vec<ColdAccountSpec<Self::Variant>> {
        let requirements = self.account_requirements(kind);
        let mut cold_specs = Vec::new();

        for req in &requirements {
            if !req.compressible {
                continue;
            }
            match req.kind {
                AccountKind::Pda => {
                    if let Some(pubkey) = req.pubkey {
                        if let Some(spec) = self.program_owned_specs.get(&pubkey) {
                            if spec.is_cold() {
                                if let Some(compressed) = spec.compressed() {
                                    cold_specs.push(ColdAccountSpec::Pda {
                                        key: pubkey,
                                        compressed: compressed.clone(),
                                        variant: spec.variant.clone(),
                                        program_id: PROGRAM_ID,
                                    });
                                }
                            }
                        }
                    }
                }
                AccountKind::Token => {
                    if let Some(pubkey) = req.pubkey {
                        if let Some(spec) = self.program_owned_specs.get(&pubkey) {
                            if spec.is_cold() {
                                // Token vaults use ColdContext::Account after conversion
                                if let Some(compressed) = spec.compressed() {
                                    cold_specs.push(ColdAccountSpec::Pda {
                                        key: pubkey,
                                        compressed: compressed.clone(),
                                        variant: spec.variant.clone(),
                                        program_id: PROGRAM_ID,
                                    });
                                }
                            }
                        }
                    }
                }
                AccountKind::Mint => {
                    if let Some(mint_pubkey) = req.pubkey {
                        if let Some(spec) = self.mint_specs.get(&mint_pubkey) {
                            if spec.is_cold() {
                                if let Some(compressed) = spec.as_compressed_account() {
                                    cold_specs.push(ColdAccountSpec::Mint {
                                        key: mint_pubkey,
                                        compressed: compressed.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        cold_specs
    }
}

impl LightAmmInterface for AmmSdk {
    fn swap_instruction_kind(&self) -> Self::InstructionKind {
        AmmInstruction::Swap
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

    /// Creates proof inputs for InitializePool instruction.
    /// Pass on-chain addresses (pool_state PDA, observation_state PDA, lp_mint).
    pub fn create_initialize_pool_proof_inputs(
        pool_state: Pubkey,
        observation_state: Pubkey,
        lp_mint: Pubkey,
    ) -> Vec<CreateAccountsProofInput> {
        vec![
            CreateAccountsProofInput::pda(pool_state),
            CreateAccountsProofInput::pda(observation_state),
            CreateAccountsProofInput::mint(lp_mint),
        ]
    }

    /// Get reserve vault amounts for quoting.
    pub fn get_vault_amounts(&self) -> Option<(u64, u64)> {
        let vault_0 = self.token_0_vault?;
        let vault_1 = self.token_1_vault?;

        let spec_0 = self.program_owned_specs.get(&vault_0)?;
        let spec_1 = self.program_owned_specs.get(&vault_1)?;

        // Parse token amounts from variant
        let amount_0 = match &spec_0.variant {
            LightAccountVariant::CTokenData(ct) => ct.token_data.amount,
            _ => return None,
        };
        let amount_1 = match &spec_1.variant {
            LightAccountVariant::CTokenData(ct) => ct.token_data.amount,
            _ => return None,
        };

        Some((amount_0, amount_1))
    }
}

// =============================================================================
// Jupiter Amm Trait Implementation
// =============================================================================

mod jupiter_impl {
    use super::*;
    use jupiter_amm_interface::{
        AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams, SwapAndAccountMetas,
        SwapParams,
    };

    /// Error type for Jupiter Amm trait operations.
    #[derive(Debug)]
    pub struct JupiterAmmError(pub String);

    impl std::fmt::Display for JupiterAmmError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "JupiterAmmError: {}", self.0)
        }
    }

    impl std::error::Error for JupiterAmmError {}

    impl From<AmmSdkError> for JupiterAmmError {
        fn from(e: AmmSdkError) -> Self {
            JupiterAmmError(e.to_string())
        }
    }

    impl Amm for AmmSdk {
        fn from_keyed_account(
            keyed_account: &KeyedAccount,
            _amm_context: &AmmContext,
        ) -> Result<Self, anyhow::Error>
        where
            Self: Sized,
        {
            let interface = AccountInterface::hot(keyed_account.key, keyed_account.account.clone());
            let sdk = <AmmSdk as LightProgramInterface>::from_keyed_accounts(&[interface])
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            Ok(sdk)
        }

        fn label(&self) -> String {
            "LightAMM".to_string()
        }

        fn program_id(&self) -> Pubkey {
            PROGRAM_ID
        }

        fn key(&self) -> Pubkey {
            self.pool_state_pubkey.unwrap_or_default()
        }

        fn get_reserve_mints(&self) -> Vec<Pubkey> {
            [self.token_0_mint, self.token_1_mint]
                .into_iter()
                .flatten()
                .collect()
        }

        fn get_accounts_to_update(&self) -> Vec<Pubkey> {
            // For Jupiter, return all accounts for swap
            self.get_swap_accounts()
                .into_iter()
                .map(|a| a.pubkey())
                .collect()
        }

        fn update(&mut self, account_map: &AccountMap) -> Result<(), anyhow::Error> {
            // Convert AccountMap entries to AccountInterface
            let interfaces: Vec<AccountInterface> = account_map
                .iter()
                .map(|(key, account)| AccountInterface::hot(*key, account.clone()))
                .collect();

            self.update_with_interfaces(&interfaces)
                .map_err(|e| anyhow::anyhow!("{}", e))
        }

        fn quote(&self, params: &QuoteParams) -> Result<Quote, anyhow::Error> {
            // Simple constant product quote for testing
            let (reserve_0, reserve_1) = self
                .get_vault_amounts()
                .ok_or_else(|| anyhow::anyhow!("Missing vault amounts"))?;

            let (input_reserve, output_reserve) = if params.input_mint == self.token_0_mint.unwrap()
            {
                (reserve_0, reserve_1)
            } else {
                (reserve_1, reserve_0)
            };

            // Constant product: (x + dx) * (y - dy) = x * y
            // dy = y * dx / (x + dx)
            let input_amount = params.amount;
            let output_amount = (output_reserve as u128)
                .checked_mul(input_amount as u128)
                .and_then(|n| n.checked_div((input_reserve as u128) + (input_amount as u128)))
                .ok_or_else(|| anyhow::anyhow!("Quote calculation overflow"))?
                as u64;

            Ok(Quote {
                in_amount: input_amount,
                out_amount: output_amount,
                fee_amount: 0,
                fee_mint: params.input_mint,
                fee_pct: rust_decimal::Decimal::ZERO,
            })
        }

        fn get_swap_and_account_metas(
            &self,
            params: &SwapParams,
        ) -> Result<SwapAndAccountMetas, anyhow::Error> {
            use anchor_lang::ToAccountMetas;
            use csdk_anchor_full_derived_test::amm_test::TradeDirection;

            let pool_state = self
                .pool_state_pubkey
                .ok_or_else(|| anyhow::anyhow!("Pool state not set"))?;
            let authority = self
                .authority
                .ok_or_else(|| anyhow::anyhow!("Authority not set"))?;
            let observation = self
                .observation_key
                .ok_or_else(|| anyhow::anyhow!("Observation not set"))?;

            // Determine direction based on input mint
            let is_zero_for_one = params.source_mint == self.token_0_mint.unwrap();
            let (input_vault, output_vault, input_mint, output_mint) = if is_zero_for_one {
                (
                    self.token_0_vault.unwrap(),
                    self.token_1_vault.unwrap(),
                    self.token_0_mint.unwrap(),
                    self.token_1_mint.unwrap(),
                )
            } else {
                (
                    self.token_1_vault.unwrap(),
                    self.token_0_vault.unwrap(),
                    self.token_1_mint.unwrap(),
                    self.token_0_mint.unwrap(),
                )
            };

            let accounts = csdk_anchor_full_derived_test::accounts::Swap {
                payer: params.source_token_account,
                authority,
                pool_state,
                input_token_account: params.source_token_account,
                output_token_account: params.destination_token_account,
                input_vault,
                output_vault,
                input_token_program: light_token::instruction::LIGHT_TOKEN_PROGRAM_ID,
                output_token_program: light_token::instruction::LIGHT_TOKEN_PROGRAM_ID,
                input_token_mint: input_mint,
                output_token_mint: output_mint,
                observation_state: observation,
            };

            let direction = if is_zero_for_one {
                TradeDirection::ZeroForOne
            } else {
                TradeDirection::OneForZero
            };

            let _ix_data = csdk_anchor_full_derived_test::instruction::Swap {
                amount_in: params.in_amount,
                minimum_amount_out: params.out_amount,
                direction,
            };

            Ok(SwapAndAccountMetas {
                swap: jupiter_amm_interface::Swap::TokenSwap,
                account_metas: accounts.to_account_metas(None),
            })
        }

        fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
            Box::new(self.clone())
        }

        fn has_dynamic_accounts(&self) -> bool {
            false
        }

        fn supports_exact_out(&self) -> bool {
            false
        }

        fn is_active(&self) -> bool {
            self.pool_state_pubkey.is_some()
        }
    }
}

pub use jupiter_impl::JupiterAmmError;
