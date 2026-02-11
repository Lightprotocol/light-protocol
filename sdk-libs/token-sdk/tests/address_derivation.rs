//! Tests for ATA and SPL interface PDA derivation functions.

use light_token::instruction::{
    derive_associated_token_account, get_associated_token_address,
    get_associated_token_address_and_bump, get_spl_interface_pda_and_bump, LIGHT_TOKEN_PROGRAM_ID,
};
use solana_pubkey::Pubkey;

/// Verify ATA derivation produces a valid PDA for a single owner/mint pair.
#[test]
fn test_derive_ata_single_owner_mint() {
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let ata = derive_associated_token_account(&owner, &mint);

    // Verify the PDA is valid by checking we can recreate it
    let (recreated_ata, _recreated_bump) = Pubkey::find_program_address(
        &[
            owner.as_ref(),
            LIGHT_TOKEN_PROGRAM_ID.as_ref(),
            mint.as_ref(),
        ],
        &LIGHT_TOKEN_PROGRAM_ID,
    );

    assert_eq!(ata, recreated_ata);

    // ATA should not equal owner, mint, or program ID
    assert_ne!(ata, owner);
    assert_ne!(ata, mint);
    assert_ne!(ata, LIGHT_TOKEN_PROGRAM_ID);
}

/// Verify different owners produce different ATAs for the same mint.
#[test]
fn test_derive_ata_different_owners_different_result() {
    let owner1 = Pubkey::new_unique();
    let owner2 = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let ata1 = derive_associated_token_account(&owner1, &mint);
    let ata2 = derive_associated_token_account(&owner2, &mint);

    // Different owners should produce different ATAs
    assert_ne!(
        ata1, ata2,
        "Different owners should produce different ATAs for the same mint"
    );
}

/// Verify different mints produce different ATAs for the same owner.
#[test]
fn test_derive_ata_different_mints_different_result() {
    let owner = Pubkey::new_unique();
    let mint1 = Pubkey::new_unique();
    let mint2 = Pubkey::new_unique();

    let ata1 = derive_associated_token_account(&owner, &mint1);
    let ata2 = derive_associated_token_account(&owner, &mint2);

    // Different mints should produce different ATAs
    assert_ne!(
        ata1, ata2,
        "Different mints should produce different ATAs for the same owner"
    );
}

/// Verify same inputs always produce same result (deterministic derivation).
#[test]
fn test_derive_ata_consistency() {
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    // Derive multiple times
    let ata1 = derive_associated_token_account(&owner, &mint);
    let ata2 = derive_associated_token_account(&owner, &mint);
    let ata3 = derive_associated_token_account(&owner, &mint);

    // All derivations should match
    assert_eq!(ata1, ata2);
    assert_eq!(ata2, ata3);
}

/// Verify SPL interface PDA derivation works correctly.
#[test]
fn test_spl_interface_pda_derivation() {
    let mint = Pubkey::new_unique();

    let (pda, bump) = get_spl_interface_pda_and_bump(&mint);

    // Verify the PDA is valid by checking we can recreate it
    let pool_seed: &[u8] = b"pool";
    let (recreated_pda, recreated_bump) =
        Pubkey::find_program_address(&[pool_seed, mint.as_ref()], &LIGHT_TOKEN_PROGRAM_ID);

    assert_eq!(pda, recreated_pda);
    assert_eq!(bump, recreated_bump);

    // PDA should not equal the mint or program ID
    assert_ne!(pda, mint);
    assert_ne!(pda, LIGHT_TOKEN_PROGRAM_ID);
}

/// Verify different mints produce different SPL interface PDAs.
#[test]
fn test_spl_interface_pda_different_mints() {
    let mint1 = Pubkey::new_unique();
    let mint2 = Pubkey::new_unique();

    let (pda1, _bump1) = get_spl_interface_pda_and_bump(&mint1);
    let (pda2, _bump2) = get_spl_interface_pda_and_bump(&mint2);

    assert_ne!(
        pda1, pda2,
        "Different mints should produce different SPL interface PDAs"
    );
}

/// Verify SPL interface PDA derivation is deterministic.
#[test]
fn test_spl_interface_pda_consistency() {
    let mint = Pubkey::new_unique();

    let (pda1, bump1) = get_spl_interface_pda_and_bump(&mint);
    let (pda2, bump2) = get_spl_interface_pda_and_bump(&mint);

    assert_eq!(pda1, pda2, "Same mint should always produce same PDA");
    assert_eq!(bump1, bump2, "Same mint should always produce same bump");
}

/// Verify get_associated_token_address matches derive_associated_token_account.
#[test]
fn test_get_associated_token_address_matches_derive() {
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    // Get address without bump
    let ata = get_associated_token_address(&owner, &mint);

    // Get address with bump
    let (ata_with_bump, _bump) = get_associated_token_address_and_bump(&owner, &mint);

    // Both should match the derive function
    let derived_ata = derive_associated_token_account(&owner, &mint);

    assert_eq!(
        ata, ata_with_bump,
        "get_associated_token_address should match get_associated_token_address_and_bump"
    );
    assert_eq!(
        ata, derived_ata,
        "get_associated_token_address should match derive_associated_token_account"
    );
}

/// Verify that known fixed pubkeys produce deterministic ATAs.
/// This tests that the derivation uses the correct seeds in the correct order.
#[test]
fn test_derive_ata_seed_order() {
    // Use fixed pubkeys to ensure deterministic testing
    let owner = Pubkey::new_from_array([1u8; 32]);
    let mint = Pubkey::new_from_array([2u8; 32]);

    let ata = derive_associated_token_account(&owner, &mint);

    // Verify with swapped order produces different result (confirms seed order matters)
    let (ata_swapped, _) = Pubkey::find_program_address(
        &[
            mint.as_ref(),
            LIGHT_TOKEN_PROGRAM_ID.as_ref(),
            owner.as_ref(),
        ],
        &LIGHT_TOKEN_PROGRAM_ID,
    );

    assert_ne!(ata, ata_swapped, "Seed order should affect the derived PDA");
}
