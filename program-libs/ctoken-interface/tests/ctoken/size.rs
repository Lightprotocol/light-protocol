use light_ctoken_interface::{
    state::{calculate_ctoken_account_size, ExtensionStructConfig},
    BASE_TOKEN_ACCOUNT_SIZE,
};

#[test]
fn test_ctoken_account_size_calculation() {
    // Base only (no extensions) - SPL-compatible 165 bytes
    assert_eq!(
        calculate_ctoken_account_size(None).unwrap(),
        BASE_TOKEN_ACCOUNT_SIZE as usize
    );

    // With pausable only (165 base + 1 account_type + 4 vec length + 1 discriminant = 171)
    let pausable_size =
        calculate_ctoken_account_size(Some(&[ExtensionStructConfig::PausableAccount(())]))
            .unwrap();
    assert_eq!(pausable_size, 171);

    // With permanent_delegate only (165 + 1 + 4 + 1 = 171)
    let perm_delegate_size = calculate_ctoken_account_size(Some(&[
        ExtensionStructConfig::PermanentDelegateAccount(()),
    ]))
    .unwrap();
    assert_eq!(perm_delegate_size, 171);

    // With pausable + permanent_delegate (165 + 1 + 4 + 1 + 1 = 172)
    let both_size = calculate_ctoken_account_size(Some(&[
        ExtensionStructConfig::PausableAccount(()),
        ExtensionStructConfig::PermanentDelegateAccount(()),
    ]))
    .unwrap();
    assert_eq!(both_size, 172);

    // With transfer_fee only (165 + 1 + 4 + 1 + 8 = 179)
    let transfer_fee_size =
        calculate_ctoken_account_size(Some(&[ExtensionStructConfig::TransferFeeAccount(())]))
            .unwrap();
    assert_eq!(transfer_fee_size, 179);

    // With transfer_hook only (165 + 1 + 4 + 1 + 1 = 172)
    let transfer_hook_size =
        calculate_ctoken_account_size(Some(&[ExtensionStructConfig::TransferHookAccount(())]))
            .unwrap();
    assert_eq!(transfer_hook_size, 172);

    // With all 4 extensions (165 + 1 + 4 + 1 + 1 + 9 + 2 = 183)
    let all_size = calculate_ctoken_account_size(Some(&[
        ExtensionStructConfig::PausableAccount(()),
        ExtensionStructConfig::PermanentDelegateAccount(()),
        ExtensionStructConfig::TransferFeeAccount(()),
        ExtensionStructConfig::TransferHookAccount(()),
    ]))
    .unwrap();
    assert_eq!(all_size, 183);
}
