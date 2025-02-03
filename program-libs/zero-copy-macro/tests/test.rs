#[cfg(test)]
mod tests {
    use super::*;
    use light_zero_copy::{borsh::Deserialize, errors::ZeroCopyError};
    use zerocopy::{Ref, FromBytes, AsBytes, U16, U32, U64};
    use zero_copy_macro::ZeroCopyAccount;

    // Define a test struct with various field types and options
    #[derive(Debug, PartialEq, Clone, ZeroCopyAccount)]
    struct TestAccount {
        id: u32,
        balance: u64,
        maybe_flag: Option<u16>,
        maybe_address: Option<[u8; 32]>,
    }

    // Test if the zero-copy struct and meta struct are generated correctly
    #[test]
    fn test_struct_generation() {
        // This test is mostly compile-time. If the code compiles, the structs exist.
        let _ = ZTestAccount {
            meta: Ref::new(&[]).unwrap(),
            maybe_flag: None,
            maybe_address: None,
        };
    }

    // Test the meta struct layout and field conversions
    #[test]
    fn test_meta_struct() {
        let meta = TestAccountDesMeta {
            id: U32::new(123),
            balance: U64::new(456),
            maybe_flag_option: 1,
            maybe_address_option: 0,
        };

        // Check that original u32/u64 are converted to zerocopy types
        assert_eq!(meta.id.get(), 123);
        assert_eq!(meta.balance.get(), 456);
        // Check option flags
        assert_eq!(meta.maybe_flag_option, 1);
        assert_eq!(meta.maybe_address_option, 0);
    }

    // Test full deserialization cycle with optional fields
    #[test]
    fn test_deserialization() -> Result<(), ZeroCopyError> {
        // Original data
        let original = TestAccount {
            id: 123,
            balance: 456,
            maybe_flag: Some(42),
            maybe_address: None,
        };

        // Serialize meta struct
        let meta = TestAccountDesMeta {
            id: U32::new(original.id),
            balance: U64::new(original.balance),
            maybe_flag_option: 1, // Some
            maybe_address_option: 0, // None
        };

        let mut bytes = meta.as_bytes().to_vec();

        // Serialize optional fields
        if let Some(flag) = original.maybe_flag {
            bytes.extend_from_slice(&flag.to_le_bytes());
        }

        // Deserialize
        let (z_account, remaining) = ZTestAccount::zero_copy_at(&bytes)?;
        assert!(remaining.is_empty(), "Bytes should be fully consumed");

        // Check meta fields
        assert_eq!(z_account.id.get(), original.id);
        assert_eq!(z_account.balance.get(), original.balance);

        // Check optional fields
        assert_eq!(
            z_account.maybe_flag.map(|r| r.get()),
            original.maybe_flag
        );
        assert_eq!(z_account.maybe_address, None);

        // Convert back to original type
        let reconstructed = TestAccount::from(&z_account);
        assert_eq!(reconstructed, original);

        Ok(())
    }

    // Test deserialization with all optional fields present
    #[test]
    fn test_all_options_present() -> Result<(), ZeroCopyError> {
        let original = TestAccount {
            id: 789,
            balance: 101112,
            maybe_flag: Some(99),
            maybe_address: Some([255; 32]),
        };

        let meta = TestAccountDesMeta {
            id: U32::new(original.id),
            balance: U64::new(original.balance),
            maybe_flag_option: 1,
            maybe_address_option: 1,
        };

        let mut bytes = meta.as_bytes().to_vec();
        bytes.extend_from_slice(&original.maybe_flag.unwrap().to_le_bytes());
        bytes.extend_from_slice(&original.maybe_address.unwrap());

        let (z_account, remaining) = ZTestAccount::zero_copy_at(&bytes)?;
        assert!(remaining.is_empty());

        assert_eq!(z_account.maybe_flag.unwrap().get(), 99);
        assert_eq!(*z_account.maybe_address.unwrap(), [255; 32]);

        Ok(())
    }

    // Test deserialization with no optional fields
    #[test]
    fn test_no_options_present() -> Result<(), ZeroCopyError> {
        let original = TestAccount {
            id: 456,
            balance: 789,
            maybe_flag: None,
            maybe_address: None,
        };

        let meta = TestAccountDesMeta {
            id: U32::new(original.id),
            balance: U64::new(original.balance),
            maybe_flag_option: 0,
            maybe_address_option: 0,
        };

        let bytes = meta.as_bytes();

        let (z_account, remaining) = ZTestAccount::zero_copy_at(bytes)?;
        assert!(remaining.is_empty());

        assert_eq!(z_account.maybe_flag, None);
        assert_eq!(z_account.maybe_address, None);

        Ok(())
    }

    // Test error handling for invalid data
    #[test]
    fn test_invalid_data() {
        // Missing bytes for optional field
        let meta = TestAccountDesMeta {
            id: U32::new(1),
            balance: U64::new(2),
            maybe_flag_option: 1, // Claims flag exists
            maybe_address_option: 0,
        };

        let bytes = meta.as_bytes(); // No flag data appended

        let result = ZTestAccount::zero_copy_at(bytes);
        assert!(
            matches!(result, Err(ZeroCopyError::DeserializationError)),
            "Should fail when missing optional field data"
        );
    }
}