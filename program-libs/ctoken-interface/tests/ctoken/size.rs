use light_ctoken_interface::{state::calculate_ctoken_account_size, BASE_TOKEN_ACCOUNT_SIZE};

#[test]
fn test_ctoken_account_size_calculation() {
    // Base only (no extensions) - includes compression info in base struct (258 bytes)
    assert_eq!(
        calculate_ctoken_account_size(false, false, false, false),
        BASE_TOKEN_ACCOUNT_SIZE
    );

    // With pausable only (258 + 4 metadata + 1 discriminant = 263)
    assert_eq!(
        calculate_ctoken_account_size(true, false, false, false),
        263
    );

    // With permanent_delegate only (258 + 4 metadata + 1 discriminant = 263)
    assert_eq!(
        calculate_ctoken_account_size(false, true, false, false),
        263
    );

    // With pausable + permanent_delegate (258 + 4 metadata + 1 + 1 = 264)
    assert_eq!(
        calculate_ctoken_account_size(true, true, false, false),
        264
    );

    // With transfer_fee only (258 + 4 metadata + 9 = 271)
    assert_eq!(
        calculate_ctoken_account_size(false, false, true, false),
        271
    );

    // With transfer_hook only (258 + 4 metadata + 2 = 264)
    assert_eq!(
        calculate_ctoken_account_size(false, false, false, true),
        264
    );

    // With all 4 extensions (258 + 4 + 1 + 1 + 9 + 2 = 275)
    assert_eq!(
        calculate_ctoken_account_size(true, true, true, true),
        275
    );
}
