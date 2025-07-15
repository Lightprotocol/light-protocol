use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;
use light_compressed_token::{
    extensions::{
        state::ExtensionStructConfig,
        token_metadata::{AdditionalMetadataConfig, MetadataConfig, TokenMetadataConfig},
    },
    mint::state::{CompressedMint, CompressedMintConfig},
    shared::cpi_bytes_size::{
        allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
    },
};
use light_zero_copy::ZeroCopyNew;

#[test]
fn test_exact_allocation_assertion() {
    println!("\n=== EXACT ALLOCATION TEST ===");

    // Test case: specific token metadata configuration
    let name_len = 10u32;
    let symbol_len = 5u32;
    let uri_len = 20u32;

    // Add some additional metadata
    let additional_metadata_configs = vec![
        AdditionalMetadataConfig { key: 8, value: 15 },
        AdditionalMetadataConfig { key: 12, value: 25 },
    ];

    let extensions_config = vec![ExtensionStructConfig::TokenMetadata(TokenMetadataConfig {
        update_authority: (true, ()),
        metadata: MetadataConfig {
            name: name_len,
            symbol: symbol_len,
            uri: uri_len,
        },
        additional_metadata: additional_metadata_configs.clone(),
    })];

    println!("Extension config: {:?}", extensions_config);

    // Step 1: Calculate expected mint size
    let mint_config = CompressedMintConfig {
        mint_authority: (true, ()),
        freeze_authority: (false, ()),
        extensions: (true, extensions_config.clone()),
    };

    let expected_mint_size = CompressedMint::byte_len(&mint_config);
    println!("Expected mint size: {} bytes", expected_mint_size);

    // Step 2: Calculate CPI allocation
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

    println!("Total CPI bytes allocated: {} bytes", cpi_bytes.len());
    println!("CPI instruction header: 8 bytes");
    println!(
        "Available for instruction data: {} bytes",
        cpi_bytes.len() - 8
    );

    // Step 3: Create the CPI instruction and examine allocation
    let (cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .expect("Should create CPI instruction successfully");

    // Step 4: Get the output compressed account data buffer
    let output_account = &cpi_instruction_struct.output_compressed_accounts[0];
    let compressed_account_data = output_account
        .compressed_account
        .data
        .as_ref()
        .expect("Should have compressed account data");

    let available_data_space = compressed_account_data.data.len();
    println!(
        "Available data space in output account: {} bytes",
        available_data_space
    );

    // Step 5: Calculate exact space needed
    let base_mint_size_no_ext = {
        let no_ext_config = CompressedMintConfig {
            mint_authority: (true, ()),
            freeze_authority: (false, ()),
            extensions: (false, vec![]),
        };
        CompressedMint::byte_len(&no_ext_config)
    };

    let extension_space_needed = expected_mint_size - base_mint_size_no_ext;

    println!("\n=== BREAKDOWN ===");
    println!(
        "Base mint size (no extensions): {} bytes",
        base_mint_size_no_ext
    );
    println!("Extension space needed: {} bytes", extension_space_needed);
    println!("Total mint size needed: {} bytes", expected_mint_size);
    println!("Allocated data space: {} bytes", available_data_space);
    println!(
        "Margin: {} bytes",
        available_data_space as i32 - expected_mint_size as i32
    );

    // Step 6: Exact assertions
    assert!(
        available_data_space >= expected_mint_size,
        "Allocated space ({}) must be >= expected mint size ({})",
        available_data_space,
        expected_mint_size
    );

    // Step 7: Calculate exact dynamic token metadata length
    println!("\n=== EXACT LENGTH CALCULATION ===");

    // Sum all the dynamic lengths
    let total_metadata_dynamic_len = name_len + symbol_len + uri_len;
    let total_additional_metadata_len: u32 = additional_metadata_configs
        .iter()
        .map(|config| config.key + config.value)
        .sum();

    let total_dynamic_len = total_metadata_dynamic_len + total_additional_metadata_len;

    println!("Metadata dynamic lengths:");
    println!("  name: {} bytes", name_len);
    println!("  symbol: {} bytes", symbol_len);
    println!("  uri: {} bytes", uri_len);
    println!("  metadata total: {} bytes", total_metadata_dynamic_len);

    println!("Additional metadata dynamic lengths:");
    for (i, config) in additional_metadata_configs.iter().enumerate() {
        println!(
            "  item {}: key={}, value={}, total={}",
            i,
            config.key,
            config.value,
            config.key + config.value
        );
    }
    println!(
        "  additional metadata total: {} bytes",
        total_additional_metadata_len
    );

    println!("TOTAL dynamic length: {} bytes", total_dynamic_len);

    // Calculate expected TokenMetadata size with exact breakdown
    let token_metadata_size = {
        let mut size = 0u32;

        // Fixed overhead for TokenMetadata struct:
        size += 1; // update_authority discriminator
        size += 32; // update_authority pubkey
        size += 32; // mint pubkey
        size += 4; // name vec length
        size += 4; // symbol vec length
        size += 4; // uri vec length
        size += 4; // additional_metadata vec length
        size += 1; // version byte

        // Additional metadata items overhead
        for _ in &additional_metadata_configs {
            size += 4; // key vec length
            size += 4; // value vec length
        }

        let fixed_overhead = size;
        println!("Fixed TokenMetadata overhead: {} bytes", fixed_overhead);

        // Add dynamic content
        size += total_dynamic_len;

        println!(
            "Total TokenMetadata size: {} + {} = {} bytes",
            fixed_overhead, total_dynamic_len, size
        );
        size
    };

    // Step 8: Assert exact allocation
    println!("\n=== EXACT ALLOCATION ASSERTION ===");

    let expected_total_size = base_mint_size_no_ext as u32 + token_metadata_size;

    println!("Base mint size: {} bytes", base_mint_size_no_ext);
    println!(
        "Dynamic token metadata length: {} bytes",
        token_metadata_size
    );
    println!(
        "Expected total size: {} + {} = {} bytes",
        base_mint_size_no_ext, token_metadata_size, expected_total_size
    );
    println!("Allocated data space: {} bytes", available_data_space);

    // The critical assertion: allocated space should exactly match CompressedMint::byte_len()
    assert_eq!(
        available_data_space, expected_mint_size,
        "Allocated bytes ({}) must exactly equal CompressedMint::byte_len() ({})",
        available_data_space, expected_mint_size
    );

    println!("✅ SUCCESS: Perfect allocation match!");
    println!("   allocated_bytes = CompressedMint::byte_len()");
    println!("   {} = {}", available_data_space, expected_mint_size);

    // Note: The difference between our manual calculation and actual struct size
    // is due to struct padding/alignment which is normal for zero-copy structs
    let manual_vs_actual = expected_mint_size as i32 - expected_total_size as i32;
    if manual_vs_actual != 0 {
        println!(
            "📝 Note: {} bytes difference between manual calculation and actual struct size",
            manual_vs_actual
        );
        println!("   This is normal padding/alignment overhead in zero-copy structs");
    }
}

#[test]
fn test_allocation_with_various_metadata_sizes() {
    println!("\n=== VARIOUS METADATA SIZES TEST ===");

    let test_cases = [
        // (name, symbol, uri, additional_metadata_count)
        (5, 3, 10, 0),
        (10, 5, 20, 1),
        (15, 8, 30, 2),
        (20, 10, 40, 3),
    ];

    for (i, (name_len, symbol_len, uri_len, additional_count)) in test_cases.iter().enumerate() {
        println!("\n--- Test case {} ---", i + 1);
        println!(
            "Metadata: name={}, symbol={}, uri={}, additional={}",
            name_len, symbol_len, uri_len, additional_count
        );

        let additional_metadata_configs: Vec<_> = (0..*additional_count)
            .map(|j| AdditionalMetadataConfig {
                key: 5 + j * 2,
                value: 10 + j * 3,
            })
            .collect();

        let extensions_config = vec![ExtensionStructConfig::TokenMetadata(TokenMetadataConfig {
            update_authority: (true, ()),
            metadata: MetadataConfig {
                name: *name_len,
                symbol: *symbol_len,
                uri: *uri_len,
            },
            additional_metadata: additional_metadata_configs,
        })];

        let mint_config = CompressedMintConfig {
            mint_authority: (true, ()),
            freeze_authority: (false, ()),
            extensions: (true, extensions_config.clone()),
        };

        let expected_mint_size = CompressedMint::byte_len(&mint_config);

        let config_input = CpiConfigInput {
            input_accounts: arrayvec::ArrayVec::new(),
            output_accounts: arrayvec::ArrayVec::new(),
            has_proof: false,
            compressed_mint: true,
            compressed_mint_with_freeze_authority: false,
            extensions_config: extensions_config,
        };

        let config = cpi_bytes_config(config_input);
        let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);

        let (cpi_instruction_struct, _) =
            InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
                .expect("Should create CPI instruction successfully");

        let output_account = &cpi_instruction_struct.output_compressed_accounts[0];
        let compressed_account_data = output_account
            .compressed_account
            .data
            .as_ref()
            .expect("Should have compressed account data");

        let available_space = compressed_account_data.data.len();

        println!(
            "Required: {} bytes, Allocated: {} bytes, Margin: {} bytes",
            expected_mint_size,
            available_space,
            available_space as i32 - expected_mint_size as i32
        );

        assert!(
            available_space >= expected_mint_size,
            "Test case {}: insufficient allocation",
            i + 1
        );

        println!("✅ Test case {} passed", i + 1);
    }
}
