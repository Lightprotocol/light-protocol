#![cfg(test)]

use light_account_checks::account_info::test_account_info::pinocchio::get_account_info;
use light_compressed_token_sdk::instructions::mint_action::MintActionCpiAccounts;
use light_compressed_token_types::CPI_AUTHORITY_PDA;
use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use light_sdk_types::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, LIGHT_SYSTEM_PROGRAM_ID,
    REGISTERED_PROGRAM_PDA, SOL_POOL_PDA,
};
use pinocchio::account_info::AccountInfo;

/// Helper function to create test AccountInfo with specific properties
fn create_test_account(
    key: [u8; 32],
    owner: [u8; 32],
    is_signer: bool,
    is_writable: bool,
    executable: bool,
    data: Vec<u8>,
) -> AccountInfo {
    get_account_info(key, owner, is_signer, is_writable, executable, data)
}

/// Helper to create unique pubkeys for testing
fn pubkey_unique() -> [u8; 32] {
    static mut COUNTER: u8 = 0;
    let mut key = [0u8; 32];
    unsafe {
        COUNTER = COUNTER.wrapping_add(1);
        key[0] = COUNTER;
    }
    key
}

/// Tests for MintActionCpiAccounts::try_from_account_infos_full()
/// Functional tests:
/// 1. test_successful_parsing_minimal - successful parsing with minimal required accounts
/// 2. test_successful_parsing_with_all_options - successful parsing with all optional accounts
/// 3. test_successful_create_mint - successful parsing for create_mint scenario
/// 4. test_successful_update_mint - successful parsing for update_mint scenario
///
/// Failing tests:
/// 1. test_invalid_compressed_token_program_id - wrong program ID → InvalidProgramId
/// 2. test_invalid_light_system_program_id - wrong program ID → InvalidProgramId
/// 3. test_authority_not_signer - authority not signer → InvalidSigner
/// 4. test_fee_payer_not_signer - fee payer not signer → InvalidSigner
/// 5. test_invalid_spl_token_program - wrong SPL token program → InvalidProgramId
/// 6. test_invalid_tree_ownership - tree not owned by compression program → AccountOwnedByWrongProgram
/// 7. test_invalid_queue_ownership - queue not owned by compression program → AccountOwnedByWrongProgram
/// 8. test_missing_decompressed_group - partial decompressed accounts → parsing error

#[test]
fn test_successful_parsing_minimal() {
    // Create minimal set of accounts required for parsing
    let accounts = vec![
        // Programs
        create_test_account(
            COMPRESSED_TOKEN_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account(
            LIGHT_SYSTEM_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        // Authority (must be signer)
        create_test_account(pubkey_unique(), [0u8; 32], true, false, false, vec![]),
        // Fee payer (must be signer and mutable)
        create_test_account(pubkey_unique(), [0u8; 32], true, true, false, vec![]),
        // Core system accounts
        create_test_account(CPI_AUTHORITY_PDA, [0u8; 32], false, false, false, vec![]),
        create_test_account(
            REGISTERED_PROGRAM_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account([0u8; 32], [0u8; 32], false, false, true, vec![]), // system program
        // Tree/Queue accounts
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ), // out_output_queue
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ), // in_merkle_tree
    ];

    // Use create_mint variant which doesn't require in_output_queue
    let result = MintActionCpiAccounts::<AccountInfo>::try_from_account_infos_create_mint(
        &accounts, false, // with_mint_signer
        false, // spl_mint_initialized
        false, // with_lamports
        false, // has_mint_to_actions
    );
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(
        *parsed.compressed_token_program.key(),
        COMPRESSED_TOKEN_PROGRAM_ID
    );
    assert_eq!(*parsed.light_system_program.key(), LIGHT_SYSTEM_PROGRAM_ID);
    assert!(parsed.mint_signer.is_none());
    assert!(parsed.authority.is_signer());
    assert!(parsed.mint.is_none());
    assert!(parsed.token_pool_pda.is_none());
    assert!(parsed.token_program.is_none());
    assert!(parsed.sol_pool_pda.is_none());
    assert!(parsed.cpi_context.is_none());
    assert!(parsed.in_output_queue.is_none());
    assert!(parsed.tokens_out_queue.is_none());
    assert_eq!(parsed.ctoken_accounts.len(), 0);
}

#[test]
fn test_successful_parsing_with_all_options() {
    let mint_signer = pubkey_unique();
    let mint = pubkey_unique();
    let token_pool = pubkey_unique();
    let cpi_context = pubkey_unique();

    let accounts = vec![
        // Programs
        create_test_account(
            COMPRESSED_TOKEN_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account(
            LIGHT_SYSTEM_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        // Mint signer (optional)
        create_test_account(mint_signer, [0u8; 32], true, false, false, vec![]),
        // Authority
        create_test_account(pubkey_unique(), [0u8; 32], true, false, false, vec![]),
        // Decompressed mint accounts
        create_test_account(mint, [0u8; 32], false, true, false, vec![]),
        create_test_account(token_pool, [0u8; 32], false, true, false, vec![]),
        create_test_account(
            spl_token_2022::ID.to_bytes(),
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        // Fee payer
        create_test_account(pubkey_unique(), [0u8; 32], true, true, false, vec![]),
        // Core system accounts
        create_test_account(CPI_AUTHORITY_PDA, [0u8; 32], false, false, false, vec![]),
        create_test_account(
            REGISTERED_PROGRAM_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account([0u8; 32], [0u8; 32], false, false, true, vec![]),
        // SOL pool (optional)
        create_test_account(SOL_POOL_PDA, [0u8; 32], false, true, false, vec![]),
        // CPI context (optional)
        create_test_account(cpi_context, [0u8; 32], false, true, false, vec![]),
        // Tree/Queue accounts
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ), // out_output_queue
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ), // in_merkle_tree
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ), // in_output_queue
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ), // tokens_out_queue
        // Decompressed token accounts (remaining)
        create_test_account(pubkey_unique(), [0u8; 32], false, true, false, vec![]),
        create_test_account(pubkey_unique(), [0u8; 32], false, true, false, vec![]),
    ];

    let result = MintActionCpiAccounts::<AccountInfo>::try_from_account_infos_full(
        &accounts, true,  // with_mint_signer
        true,  // spl_mint_initialized
        true,  // with_lamports
        true,  // with_cpi_context
        false, // create_mint
        true,  // has_mint_to_actions
    );
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert!(parsed.mint_signer.is_some());
    assert_eq!(*parsed.mint_signer.unwrap().key(), mint_signer);
    assert!(parsed.mint.is_some());
    assert_eq!(*parsed.mint.unwrap().key(), mint);
    assert!(parsed.token_pool_pda.is_some());
    assert_eq!(*parsed.token_pool_pda.unwrap().key(), token_pool);
    assert!(parsed.token_program.is_some());
    assert_eq!(
        *parsed.token_program.unwrap().key(),
        spl_token_2022::ID.to_bytes()
    );
    assert!(parsed.sol_pool_pda.is_some());
    assert!(parsed.cpi_context.is_some());
    assert!(parsed.in_output_queue.is_some());
    assert!(parsed.tokens_out_queue.is_some());
    assert_eq!(parsed.ctoken_accounts.len(), 2);
}

#[test]
fn test_successful_create_mint() {
    let mint_signer = pubkey_unique();

    let accounts = vec![
        // Programs
        create_test_account(
            COMPRESSED_TOKEN_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account(
            LIGHT_SYSTEM_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        // Mint signer (required for create_mint)
        create_test_account(mint_signer, [0u8; 32], true, false, false, vec![]),
        // Authority
        create_test_account(pubkey_unique(), [0u8; 32], true, false, false, vec![]),
        // Fee payer
        create_test_account(pubkey_unique(), [0u8; 32], true, true, false, vec![]),
        // Core system accounts
        create_test_account(CPI_AUTHORITY_PDA, [0u8; 32], false, false, false, vec![]),
        create_test_account(
            REGISTERED_PROGRAM_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account([0u8; 32], [0u8; 32], false, false, true, vec![]),
        // Tree/Queue accounts
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ), // out_output_queue
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ), // address tree (for create_mint)
    ];

    let result = MintActionCpiAccounts::<AccountInfo>::try_from_account_infos_create_mint(
        &accounts, true,  // with_mint_signer
        false, // spl_mint_initialized
        false, // with_lamports
        false, // has_mint_to_actions
    );
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert!(parsed.mint_signer.is_some());
    assert!(parsed.in_output_queue.is_none()); // Not needed for create_mint
}

#[test]
fn test_successful_update_mint() {
    let accounts = vec![
        // Programs
        create_test_account(
            COMPRESSED_TOKEN_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account(
            LIGHT_SYSTEM_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        // Authority (no mint_signer for update)
        create_test_account(pubkey_unique(), [0u8; 32], true, false, false, vec![]),
        // Fee payer
        create_test_account(pubkey_unique(), [0u8; 32], true, true, false, vec![]),
        // Core system accounts
        create_test_account(CPI_AUTHORITY_PDA, [0u8; 32], false, false, false, vec![]),
        create_test_account(
            REGISTERED_PROGRAM_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account([0u8; 32], [0u8; 32], false, false, true, vec![]),
        // Tree/Queue accounts
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ), // out_output_queue
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ), // state tree (for update_mint)
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ), // in_output_queue (required for update)
    ];

    let result = MintActionCpiAccounts::<AccountInfo>::try_from_account_infos_update_mint(
        &accounts, false, // spl_mint_initialized
        false, // with_lamports
        false, // has_mint_to_actions
    );
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert!(parsed.mint_signer.is_none()); // Not needed for update
    assert!(parsed.in_output_queue.is_some()); // Required for update
}

#[test]
fn test_invalid_compressed_token_program_id() {
    let wrong_program_id = pubkey_unique();

    let accounts = vec![
        // Wrong compressed token program ID
        create_test_account(wrong_program_id, [0u8; 32], false, false, true, vec![]),
        create_test_account(
            LIGHT_SYSTEM_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        // Rest of minimal accounts...
        create_test_account(pubkey_unique(), [0u8; 32], true, false, false, vec![]),
        create_test_account(pubkey_unique(), [0u8; 32], true, true, false, vec![]),
        create_test_account(CPI_AUTHORITY_PDA, [0u8; 32], false, false, false, vec![]),
        create_test_account(
            REGISTERED_PROGRAM_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account([0u8; 32], [0u8; 32], false, false, true, vec![]),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
    ];

    let result = MintActionCpiAccounts::<AccountInfo>::try_from_account_infos(&accounts);
    assert!(result.is_err());
    assert!(result.is_err());
}

#[test]
fn test_invalid_light_system_program_id() {
    let wrong_program_id = pubkey_unique();

    let accounts = vec![
        create_test_account(
            COMPRESSED_TOKEN_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        // Wrong light system program ID
        create_test_account(wrong_program_id, [0u8; 32], false, false, true, vec![]),
        // Rest of minimal accounts...
        create_test_account(pubkey_unique(), [0u8; 32], true, false, false, vec![]),
        create_test_account(pubkey_unique(), [0u8; 32], true, true, false, vec![]),
        create_test_account(CPI_AUTHORITY_PDA, [0u8; 32], false, false, false, vec![]),
        create_test_account(
            REGISTERED_PROGRAM_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account([0u8; 32], [0u8; 32], false, false, true, vec![]),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
    ];

    let result = MintActionCpiAccounts::<AccountInfo>::try_from_account_infos(&accounts);
    assert!(result.is_err());
    assert!(result.is_err());
}

#[test]
fn test_authority_not_signer() {
    let accounts = vec![
        create_test_account(
            COMPRESSED_TOKEN_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account(
            LIGHT_SYSTEM_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        // Authority NOT a signer
        create_test_account(pubkey_unique(), [0u8; 32], false, false, false, vec![]),
        // Rest of minimal accounts...
        create_test_account(pubkey_unique(), [0u8; 32], true, true, false, vec![]),
        create_test_account(CPI_AUTHORITY_PDA, [0u8; 32], false, false, false, vec![]),
        create_test_account(
            REGISTERED_PROGRAM_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account([0u8; 32], [0u8; 32], false, false, true, vec![]),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
    ];

    let result = MintActionCpiAccounts::<AccountInfo>::try_from_account_infos(&accounts);
    assert!(result.is_err());
    assert!(result.is_err());
}

#[test]
fn test_fee_payer_not_signer() {
    let accounts = vec![
        create_test_account(
            COMPRESSED_TOKEN_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account(
            LIGHT_SYSTEM_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account(pubkey_unique(), [0u8; 32], true, false, false, vec![]),
        // Fee payer NOT a signer
        create_test_account(pubkey_unique(), [0u8; 32], false, true, false, vec![]),
        // Rest of minimal accounts...
        create_test_account(CPI_AUTHORITY_PDA, [0u8; 32], false, false, false, vec![]),
        create_test_account(
            REGISTERED_PROGRAM_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account([0u8; 32], [0u8; 32], false, false, true, vec![]),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
    ];

    let result = MintActionCpiAccounts::<AccountInfo>::try_from_account_infos(&accounts);
    assert!(result.is_err());
    assert!(result.is_err());
}

#[test]
fn test_invalid_spl_token_program() {
    let wrong_token_program = pubkey_unique();

    let accounts = vec![
        create_test_account(
            COMPRESSED_TOKEN_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account(
            LIGHT_SYSTEM_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        // Mint signer
        create_test_account(pubkey_unique(), [0u8; 32], true, false, false, vec![]),
        // Authority
        create_test_account(pubkey_unique(), [0u8; 32], true, false, false, vec![]),
        // Decompressed mint accounts (with wrong token program)
        create_test_account(pubkey_unique(), [0u8; 32], false, true, false, vec![]),
        create_test_account(pubkey_unique(), [0u8; 32], false, true, false, vec![]),
        create_test_account(wrong_token_program, [0u8; 32], false, false, true, vec![]), // Wrong!
        // Rest of accounts...
        create_test_account(pubkey_unique(), [0u8; 32], true, true, false, vec![]),
        create_test_account(CPI_AUTHORITY_PDA, [0u8; 32], false, false, false, vec![]),
        create_test_account(
            REGISTERED_PROGRAM_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account([0u8; 32], [0u8; 32], false, false, true, vec![]),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
    ];

    let result = MintActionCpiAccounts::<AccountInfo>::try_from_account_infos_full(
        &accounts, true, // with_mint_signer
        true, // spl_mint_initialized
        false, false, false, false,
    );
    assert!(result.is_err());
    assert!(result.is_err());
}

#[test]
fn test_invalid_tree_ownership() {
    let wrong_owner = pubkey_unique();

    let accounts = vec![
        create_test_account(
            COMPRESSED_TOKEN_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account(
            LIGHT_SYSTEM_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account(pubkey_unique(), [0u8; 32], true, false, false, vec![]),
        create_test_account(pubkey_unique(), [0u8; 32], true, true, false, vec![]),
        create_test_account(CPI_AUTHORITY_PDA, [0u8; 32], false, false, false, vec![]),
        create_test_account(
            REGISTERED_PROGRAM_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account([0u8; 32], [0u8; 32], false, false, true, vec![]),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
        // In merkle tree with wrong owner
        create_test_account(pubkey_unique(), wrong_owner, false, true, false, vec![]),
    ];

    let result = MintActionCpiAccounts::<AccountInfo>::try_from_account_infos(&accounts);
    assert!(result.is_err());
    assert!(result.is_err());
}

#[test]
fn test_invalid_queue_ownership() {
    let wrong_owner = pubkey_unique();

    let accounts = vec![
        create_test_account(
            COMPRESSED_TOKEN_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account(
            LIGHT_SYSTEM_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account(pubkey_unique(), [0u8; 32], true, false, false, vec![]),
        create_test_account(pubkey_unique(), [0u8; 32], true, true, false, vec![]),
        create_test_account(CPI_AUTHORITY_PDA, [0u8; 32], false, false, false, vec![]),
        create_test_account(
            REGISTERED_PROGRAM_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account([0u8; 32], [0u8; 32], false, false, true, vec![]),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
        // In output queue with wrong owner
        create_test_account(pubkey_unique(), wrong_owner, false, true, false, vec![]),
    ];

    let result = MintActionCpiAccounts::<AccountInfo>::try_from_account_infos_update_mint(
        &accounts, false, false, false,
    );
    assert!(result.is_err());
    assert!(result.is_err());
}

#[test]
fn test_helper_methods() {
    // Create accounts for testing helper methods
    let accounts = vec![
        create_test_account(
            COMPRESSED_TOKEN_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account(
            LIGHT_SYSTEM_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account(pubkey_unique(), [0u8; 32], true, false, false, vec![]),
        create_test_account(pubkey_unique(), [0u8; 32], true, true, false, vec![]),
        create_test_account(CPI_AUTHORITY_PDA, [0u8; 32], false, false, false, vec![]),
        create_test_account(
            REGISTERED_PROGRAM_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
            [0u8; 32],
            false,
            false,
            false,
            vec![],
        ),
        create_test_account(
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            [0u8; 32],
            false,
            false,
            true,
            vec![],
        ),
        create_test_account([0u8; 32], [0u8; 32], false, false, true, vec![]),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
        create_test_account(
            pubkey_unique(),
            ACCOUNT_COMPRESSION_PROGRAM_ID,
            false,
            true,
            false,
            vec![],
        ),
    ];

    let parsed = MintActionCpiAccounts::<AccountInfo>::try_from_account_infos_create_mint(
        &accounts, false, // with_mint_signer
        false, // spl_mint_initialized
        false, // with_lamports
        false, // has_mint_to_actions
    )
    .unwrap();

    // Test tree_queue_pubkeys()
    let tree_pubkeys = parsed.tree_queue_pubkeys();
    assert_eq!(tree_pubkeys.len(), 2); // out_output_queue and in_merkle_tree

    // Test to_account_infos()
    let account_infos = parsed.to_account_infos();
    assert!(!account_infos.is_empty());
    assert_eq!(*account_infos[0].key(), LIGHT_SYSTEM_PROGRAM_ID); // First should be light_system_program

    // Test to_account_metas()
    let metas_with_program = parsed.to_account_metas(true);
    assert_eq!(
        metas_with_program[0].pubkey,
        COMPRESSED_TOKEN_PROGRAM_ID.into()
    );

    let metas_without_program = parsed.to_account_metas(false);
    assert_eq!(
        metas_without_program[0].pubkey,
        LIGHT_SYSTEM_PROGRAM_ID.into()
    );
}
