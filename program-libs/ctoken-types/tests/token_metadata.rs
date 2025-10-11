// Tests compatibility between Light Protocol TokenMetadata and SPL TokenMetadata
// Verifies that both implementations correctly serialize/deserialize their data
// and maintain logical equivalence of metadata fields.
// Note: Binary compatibility is not tested as the formats differ (Vec<u8> vs String).

use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_ctoken_types::state::extensions::{
    AdditionalMetadata, TokenMetadata as LightTokenMetadata,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use spl_pod::optional_keys::OptionalNonZeroPubkey;
use spl_token_metadata_interface::state::TokenMetadata as SplTokenMetadata;

/// Test data tuple type for metadata generation
type MetadataTestData = (
    Option<Pubkey>,
    Pubkey,
    String,
    String,
    String,
    Vec<(String, String)>,
);

/// Generate random test data that can be represented in both formats
fn generate_random_metadata() -> MetadataTestData {
    let mut rng = thread_rng();

    let update_authority = if rng.gen_bool(0.7) {
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        Some(Pubkey::from(bytes))
    } else {
        None
    };

    let mut mint_bytes = [0u8; 32];
    rng.fill(&mut mint_bytes);
    let mint = Pubkey::from(mint_bytes);

    // Generate random alphanumeric strings with reasonable lengths
    let name_len = rng.gen_range(1..=32);
    let name: String = (&mut rng)
        .sample_iter(&Alphanumeric)
        .take(name_len)
        .map(char::from)
        .collect();

    let symbol_len = rng.gen_range(1..=10);
    let symbol: String = (&mut rng)
        .sample_iter(&Alphanumeric)
        .take(symbol_len)
        .map(char::from)
        .collect();

    let uri_len = rng.gen_range(0..=200);
    let uri: String = (&mut rng)
        .sample_iter(&Alphanumeric)
        .take(uri_len)
        .map(char::from)
        .collect();

    let num_metadata = rng.gen_range(0..=5);
    let additional_metadata: Vec<(String, String)> = (0..num_metadata)
        .map(|_| {
            let key_len = rng.gen_range(1..=20);
            let key: String = (&mut rng)
                .sample_iter(&Alphanumeric)
                .take(key_len)
                .map(char::from)
                .collect();
            let value_len = rng.gen_range(0..=100);
            let value: String = (&mut rng)
                .sample_iter(&Alphanumeric)
                .take(value_len)
                .map(char::from)
                .collect();
            (key, value)
        })
        .collect();

    (
        update_authority,
        mint,
        name,
        symbol,
        uri,
        additional_metadata,
    )
}

/// Compare Light and SPL metadata structures for logical equivalence
/// Also tests that each format can serialize/deserialize its own data correctly
fn compare_metadata(light: &LightTokenMetadata, spl: &SplTokenMetadata, iteration: usize) {
    // Compare update authority (Light uses zero pubkey for None)
    let light_authority_bytes = if light.update_authority == Pubkey::from([0u8; 32]) {
        None
    } else {
        Some(light.update_authority.to_bytes())
    };
    let spl_authority_bytes =
        Option::<solana_pubkey::Pubkey>::from(spl.update_authority).map(|p| p.to_bytes());
    assert_eq!(
        light_authority_bytes, spl_authority_bytes,
        "Update authority mismatch at iteration {}",
        iteration
    );

    // Compare mint
    assert_eq!(
        light.mint.to_bytes(),
        spl.mint.to_bytes(),
        "Mint mismatch at iteration {}",
        iteration
    );

    // Compare name
    let light_name = String::from_utf8(light.name.clone()).unwrap_or_default();
    assert_eq!(
        light_name, spl.name,
        "Name mismatch at iteration {}",
        iteration
    );

    // Compare symbol
    let light_symbol = String::from_utf8(light.symbol.clone()).unwrap_or_default();
    assert_eq!(
        light_symbol, spl.symbol,
        "Symbol mismatch at iteration {}",
        iteration
    );

    // Compare URI
    let light_uri = String::from_utf8(light.uri.clone()).unwrap_or_default();
    assert_eq!(
        light_uri, spl.uri,
        "URI mismatch at iteration {}",
        iteration
    );

    // Compare additional metadata count
    assert_eq!(
        light.additional_metadata.len(),
        spl.additional_metadata.len(),
        "Additional metadata count mismatch at iteration {}",
        iteration
    );

    // Compare each additional metadata entry
    for (idx, (light_meta, spl_meta)) in light
        .additional_metadata
        .iter()
        .zip(spl.additional_metadata.iter())
        .enumerate()
    {
        let light_key = String::from_utf8(light_meta.key.clone()).unwrap_or_default();
        let light_value = String::from_utf8(light_meta.value.clone()).unwrap_or_default();
        assert_eq!(
            light_key, spl_meta.0,
            "Additional metadata key mismatch at iteration {}, index {}",
            iteration, idx
        );
        assert_eq!(
            light_value, spl_meta.1,
            "Additional metadata value mismatch at iteration {}, index {}",
            iteration, idx
        );
    }

    // Test Light serialization round-trip
    let light_bytes = light.try_to_vec().unwrap();
    let light_restored = LightTokenMetadata::try_from_slice(&light_bytes).unwrap();

    // Single assertion for complete Light struct
    assert_eq!(
        light, &light_restored,
        "Light serialization round-trip failed at iteration {}",
        iteration
    );

    // Test SPL serialization round-trip
    // SPL uses borsh v1.5 while Light uses borsh v0.10, so we need scoped imports
    use spl_token_metadata_interface::borsh::{BorshDeserialize, BorshSerialize};
    let mut spl_bytes = Vec::new();
    spl.serialize(&mut spl_bytes).unwrap();
    let spl_restored = SplTokenMetadata::deserialize(&mut spl_bytes.as_slice()).unwrap();

    // Single assertion for complete SPL struct
    assert_eq!(
        spl, &spl_restored,
        "SPL serialization round-trip failed at iteration {}",
        iteration
    );

    // Verify serialized byte lengths are reasonable
    assert!(
        !light_bytes.is_empty() && light_bytes.len() < 10000,
        "Light serialized size {} is unreasonable at iteration {}",
        light_bytes.len(),
        iteration
    );
    assert!(
        !spl_bytes.is_empty() && spl_bytes.len() < 10000,
        "SPL serialized size {} is unreasonable at iteration {}",
        spl_bytes.len(),
        iteration
    );
    assert_eq!(light_bytes, spl_bytes);
}

/// Randomized compatibility test for TokenMetadata borsh serialization (1k iterations)
#[test]
fn test_token_metadata_borsh_compatibility() {
    for i in 0..1000 {
        // Generate random data
        let (update_authority, mint, name, symbol, uri, additional_metadata) =
            generate_random_metadata();

        // Create Light Protocol TokenMetadata (uses zero pubkey for None)
        let light_metadata = LightTokenMetadata {
            update_authority: update_authority.unwrap_or_else(|| Pubkey::from([0u8; 32])),
            mint,
            name: name.as_bytes().to_vec(),
            symbol: symbol.as_bytes().to_vec(),
            uri: uri.as_bytes().to_vec(),
            additional_metadata: additional_metadata
                .iter()
                .map(|(k, v)| AdditionalMetadata {
                    key: k.as_bytes().to_vec(),
                    value: v.as_bytes().to_vec(),
                })
                .collect(),
            // Sha256Flat - currently the only supported version
        };

        // Create SPL TokenMetadata
        let spl_update_authority = if let Some(pubkey) = update_authority {
            OptionalNonZeroPubkey::try_from(Some(solana_pubkey::Pubkey::from(pubkey.to_bytes())))
                .unwrap()
        } else {
            OptionalNonZeroPubkey::try_from(None).unwrap()
        };

        let spl_metadata = SplTokenMetadata {
            update_authority: spl_update_authority,
            mint: solana_pubkey::Pubkey::from(mint.to_bytes()),
            name: name.clone(),
            symbol: symbol.clone(),
            uri: uri.clone(),
            additional_metadata: additional_metadata.clone(),
        };

        // Compare both metadata structures comprehensively
        compare_metadata(&light_metadata, &spl_metadata, i);
    }
}

/// Test edge cases and boundary conditions
#[test]
fn test_token_metadata_edge_cases() {
    // Test with empty additional metadata
    let light_empty = LightTokenMetadata {
        update_authority: Pubkey::from([0u8; 32]), // Zero pubkey represents None
        mint: Pubkey::from([0u8; 32]),
        name: b"X".to_vec(),   // Minimum length name
        symbol: b"X".to_vec(), // Minimum length symbol
        uri: vec![],           // Empty URI is allowed
        additional_metadata: vec![],
    };

    // Create corresponding SPL metadata
    let spl_empty = SplTokenMetadata {
        update_authority: OptionalNonZeroPubkey::try_from(None).unwrap(),
        mint: solana_pubkey::Pubkey::from([0u8; 32]),
        name: "X".to_string(),
        symbol: "X".to_string(),
        uri: String::new(),
        additional_metadata: vec![],
    };

    // Use compare_metadata for consistency
    compare_metadata(&light_empty, &spl_empty, 0);

    // Test with maximum reasonable metadata entries
    let mut max_metadata_light = vec![];
    let mut max_metadata_spl = vec![];
    for i in 0..10 {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        max_metadata_light.push(AdditionalMetadata {
            key: key.as_bytes().to_vec(),
            value: value.as_bytes().to_vec(),
        });
        max_metadata_spl.push((key, value));
    }

    let authority = Pubkey::from([255u8; 32]);
    let mint = Pubkey::from([1u8; 32]);

    let light_max = LightTokenMetadata {
        update_authority: authority,
        mint,
        name: b"Maximum Length Token Name Here32".to_vec(), // 32 chars
        symbol: b"MAXSYMBOL1".to_vec(),                     // 10 chars
        uri: vec![b'h'; 200],                               // Maximum tested URI length
        additional_metadata: max_metadata_light,
    };

    let spl_max = SplTokenMetadata {
        update_authority: OptionalNonZeroPubkey::try_from(Some(solana_pubkey::Pubkey::from(
            [255u8; 32],
        )))
        .unwrap(),
        mint: solana_pubkey::Pubkey::from([1u8; 32]),
        name: "Maximum Length Token Name Here32".to_string(),
        symbol: "MAXSYMBOL1".to_string(),
        uri: "h".repeat(200),
        additional_metadata: max_metadata_spl,
    };

    // Use compare_metadata for consistency
    compare_metadata(&light_max, &spl_max, 1);
}
