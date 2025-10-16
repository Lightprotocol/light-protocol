// Note: borsh imports removed as they are not needed for allocation tests
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly;
use light_compressed_token::shared::cpi_bytes_size::{
    allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
};
use light_ctoken_types::state::{
    extensions::TokenMetadataConfig, CompressedMint, CompressedMintConfig, ExtensionStructConfig,
};
use light_zero_copy::{traits::ZeroCopyAt, ZeroCopyNew};

#[test]
fn test_extension_allocation_only() {
    // Test 1: No extensions - should work
    let mint_config_no_ext = CompressedMintConfig {
        base: (),
        metadata: (),
        extensions: (false, vec![]),
    };
    let expected_mint_size_no_ext = CompressedMint::byte_len(&mint_config_no_ext).unwrap();

    let mut outputs_no_ext = tinyvec::ArrayVec::<[(bool, u32); 35]>::new();
    outputs_no_ext.push((true, expected_mint_size_no_ext as u32)); // Mint account has address

    let config_input_no_ext = CpiConfigInput {
        input_accounts: tinyvec::ArrayVec::<[bool; 8]>::new(),
        output_accounts: outputs_no_ext,
        has_proof: false,
        new_address_params: 1,
    };

    let config_no_ext = cpi_bytes_config(config_input_no_ext);
    let cpi_bytes_no_ext = allocate_invoke_with_read_only_cpi_bytes(&config_no_ext).unwrap();

    println!(
        "No extensions - CPI bytes length: {}",
        cpi_bytes_no_ext.len()
    );

    // Test 2: With minimal token metadata extension
    let extensions_config = vec![ExtensionStructConfig::TokenMetadata(TokenMetadataConfig {
        name: 5,                     // 5 bytes
        symbol: 3,                   // 3 bytes
        uri: 10,                     // 10 bytes
        additional_metadata: vec![], // No additional metadata
    })];

    let mint_config_with_ext = CompressedMintConfig {
        base: (),
        metadata: (),
        extensions: (true, extensions_config.clone()),
    };
    let expected_mint_size_with_ext = CompressedMint::byte_len(&mint_config_with_ext).unwrap();

    let mut outputs_with_ext = tinyvec::ArrayVec::<[(bool, u32); 35]>::new();
    outputs_with_ext.push((true, expected_mint_size_with_ext as u32)); // Mint account has address

    let config_input_with_ext = CpiConfigInput {
        input_accounts: tinyvec::ArrayVec::<[bool; 8]>::new(),
        output_accounts: outputs_with_ext,
        has_proof: false,
        new_address_params: 1,
    };

    let config_with_ext = cpi_bytes_config(config_input_with_ext);
    let cpi_bytes_with_ext = allocate_invoke_with_read_only_cpi_bytes(&config_with_ext).unwrap();

    println!(
        "With extensions - CPI bytes length: {}",
        cpi_bytes_with_ext.len()
    );
    println!(
        "Difference: {}",
        cpi_bytes_with_ext.len() as i32 - cpi_bytes_no_ext.len() as i32
    );

    // Test 3: Calculate expected mint size with extensions
    println!(
        "Expected mint size with extensions: {} bytes",
        expected_mint_size_with_ext
    );
    println!(
        "Expected mint size without extensions: {} bytes",
        expected_mint_size_no_ext
    );

    // Test 4: Verify allocation correctness with zero-copy compatibility
    let mut cpi_bytes_copy = cpi_bytes_with_ext.clone();
    let (cpi_instruction_struct, _) = InstructionDataInvokeCpiWithReadOnly::new_zero_copy(
        &mut cpi_bytes_copy[8..],
        config_with_ext,
    )
    .expect("CPI instruction creation should succeed");

    // Verify the allocation structure is correct
    assert_eq!(
        cpi_instruction_struct.output_compressed_accounts.len(),
        1,
        "Should have exactly 1 output account"
    );
    assert_eq!(
        cpi_instruction_struct.input_compressed_accounts.len(),
        0,
        "Should have no input accounts"
    );

    let output_account = &cpi_instruction_struct.output_compressed_accounts[0];

    if let Some(ref account_data) = output_account.compressed_account.data {
        let available_space = account_data.data.len();

        // CRITICAL ASSERTION: Exact allocation matches expected mint size
        assert_eq!(
            available_space, expected_mint_size_with_ext,
            "Allocated space ({}) must exactly equal expected mint size ({})",
            available_space, expected_mint_size_with_ext
        );

        // Test that we can create a CompressedMint with the allocated space (zero-copy compatibility)
        let mint_test_data = vec![0u8; available_space];
        let test_mint_result = CompressedMint::zero_copy_at(&mint_test_data);
        assert!(
            test_mint_result.is_ok(),
            "Allocated space should be valid for zero-copy CompressedMint creation"
        );

        println!(
            "✅ Allocation test successful - {} bytes exactly allocated for mint with extensions",
            available_space
        );
    } else {
        panic!("Output account must have data space allocated");
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
            name: name_len,
            symbol: symbol_len,
            uri: uri_len,
            additional_metadata: vec![],
        })];

        let mint_config = CompressedMintConfig {
            base: (),
            metadata: (),
            extensions: (true, extensions_config),
        };

        let expected_mint_size = CompressedMint::byte_len(&mint_config).unwrap();
        println!("Expected mint size: {}", expected_mint_size);

        let mut outputs = tinyvec::ArrayVec::<[(bool, u32); 35]>::new();
        outputs.push((true, expected_mint_size as u32)); // Mint account has address

        let config_input = CpiConfigInput {
            input_accounts: tinyvec::ArrayVec::<[bool; 8]>::new(),
            output_accounts: outputs,
            has_proof: false,
            new_address_params: 1,
        };

        let config = cpi_bytes_config(config_input);
        let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config).unwrap();

        println!("CPI bytes allocated: {}", cpi_bytes.len());

        let (cpi_instruction_struct, _) =
            InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
                .unwrap_or_else(|_| {
                    panic!(
                "CPI instruction creation should succeed for sizes: name={}, symbol={}, uri={}",
                name_len, symbol_len, uri_len
            )
                });

        // Verify allocation correctness with zero-copy compatibility
        assert_eq!(
            cpi_instruction_struct.output_compressed_accounts.len(),
            1,
            "Should have exactly 1 output account for sizes: name={}, symbol={}, uri={}",
            name_len,
            symbol_len,
            uri_len
        );
        assert_eq!(
            cpi_instruction_struct.input_compressed_accounts.len(),
            0,
            "Should have no input accounts for sizes: name={}, symbol={}, uri={}",
            name_len,
            symbol_len,
            uri_len
        );

        let output_account = &cpi_instruction_struct.output_compressed_accounts[0];

        if let Some(ref account_data) = output_account.compressed_account.data {
            let available_space = account_data.data.len();

            // CRITICAL ASSERTION: Allocation matches expected mint size
            assert_eq!(
                available_space, expected_mint_size,
                "Sizes name={}, symbol={}, uri={}: Allocated space ({}) must exactly equal expected mint size ({})",
                name_len, symbol_len, uri_len, available_space, expected_mint_size
            );

            // Test zero-copy compatibility - verify allocated space can be used for CompressedMint
            let mint_test_data = vec![0u8; available_space];
            let test_mint_result = CompressedMint::zero_copy_at(&mint_test_data);
            assert!(test_mint_result.is_ok(), "Sizes name={}, symbol={}, uri={}: Allocated space should be valid for zero-copy CompressedMint", name_len, symbol_len, uri_len);

            println!("✅ Success - Allocation verified for sizes: name={}, symbol={}, uri={} - {} bytes exactly allocated", name_len, symbol_len, uri_len, available_space);
        } else {
            panic!(
                "Sizes name={}, symbol={}, uri={}: Output account must have data space allocated",
                name_len, symbol_len, uri_len
            );
        }
    }
}
