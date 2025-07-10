use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{
    address::derive_address,
    compressed_account::{CompressedAccount, CompressedAccountData},
    instruction_data::{
        data::OutputCompressedAccountWithPackedContext,
        with_readonly::InstructionDataInvokeCpiWithReadOnly,
    },
    Pubkey,
};
use light_compressed_token::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    extensions::{
        instruction_data::{ExtensionInstructionData, ZExtensionInstructionData},
        token_metadata::{AdditionalMetadata, Metadata, TokenMetadataInstructionData},
    },
    mint::{
        output::create_output_compressed_mint_account,
        state::{CompressedMint, CompressedMintConfig},
    },
    shared::cpi_bytes_size::{
        allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
    },
};
use light_zero_copy::ZeroCopyNew;
use rand::Rng;

#[test]
fn test_rnd_create_compressed_mint_account() {
    let mut rng = rand::thread_rng();
    let iter = 100;

    for _ in 0..iter {
        // Generate random mint parameters
        let mint_pda = Pubkey::new_from_array(rng.gen::<[u8; 32]>());
        let decimals = rng.gen_range(0..=18u8);
        let program_id = Pubkey::new_from_array(rng.gen::<[u8; 32]>());
        let address_merkle_tree = Pubkey::new_from_array(rng.gen::<[u8; 32]>());

        // Random freeze authority (50% chance)
        let freeze_authority = if rng.gen_bool(0.5) {
            Some(Pubkey::new_from_array(rng.gen::<[u8; 32]>()))
        } else {
            None
        };

        let mint_authority = Some(Pubkey::new_from_array(rng.gen::<[u8; 32]>()));

        // Generate version for use in extensions
        let version = rng.gen_range(0..=255u8);

        // Random extensions (30% chance of having token metadata)
        let (extensions, extensions_config) = if rng.gen_bool(0.3) {
            let update_authority = if rng.gen_bool(0.7) { 
                Some(Pubkey::new_from_array(rng.gen::<[u8; 32]>())) 
            } else { 
                None 
            };
            
            // Generate random metadata
            let name_len = rng.gen_range(1..=32);
            let symbol_len = rng.gen_range(1..=8);
            let uri_len = rng.gen_range(10..=64);
            
            let name: Vec<u8> = (0..name_len).map(|_| rng.gen_range(b'A'..=b'Z')).collect();
            let symbol: Vec<u8> = (0..symbol_len).map(|_| rng.gen_range(b'A'..=b'Z')).collect();
            let uri: Vec<u8> = (0..uri_len).map(|_| rng.gen_range(b'a'..=b'z')).collect();
            
            // Random additional metadata (50% chance)
            let additional_metadata = if rng.gen_bool(0.5) {
                let num_items = rng.gen_range(1..=3);
                Some((0..num_items).map(|_| {
                    let key_len = rng.gen_range(3..=16);
                    let value_len = rng.gen_range(5..=32);
                    AdditionalMetadata {
                        key: (0..key_len).map(|_| rng.gen_range(b'a'..=b'z')).collect(),
                        value: (0..value_len).map(|_| rng.gen_range(b'a'..=b'z')).collect(),
                    }
                }).collect())
            } else {
                None
            };
            
            let token_metadata = TokenMetadataInstructionData {
                update_authority,
                metadata: Metadata {
                    name: name.clone(),
                    symbol: symbol.clone(),
                    uri: uri.clone(),
                },
                additional_metadata: additional_metadata.clone(),
                version,
            };
            
            let extensions = vec![ExtensionInstructionData::TokenMetadata(token_metadata)];
            
            // Create extension config
            use light_compressed_token::extensions::state::{ExtensionStructConfig, TokenMetadataConfig, MetadataConfig, AdditionalMetadataConfig};
            
            let additional_metadata_configs = if let Some(ref additional_metadata) = additional_metadata {
                additional_metadata.iter().map(|item| AdditionalMetadataConfig {
                    key: item.key.len() as u32,
                    value: item.value.len() as u32,
                }).collect()
            } else {
                vec![]
            };
            
            let extensions_config = vec![ExtensionStructConfig::TokenMetadata(TokenMetadataConfig {
                update_authority: (update_authority.is_some(), ()),
                metadata: MetadataConfig {
                    name: name.len() as u32,
                    symbol: symbol.len() as u32,
                    uri: uri.len() as u32,
                },
                additional_metadata: additional_metadata_configs,
            })];
            
            (Some(extensions), extensions_config)
        } else {
            (None, vec![])
        };

        // Create mint config - match the real usage pattern (always reserve mint_authority space)
        let mint_config = CompressedMintConfig {
            mint_authority: (true, ()), // Always true like in cpi_bytes_config and mint_to_compressed
            freeze_authority: (freeze_authority.is_some(), ()),
            extensions: (extensions.is_some(), extensions_config),
        };
        // Derive compressed account address
        let compressed_account_address = derive_address(
            &mint_pda.to_bytes(),
            &address_merkle_tree.to_bytes(),
            &program_id.to_bytes(),
        );

        // Create a simple test structure for just the output account
        let config_input = CpiConfigInput {
            input_accounts: arrayvec::ArrayVec::new(),
            output_accounts: arrayvec::ArrayVec::new(),
            has_proof: false,
            compressed_mint: true,
            compressed_mint_with_freeze_authority: freeze_authority.is_some(),
        };

        let config = cpi_bytes_config(config_input);
        let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);
        let (mut cpi_instruction_struct, _) =
            light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly::new_zero_copy(
                &mut cpi_bytes[8..],
                config,
            )
            .unwrap();

        // Get the input and output compressed accounts
        let input_account = &mut cpi_instruction_struct.input_compressed_accounts[0];
        let output_account = &mut cpi_instruction_struct.output_compressed_accounts[0];

        // Create mock input data for the input compressed mint account test
        use light_compressed_account::compressed_account::PackedMerkleContext;
        use light_compressed_token::mint_to_compressed::instructions::CompressedMintInputs;
        use light_compressed_token::shared::context::TokenContext;
        use light_zero_copy::borsh::Deserialize;

        // Generate random values for more comprehensive testing
        let input_supply = rng.gen_range(0..=u64::MAX);
        let output_supply = rng.gen_range(0..=u64::MAX); // Random supply for output account
        let is_decompressed = rng.gen_bool(0.1); // 10% chance
        let merkle_tree_pubkey_index = rng.gen_range(0..=255u8);
        let queue_pubkey_index = rng.gen_range(0..=255u8);
        let leaf_index = rng.gen::<u32>();
        let prove_by_index = rng.gen_bool(0.5);
        let root_index = rng.gen::<u16>();
        let output_merkle_tree_index = rng.gen_range(0..=255u8);

        // Compute extension hash for input if extensions are present
        let extension_hash = if let Some(ref extensions) = extensions {
            use light_hasher::Poseidon;
            let mut context = TokenContext::new();
            let mut extension_hashes = Vec::new();
            
            for extension in extensions {
                let hash = extension.hash::<Poseidon>(mint_pda, &mut context).unwrap();
                extension_hashes.push(hash);
            }
            
            if extension_hashes.len() == 1 {
                extension_hashes[0]
            } else {
                // Chain multiple extension hashes if needed
                let mut chained_hash = extension_hashes[0];
                for hash in &extension_hashes[1..] {
                    chained_hash = Poseidon::hashv(&[chained_hash.as_slice(), hash.as_slice()]).unwrap();
                }
                chained_hash
            }
        } else {
            [0; 32]
        };

        // Create mock input compressed mint data
        let input_compressed_mint = CompressedMintInputs {
            compressed_mint_input:
                light_compressed_token::mint_to_compressed::instructions::CompressedMintInput {
                    spl_mint: mint_pda,
                    supply: input_supply,
                    decimals,
                    is_decompressed,
                    freeze_authority_is_set: freeze_authority.is_some(),
                    freeze_authority: freeze_authority.unwrap_or_default(),
                    version,
                    extension_hash,
                },
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index,
                queue_pubkey_index,
                leaf_index,
                prove_by_index,
            },
            root_index,
            address: compressed_account_address,
            output_merkle_tree_index,
        };

        // Serialize and get zero-copy reference
        let input_data = input_compressed_mint.try_to_vec().unwrap();
        let (z_compressed_mint_inputs, _) =
            CompressedMintInputs::zero_copy_at(&input_data).unwrap();

        // Create token context and call input function
        let mut context = TokenContext::new();
        let hashed_mint_authority = context.get_or_hash_pubkey(&mint_authority.unwrap().into());
        light_compressed_token::mint::input::create_input_compressed_mint_account(
            input_account,
            &mut context,
            &z_compressed_mint_inputs,
            &hashed_mint_authority,
        )
        .unwrap();

        // Prepare extensions for zero-copy usage
        let z_extensions = if let Some(ref extensions) = extensions {
            // Serialize extensions for zero-copy
            use borsh::BorshSerialize;
            let mut extensions_data = Vec::new();
            for extension in extensions {
                extension.serialize(&mut extensions_data).unwrap();
            }

            // Create ZExtensionInstructionData from serialized data
            use light_zero_copy::borsh::Deserialize;
            let mut z_extensions = Vec::new();
            let mut offset = 0;
            for _ in extensions {
                let (z_ext, remaining) = ExtensionInstructionData::zero_copy_at(&extensions_data[offset..]).unwrap();
                z_extensions.push(z_ext);
                offset = extensions_data.len() - remaining.len();
            }
            Some(z_extensions)
        } else {
            None
        };

        // Call the function under test
        let base_mint_len = CompressedMint::byte_len(&mint_config);
        create_output_compressed_mint_account(
            output_account,
            mint_pda,
            decimals,
            freeze_authority,
            mint_authority,
            output_supply.into(), // supply parameter (U64 type)
            &program_id,
            mint_config,
            compressed_account_address,
            output_merkle_tree_index,
            version,
            z_extensions.as_deref(),
            base_mint_len,
        )
        .unwrap();

        // Final comparison with borsh deserialization - same pattern as token account tests
        let cpi_borsh =
            InstructionDataInvokeCpiWithReadOnly::deserialize(&mut &cpi_bytes[8..]).unwrap();

        // Build expected output
        let expected_compressed_mint = CompressedMint {
            spl_mint: mint_pda,
            supply: output_supply,
            decimals,
            is_decompressed: false,
            mint_authority,
            freeze_authority,
            version,
            extensions: if extensions.is_some() {
                Some(extension_hash)
            } else {
                None
            },
        };

        let expected_data_hash = expected_compressed_mint.hash().unwrap();

        let expected_account = OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                address: Some(compressed_account_address),
                owner: program_id,
                lamports: 0,
                data: Some(CompressedAccountData {
                    data: expected_compressed_mint.try_to_vec().unwrap(),
                    discriminator: COMPRESSED_MINT_DISCRIMINATOR,
                    data_hash: expected_data_hash,
                }),
            },
            merkle_tree_index: output_merkle_tree_index,
        };

        // Create expected input account data that matches what the input function should produce
        let expected_input_compressed_mint = CompressedMint {
            spl_mint: mint_pda,
            supply: input_supply,
            decimals,
            is_decompressed,
            mint_authority, // Use the actual mint authority passed to the function
            freeze_authority,
            version,
            extensions: None,
        };
        let expected_input_data_hash = expected_input_compressed_mint.hash().unwrap();

        let expected_input_account =
            light_compressed_account::instruction_data::with_readonly::InAccount {
                discriminator: COMPRESSED_MINT_DISCRIMINATOR,
                data_hash: expected_input_data_hash,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index,
                    queue_pubkey_index,
                    leaf_index,
                    prove_by_index,
                },
                root_index,
                lamports: 0,
                address: Some(compressed_account_address),
            };

        let expected = InstructionDataInvokeCpiWithReadOnly {
            input_compressed_accounts: vec![expected_input_account],
            output_compressed_accounts: vec![expected_account],
            ..Default::default()
        };

        assert_eq!(cpi_borsh, expected);
    }
}
