//! Test that top_up_lamports_from_slice produces identical results to full deserialization.

use light_compressed_account::Pubkey;
use light_token_interface::state::{
    top_up_lamports_from_slice, Token, TokenConfig, CompressibleExtensionConfig,
    CompressionInfoConfig, ExtensionStructConfig,
};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyNew};

#[test]
fn test_top_up_lamports_matches_full_deserialization() {
    // Create a Token with Compressible extension
    let config = TokenConfig {
        mint: Pubkey::default(),
        owner: Pubkey::default(),
        state: 1,
        extensions: Some(vec![ExtensionStructConfig::Compressible(
            CompressibleExtensionConfig {
                info: CompressionInfoConfig { rent_config: () },
            },
        )]),
    };

    let size = Token::byte_len(&config).unwrap();
    let mut buffer = vec![0u8; size];
    let (mut token, _) = Token::new_zero_copy(&mut buffer, config).unwrap();

    // Set known values in CompressionInfo via zero-copy
    let ext = token.extensions.as_mut().unwrap();
    let compressible = ext
        .iter_mut()
        .find_map(|e| match e {
            light_token_interface::state::ZExtensionStructMut::Compressible(c) => Some(c),
            _ => None,
        })
        .unwrap();

    // Set test values
    compressible.info.lamports_per_write = 1000.into();
    compressible.info.last_claimed_slot = 13500.into(); // Epoch 1
    compressible.info.rent_exemption_paid = 50_000.into();
    compressible.info.rent_config.base_rent = 128.into();
    compressible.info.rent_config.compression_cost = 11000.into();
    compressible.info.rent_config.lamports_per_byte_per_epoch = 1;
    compressible.info.rent_config.max_funded_epochs = 2;

    // Test parameters
    let current_slot = 27000u64; // Epoch 2
    let current_lamports = 100_000u64;

    // Calculate using optimized function
    let optimized_result = top_up_lamports_from_slice(&buffer, current_lamports, current_slot)
        .expect("Should return Some");

    // Calculate using full deserialization
    let (ctoken_read, _) = Token::zero_copy_at(&buffer).unwrap();
    let compressible_read = ctoken_read
        .extensions
        .as_ref()
        .unwrap()
        .iter()
        .find_map(|e| match e {
            light_token_interface::state::ZExtensionStruct::Compressible(c) => Some(c),
            _ => None,
        })
        .unwrap();

    let full_deser_result = compressible_read
        .info
        .calculate_top_up_lamports(buffer.len() as u64, current_slot, current_lamports)
        .expect("Should succeed");

    assert_eq!(
        optimized_result, full_deser_result,
        "Optimized result should match full deserialization"
    );
}
