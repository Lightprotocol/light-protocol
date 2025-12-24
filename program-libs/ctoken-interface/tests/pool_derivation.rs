use light_ctoken_interface::{
    find_spl_interface_pda, find_spl_interface_pda_with_index, is_valid_spl_interface_pda,
    NUM_MAX_POOL_ACCOUNTS,
};
use solana_pubkey::Pubkey;

#[test]
fn test_spl_interface_derivation_index_0() {
    let mint = Pubkey::new_unique();

    let (pda, bump) = find_spl_interface_pda(&mint, false);

    // Verify with bump
    assert!(is_valid_spl_interface_pda(
        mint.as_ref(),
        &pda,
        0,
        Some(bump),
        false
    ));

    // Verify without bump
    assert!(is_valid_spl_interface_pda(
        mint.as_ref(),
        &pda,
        0,
        None,
        false
    ));

    // Verify restricted derivation doesn't match
    assert!(!is_valid_spl_interface_pda(
        mint.as_ref(),
        &pda,
        0,
        None,
        true
    ));
}

#[test]
fn test_restricted_spl_interface_derivation_index_0() {
    let mint = Pubkey::new_unique();

    let (pda, bump) = find_spl_interface_pda(&mint, true);

    // Verify with bump
    assert!(is_valid_spl_interface_pda(
        mint.as_ref(),
        &pda,
        0,
        Some(bump),
        true
    ));

    // Verify without bump
    assert!(is_valid_spl_interface_pda(
        mint.as_ref(),
        &pda,
        0,
        None,
        true
    ));

    // Verify non-restricted derivation doesn't match
    assert!(!is_valid_spl_interface_pda(
        mint.as_ref(),
        &pda,
        0,
        None,
        false
    ));
}

#[test]
fn test_spl_interface_derivation_with_index() {
    let mint = Pubkey::new_unique();

    for index in 1..NUM_MAX_POOL_ACCOUNTS {
        let (pda, bump) = find_spl_interface_pda_with_index(&mint, index, false);

        assert!(is_valid_spl_interface_pda(
            mint.as_ref(),
            &pda,
            index,
            Some(bump),
            false
        ));
    }
}

#[test]
fn test_restricted_spl_interface_derivation_with_index() {
    let mint = Pubkey::new_unique();

    for index in 1..NUM_MAX_POOL_ACCOUNTS {
        let (pda, bump) = find_spl_interface_pda_with_index(&mint, index, true);

        assert!(is_valid_spl_interface_pda(
            mint.as_ref(),
            &pda,
            index,
            Some(bump),
            true
        ));
    }
}

#[test]
fn test_different_mints_different_pdas() {
    let mint1 = Pubkey::new_unique();
    let mint2 = Pubkey::new_unique();

    let (pda1, _) = find_spl_interface_pda(&mint1, false);
    let (pda2, _) = find_spl_interface_pda(&mint2, false);

    assert_ne!(pda1, pda2);
}

#[test]
fn test_restricted_vs_non_restricted_different_pdas() {
    let mint = Pubkey::new_unique();

    let (regular_pda, _) = find_spl_interface_pda(&mint, false);
    let (restricted_pda, _) = find_spl_interface_pda(&mint, true);

    assert_ne!(regular_pda, restricted_pda);
}
