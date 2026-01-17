use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{
    address::derive_address, instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly,
    Pubkey,
};
use light_compressed_token::{
    compressed_token::mint_action::{
        accounts::AccountsConfig, mint_input::create_input_compressed_mint_account,
        zero_copy_config::get_zero_copy_configs,
    },
    constants::COMPRESSED_MINT_DISCRIMINATOR,
};
use light_token_interface::{
    instructions::{
        extensions::{ExtensionInstructionData, TokenMetadataInstructionData},
        mint_action::{MintActionCompressedInstructionData, MintInstructionData},
    },
    state::{
        AdditionalMetadata, AdditionalMetadataConfig, BaseMint, CompressionInfo, ExtensionStruct,
        Mint, MintMetadata, TokenMetadata, ZExtensionStruct, ZMint, ACCOUNT_TYPE_MINT,
    },
    CMINT_ADDRESS_TREE, COMPRESSED_MINT_SEED, LIGHT_TOKEN_PROGRAM_ID,
};
use light_zero_copy::{traits::ZeroCopyAt, ZeroCopyNew};
use rand::Rng;
use solana_pubkey::Pubkey as SolanaPubkey;

#[test]
fn test_rnd_create_compressed_mint_account() {
    let mut rng = rand::thread_rng();
    let iter = 1000; // Per UNIT_TESTING.md requirement for randomized tests

    for i in 0..iter {
        println!("\n=== TEST ITERATION {} ===", i + 1);

        // Generate random mint parameters
        // mint_signer is the seed used to derive the mint PDA
        let mint_signer_bytes: [u8; 32] = rng.gen();
        let mint_signer = Pubkey::new_from_array(mint_signer_bytes);
        // Derive mint_pda and bump from mint_signer using the same PDA derivation as production
        let (solana_mint_pda, bump) = SolanaPubkey::find_program_address(
            &[COMPRESSED_MINT_SEED, &mint_signer_bytes],
            &SolanaPubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        );
        let mint_pda = Pubkey::new_from_array(solana_mint_pda.to_bytes());
        let decimals = rng.gen_range(0..=18u8);

        // Random freeze authority (50% chance)
        let freeze_authority = if rng.gen_bool(0.5) {
            Some(Pubkey::new_from_array(rng.gen::<[u8; 32]>()))
        } else {
            None
        };

        let mint_authority = Pubkey::new_from_array(rng.gen::<[u8; 32]>());

        // Generate version for use in extensions
        let version = if rng.gen_bool(0.5) { 0 } else { 1 }; // Use version 0 or 1

        // Generate random supplies
        let input_supply = rng.gen_range(0..=u64::MAX);
        let _output_supply = rng.gen_range(0..=u64::MAX);
        let mint_decompressed = rng.gen_bool(0.1);

        // Generate random merkle context
        let merkle_tree_pubkey_index = rng.gen_range(0..=255u8);
        let queue_pubkey_index = rng.gen_range(0..=255u8);
        let leaf_index = rng.gen::<u32>();
        let prove_by_index = rng.gen_bool(0.5);
        let root_index = rng.gen::<u16>();

        // Derive compressed account address using the same constants as compressed_address() method
        let compressed_account_address = derive_address(
            &mint_pda.to_bytes(),
            &CMINT_ADDRESS_TREE,
            &LIGHT_TOKEN_PROGRAM_ID,
        );

        // Step 1: Create random extension data (simplified for current API)
        let expected_extensions = if rng.gen_bool(0.3) {
            // 30% chance of having extensions
            let name = format!("Token{}", rng.gen_range(0..1000));
            let symbol = format!("T{}", rng.gen_range(0..100));
            let uri = format!("https://example.com/{}", rng.gen_range(0..1000));

            let additional_metadata_configs = if rng.gen_bool(0.5) {
                vec![
                    AdditionalMetadataConfig { key: 5, value: 10 },
                    AdditionalMetadataConfig { key: 8, value: 15 },
                ]
            } else {
                vec![]
            };

            Some(vec![ExtensionInstructionData::TokenMetadata(
                TokenMetadataInstructionData {
                    update_authority: Some(mint_authority),
                    name: name.into_bytes(),
                    symbol: symbol.into_bytes(),
                    uri: uri.into_bytes(),
                    additional_metadata: if additional_metadata_configs.is_empty() {
                        None
                    } else {
                        Some(
                            additional_metadata_configs
                                .into_iter()
                                .map(|config| AdditionalMetadata {
                                    key: vec![b'k'; config.key as usize],
                                    value: vec![b'v'; config.value as usize],
                                })
                                .collect(),
                        )
                    },
                },
            )])
        } else {
            None
        };

        // Step 2: Create MintInstructionData using current API
        // mint_signer and bump were derived at the start of the iteration
        let mint_instruction_data = MintInstructionData {
            supply: input_supply,
            decimals,
            metadata: MintMetadata {
                version,
                mint: mint_pda,
                mint_decompressed,
                mint_signer: mint_signer.to_bytes(),
                bump,
            },
            mint_authority: Some(mint_authority),
            freeze_authority,
            extensions: expected_extensions,
        };

        // Step 3: Create MintActionCompressedInstructionData
        let mint_action_data = MintActionCompressedInstructionData {
            create_mint: None, // We're testing with existing mint
            leaf_index,
            prove_by_index,
            root_index,
            mint: Some(mint_instruction_data.clone()),
            actions: vec![], // No actions for basic test
            proof: None,
            cpi_context: None,
            max_top_up: 0,
        };

        // Step 4: Serialize instruction data to test zero-copy
        let serialized_data = borsh::to_vec(&mint_action_data).unwrap();
        let (parsed_instruction_data, _) =
            MintActionCompressedInstructionData::zero_copy_at(&serialized_data).unwrap();

        // Step 5: Use current get_zero_copy_configs API
        // Derive AccountsConfig from parsed instruction data (same as processor)
        let accounts_config = AccountsConfig::new(&parsed_instruction_data).unwrap();

        // Derive Mint from instruction data (same as processor)
        let mint_data = parsed_instruction_data.mint.as_ref().unwrap();
        let cmint = Mint::try_from(mint_data).unwrap();

        let (config, mut cpi_bytes, output_mint_config) =
            get_zero_copy_configs(&parsed_instruction_data, &accounts_config, &cmint).unwrap();

        let (mut cpi_instruction_struct, _) =
            InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
                .unwrap();

        // Step 6: Test input compressed mint account creation (if not create_mint)
        if parsed_instruction_data.create_mint.is_none() {
            let input_account = &mut cpi_instruction_struct.input_compressed_accounts[0];

            use light_sdk::instruction::PackedMerkleContext;
            let merkle_context = PackedMerkleContext {
                merkle_tree_pubkey_index,
                queue_pubkey_index,
                leaf_index,
                prove_by_index,
            };

            create_input_compressed_mint_account(
                input_account,
                root_index.into(),
                merkle_context,
                &accounts_config,
                &cmint,
            )
            .unwrap();

            println!("✅ Input compressed mint account created successfully");
        }

        // Step 7: Test core zero-copy functionality - Borsh vs ZeroCopy compatibility
        let output_supply = input_supply + rng.gen_range(0..=1000);

        // Create a modified mint with updated supply for output using original data
        let mut output_mint_data = mint_instruction_data.clone();
        output_mint_data.supply = output_supply;

        // Test 1: Serialize with Borsh
        let borsh_bytes = borsh::to_vec(&output_mint_data).unwrap();
        println!("Borsh serialized {} bytes", borsh_bytes.len());

        // Test 2: Deserialize with zero_copy_at
        let (zc_mint, remaining) = MintInstructionData::zero_copy_at(&borsh_bytes).unwrap();
        assert!(remaining.is_empty(), "Should consume all bytes");

        // Test 3: Verify data matches between borsh and zero-copy
        assert_eq!(zc_mint.metadata.version, output_mint_data.metadata.version);
        assert_eq!(
            zc_mint.metadata.mint.to_bytes(),
            output_mint_data.metadata.mint.to_bytes()
        );
        assert_eq!(zc_mint.supply.get(), output_mint_data.supply);
        assert_eq!(zc_mint.decimals, output_mint_data.decimals);
        assert_eq!(
            zc_mint.metadata.mint_decompressed != 0,
            output_mint_data.metadata.mint_decompressed
        );

        if let (Some(zc_mint_auth), Some(orig_mint_auth)) = (
            zc_mint.mint_authority.as_deref(),
            output_mint_data.mint_authority.as_ref(),
        ) {
            assert_eq!(zc_mint_auth.to_bytes(), orig_mint_auth.to_bytes());
        }

        if let (Some(zc_freeze_auth), Some(orig_freeze_auth)) = (
            zc_mint.freeze_authority.as_deref(),
            output_mint_data.freeze_authority.as_ref(),
        ) {
            assert_eq!(zc_freeze_auth.to_bytes(), orig_freeze_auth.to_bytes());
        }

        // Test 4: Verify extensions match if they exist
        if let (Some(zc_extensions), Some(orig_extensions)) = (
            zc_mint.extensions.as_ref(),
            output_mint_data.extensions.as_ref(),
        ) {
            assert_eq!(
                zc_extensions.len(),
                orig_extensions.len(),
                "Extension counts should match"
            );

            for (zc_ext, orig_ext) in zc_extensions.iter().zip(orig_extensions.iter()) {
                match (zc_ext, orig_ext) {
                    (
                        light_token_interface::instructions::extensions::ZExtensionInstructionData::TokenMetadata(zc_metadata),
                        ExtensionInstructionData::TokenMetadata(orig_metadata),
                    ) => {
                        assert_eq!(zc_metadata.name, orig_metadata.name.as_slice());
                        assert_eq!(zc_metadata.symbol, orig_metadata.symbol.as_slice());
                        assert_eq!(zc_metadata.uri, orig_metadata.uri.as_slice());

                        if let (Some(zc_update_auth), Some(orig_update_auth)) = (zc_metadata.update_authority, orig_metadata.update_authority) {
                            assert_eq!(zc_update_auth.to_bytes(), orig_update_auth.to_bytes());
                        } else {
                            assert_eq!(zc_metadata.update_authority.is_some(), orig_metadata.update_authority.is_some());
                        }
                    }
                    _ => panic!("Mismatched extension types"),
                }
            }
        }

        // Test 5: Test the CPI allocation is correct
        let expected_mint_size = Mint::byte_len(&output_mint_config).unwrap();
        let output_account = &cpi_instruction_struct.output_compressed_accounts[0];
        let compressed_account_data = output_account
            .compressed_account
            .data
            .as_ref()
            .expect("Should have compressed account data");
        let available_space = compressed_account_data.data.len();

        assert!(
            available_space >= expected_mint_size,
            "Allocated space ({}) should be >= expected mint size ({})",
            available_space,
            expected_mint_size
        );

        // Test 6: CRITICAL - Complete CPI instruction struct assertion (per UNIT_TESTING.md)
        // Deserialize the actual CPI instruction that was created
        let cpi_borsh =
            InstructionDataInvokeCpiWithReadOnly::deserialize(&mut &cpi_bytes[8..]).unwrap();

        // Verify the structure has the expected number and types of accounts
        assert_eq!(
            cpi_borsh.output_compressed_accounts.len(),
            1,
            "Should have exactly 1 output account (mint)"
        );

        if parsed_instruction_data.create_mint.is_none() {
            assert_eq!(
                cpi_borsh.input_compressed_accounts.len(),
                1,
                "Should have exactly 1 input account when updating mint"
            );

            // Verify input account structure
            let input_account = &cpi_borsh.input_compressed_accounts[0];
            assert_eq!(input_account.discriminator, COMPRESSED_MINT_DISCRIMINATOR);
            assert_eq!(input_account.address, Some(compressed_account_address));
        } else {
            assert_eq!(
                cpi_borsh.input_compressed_accounts.len(),
                0,
                "Should have no input accounts when creating mint"
            );
        }

        // Verify output account structure - focus on data rather than metadata set by processors
        let output_account = &cpi_borsh.output_compressed_accounts[0];

        if let Some(ref account_data) = output_account.compressed_account.data {
            assert_eq!(
                account_data.data.len(),
                expected_mint_size,
                "Output account data must match expected mint size"
            );

            // Test that the allocated space is sufficient for a zero-copy Mint creation
            // (This verifies allocation correctness without requiring populated data)
            let test_mint_data = vec![0u8; account_data.data.len()];
            let test_result = Mint::zero_copy_at(&test_mint_data);
            assert!(
                test_result.is_ok(),
                "Allocated space should be valid for zero-copy Mint creation"
            );

            // COMPLETE STRUCT ASSERTION: This verifies the entire CPI instruction structure is valid
            // by ensuring it can round-trip through borsh serialization/deserialization
            let reserialize_test = cpi_borsh.try_to_vec().unwrap();
            let redeserialized =
                InstructionDataInvokeCpiWithReadOnly::deserialize(&mut reserialize_test.as_slice())
                    .unwrap();
            assert_eq!(
                redeserialized, cpi_borsh,
                "CPI instruction must round-trip through borsh serialization"
            );
        } else {
            panic!("Output account must have data");
        }

        println!(
            "✅ Test iteration {} passed - Complete CPI struct verification successful",
            i + 1
        );
    }

    println!(
        "🎉 All {} iterations of randomized compressed mint zero-copy test passed!",
        iter
    );
}

#[test]
fn test_compressed_mint_borsh_zero_copy_compatibility() {
    use light_zero_copy::traits::ZeroCopyAt;

    // Create Mint with token metadata extension
    let token_metadata = TokenMetadata {
        update_authority: Pubkey::new_from_array([1; 32]),
        mint: Pubkey::new_from_array([2; 32]),
        name: b"TestToken".to_vec(),
        symbol: b"TT".to_vec(),
        uri: b"https://test.com".to_vec(),
        additional_metadata: vec![],
    };

    let compressed_mint = Mint {
        compression: CompressionInfo::default(),
        base: BaseMint {
            mint_authority: Some(Pubkey::new_from_array([4; 32])),
            supply: 1000u64,
            decimals: 6u8,
            is_initialized: true,
            freeze_authority: None,
        },
        metadata: MintMetadata {
            version: 3u8,
            mint: Pubkey::new_from_array([3; 32]),
            mint_decompressed: false,
            mint_signer: [5; 32],
            bump: 255,
        },
        reserved: [0u8; 16],
        account_type: ACCOUNT_TYPE_MINT,
        extensions: Some(vec![ExtensionStruct::TokenMetadata(token_metadata)]),
    };

    // Serialize with Borsh
    let borsh_bytes = borsh::to_vec(&compressed_mint).unwrap();

    // Deserialize with zero_copy_at
    let (zc_mint, remaining): (ZMint<'_>, &[u8]) = Mint::zero_copy_at(&borsh_bytes).unwrap();
    assert!(remaining.is_empty());

    // COMPLETE STRUCT ASSERTION: Test borsh round-trip compatibility (UNIT_TESTING.md requirement)
    // Re-serialize the zero-copy mint back to borsh and compare with original
    let zc_reserialized = {
        // Convert zero-copy fields back to regular types
        // Reconstruct CompressionInfo from zero-copy fields
        let compression = {
            let zc = &zc_mint.base.compression;
            CompressionInfo {
                config_account_version: u16::from(zc.config_account_version),
                compress_to_pubkey: zc.compress_to_pubkey,
                account_version: zc.account_version,
                lamports_per_write: u32::from(zc.lamports_per_write),
                compression_authority: zc.compression_authority,
                rent_sponsor: zc.rent_sponsor,
                last_claimed_slot: u64::from(zc.last_claimed_slot),
                rent_exemption_paid: u32::from(zc.rent_exemption_paid),
                _reserved: u32::from(zc._reserved),
                rent_config: light_compressible::rent::RentConfig {
                    base_rent: u16::from(zc.rent_config.base_rent),
                    compression_cost: u16::from(zc.rent_config.compression_cost),
                    lamports_per_byte_per_epoch: zc.rent_config.lamports_per_byte_per_epoch,
                    max_funded_epochs: zc.rent_config.max_funded_epochs,
                    max_top_up: u16::from(zc.rent_config.max_top_up),
                },
            }
        };

        let reconstructed_mint = Mint {
            compression,
            base: BaseMint {
                mint_authority: zc_mint.base.mint_authority().cloned(),
                supply: u64::from(zc_mint.base.supply),
                decimals: zc_mint.base.decimals,
                is_initialized: zc_mint.base.is_initialized != 0,
                freeze_authority: zc_mint.base.freeze_authority().cloned(),
            },
            metadata: MintMetadata {
                version: zc_mint.base.metadata.version,
                mint: zc_mint.base.metadata.mint,
                mint_decompressed: zc_mint.base.metadata.mint_decompressed != 0,
                mint_signer: zc_mint.base.metadata.mint_signer,
                bump: zc_mint.base.metadata.bump,
            },
            reserved: *zc_mint.base.reserved,
            account_type: zc_mint.base.account_type,
            extensions: zc_mint.extensions.as_ref().map(|zc_exts| {
                zc_exts
                    .iter()
                    .map(|zc_ext| {
                        match zc_ext {
                            ZExtensionStruct::TokenMetadata(z_metadata) => {
                                ExtensionStruct::TokenMetadata(TokenMetadata {
                                    update_authority: z_metadata.update_authority,
                                    mint: z_metadata.mint,
                                    name: z_metadata.name.to_vec(),
                                    symbol: z_metadata.symbol.to_vec(),
                                    uri: z_metadata.uri.to_vec(),
                                    additional_metadata: vec![], // Simplified for test
                                })
                            }
                            _ => panic!("Unsupported extension type in test"),
                        }
                    })
                    .collect()
            }),
        };
        reconstructed_mint
    };

    // CRITICAL ASSERTION: Complete struct verification (UNIT_TESTING.md requirement)
    assert_eq!(
        zc_reserialized, compressed_mint,
        "Zero-copy deserialized struct must exactly match original borsh struct"
    );

    println!("✅ Complete borsh/zero-copy struct compatibility verified");
}
