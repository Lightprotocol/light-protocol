use light_token_interface::{
    state::{calculate_token_account_size, CompressedOnlyExtension, ExtensionStructConfig},
    BASE_TOKEN_ACCOUNT_SIZE,
};

#[test]
fn test_ctoken_account_size_calculation() {
    // Base only (no extensions) - SPL-compatible 165 bytes
    assert_eq!(
        calculate_token_account_size(None).unwrap(),
        BASE_TOKEN_ACCOUNT_SIZE as usize
    );

    // With pausable only (165 base + 1 account_type + 1 Option discriminator + 4 vec length + 1 ext discriminant = 172)
    let pausable_size =
        calculate_token_account_size(Some(&[ExtensionStructConfig::PausableAccount(())])).unwrap();
    assert_eq!(pausable_size, 172);

    // With permanent_delegate only (165 + 1 + 1 + 4 + 1 = 172)
    let perm_delegate_size =
        calculate_token_account_size(Some(&[ExtensionStructConfig::PermanentDelegateAccount(())]))
            .unwrap();
    assert_eq!(perm_delegate_size, 172);

    // With pausable + permanent_delegate (165 + 1 + 1 + 4 + 1 + 1 = 173)
    let both_size = calculate_token_account_size(Some(&[
        ExtensionStructConfig::PausableAccount(()),
        ExtensionStructConfig::PermanentDelegateAccount(()),
    ]))
    .unwrap();
    assert_eq!(both_size, 173);

    // With transfer_fee only (165 + 1 + 1 + 4 + 1 + 8 = 180)
    let transfer_fee_size =
        calculate_token_account_size(Some(&[ExtensionStructConfig::TransferFeeAccount(())]))
            .unwrap();
    assert_eq!(transfer_fee_size, 180);

    // With transfer_hook only (165 + 1 + 1 + 4 + 1 + 1 = 173)
    let transfer_hook_size =
        calculate_token_account_size(Some(&[ExtensionStructConfig::TransferHookAccount(())]))
            .unwrap();
    assert_eq!(transfer_hook_size, 173);

    // With all 4 extensions (165 + 1 + 1 + 4 + 1 + 1 + 9 + 2 = 184)
    let all_size = calculate_token_account_size(Some(&[
        ExtensionStructConfig::PausableAccount(()),
        ExtensionStructConfig::PermanentDelegateAccount(()),
        ExtensionStructConfig::TransferFeeAccount(()),
        ExtensionStructConfig::TransferHookAccount(()),
    ]))
    .unwrap();
    assert_eq!(all_size, 184);
}

#[test]
fn test_compressed_only_extension_size() {
    use light_token_interface::state::ExtensionStruct;
    use light_zero_copy::ZeroCopyNew;

    // CompressedOnlyExtension: delegated_amount (u64=8) + withheld_transfer_fee (u64=8) + is_ata (u8=1) = 17 bytes
    assert_eq!(
        CompressedOnlyExtension::LEN,
        17,
        "CompressedOnlyExtension should be 17 bytes (8 + 8 + 1)"
    );

    // Verify ExtensionStruct::byte_len matches 1 (discriminant) + LEN
    let config = ExtensionStructConfig::CompressedOnly(());
    assert_eq!(
        ExtensionStruct::byte_len(&config).unwrap(),
        1 + CompressedOnlyExtension::LEN,
        "ExtensionStruct byte_len should be 1 + LEN"
    );
}
