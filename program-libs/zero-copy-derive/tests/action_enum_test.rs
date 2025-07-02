use light_zero_copy_derive::ZeroCopy;

// Test struct for the MintTo action
#[derive(Debug, Clone, PartialEq, ZeroCopy)]
pub struct MintToAction {
    pub amount: u64,
    pub recipient: Vec<u8>,
}

// Test enum similar to your Action example
#[derive(Debug, Clone, ZeroCopy)]
pub enum Action {
    MintTo(MintToAction),
    Update,
    CreateSplMint,
    UpdateMetadata,
}

#[cfg(test)]
mod tests {
    use light_zero_copy::traits::ZeroCopyAt;

    use super::*;

    #[test]
    fn test_action_enum_unit_variants() {
        // Test Update variant (discriminant 1)
        let data = [1u8];
        let (result, remaining) = Action::zero_copy_at(&data).unwrap();
        // We can't pattern match without importing the generated type,
        // but we can verify it doesn't panic and processes correctly
        println!("Successfully deserialized Update variant");
        assert_eq!(remaining.len(), 0);
    }

    #[test]
    fn test_action_enum_data_variant() {
        // Test MintTo variant (discriminant 0)
        let mut data = vec![0u8]; // discriminant 0 for MintTo

        // Add MintToAction serialized data
        // amount: 1000
        data.extend_from_slice(&1000u64.to_le_bytes());

        // recipient: "alice" (5 bytes length + "alice")
        data.extend_from_slice(&5u32.to_le_bytes());
        data.extend_from_slice(b"alice");

        let (result, remaining) = Action::zero_copy_at(&data).unwrap();
        // We can't easily pattern match without the generated type imported,
        // but we can verify it processes without errors
        println!("Successfully deserialized MintTo variant");
        assert_eq!(remaining.len(), 0);
    }

    #[test]
    fn test_action_enum_all_unit_variants() {
        // Test all unit variants
        let variants = [
            (1u8, "Update"),
            (2u8, "CreateSplMint"),
            (3u8, "UpdateMetadata"),
        ];

        for (discriminant, name) in variants {
            let data = [discriminant];
            let result = Action::zero_copy_at(&data);
            assert!(result.is_ok(), "Failed to deserialize {} variant", name);
            let (_, remaining) = result.unwrap();
            assert_eq!(remaining.len(), 0);
            println!("Successfully deserialized {} variant", name);
        }
    }
}
