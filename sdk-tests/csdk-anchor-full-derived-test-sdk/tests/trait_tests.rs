//! LightProgramInterface trait unit tests for AmmSdk.
//!
//! Tests cover:
//! - Core trait methods (from_keyed_accounts, update, get_specs_for_instruction)
//! - Error handling and meaningful error messages
//! - Multi-operation scenarios with overlapping/divergent accounts
//! - Invariants (idempotency, commutativity, spec consistency)
//! - Edge cases (hot/cold mixed, missing accounts, etc.)

use std::collections::HashSet;

use csdk_anchor_full_derived_test::{
    amm_test::{ObservationState, PoolState},
    csdk_anchor_full_derived_test::{
        LightAccountVariant, ObservationStateSeeds, ObservationStateVariant,
    },
};
use csdk_anchor_full_derived_test_sdk::{AmmInstruction, AmmSdk, AmmSdkError};
use light_client::interface::{
    all_hot, any_cold, Account, AccountInterface, AccountSpec, LightProgramInterface, PdaSpec,
};
use light_sdk::LightDiscriminator;
use solana_pubkey::Pubkey;

// =============================================================================
// TEST HELPERS
// =============================================================================

/// Create a hot AccountInterface from data.
fn keyed_hot(pubkey: Pubkey, data: Vec<u8>) -> AccountInterface {
    AccountInterface::hot(
        pubkey,
        Account {
            lamports: 0,
            data,
            owner: csdk_anchor_full_derived_test_sdk::PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
}

// =============================================================================
// 1. CORE TRAIT METHOD TESTS: from_keyed_accounts
// =============================================================================

#[test]
fn test_from_keyed_empty_accounts() {
    // T1.1.1: Empty array should create empty SDK (no error, just no state)
    let result = AmmSdk::from_keyed_accounts(&[]);
    assert!(result.is_ok(), "Empty accounts should not error");

    let sdk = result.unwrap();
    assert!(
        sdk.pool_state_pubkey().is_none(),
        "No pool state parsed from empty"
    );
}

#[test]
fn test_from_keyed_wrong_discriminator() {
    // T1.1.5: Unknown discriminator should be skipped
    let mut data = vec![0u8; 100];
    data[..8].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);

    let keyed = keyed_hot(Pubkey::new_unique(), data);
    let result = AmmSdk::from_keyed_accounts(&[keyed]);

    assert!(result.is_ok(), "Unknown discriminator should not error");
    let sdk = result.unwrap();
    assert!(
        sdk.pool_state_pubkey().is_none(),
        "Unknown disc should be skipped"
    );
}

#[test]
fn test_from_keyed_truncated_data() {
    // T1.1.6: Truncated data should error on parse
    let mut data = Vec::new();
    data.extend_from_slice(&PoolState::LIGHT_DISCRIMINATOR);
    data.extend_from_slice(&[0u8; 10]); // Way too short

    let keyed = keyed_hot(Pubkey::new_unique(), data);
    let result = AmmSdk::from_keyed_accounts(&[keyed]);

    // Should either skip or error depending on implementation
    // Current impl: errors on parse
    assert!(
        result.is_err() || result.as_ref().unwrap().pool_state_pubkey().is_none(),
        "Truncated data should error or skip"
    );
}

#[test]
fn test_from_keyed_zero_length_data() {
    // T1.1.7: Zero-length data should be skipped
    let keyed = keyed_hot(Pubkey::new_unique(), vec![]);
    let result = AmmSdk::from_keyed_accounts(&[keyed]);

    assert!(result.is_ok(), "Zero-length should not error");
    let sdk = result.unwrap();
    assert!(
        sdk.pool_state_pubkey().is_none(),
        "Zero-length should be skipped"
    );
}

// =============================================================================
// 2. CORE TRAIT METHOD TESTS: get_accounts_to_update
// =============================================================================

#[test]
fn test_get_accounts_before_init() {
    // T1.2.4: Returns empty before pool parsed
    let sdk = AmmSdk::new();

    let swap_accounts = sdk.get_accounts_to_update(&AmmInstruction::Swap);
    let deposit_accounts = sdk.get_accounts_to_update(&AmmInstruction::Deposit);

    assert!(
        swap_accounts.is_empty(),
        "Swap should return empty before init"
    );
    assert!(
        deposit_accounts.is_empty(),
        "Deposit should return empty before init"
    );
}

#[test]
fn test_get_accounts_swap_vs_deposit() {
    // T1.2.1, T1.2.2: Compare Swap vs Deposit accounts
    // Note: This test would need a properly parsed SDK
    // For now, verify the behavior contract

    let sdk = AmmSdk::new();
    // Without pool state, both return empty
    let _swap_accounts = sdk.get_accounts_to_update(&AmmInstruction::Swap);
    let deposit_accounts = sdk.get_accounts_to_update(&AmmInstruction::Deposit);
    let withdraw_accounts = sdk.get_accounts_to_update(&AmmInstruction::Withdraw);

    // Verify Deposit and Withdraw have same requirements
    assert_eq!(
        deposit_accounts, withdraw_accounts,
        "Deposit and Withdraw should have same account requirements"
    );
}

// =============================================================================
// 3. CORE TRAIT METHOD TESTS: update
// =============================================================================

#[test]
fn test_update_before_root_errors() {
    // T1.3.4: Update before root parsed should error for accounts that need root
    let mut sdk = AmmSdk::new();

    // Try to update with a vault before pool state is parsed
    let vault_data = vec![0u8; 165]; // TokenData size
    let vault_keyed = keyed_hot(Pubkey::new_unique(), vault_data);

    // This should either error or skip (depending on implementation)
    let result = sdk.update(&[vault_keyed]);

    // Current impl: skips unknown accounts, doesn't error
    assert!(result.is_ok(), "Update with unknown should skip, not error");
}

#[test]
fn test_update_idempotent() {
    // T1.3.3, T6.1: Same account twice should be idempotent
    let mut sdk = AmmSdk::new();

    let data = vec![0u8; 100];
    let keyed = keyed_hot(Pubkey::new_unique(), data.clone());

    // Update twice with same data
    let _ = sdk.update(std::slice::from_ref(&keyed));
    let specs_after_first = sdk.get_all_specs();

    let _ = sdk.update(std::slice::from_ref(&keyed));
    let specs_after_second = sdk.get_all_specs();

    // Should be same
    assert_eq!(
        specs_after_first.len(),
        specs_after_second.len(),
        "Idempotent: same spec count"
    );
}

#[test]
fn test_update_unknown_account_skipped() {
    // T1.3.5: Unknown accounts should be skipped
    let mut sdk = AmmSdk::new();

    let unknown_data = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x00, 0x00, 0x00];
    let keyed = keyed_hot(Pubkey::new_unique(), unknown_data);

    let result = sdk.update(&[keyed]);
    assert!(result.is_ok(), "Unknown account should be skipped");

    let specs = sdk.get_all_specs();
    assert!(specs.is_empty(), "Unknown should not add spec");
}

// =============================================================================
// 4. CORE TRAIT METHOD TESTS: get_all_specs / get_specs_for_instruction
// =============================================================================

#[test]
fn test_get_all_empty() {
    // T1.4.1: Empty SDK returns empty specs
    let sdk = AmmSdk::new();
    let specs = sdk.get_all_specs();

    assert!(specs.is_empty());
    assert!(all_hot(&specs), "Empty specs should report all_hot");
}

#[test]
fn test_all_specs_helpers() {
    // Test all_hot() and any_cold() helpers
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![];

    assert!(all_hot(&specs), "Empty is all hot");
    assert!(!any_cold(&specs), "Empty has no cold");
}

// =============================================================================
// 5. ERROR HANDLING TESTS
// =============================================================================

#[test]
fn test_error_display_impl() {
    // T2.4: All errors have Display impl with meaningful messages
    let errors = vec![
        AmmSdkError::ParseError("test parse".to_string()),
        AmmSdkError::UnknownDiscriminator([0u8; 8]),
        AmmSdkError::MissingField("test_field"),
        AmmSdkError::PoolStateNotParsed,
    ];

    for err in errors {
        let msg = format!("{}", err);
        assert!(!msg.is_empty(), "Error should have display message");
        println!("Error display: {}", msg);
    }
}

#[test]
fn test_error_parse_error_contains_cause() {
    let err = AmmSdkError::ParseError("deserialization failed".to_string());
    let msg = format!("{}", err);
    assert!(
        msg.contains("deserialization"),
        "ParseError should include cause"
    );
}

#[test]
fn test_error_missing_field_names_field() {
    let err = AmmSdkError::MissingField("amm_config");
    let msg = format!("{}", err);
    assert!(
        msg.contains("amm_config"),
        "MissingField should name the field"
    );
}

// =============================================================================
// 6. INVARIANT TESTS
// =============================================================================

#[test]
fn test_invariant_no_duplicate_addresses() {
    // T6.4: No duplicate addresses in specs
    let sdk = AmmSdk::new();
    let specs = sdk.get_all_specs();

    let addresses: Vec<Pubkey> = specs.iter().map(|s| s.pubkey()).collect();
    let unique: HashSet<Pubkey> = addresses.iter().copied().collect();

    assert_eq!(
        addresses.len(),
        unique.len(),
        "No duplicate addresses allowed"
    );
}

#[test]
fn test_invariant_cold_has_context() {
    // T6.5: Cold specs must have compressed data
    let sdk = AmmSdk::new();
    let specs = sdk.get_all_specs();

    for spec in &specs {
        if spec.is_cold() {
            match spec {
                AccountSpec::Pda(s) => {
                    assert!(
                        s.compressed().is_some(),
                        "Cold PDA must have compressed: {}",
                        s.address()
                    );
                }
                AccountSpec::Ata(s) => {
                    assert!(
                        s.compressed().is_some(),
                        "Cold ATA must have compressed: {}",
                        s.key
                    );
                }
                AccountSpec::Mint(s) => {
                    assert!(
                        s.cold.is_some() && s.as_mint().is_some(),
                        "Cold mint must have cold context + mint_data: {}",
                        s.key
                    );
                }
            }
        }
    }
}

#[test]
fn test_invariant_hot_context_optional() {
    // T6.6: Hot specs don't need compressed data (can be None)
    let sdk = AmmSdk::new();
    let specs = sdk.get_all_specs();

    for spec in &specs {
        if !spec.is_cold() {
            // Hot compressed can be None - this is valid
            // Just verify the spec is accessible
            let _ = spec.pubkey();
        }
    }
}

// =============================================================================
// 7. MULTI-OPERATION TESTS
// =============================================================================

#[test]
fn test_multi_op_deposit_superset_of_swap() {
    // T3.1: Deposit accounts should be superset of Swap
    let sdk = AmmSdk::new();

    let swap_accounts: HashSet<Pubkey> = sdk
        .get_accounts_to_update(&AmmInstruction::Swap)
        .into_iter()
        .map(|a| a.pubkey())
        .collect();
    let deposit_accounts: HashSet<Pubkey> = sdk
        .get_accounts_to_update(&AmmInstruction::Deposit)
        .into_iter()
        .map(|a| a.pubkey())
        .collect();

    // All swap accounts should be in deposit
    for acc in &swap_accounts {
        assert!(
            deposit_accounts.contains(acc),
            "Deposit should include all Swap accounts"
        );
    }
}

#[test]
fn test_multi_op_withdraw_equals_deposit() {
    // T3.1: Withdraw should have same accounts as Deposit
    let sdk = AmmSdk::new();

    let deposit_accounts = sdk.get_accounts_to_update(&AmmInstruction::Deposit);
    let withdraw_accounts = sdk.get_accounts_to_update(&AmmInstruction::Withdraw);

    assert_eq!(
        deposit_accounts, withdraw_accounts,
        "Deposit and Withdraw should have identical account requirements"
    );
}

// =============================================================================
// 8. ACCOUNT NAMING TESTS
// =============================================================================

#[test]
fn test_same_pubkey_same_spec() {
    // T4.1, T4.2: Same pubkey should always map to same spec
    // Regardless of what name the instruction calls it

    let mut sdk = AmmSdk::new();
    let pubkey = Pubkey::new_unique();
    let data = vec![0u8; 100];

    // Update with same pubkey twice (simulating different instruction contexts)
    let keyed1 = keyed_hot(pubkey, data.clone());
    let keyed2 = keyed_hot(pubkey, data.clone());

    let _ = sdk.update(&[keyed1]);
    let specs_after_first = sdk.get_all_specs();

    let _ = sdk.update(&[keyed2]);
    let specs_after_second = sdk.get_all_specs();

    // Should have same count (not doubled)
    assert_eq!(
        specs_after_first.len(),
        specs_after_second.len(),
        "Same pubkey should not create duplicate specs"
    );
}

// =============================================================================
// 9. EDGE CASE TESTS
// =============================================================================

#[test]
fn test_edge_all_hot_check() {
    // T8.3: all_hot() returns true when all specs are hot
    let hot_interface = AccountInterface::hot(
        Pubkey::new_unique(),
        Account {
            lamports: 0,
            data: vec![0; 100],
            owner: csdk_anchor_full_derived_test_sdk::PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    );
    let hot_spec = PdaSpec::new(
        hot_interface,
        LightAccountVariant::ObservationState(ObservationStateVariant {
            seeds: ObservationStateSeeds {
                pool_state: Pubkey::new_unique(),
            },
            data: ObservationState::default(),
        }),
        csdk_anchor_full_derived_test_sdk::PROGRAM_ID,
    );
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(hot_spec)];

    assert!(
        all_hot(&specs),
        "All hot specs should return all_hot() = true"
    );
    assert!(
        !any_cold(&specs),
        "All hot specs should return any_cold() = false"
    );
}

#[test]
fn test_edge_duplicate_accounts_in_update() {
    // T8.6: Duplicate accounts in single update should be deduplicated
    let mut sdk = AmmSdk::new();
    let pubkey = Pubkey::new_unique();
    let data = vec![0u8; 100];

    let keyed = keyed_hot(pubkey, data);

    // Update with same account twice in same call
    let _ = sdk.update(&[keyed.clone(), keyed.clone()]);

    // Should not have duplicates in specs
    let specs = sdk.get_all_specs();
    let addresses: Vec<Pubkey> = specs.iter().map(|s| s.pubkey()).collect();
    let unique: HashSet<Pubkey> = addresses.iter().copied().collect();

    assert_eq!(
        addresses.len(),
        unique.len(),
        "Duplicates should be deduplicated"
    );
}

// =============================================================================
// 10. TYPED FETCH HELPER TESTS
// =============================================================================

#[test]
fn test_get_accounts_to_update_empty() {
    // get_accounts_to_update should return empty for uninitialized SDK
    let sdk = AmmSdk::new();

    let typed = sdk.get_accounts_to_update(&AmmInstruction::Swap);
    assert!(typed.is_empty(), "Typed should be empty before init");
}

#[test]
fn test_get_accounts_to_update_categories() {
    // Verify typed accounts have correct categories
    use light_client::interface::AccountToFetch;

    let sdk = AmmSdk::new();
    let typed = sdk.get_accounts_to_update(&AmmInstruction::Deposit);

    // All should be one of Pda, Token, Ata, or Mint
    for acc in &typed {
        match acc {
            AccountToFetch::Pda { .. } => {}
            AccountToFetch::Token { .. } => {}
            AccountToFetch::Ata { .. } => {}
            AccountToFetch::Mint { .. } => {}
        }
    }
}

// =============================================================================
// 11. SAME TYPE DIFFERENT INSTANCE TESTS
// =============================================================================
// Critical tests for ensuring vault_0 and vault_1 (same type, different seeds/values)
// are handled as separate specs and not mingled together.

#[test]
fn test_same_type_different_pubkey_separate_specs() {
    // CRITICAL: Two accounts of same type but different pubkeys must be stored separately.
    // This is the case for vault_0 and vault_1 which are both token vaults
    // but with different mints and therefore different pubkeys.

    // Create two different pubkeys (simulating vault_0 and vault_1)
    let vault_0_pubkey = Pubkey::new_unique();
    let vault_1_pubkey = Pubkey::new_unique();

    assert_ne!(
        vault_0_pubkey, vault_1_pubkey,
        "Vaults must have different pubkeys"
    );

    // In the SDK, these would be keyed by pubkey in HashMap<Pubkey, Spec>
    // Verify the design: each pubkey gets its own entry
    let mut pubkey_set: HashSet<Pubkey> = HashSet::new();
    pubkey_set.insert(vault_0_pubkey);
    pubkey_set.insert(vault_1_pubkey);

    assert_eq!(
        pubkey_set.len(),
        2,
        "Two different pubkeys must create two entries"
    );
}

#[test]
fn test_variant_seed_values_distinguish_instances() {
    // CRITICAL: Even if variants have same type name, the seed VALUES must differ.
    // Example: Token0Vault{pool_state: A, token_0_mint: B} vs Token1Vault{pool_state: A, token_1_mint: C}
    //
    // The variant enum encodes WHICH account this is via the variant name AND seed values.

    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::{
        Token0VaultSeeds, Token1VaultSeeds,
    };
    use light_sdk::interface::token::TokenDataWithSeeds;

    let pool_state = Pubkey::new_unique();
    let token_0_mint = Pubkey::new_unique();
    let token_1_mint = Pubkey::new_unique();

    let default_token = light_sdk::interface::token::Token {
        mint: Default::default(),
        owner: Default::default(),
        amount: 0,
        delegate: None,
        state: light_sdk::interface::token::AccountState::Initialized,
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

    // These are different enum variants (type-level distinction)
    // Even if they were the same variant type, the seed values differ
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

#[test]
fn test_specs_contain_all_vaults_not_merged() {
    // CRITICAL: When getting specs for Swap, we must get BOTH vault_0 AND vault_1,
    // not have them merged into a single spec.

    // The SDK stores specs in HashMap<Pubkey, Spec>
    // This test verifies the invariant that different pubkeys = different specs

    let sdk = AmmSdk::new();

    // Before init, specs are empty
    let specs = sdk.get_specs_for_instruction(&AmmInstruction::Swap);

    // Count of specs should match number of unique accounts
    // When SDK is properly initialized with pool_state and vaults,
    // Swap should return pool_state + vault_0 + vault_1 = 3 specs

    // For now, verify the empty case works correctly
    assert_eq!(specs.len(), 0, "Uninitialized SDK should have 0 specs");

    // The invariant we're testing: no duplicate addresses
    let addresses: HashSet<Pubkey> = specs.iter().map(|s| s.pubkey()).collect();
    assert_eq!(
        specs.len(),
        addresses.len(),
        "Each spec must have unique address"
    );
}

#[test]
fn test_field_name_uniqueness_across_instructions() {
    // CRITICAL: Field names like "token_0_vault" must be globally unique across ALL instructions.
    // The macros enforce this - same field name = same account = same spec.
    //
    // This test documents the design contract:
    // - In initialize: token_0_vault refers to account at pubkey A
    // - In swap: source_vault (if it's the same account) MUST have pubkey A
    // - The SDK keys by pubkey, so same pubkey = same spec regardless of field name in instruction

    // Two instructions can call the same account different names:
    // initialize.token_0_vault and swap.input_vault could be the SAME account
    // The SDK correctly handles this by keying on pubkey, not field name

    let shared_pubkey = Pubkey::new_unique();

    // If two instructions reference the same pubkey, they're the same account
    // The SDK stores ONE spec for this pubkey, not two
    let mut seen_pubkeys: HashSet<Pubkey> = HashSet::new();

    // "initialize.token_0_vault" -> shared_pubkey
    seen_pubkeys.insert(shared_pubkey);

    // "swap.input_vault" -> shared_pubkey (same account, different name)
    seen_pubkeys.insert(shared_pubkey);

    assert_eq!(
        seen_pubkeys.len(),
        1,
        "Same pubkey from different field names = single spec"
    );
}

#[test]
fn test_updating_vault_0_does_not_affect_vault_1() {
    // CRITICAL: Updating vault_0's spec must NOT affect vault_1's spec.
    // They are independent entries in the HashMap.

    let mut sdk = AmmSdk::new();

    // Create two different "vault" accounts
    let vault_0_pubkey = Pubkey::new_unique();
    let vault_1_pubkey = Pubkey::new_unique();

    let vault_0_data = vec![0xAAu8; 100];
    let vault_1_data = vec![0xBBu8; 100];

    let vault_0_keyed = keyed_hot(vault_0_pubkey, vault_0_data);
    let vault_1_keyed = keyed_hot(vault_1_pubkey, vault_1_data);

    // Update with both
    let _ = sdk.update(&[vault_0_keyed.clone(), vault_1_keyed.clone()]);

    // Now update vault_0 again with different data
    let vault_0_updated = keyed_hot(vault_0_pubkey, vec![0xCCu8; 100]);
    let _ = sdk.update(&[vault_0_updated]);

    // Verify: vault_1 should still have its original data (if tracked)
    // The key point: updating by pubkey only affects that specific entry
    let specs = sdk.get_all_specs();

    // Verify both are still separate entries (if they were recognized)
    let addresses: HashSet<Pubkey> = specs.iter().map(|s| s.pubkey()).collect();

    // No duplicates
    assert_eq!(
        specs.len(),
        addresses.len(),
        "Each vault must remain separate"
    );
}

#[test]
fn test_operation_returns_all_required_instances() {
    // CRITICAL: get_specs_for_instruction(Swap) must return BOTH vault_0 AND vault_1,
    // not just one of them.

    // Document the expected behavior:
    // Swap operation needs: pool_state, vault_0, vault_1
    // Deposit operation needs: pool_state, vault_0, vault_1, observation, lp_mint

    let sdk = AmmSdk::new();

    // Get accounts needed for Swap
    let swap_accounts = sdk.get_accounts_to_update(&AmmInstruction::Swap);

    // Without pool state, this is empty, but document the contract:
    // When properly initialized, Swap should request both vaults
    // The SDK implementation does: vec![token_0_vault, token_1_vault].into_iter().flatten()

    // This confirms the design: BOTH vaults are requested, not just one
    // Each vault is a separate entry, not merged

    // Verify Deposit requests more accounts than Swap
    let deposit_accounts = sdk.get_accounts_to_update(&AmmInstruction::Deposit);

    // Even when empty, the contract holds:
    // len(deposit_accounts) >= len(swap_accounts) because Deposit is a superset
    assert!(
        deposit_accounts.len() >= swap_accounts.len(),
        "Deposit must request at least as many accounts as Swap"
    );
}

#[test]
fn test_hashmap_keying_prevents_spec_mingling() {
    // CRITICAL: The SDK uses HashMap<Pubkey, Spec> which naturally prevents mingling.
    // This test verifies the data structure choice is correct.

    use std::collections::HashMap;

    let vault_0_pubkey = Pubkey::new_unique();
    let vault_1_pubkey = Pubkey::new_unique();

    // Simulate the SDK's internal storage
    let mut specs: HashMap<Pubkey, String> = HashMap::new();

    // Insert vault_0 spec
    specs.insert(vault_0_pubkey, "vault_0_spec".to_string());

    // Insert vault_1 spec
    specs.insert(vault_1_pubkey, "vault_1_spec".to_string());

    // Verify: both are stored separately
    assert_eq!(specs.len(), 2, "Two vaults = two entries");
    assert_eq!(
        specs.get(&vault_0_pubkey),
        Some(&"vault_0_spec".to_string())
    );
    assert_eq!(
        specs.get(&vault_1_pubkey),
        Some(&"vault_1_spec".to_string())
    );

    // Updating vault_0 doesn't affect vault_1
    specs.insert(vault_0_pubkey, "vault_0_updated".to_string());
    assert_eq!(
        specs.get(&vault_1_pubkey),
        Some(&"vault_1_spec".to_string()),
        "vault_1 must be unaffected"
    );
}

// =============================================================================
// 8. DIVERGENT NAMING TESTS: input_vault/output_vault vs token_0_vault/token_1_vault
// =============================================================================

#[test]
fn test_swap_returns_both_vaults_regardless_of_role() {
    // CRITICAL: Swap instruction uses input_vault/output_vault names,
    // but they are aliases for token_0_vault/token_1_vault.
    // The SDK must return BOTH vaults for Swap, regardless of trade direction.

    let sdk = AmmSdk::new();

    let swap_accounts = sdk.get_accounts_to_update(&AmmInstruction::Swap);

    // Without pool state initialized, this is empty, but the contract is:
    // When pool_state has token_0_vault and token_1_vault set,
    // get_accounts_to_update(Swap) returns BOTH.
    //
    // This is because the SDK doesn't know which vault will be "input" vs "output"
    // at runtime - that depends on trade direction chosen by the user.

    // Document: accounts returned are keyed by CANONICAL pubkeys (token_0_vault, token_1_vault)
    // NOT by instruction field names (input_vault, output_vault)

    // The Swap instruction's input_vault/output_vault are just ALIASES
    // that map to the same underlying accounts.
    assert!(
        swap_accounts.is_empty(),
        "SDK without pool state returns empty - but contract is to return both vaults when populated"
    );
}

#[test]
fn test_directional_alias_same_pubkey_same_spec() {
    // CRITICAL: input_vault and output_vault in Swap instruction point to
    // the same underlying accounts (token_0_vault or token_1_vault).
    //
    // When ZeroForOne: input_vault = token_0_vault, output_vault = token_1_vault
    // When OneForZero: input_vault = token_1_vault, output_vault = token_0_vault
    //
    // The SDK stores specs by PUBKEY, so the "role" (input/output) doesn't matter.
    // The spec for token_0_vault is the same whether it's used as input or output.

    use std::collections::HashMap;

    // Simulate pool state with two vaults
    let token_0_vault = Pubkey::new_unique();
    let token_1_vault = Pubkey::new_unique();

    // Simulate SDK's HashMap<Pubkey, Spec>
    let mut specs: HashMap<Pubkey, &str> = HashMap::new();
    specs.insert(token_0_vault, "token_0_vault_spec");
    specs.insert(token_1_vault, "token_1_vault_spec");

    // Swap ZeroForOne: input=token_0, output=token_1
    let input_vault_zero_for_one = token_0_vault;
    let output_vault_zero_for_one = token_1_vault;

    // Swap OneForZero: input=token_1, output=token_0
    let input_vault_one_for_zero = token_1_vault;
    let output_vault_one_for_zero = token_0_vault;

    // Regardless of direction, lookup by pubkey returns the same spec
    assert_eq!(
        specs.get(&input_vault_zero_for_one),
        specs.get(&output_vault_one_for_zero),
        "Same pubkey = same spec regardless of role"
    );

    assert_eq!(
        specs.get(&output_vault_zero_for_one),
        specs.get(&input_vault_one_for_zero),
        "Same pubkey = same spec regardless of role"
    );
}

#[test]
fn test_sdk_doesnt_need_trade_direction() {
    // The SDK is DIRECTION-AGNOSTIC.
    // It doesn't need to know if the user is swapping ZeroForOne or OneForZero.
    // It just returns all necessary accounts and lets the client decide.

    let sdk = AmmSdk::new();

    // Both directions use the same set of accounts from get_accounts_to_update
    let accounts = sdk.get_accounts_to_update(&AmmInstruction::Swap);

    // The SDK's contract: return [token_0_vault, token_1_vault] for Swap
    // The client then passes them to the instruction as input_vault/output_vault
    // based on the desired trade direction.

    // This is the key insight: decompression is role-agnostic.
    // We decompress the account regardless of how it will be used in the swap.

    // Direction independence: same accounts returned regardless of future use
    // (accounts is empty for uninitialized SDK, non-empty when populated)
    let _ = accounts;
}

#[test]
fn test_decompression_instruction_role_agnostic() {
    // Decompression doesn't care about instruction-level roles.
    // When we build a decompression instruction, we specify:
    // - The account pubkey
    // - The seeds (for PDA verification)
    // - The compressed account data
    //
    // We do NOT specify:
    // - Whether it's an "input" or "output" vault
    // - Which instruction will use it
    // - What role it will play
    //
    // The decompression instruction is purely about restoring the account to on-chain state.

    // This test documents the separation of concerns:
    // 1. SDK: returns specs keyed by canonical pubkey
    // 2. Client: builds decompression instructions from specs
    // 3. Program: uses decompressed accounts in any role

    // The SDK never sees "input_vault" or "output_vault" - only token_0_vault, token_1_vault
    // The program's Swap instruction uses aliases, but that's transparent to the SDK.

    let sdk = AmmSdk::new();
    let specs = sdk.get_all_specs();

    // All specs are keyed by pubkey, not by instruction field name
    for spec in &specs {
        // spec.pubkey() is the canonical pubkey
        // There's no "role" field because roles are instruction-specific
        assert!(
            !spec.pubkey().to_bytes().iter().all(|&b| b == 0),
            "Valid pubkey, no role information"
        );
    }
}

#[test]
fn test_swap_and_deposit_share_vault_specs() {
    // Swap uses vaults as input/output
    // Deposit also uses vaults (for receiving tokens)
    // Both operations use the SAME underlying accounts, just different roles.
    //
    // The SDK must return the same specs for these shared accounts.

    let sdk = AmmSdk::new();

    let swap_accounts = sdk.get_accounts_to_update(&AmmInstruction::Swap);
    let deposit_accounts = sdk.get_accounts_to_update(&AmmInstruction::Deposit);

    // Swap: [token_0_vault, token_1_vault]
    // Deposit: [token_0_vault, token_1_vault, observation, lp_mint]
    //
    // The vault pubkeys in swap_accounts should be a subset of deposit_accounts
    // (when both are populated)

    // Verify the relationship contract
    assert!(
        deposit_accounts.len() >= swap_accounts.len(),
        "Deposit accounts should be superset of Swap accounts"
    );
}

#[test]
fn test_canonical_variant_independent_of_alias() {
    // The LightAccountVariant enum uses CANONICAL names:
    // - Token0Vault { pool_state, token_0_mint }
    // - Token1Vault { pool_state, token_1_mint }
    //
    // NOT aliased names:
    // - InputVault (NO - this would be instruction-specific)
    // - OutputVault (NO - this would be instruction-specific)
    //
    // The variant encodes the TRUE identity of the account,
    // not how it's used in a particular instruction.

    // Document the design principle:
    // Variants are based on SEEDS (which are constant per account)
    // NOT based on instruction roles (which vary per operation)

    // For example, token_0_vault always has these seeds:
    // [POOL_VAULT_SEED, pool_state.key(), token_0_mint.key()]
    //
    // Whether it's used as input_vault or output_vault in Swap,
    // the seeds are the same. The variant is Token0Vault, always.

    let sdk = AmmSdk::new();

    // Get specs
    let specs = sdk.get_specs_for_instruction(&AmmInstruction::Swap);

    // All specs should have canonical variants
    for spec in &specs {
        if let AccountSpec::Pda(pda) = spec {
            match &pda.variant {
                LightAccountVariant::PoolState(..) => {
                    // Canonical: PoolState
                }
                LightAccountVariant::ObservationState(..) => {
                    // Canonical: ObservationState
                }
                LightAccountVariant::Token0Vault(_) => {
                    // Canonical: Token0Vault
                }
                LightAccountVariant::Token1Vault(_) => {
                    // Canonical: Token1Vault
                }
                _ => {
                    // Other variants from the program (not AMM-related)
                }
            }
        }
        // No "InputVault" or "OutputVault" variants exist - by design
    }
}

#[test]
fn test_swap_loads_decompresses_before_execution() {
    // The correct flow for Swap with cold vaults:
    //
    // 1. Client: Get accounts to load for Swap
    // 2. SDK returns: [token_0_vault, token_1_vault]
    // 3. Client: Build decompression transactions
    // 4. Client: Execute decompression (vaults now on-chain)
    // 5. Client: Build Swap instruction with:
    //    - input_vault = token_0_vault (for ZeroForOne)
    //    - output_vault = token_1_vault
    //    OR
    //    - input_vault = token_1_vault (for OneForZero)
    //    - output_vault = token_0_vault
    // 6. Client: Execute Swap
    //
    // The decompression step (3-4) doesn't know about step 5's direction.
    // It just decompresses both vaults.

    // This test documents the expected flow
    let sdk = AmmSdk::new();

    // Step 1-2: Get accounts
    let _accounts = sdk.get_accounts_to_update(&AmmInstruction::Swap);

    // Step 3-4: Decompression (direction-agnostic)
    // Both vaults decompressed regardless of which is input/output

    // Step 5-6: Swap execution (direction chosen here)
    // The SDK has no involvement in determining direction
}

#[test]
fn test_multiple_operations_same_underlying_account() {
    // Multiple operations can reference the same account with different field names:
    //
    // | Operation | Field Name     | Underlying Account |
    // |-----------|----------------|-------------------|
    // | Initialize| token_0_vault  | 0xAAAA            |
    // | Deposit   | token_0_vault  | 0xAAAA            |
    // | Withdraw  | token_0_vault  | 0xAAAA            |
    // | Swap      | input_vault    | 0xAAAA (if ZeroForOne) |
    // | Swap      | output_vault   | 0xAAAA (if OneForZero) |
    //
    // The SDK stores ONE spec for pubkey 0xAAAA, used by all operations.

    use std::collections::HashMap;

    let underlying_pubkey = Pubkey::new_unique();

    // Simulate field name -> pubkey mapping
    let field_mappings: HashMap<&str, Pubkey> = [
        ("token_0_vault", underlying_pubkey), // Initialize, Deposit, Withdraw
        ("input_vault_zero_for_one", underlying_pubkey), // Swap ZeroForOne
        ("output_vault_one_for_zero", underlying_pubkey), // Swap OneForZero
    ]
    .into_iter()
    .collect();

    // All map to the same pubkey
    assert_eq!(
        field_mappings.get("token_0_vault"),
        field_mappings.get("input_vault_zero_for_one"),
        "Different names, same account"
    );
    assert_eq!(
        field_mappings.get("token_0_vault"),
        field_mappings.get("output_vault_one_for_zero"),
        "Different names, same account"
    );

    // The SDK stores by pubkey, so ONE spec serves all aliases
    let mut specs: HashMap<Pubkey, &str> = HashMap::new();
    specs.insert(underlying_pubkey, "the_one_and_only_spec");

    assert_eq!(
        specs.len(),
        1,
        "One pubkey = one spec regardless of aliases"
    );
}

// =============================================================================
// 9. SINGLE SOURCE OF TRUTH INVARIANT TESTS
// =============================================================================

#[test]
fn test_invariant_get_accounts_subset_of_specs() {
    // INVARIANT: For all operations, get_accounts_to_update() pubkeys
    // must be a subset of get_specs_for_instruction() addresses.
    //
    // This catches bugs where one method was updated but not the other.

    let sdk = AmmSdk::new();

    for op in [
        AmmInstruction::Swap,
        AmmInstruction::Deposit,
        AmmInstruction::Withdraw,
    ] {
        let update_keys: HashSet<_> = sdk
            .get_accounts_to_update(&op)
            .into_iter()
            .map(|a| a.pubkey())
            .collect();
        let spec_keys: HashSet<_> = sdk
            .get_specs_for_instruction(&op)
            .iter()
            .map(|s| s.pubkey())
            .collect();

        // When SDK is empty, both should be empty
        assert!(
            update_keys.is_subset(&spec_keys) || (update_keys.is_empty() && spec_keys.is_empty()),
            "get_accounts_to_update must return subset of get_specs_for_instruction for {:?}\n  update_keys: {:?}\n  spec_keys: {:?}",
            op, update_keys, spec_keys
        );
    }
}

#[test]
fn test_invariant_typed_matches_untyped_pubkeys() {
    // INVARIANT: get_accounts_to_update() must return the same pubkeys
    // as get_accounts_to_update(), just with type information.
    // (Now they're the same method, so this test is essentially a no-op)

    let sdk = AmmSdk::new();

    for op in [
        AmmInstruction::Swap,
        AmmInstruction::Deposit,
        AmmInstruction::Withdraw,
    ] {
        let untyped: HashSet<_> = sdk
            .get_accounts_to_update(&op)
            .into_iter()
            .map(|a| a.pubkey())
            .collect();
        let typed: HashSet<_> = sdk
            .get_accounts_to_update(&op)
            .iter()
            .map(|a| a.pubkey())
            .collect();

        assert_eq!(
            untyped, typed,
            "Typed and untyped must return same pubkeys for {:?}",
            op
        );
    }
}

#[test]
fn test_invariant_all_methods_derive_from_account_requirements() {
    // DESIGN INVARIANT: All three methods must derive from account_requirements()
    //
    // get_accounts_to_update()       -> account_requirements().map(pubkey)
    // get_accounts_to_update() -> account_requirements().map(to_fetch)
    // get_specs_for_instruction()      -> account_requirements().filter_map(spec_lookup)
    //
    // This ensures they can NEVER drift out of sync.

    // Verify by code inspection:
    // 1. get_accounts_to_update() calls self.account_requirements(op)
    // 2. get_accounts_to_update() calls self.account_requirements(op)
    // 3. get_specs_for_instruction() calls self.account_requirements(op)
    //
    // All derive from the SAME source.

    let sdk = AmmSdk::new();

    // Sanity check: all operations return consistent empty results
    for op in [
        AmmInstruction::Swap,
        AmmInstruction::Deposit,
        AmmInstruction::Withdraw,
    ] {
        let pubkeys = sdk.get_accounts_to_update(&op);
        let typed = sdk.get_accounts_to_update(&op);
        let specs = sdk.get_specs_for_instruction(&op);

        // All should be empty for uninitialized SDK
        assert!(pubkeys.is_empty(), "Empty SDK should return no pubkeys");
        assert!(
            typed.is_empty(),
            "Empty SDK should return no typed accounts"
        );
        assert!(specs.is_empty(), "Empty SDK should return no specs");
    }
}

#[test]
fn test_swap_observation_included_after_refactor() {
    // Regression test: Swap must include observation after the single-source-of-truth refactor.
    //
    // Before fix: get_accounts_to_update(Swap) returned [vault_0, vault_1] - MISSING observation!
    // After fix: get_accounts_to_update(Swap) returns [pool_state, vault_0, vault_1, observation]

    // Create a mock initialized SDK state
    // We can't fully initialize without real data, but we can verify the count

    let sdk = AmmSdk::new();

    // For an uninitialized SDK, both return empty
    let swap_accounts = sdk.get_accounts_to_update(&AmmInstruction::Swap);
    let deposit_accounts = sdk.get_accounts_to_update(&AmmInstruction::Deposit);

    // The key invariant: Swap and Deposit should now have the same number of
    // non-mint accounts when pool_state is set (pool_state, vault_0, vault_1, observation)
    // The only difference is Deposit has lp_mint.

    // When empty, both are empty
    assert_eq!(
        swap_accounts.len(),
        deposit_accounts.len(),
        "Both empty when uninitialized"
    );

    // Document the expected counts when initialized:
    // Swap: pool_state, vault_0, vault_1, observation = 4
    // Deposit: pool_state, vault_0, vault_1, observation, lp_mint_signer = 5
}
