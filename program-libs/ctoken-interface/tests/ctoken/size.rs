use light_ctoken_interface::{
    state::calculate_ctoken_account_size, BASE_TOKEN_ACCOUNT_SIZE,
    COMPRESSIBLE_PAUSABLE_TOKEN_ACCOUNT_SIZE, COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};

#[test]
fn test_ctoken_account_size_calculation() {
    // Base only (no extensions)
    assert_eq!(
        calculate_ctoken_account_size(false, false, false, false, false),
        BASE_TOKEN_ACCOUNT_SIZE
    );

    // With compressible only
    assert_eq!(
        calculate_ctoken_account_size(true, false, false, false, false),
        COMPRESSIBLE_TOKEN_ACCOUNT_SIZE
    );

    // With compressible + pausable
    assert_eq!(
        calculate_ctoken_account_size(true, true, false, false, false),
        COMPRESSIBLE_PAUSABLE_TOKEN_ACCOUNT_SIZE
    );

    // With compressible + pausable + permanent_delegate (264 + 1 = 265)
    assert_eq!(
        calculate_ctoken_account_size(true, true, true, false, false),
        265
    );

    // With pausable only (165 + 1 = 166)
    assert_eq!(
        calculate_ctoken_account_size(false, true, false, false, false),
        166
    );

    // With permanent_delegate only (165 + 1 = 166)
    assert_eq!(
        calculate_ctoken_account_size(false, false, true, false, false),
        166
    );

    // With pausable + permanent_delegate (165 + 1 + 1 = 167)
    assert_eq!(
        calculate_ctoken_account_size(false, true, true, false, false),
        167
    );

    // With compressible + permanent_delegate (263 + 1 = 264)
    assert_eq!(
        calculate_ctoken_account_size(true, false, true, false, false),
        264
    );

    // With transfer_fee only (165 + 9 = 174)
    assert_eq!(
        calculate_ctoken_account_size(false, false, false, true, false),
        174
    );

    // With compressible + transfer_fee (263 + 9 = 272)
    assert_eq!(
        calculate_ctoken_account_size(true, false, false, true, false),
        272
    );

    // With 4 extensions (263 + 1 + 1 + 9 = 274)
    assert_eq!(
        calculate_ctoken_account_size(true, true, true, true, false),
        274
    );

    // With all 5 extensions (263 + 1 + 1 + 9 + 2 = 276)
    assert_eq!(
        calculate_ctoken_account_size(true, true, true, true, true),
        276
    );

    // With transfer_hook only (165 + 2 = 167)
    assert_eq!(
        calculate_ctoken_account_size(false, false, false, false, true),
        167
    );
}
