use borsh::BorshSerialize;
use light_compressed_token::extensions::token_metadata::Metadata;
use light_hasher::{to_byte_array::ToByteArray, DataHasher};
use light_zero_copy::{borsh::Deserialize, borsh_mut::DeserializeMut};
// TODO: add random test
#[test]
fn test_metadata_hash_consistency() {
    // Create test data
    let metadata = Metadata {
        name: b"MyToken".to_vec(),
        symbol: b"MTK".to_vec(),
        uri: b"https://example.com/metadata.json".to_vec(),
    };

    // Deserialize to ZStruct
    let mut serialized = metadata.try_to_vec().unwrap();
    let (z_metadata, _) = Metadata::zero_copy_at_mut(&mut serialized).unwrap();

    // Hash both structs
    let original_hash = metadata.hash::<light_hasher::Poseidon>().unwrap();
    let z_struct_hash = z_metadata.hash::<light_hasher::Poseidon>().unwrap();

    // They should now produce the same hash
    assert_eq!(
        original_hash, z_struct_hash,
        "Hashes should match between original struct and ZStruct"
    );

    println!("Original hash: {:?}", original_hash);
    println!("ZStruct hash:  {:?}", z_struct_hash);
}

#[test]
fn test_metadata_to_byte_array_consistency() {
    let metadata = Metadata {
        name: b"MyToken".to_vec(),
        symbol: b"MTK".to_vec(),
        uri: b"https://example.com/metadata.json".to_vec(),
    };

    let mut serialized = metadata.try_to_vec().unwrap();
    let (z_metadata, _) = Metadata::zero_copy_at_mut(&mut serialized).unwrap();

    let original_bytes = metadata.to_byte_array().unwrap();
    let z_struct_bytes = z_metadata.to_byte_array().unwrap();

    assert_eq!(
        original_bytes, z_struct_bytes,
        "to_byte_array should produce same result"
    );
}
