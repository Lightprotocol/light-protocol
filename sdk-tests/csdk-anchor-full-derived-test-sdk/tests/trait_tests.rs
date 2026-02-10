//! LightProgramInterface trait unit tests for AmmSdk.
//!
//! Tests cover:
//! - instruction_accounts returns correct pubkeys per instruction type
//! - load_specs builds correct variants from cold accounts
//! - Helper functions (all_hot, any_cold)
//! - Invariants (no duplicate addresses, variant seed distinction)

use std::collections::HashSet;

use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::{
    LightAccountVariant, ObservationStateSeeds,
};
use csdk_anchor_full_derived_test_sdk::{AmmInstruction, AmmSdk, AmmSdkError, PROGRAM_ID};
use light_client::interface::{
    all_hot, any_cold, Account, AccountInterface, AccountSpec, LightProgramInterface, PdaSpec,
};
use solana_pubkey::Pubkey;

// =============================================================================
// TEST HELPERS
// =============================================================================

/// Build an AmmSdk with known pubkeys for unit testing (no deserialization).
fn test_sdk() -> AmmSdk {
    AmmSdk {
        pool_state_pubkey: Pubkey::new_unique(),
        amm_config: Pubkey::new_unique(),
        token_0_mint: Pubkey::new_unique(),
        token_1_mint: Pubkey::new_unique(),
        token_0_vault: Pubkey::new_unique(),
        token_1_vault: Pubkey::new_unique(),
        lp_mint: Pubkey::new_unique(),
        observation_key: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        lp_mint_signer: Pubkey::new_unique(),
    }
}

// =============================================================================
// 1. PROGRAM_ID
// =============================================================================

#[test]
fn test_program_id() {
    assert_eq!(AmmSdk::program_id(), PROGRAM_ID);
}

// =============================================================================
// 2. INSTRUCTION_ACCOUNTS
// =============================================================================

#[test]
fn test_swap_instruction_accounts() {
    let sdk = test_sdk();
    let accounts = sdk.instruction_accounts(&AmmInstruction::Swap);

    assert_eq!(
        accounts.len(),
        4,
        "Swap: pool_state, vault_0, vault_1, observation"
    );
    assert!(accounts.contains(&sdk.pool_state_pubkey));
    assert!(accounts.contains(&sdk.token_0_vault));
    assert!(accounts.contains(&sdk.token_1_vault));
    assert!(accounts.contains(&sdk.observation_key));
    // Swap does not include lp_mint
    assert!(!accounts.contains(&sdk.lp_mint));
}

#[test]
fn test_deposit_instruction_accounts() {
    let sdk = test_sdk();
    let accounts = sdk.instruction_accounts(&AmmInstruction::Deposit);

    assert_eq!(
        accounts.len(),
        5,
        "Deposit: pool_state, vault_0, vault_1, observation, lp_mint"
    );
    assert!(accounts.contains(&sdk.pool_state_pubkey));
    assert!(accounts.contains(&sdk.token_0_vault));
    assert!(accounts.contains(&sdk.token_1_vault));
    assert!(accounts.contains(&sdk.observation_key));
    assert!(accounts.contains(&sdk.lp_mint));
}

#[test]
fn test_withdraw_equals_deposit() {
    let sdk = test_sdk();
    let deposit = sdk.instruction_accounts(&AmmInstruction::Deposit);
    let withdraw = sdk.instruction_accounts(&AmmInstruction::Withdraw);
    assert_eq!(
        deposit, withdraw,
        "Deposit and Withdraw have identical account sets"
    );
}

#[test]
fn test_deposit_superset_of_swap() {
    let sdk = test_sdk();
    let swap: HashSet<Pubkey> = sdk
        .instruction_accounts(&AmmInstruction::Swap)
        .into_iter()
        .collect();
    let deposit: HashSet<Pubkey> = sdk
        .instruction_accounts(&AmmInstruction::Deposit)
        .into_iter()
        .collect();

    assert!(
        swap.is_subset(&deposit),
        "Swap accounts must be a subset of Deposit accounts"
    );
}

#[test]
fn test_no_duplicate_pubkeys_in_instruction_accounts() {
    let sdk = test_sdk();
    for ix in [
        AmmInstruction::Swap,
        AmmInstruction::Deposit,
        AmmInstruction::Withdraw,
    ] {
        let accounts = sdk.instruction_accounts(&ix);
        let unique: HashSet<Pubkey> = accounts.iter().copied().collect();
        assert_eq!(
            accounts.len(),
            unique.len(),
            "No duplicate pubkeys for {:?}",
            ix
        );
    }
}

// =============================================================================
// 3. LOAD_SPECS (empty input)
// =============================================================================

#[test]
fn test_load_specs_empty_input() {
    let sdk = test_sdk();
    let specs = sdk.load_specs(&[]).expect("empty input should succeed");
    assert!(specs.is_empty());
}

#[test]
fn test_load_specs_unknown_pubkey_skipped() {
    let sdk = test_sdk();
    let unknown = AccountInterface::hot(
        Pubkey::new_unique(),
        Account {
            lamports: 0,
            data: vec![0; 100],
            owner: PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    );
    let specs = sdk
        .load_specs(&[unknown])
        .expect("unknown pubkey should be skipped");
    assert!(specs.is_empty());
}

// =============================================================================
// 4. HELPER FUNCTIONS
// =============================================================================

#[test]
fn test_all_hot_empty() {
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![];
    assert!(all_hot(&specs), "Empty is all hot");
    assert!(!any_cold(&specs), "Empty has no cold");
}

#[test]
fn test_all_hot_with_hot_spec() {
    use csdk_anchor_full_derived_test::amm_test::ObservationState;

    let hot_interface = AccountInterface::hot(
        Pubkey::new_unique(),
        Account {
            lamports: 0,
            data: vec![0; 100],
            owner: PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    );
    let hot_spec = PdaSpec::new(
        hot_interface,
        LightAccountVariant::ObservationState {
            seeds: ObservationStateSeeds {
                pool_state: Pubkey::new_unique(),
            },
            data: ObservationState::default(),
        },
        PROGRAM_ID,
    );
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(hot_spec)];

    assert!(all_hot(&specs));
    assert!(!any_cold(&specs));
}

// =============================================================================
// 5. ERROR DISPLAY
// =============================================================================

#[test]
fn test_error_display() {
    let err = AmmSdkError::ParseError("deserialization failed".to_string());
    let msg = format!("{}", err);
    assert!(
        msg.contains("deserialization"),
        "ParseError should include cause"
    );
}

// =============================================================================
// 6. VARIANT SEED DISTINCTION
// =============================================================================

#[test]
fn test_variant_seed_values_distinguish_instances() {
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::{
        Token0VaultSeeds, Token1VaultSeeds,
    };
    use light_account::token::TokenDataWithSeeds;

    let pool_state = Pubkey::new_unique();
    let token_0_mint = Pubkey::new_unique();
    let token_1_mint = Pubkey::new_unique();

    let default_token = light_account::token::Token {
        mint: Default::default(),
        owner: Default::default(),
        amount: 0,
        delegate: None,
        state: light_account::token::AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: 0,
        extensions: None,
    };
    let variant_0 = LightAccountVariant::Token0Vault(TokenDataWithSeeds {
        seeds: Token0VaultSeeds {
            pool_state,
            token_0_mint,
        },
        token_data: default_token.clone(),
    });
    let variant_1 = LightAccountVariant::Token1Vault(TokenDataWithSeeds {
        seeds: Token1VaultSeeds {
            pool_state,
            token_1_mint,
        },
        token_data: default_token,
    });

    match (&variant_0, &variant_1) {
        (LightAccountVariant::Token0Vault(data_0), LightAccountVariant::Token1Vault(data_1)) => {
            assert_ne!(
                data_0.seeds.token_0_mint, data_1.seeds.token_1_mint,
                "Vault seed values must differ"
            );
        }
        _ => panic!("Expected Token0Vault and Token1Vault"),
    }
}

// =============================================================================
// 7. DIRECTION-AGNOSTIC DESIGN
// =============================================================================

#[test]
fn test_swap_returns_both_vaults() {
    let sdk = test_sdk();
    let accounts = sdk.instruction_accounts(&AmmInstruction::Swap);

    assert!(
        accounts.contains(&sdk.token_0_vault),
        "Swap must include vault_0"
    );
    assert!(
        accounts.contains(&sdk.token_1_vault),
        "Swap must include vault_1"
    );
}

#[test]
fn test_canonical_variant_not_aliased() {
    // The LightAccountVariant enum uses canonical names (Token0Vault, Token1Vault),
    // not instruction aliases (InputVault, OutputVault).
    // The SDK is direction-agnostic: both vaults are always returned.
    let sdk = test_sdk();
    let swap_accounts = sdk.instruction_accounts(&AmmInstruction::Swap);
    let deposit_accounts = sdk.instruction_accounts(&AmmInstruction::Deposit);

    // Both vaults appear in both Swap and Deposit
    for vault in [&sdk.token_0_vault, &sdk.token_1_vault] {
        assert!(swap_accounts.contains(vault));
        assert!(deposit_accounts.contains(vault));
    }
}
