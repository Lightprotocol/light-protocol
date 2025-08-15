/*use borsh::BorshSerialize;
use light_compressed_account::Pubkey;
use light_ctoken_types::{
    instructions::extensions::{
        metadata_pointer::{InitMetadataPointer, MetadataPointer, MetadataPointerConfig},
        ExtensionInstructionData, ZExtensionInstructionData,
    },
    state::{ExtensionStruct, ExtensionStructConfig, ZExtensionStruct, ZExtensionStructMut},
};
use light_zero_copy::{borsh::Deserialize, borsh_mut::DeserializeMut, ZeroCopyNew};

#[test]
fn test_borsh_zero_copy_compatibility() {
    let config = ExtensionStructConfig::MetadataPointer(MetadataPointerConfig {
        authority: (true, ()),
        metadata_address: (true, ()),
    });
    let byte_len = ExtensionStruct::byte_len(&config);
    let mut bytes = vec![0u8; byte_len];
    // Assert zero init
    {
        let (zero_copy_new_result, _) =
            ExtensionStruct::new_zero_copy(&mut bytes, config.clone()).unwrap();
        if let ZExtensionStructMut::MetadataPointer(metadata) = zero_copy_new_result {
            assert!(metadata.authority.is_some());
            assert!(metadata.metadata_address.is_some());

            let expected = ExtensionStruct::MetadataPointer(MetadataPointer {
                authority: Some(Pubkey::new_from_array([0; 32])),
                metadata_address: Some(Pubkey::new_from_array([0; 32])),
            });
            assert_eq!(bytes, expected.try_to_vec().unwrap());
        } else {
            panic!("Unexpected extension type");
        }
    }
    // Assert zero copy mut
    {
        let (mut zero_copy_new_result, _) = ExtensionStruct::zero_copy_at_mut(&mut bytes).unwrap();

        let new_authority = Pubkey::new_from_array([1; 32]);
        let new_metadata_address = Pubkey::new_from_array([1; 32]);
        if let ZExtensionStructMut::MetadataPointer(metadata) = &mut zero_copy_new_result {
            **metadata.authority.as_mut().unwrap() = new_authority;
            **metadata.metadata_address.as_mut().unwrap() = new_metadata_address;
        }
        let expected = ExtensionStruct::MetadataPointer(MetadataPointer {
            authority: Some(new_authority),
            metadata_address: Some(new_metadata_address),
        });
        assert_eq!(bytes, expected.try_to_vec().unwrap());
    }

    // Test zero_copy_at (immutable deserialization)
    {
        let original_metadata = MetadataPointer {
            authority: Some(Pubkey::new_from_array([5; 32])),
            metadata_address: Some(Pubkey::new_from_array([6; 32])),
        };
        let original_struct = ExtensionStruct::MetadataPointer(original_metadata.clone());
        let serialized_bytes = original_struct.try_to_vec().unwrap();

        // Test zero_copy_at immutable deserialization
        let (zero_copy_result, remaining_bytes) =
            ExtensionStruct::zero_copy_at(&serialized_bytes).unwrap();
        assert!(remaining_bytes.is_empty());

        // Verify the deserialized data matches
        if let ZExtensionStruct::MetadataPointer(metadata) = zero_copy_result {
            assert_eq!(
                *metadata.authority.unwrap(),
                Pubkey::new_from_array([5; 32])
            );
            assert_eq!(
                *metadata.metadata_address.unwrap(),
                Pubkey::new_from_array([6; 32])
            );
        } else {
            panic!("deserialization failed ")
        }
    }
}

#[test]
fn test_borsh_zero_copy_compatibility_none_fields() {
    let original_metadata = MetadataPointer {
        authority: None,
        metadata_address: None,
    };
    let original_struct = ExtensionStruct::MetadataPointer(original_metadata.clone());
    let serialized_bytes = original_struct.try_to_vec().unwrap();

    let config = ExtensionStructConfig::MetadataPointer(MetadataPointerConfig {
        authority: (false, ()),
        metadata_address: (false, ()),
    });
    let byte_len = ExtensionStruct::byte_len(&config);
    let mut bytes = vec![0u8; byte_len];

    // Assert zero init with None fields
    {
        let (zero_copy_new_result, _) =
            ExtensionStruct::new_zero_copy(&mut bytes, config.clone()).unwrap();
        if let ZExtensionStructMut::MetadataPointer(metadata) = zero_copy_new_result {
            assert!(metadata.authority.is_none());
            assert!(metadata.metadata_address.is_none());
            assert_eq!(bytes, serialized_bytes);
        } else {
            panic!("Unexpected deserialization result");
        }
    }

    // Assert zero copy mut with None fields (no mutation needed)
    {
        let (zero_copy_new_result, _) = ExtensionStruct::zero_copy_at_mut(&mut bytes).unwrap();

        if let ZExtensionStructMut::MetadataPointer(metadata) = zero_copy_new_result {
            assert!(metadata.authority.is_none());
            assert!(metadata.metadata_address.is_none());
            assert_eq!(bytes, serialized_bytes);
        } else {
            panic!("Unexpected deserialization result");
        }
    }

    // Test zero_copy_at (immutable deserialization) with None fields
    {
        // Test zero_copy_at immutable deserialization
        let (zero_copy_result, remaining_bytes) =
            ExtensionStruct::zero_copy_at(&serialized_bytes).unwrap();
        assert!(remaining_bytes.is_empty());

        // Verify the deserialized data matches (None fields)
        if let ZExtensionStruct::MetadataPointer(metadata) = zero_copy_result {
            assert!(metadata.authority.is_none());
            assert!(metadata.metadata_address.is_none());
            assert_eq!(bytes, serialized_bytes);
        } else {
            panic!("Unexpected deserialization result");
        }
    }
}

#[test]
fn test_extension_instruction_data_borsh_zero_copy_compatibility() {
    // Test with Some values
    let init_metadata_pointer = InitMetadataPointer {
        authority: Some(Pubkey::new_from_array([1; 32])),
        metadata_address: Some(Pubkey::new_from_array([2; 32])),
    };
    let instruction_data = ExtensionInstructionData::MetadataPointer(init_metadata_pointer);
    let serialized_bytes = instruction_data.try_to_vec().unwrap();

    // Test zero_copy_at deserialization
    let (zero_copy_result, remaining_bytes) =
        ExtensionInstructionData::zero_copy_at(&serialized_bytes).unwrap();
    assert!(remaining_bytes.is_empty());

    // Verify the deserialized data matches
    if let ZExtensionInstructionData::MetadataPointer(metadata) = zero_copy_result {
        assert_eq!(
            *metadata.authority.unwrap(),
            Pubkey::new_from_array([1; 32])
        );
        let address = metadata.metadata_address.unwrap();
        assert_eq!(*address, Pubkey::new_from_array([2; 32]));
    } else {
        panic!("Unexpected deserialization result");
    }
}

#[test]
fn test_extension_instruction_data_borsh_zero_copy_compatibility_none_fields() {
    // Test with None values
    let init_metadata_pointer = InitMetadataPointer {
        authority: None,
        metadata_address: None,
    };
    let instruction_data = ExtensionInstructionData::MetadataPointer(init_metadata_pointer);
    let serialized_bytes = instruction_data.try_to_vec().unwrap();

    // Test zero_copy_at deserialization
    let (zero_copy_result, remaining_bytes) =
        ExtensionInstructionData::zero_copy_at(&serialized_bytes).unwrap();
    assert!(remaining_bytes.is_empty());

    if let ZExtensionInstructionData::MetadataPointer(metadata) = zero_copy_result {
        assert!(metadata.authority.is_none());
        assert!(metadata.metadata_address.is_none());
    } else {
        panic!("Unexpected deserialization result");
    }
}
*/
