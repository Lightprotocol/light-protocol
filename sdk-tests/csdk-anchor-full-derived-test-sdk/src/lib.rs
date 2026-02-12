//! Client SDK for the AMM test program.
//!
//! Implements the `LightProgramInterface` trait to produce load specs for cold accounts.

use anchor_lang::AnchorDeserialize;
use csdk_anchor_full_derived_test::{
    amm_test::{ObservationState, PoolState, AUTH_SEED, POOL_LP_MINT_SIGNER_SEED},
    csdk_anchor_full_derived_test::{
        LightAccountVariant, ObservationStateSeeds, PoolStateSeeds, Token0VaultSeeds,
        Token1VaultSeeds,
    },
};
use light_client::interface::{AccountInterface, AccountSpec, LightProgramInterface, PdaSpec};
use solana_pubkey::Pubkey;

/// Program ID for the AMM test program.
pub const PROGRAM_ID: Pubkey = csdk_anchor_full_derived_test::ID;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmmInstruction {
    Swap,
    Deposit,
    Withdraw,
}

#[derive(Debug, Clone)]
pub enum AmmSdkError {
    ParseError(String),
}

impl std::fmt::Display for AmmSdkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for AmmSdkError {}

/// Flat SDK struct. All fields populated at construction from pool state data.
/// No Options, no HashMaps. Seeds and variants built on the fly in `load_specs`.
#[derive(Debug)]
pub struct AmmSdk {
    pub pool_state_pubkey: Pubkey,
    pub amm_config: Pubkey,
    pub token_0_mint: Pubkey,
    pub token_1_mint: Pubkey,
    pub token_0_vault: Pubkey,
    pub token_1_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub observation_key: Pubkey,
    pub authority: Pubkey,
    pub lp_mint_signer: Pubkey,
}

impl AmmSdk {
    /// Construct from pool state pubkey and its account data.
    /// Parses PoolState once, extracts all dependent addresses.
    pub fn new(pool_state_pubkey: Pubkey, pool_data: &[u8]) -> Result<Self, AmmSdkError> {
        let pool = PoolState::deserialize(&mut &pool_data[8..])
            .map_err(|e| AmmSdkError::ParseError(e.to_string()))?;

        let (authority, _) = Pubkey::find_program_address(&[AUTH_SEED.as_bytes()], &PROGRAM_ID);
        let (lp_mint_signer, _) = Pubkey::find_program_address(
            &[POOL_LP_MINT_SIGNER_SEED, pool_state_pubkey.as_ref()],
            &PROGRAM_ID,
        );

        Ok(Self {
            pool_state_pubkey,
            amm_config: pool.amm_config,
            token_0_mint: pool.token_0_mint,
            token_1_mint: pool.token_1_mint,
            token_0_vault: pool.token_0_vault,
            token_1_vault: pool.token_1_vault,
            lp_mint: pool.lp_mint,
            observation_key: pool.observation_key,
            authority,
            lp_mint_signer,
        })
    }

    pub fn derive_lp_mint_compressed_address(&self, address_tree: &Pubkey) -> [u8; 32] {
        light_compressed_token_sdk::compressed_token::create_compressed_mint::derive_mint_compressed_address(
            &self.lp_mint_signer,
            address_tree,
        )
    }
}

impl LightProgramInterface for AmmSdk {
    type Variant = LightAccountVariant;
    type Instruction = AmmInstruction;
    type Error = AmmSdkError;

    fn program_id() -> Pubkey {
        PROGRAM_ID
    }

    fn instruction_accounts(&self, ix: &Self::Instruction) -> Vec<Pubkey> {
        match ix {
            AmmInstruction::Swap => vec![
                self.pool_state_pubkey,
                self.token_0_vault,
                self.token_1_vault,
                self.observation_key,
            ],
            AmmInstruction::Deposit | AmmInstruction::Withdraw => vec![
                self.pool_state_pubkey,
                self.token_0_vault,
                self.token_1_vault,
                self.observation_key,
                self.lp_mint,
            ],
        }
    }

    fn load_specs(
        &self,
        cold_accounts: &[AccountInterface],
    ) -> Result<Vec<AccountSpec<Self::Variant>>, Self::Error> {
        use light_account::{token::TokenDataWithSeeds, Token};

        let mut specs = Vec::new();
        for account in cold_accounts {
            if account.key == self.pool_state_pubkey {
                let pool = PoolState::deserialize(&mut &account.data()[8..])
                    .map_err(|e| AmmSdkError::ParseError(e.to_string()))?;
                let variant = LightAccountVariant::PoolState {
                    seeds: PoolStateSeeds {
                        amm_config: self.amm_config,
                        token_0_mint: self.token_0_mint,
                        token_1_mint: self.token_1_mint,
                    },
                    data: pool,
                };
                specs.push(AccountSpec::Pda(PdaSpec::new(
                    account.clone(),
                    variant,
                    PROGRAM_ID,
                )));
            } else if account.key == self.observation_key {
                let observation = ObservationState::deserialize(&mut &account.data()[8..])
                    .map_err(|e| AmmSdkError::ParseError(e.to_string()))?;
                let variant = LightAccountVariant::ObservationState {
                    seeds: ObservationStateSeeds {
                        pool_state: self.pool_state_pubkey,
                    },
                    data: observation,
                };
                specs.push(AccountSpec::Pda(PdaSpec::new(
                    account.clone(),
                    variant,
                    PROGRAM_ID,
                )));
            } else if account.key == self.token_0_vault {
                let token: Token = Token::deserialize(&mut &account.data()[..])
                    .map_err(|e| AmmSdkError::ParseError(e.to_string()))?;
                let variant = LightAccountVariant::Token0Vault(TokenDataWithSeeds {
                    seeds: Token0VaultSeeds {
                        pool_state: self.pool_state_pubkey,
                        token_0_mint: self.token_0_mint,
                    },
                    token_data: token,
                });
                specs.push(AccountSpec::Pda(PdaSpec::new(
                    account.clone(),
                    variant,
                    PROGRAM_ID,
                )));
            } else if account.key == self.token_1_vault {
                let token: Token = Token::deserialize(&mut &account.data()[..])
                    .map_err(|e| AmmSdkError::ParseError(e.to_string()))?;
                let variant = LightAccountVariant::Token1Vault(TokenDataWithSeeds {
                    seeds: Token1VaultSeeds {
                        pool_state: self.pool_state_pubkey,
                        token_1_mint: self.token_1_mint,
                    },
                    token_data: token,
                });
                specs.push(AccountSpec::Pda(PdaSpec::new(
                    account.clone(),
                    variant,
                    PROGRAM_ID,
                )));
            } else if account.key == self.lp_mint {
                specs.push(AccountSpec::Mint(account.clone()));
            }
        }
        Ok(specs)
    }
}
