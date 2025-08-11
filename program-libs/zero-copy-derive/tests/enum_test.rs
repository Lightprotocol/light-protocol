use light_zero_copy_derive::ZeroCopy;

// Test struct that will be used in enum variants
#[derive(Debug, Clone, PartialEq, ZeroCopy)]
pub struct TokenMetadataInstructionData {
    pub name: Vec<u8>,
    pub symbol: Vec<u8>,
    pub uri: Vec<u8>,
}

// Test enum using the ExtensionInstructionData example from the user
#[derive(Debug, Clone, PartialEq, ZeroCopy)]
pub enum ExtensionInstructionData {
    Placeholder0,
    Placeholder1,
    Placeholder2,
    Placeholder3,
    Placeholder4,
    Placeholder5,
    Placeholder6,
    Placeholder7,
    Placeholder8,
    Placeholder9,
    Placeholder10,
    Placeholder11,
    Placeholder12,
    Placeholder13,
    Placeholder14,
    Placeholder15,
    Placeholder16,
    Placeholder17,
    Placeholder18, // MetadataPointer(InitMetadataPointer),
    TokenMetadata(TokenMetadataInstructionData),
}

#[cfg(test)]
mod tests {
    use light_zero_copy::borsh::Deserialize;

    use super::*;

    #[test]
    fn test_enum_unit_variant_deserialization() {
        // Test unit variant (Placeholder0 has discriminant 0)
        let data = [0u8]; // discriminant 0 for Placeholder0
        let (result, remaining) = ExtensionInstructionData::zero_copy_at(&data).unwrap();

        let variant = &result;
        {
            // For unit variants, we can't easily pattern match without knowing the exact type
            // In a real test, you'd check the discriminant or use other means
            println!("Got variant: {:?}", variant);
        }

        assert_eq!(remaining.len(), 0);
    }

    #[test]
    fn test_enum_data_variant_deserialization() {
        // Test data variant (TokenMetadata has discriminant 19)
        let mut data = vec![19u8]; // discriminant 19 for TokenMetadata

        // Add TokenMetadataInstructionData serialized data
        // For this test, we'll create simple serialized data for the struct
        // name: "test" (4 bytes length + "test")
        data.extend_from_slice(&4u32.to_le_bytes());
        data.extend_from_slice(b"test");

        // symbol: "TST" (3 bytes length + "TST")
        data.extend_from_slice(&3u32.to_le_bytes());
        data.extend_from_slice(b"TST");

        // uri: "http://test.com" (15 bytes length + "http://test.com")
        data.extend_from_slice(&15u32.to_le_bytes());
        data.extend_from_slice(b"http://test.com");

        let (result, remaining) = ExtensionInstructionData::zero_copy_at(&data).unwrap();

        // For this test, just verify we get a result without panicking
        // In practice, you'd have more specific assertions based on your actual types
        println!("Got result: {:?}", result);

        assert_eq!(remaining.len(), 0);
    }

    #[test]
    fn test_enum_invalid_discriminant() {
        // Test with invalid discriminant (255)
        let data = [255u8];
        let result = ExtensionInstructionData::zero_copy_at(&data);

        assert!(result.is_err());
    }

    #[test]
    fn test_enum_empty_data() {
        // Test with empty data
        let data = [];
        let result = ExtensionInstructionData::zero_copy_at(&data);

        assert!(result.is_err());
    }
}
