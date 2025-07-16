use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;
use light_compressed_token::{
    shared::cpi_bytes_size::{
        allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
    },
};
use light_ctoken_types::{
    state::{ExtensionStructConfig, CompressedMint, CompressedMintConfig},
    instructions::extensions::token_metadata::{AdditionalMetadataConfig, MetadataConfig, TokenMetadataConfig},
};
use light_zero_copy::ZeroCopyNew;

#[test]
fn test_extension_allocation_only() {
    // Test 1: No extensions - should work
    let config_input_no_ext = CpiConfigInput {
        input_accounts: arrayvec::ArrayVec::new(),
        output_accounts: arrayvec::ArrayVec::new(),
        has_proof: false,
        compressed_mint: true,
        compressed_mint_with_freeze_authority: false,
        extensions_config: vec![],
    };

    let config_no_ext = cpi_bytes_config(config_input_no_ext);
    let cpi_bytes_no_ext = allocate_invoke_with_read_only_cpi_bytes(&config_no_ext);

    println!(
        "No extensions - CPI bytes length: {}",
        cpi_bytes_no_ext.len()
    );

    // Test 2: With minimal token metadata extension
    let extensions_config = vec![ExtensionStructConfig::TokenMetadata(TokenMetadataConfig {
        update_authority: (true, ()),
        metadata: MetadataConfig {
            name: 5,   // 5 bytes
            symbol: 3, // 3 bytes
            uri: 10,   // 10 bytes
        },
        additional_metadata: vec![], // No additional metadata
    })];

    let config_input_with_ext = CpiConfigInput {
        input_accounts: arrayvec::ArrayVec::new(),
        output_accounts: arrayvec::ArrayVec::new(),
        has_proof: false,
        compressed_mint: true,
        compressed_mint_with_freeze_authority: false,
        extensions_config: extensions_config.clone(),
    };

    let config_with_ext = cpi_bytes_config(config_input_with_ext);
    let cpi_bytes_with_ext = allocate_invoke_with_read_only_cpi_bytes(&config_with_ext);

    println!(
        "With extensions - CPI bytes length: {}",
        cpi_bytes_with_ext.len()
    );
    println!(
        "Difference: {}",
        cpi_bytes_with_ext.len() - cpi_bytes_no_ext.len()
    );

    // Test 3: Calculate expected mint size with extensions
    let mint_config = CompressedMintConfig {
        mint_authority: (true, ()),
        freeze_authority: (false, ()),
        extensions: (true, extensions_config),
    };

    let expected_mint_size = CompressedMint::byte_len(&mint_config);
    println!("Expected mint size with extensions: {}", expected_mint_size);

    // Test 4: Try to create the CPI instruction structure to see if allocation is sufficient
    let mut cpi_bytes_copy = cpi_bytes_with_ext.clone();
    let result = InstructionDataInvokeCpiWithReadOnly::new_zero_copy(
        &mut cpi_bytes_copy[8..],
        config_with_ext,
    );

    match result {
        Ok(_) => println!("✅ CPI instruction creation succeeded"),
        Err(e) => println!("❌ CPI instruction creation failed: {:?}", e),
    }
}

#[test]
fn test_progressive_extension_sizes() {
    // Test progressively larger extensions to find the breaking point
    let base_sizes = [
        (1, 1, 1),   // Minimal
        (5, 3, 10),  // Small
        (10, 5, 20), // Medium
        (20, 8, 40), // Large
    ];

    for (name_len, symbol_len, uri_len) in base_sizes {
        println!(
            "\n--- Testing sizes: name={}, symbol={}, uri={} ---",
            name_len, symbol_len, uri_len
        );

        let extensions_config = vec![ExtensionStructConfig::TokenMetadata(TokenMetadataConfig {
            update_authority: (true, ()),
            metadata: MetadataConfig {
                name: name_len,
                symbol: symbol_len,
                uri: uri_len,
            },
            additional_metadata: vec![],
        })];

        let config_input = CpiConfigInput {
            input_accounts: arrayvec::ArrayVec::new(),
            output_accounts: arrayvec::ArrayVec::new(),
            has_proof: false,
            compressed_mint: true,
            compressed_mint_with_freeze_authority: false,
            extensions_config: extensions_config.clone(),
        };

        let config = cpi_bytes_config(config_input);
        let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);

        println!("CPI bytes allocated: {}", cpi_bytes.len());

        let mint_config = CompressedMintConfig {
            mint_authority: (true, ()),
            freeze_authority: (false, ()),
            extensions: (true, extensions_config),
        };

        let expected_mint_size = CompressedMint::byte_len(&mint_config);
        println!("Expected mint size: {}", expected_mint_size);

        let result =
            InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config);

        match result {
            Ok(_) => println!("✅ Success"),
            Err(e) => println!("❌ Failed: {:?}", e),
        }
    }
}
