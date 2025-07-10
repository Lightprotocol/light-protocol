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
        metadata_pointer::MetadataPointer,
        state::{ExtensionStruct, ZExtensionStruct},
        token_metadata::{
            AdditionalMetadata, AdditionalMetadataConfig, Metadata, MetadataConfig, TokenMetadata,
            TokenMetadataConfig, TokenMetadataInstructionData,
        },
    },
    mint::{
        output::create_output_compressed_mint_account,
        state::{CompressedMint, CompressedMintConfig},
    },
    shared::cpi_bytes_size::{
        allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
    },
};
use light_hasher::{Hasher, Poseidon};
use light_zero_copy::ZeroCopyNew;
use rand::Rng;
use spl_token_2022::extension;

// Function to create expected input account
fn create_expected_input_account(
    mint_pda: Pubkey,
    input_supply: u64,
    decimals: u8,
    is_decompressed: bool,
    mint_authority: Option<Pubkey>,
    freeze_authority: Option<Pubkey>,
    version: u8,
    extensions: Option<Vec<light_compressed_token::extensions::state::ExtensionStruct>>,
    compressed_account_address: [u8; 32],
    merkle_tree_pubkey_index: u8,
    queue_pubkey_index: u8,
    leaf_index: u32,
    prove_by_index: bool,
    root_index: u16,
) -> light_compressed_account::instruction_data::with_readonly::InAccount {
    let expected_input_compressed_mint = CompressedMint {
        spl_mint: mint_pda,
        supply: input_supply,
        decimals,
        is_decompressed,
        mint_authority,
        freeze_authority,
        version,
        extensions,
    };
    let expected_input_data_hash = expected_input_compressed_mint.hash().unwrap();

    light_compressed_account::instruction_data::with_readonly::InAccount {
        discriminator: COMPRESSED_MINT_DISCRIMINATOR,
        data_hash: expected_input_data_hash,
        merkle_context: light_compressed_account::compressed_account::PackedMerkleContext {
            merkle_tree_pubkey_index,
            queue_pubkey_index,
            leaf_index,
            prove_by_index,
        },
        root_index,
        lamports: 0,
        address: Some(compressed_account_address),
    }
}

// Function to create expected output account
fn create_expected_output_account(
    mint_pda: Pubkey,
    output_supply: u64,
    decimals: u8,
    mint_authority: Option<Pubkey>,
    freeze_authority: Option<Pubkey>,
    version: u8,
    extensions: Option<Vec<light_compressed_token::extensions::state::ExtensionStruct>>,
    compressed_account_address: [u8; 32],
    program_id: Pubkey,
    output_merkle_tree_index: u8,
) -> OutputCompressedAccountWithPackedContext {
    let expected_compressed_mint = CompressedMint {
        spl_mint: mint_pda,
        supply: output_supply,
        decimals,
        is_decompressed: false,
        mint_authority,
        freeze_authority,
        version,
        extensions,
    };
    let expected_data_hash = expected_compressed_mint.hash().unwrap();

    OutputCompressedAccountWithPackedContext {
        compressed_account: CompressedAccount {
            address: Some(compressed_account_address),
            owner: program_id,
            lamports: 0,
            data: Some(CompressedAccountData {
                data: borsh::to_vec(&expected_compressed_mint).unwrap(),
                discriminator: COMPRESSED_MINT_DISCRIMINATOR,
                data_hash: expected_data_hash,
            }),
        },
        merkle_tree_index: output_merkle_tree_index,
    }
}

// Function to convert expected accounts to instruction data
fn create_instruction_data_from_expected(
    expected_extensions: Option<Vec<light_compressed_token::extensions::state::ExtensionStruct>>,
) -> (
    Option<Vec<ExtensionInstructionData>>,
    Vec<light_compressed_token::extensions::state::ExtensionStructConfig>,
) {
    if let Some(extension_structs) = expected_extensions {
        let mut instruction_extensions = Vec::new();
        let mut extension_configs = Vec::new();

        for extension_struct in extension_structs {
            match extension_struct {
                light_compressed_token::extensions::state::ExtensionStruct::TokenMetadata(
                    token_metadata,
                ) => {
                    let instruction_data = TokenMetadataInstructionData {
                        update_authority: token_metadata.update_authority,
                        metadata: token_metadata.metadata.clone(),
                        additional_metadata: if token_metadata.additional_metadata.is_empty() {
                            None
                        } else {
                            Some(token_metadata.additional_metadata.clone())
                        },
                        version: token_metadata.version,
                    };
                    instruction_extensions
                        .push(ExtensionInstructionData::TokenMetadata(instruction_data));

                    let additional_metadata_configs = token_metadata
                        .additional_metadata
                        .iter()
                        .map(|item| AdditionalMetadataConfig {
                            key: item.key.len() as u32,
                            value: item.value.len() as u32,
                        })
                        .collect();

                    let config = light_compressed_token::extensions::state::ExtensionStructConfig::TokenMetadata(
                        TokenMetadataConfig {
                            update_authority: (token_metadata.update_authority.is_some(), ()),
                            metadata: MetadataConfig {
                                name: token_metadata.metadata.name.len() as u32,
                                symbol: token_metadata.metadata.symbol.len() as u32,
                                uri: token_metadata.metadata.uri.len() as u32,
                            },
                            additional_metadata: additional_metadata_configs,
                        }
                    );
                    extension_configs.push(config);
                }
                light_compressed_token::extensions::state::ExtensionStruct::MetadataPointer(
                    metadata_pointer,
                ) => {
                    let instruction_data =
                        light_compressed_token::extensions::metadata_pointer::InitMetadataPointer {
                            authority: metadata_pointer.authority,
                            metadata_address: metadata_pointer.metadata_address,
                        };
                    instruction_extensions
                        .push(ExtensionInstructionData::MetadataPointer(instruction_data));

                    let config = light_compressed_token::extensions::state::ExtensionStructConfig::MetadataPointer(
                        light_compressed_token::extensions::metadata_pointer::MetadataPointerConfig {
                            authority: (metadata_pointer.authority.is_some(), ()),
                            metadata_address: (metadata_pointer.metadata_address.is_some(), ()),
                        }
                    );
                    extension_configs.push(config);
                }
            }
        }

        (Some(instruction_extensions), extension_configs)
    } else {
        (None, vec![])
    }
}

// Function to create random extension data
fn create_random_extension_data<R: Rng>(
    rng: &mut R,
    mint_pda: Pubkey,
) -> Option<Vec<light_compressed_token::extensions::state::ExtensionStruct>> {
    if rng.gen_bool(0.3) {
        let update_authority = if rng.gen_bool(0.7) {
            Some(Pubkey::new_from_array(rng.gen::<[u8; 32]>()))
        } else {
            None
        };

        // Generate smaller random metadata for testing
        let name_len = rng.gen_range(1..=10);
        let symbol_len = rng.gen_range(1..=3);
        let uri_len = rng.gen_range(5..=20);

        let name: Vec<u8> = (0..name_len).map(|_| rng.gen_range(b'A'..=b'Z')).collect();
        let symbol: Vec<u8> = (0..symbol_len)
            .map(|_| rng.gen_range(b'A'..=b'Z'))
            .collect();
        let uri: Vec<u8> = (0..uri_len).map(|_| rng.gen_range(b'a'..=b'z')).collect();

        // Random additional metadata (50% chance)
        let additional_metadata = if rng.gen_bool(0.5) {
            let num_items = rng.gen_range(1..=3);
            (0..num_items)
                .map(|_| {
                    let key_len = rng.gen_range(3..=16);
                    let value_len = rng.gen_range(5..=31);
                    AdditionalMetadata {
                        key: (0..key_len).map(|_| rng.gen_range(b'a'..=b'z')).collect(),
                        value: (0..value_len).map(|_| rng.gen_range(b'a'..=b'z')).collect(),
                    }
                })
                .collect()
        } else {
            vec![]
        };

        use light_compressed_token::extensions::state::ExtensionStruct;
        use light_compressed_token::extensions::token_metadata::TokenMetadata;

        let expected_token_metadata = TokenMetadata {
            update_authority,
            mint: mint_pda,
            metadata: Metadata {
                name: name.clone(),
                symbol: symbol.clone(),
                uri: uri.clone(),
            },
            additional_metadata,
            version: 0, // Hardcode to version 0 (Poseidon)
        };

        Some(vec![ExtensionStruct::TokenMetadata(
            expected_token_metadata,
        )])
    } else {
        None
    }
}

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
        let version = 0; // rng.gen_range(0..=255u8);

        // Generate random supplies
        let input_supply = rng.gen_range(0..=u64::MAX);
        let output_supply = rng.gen_range(0..=u64::MAX);
        let is_decompressed = rng.gen_bool(0.1);

        // Generate random merkle context
        let merkle_tree_pubkey_index = rng.gen_range(0..=255u8);
        let queue_pubkey_index = rng.gen_range(0..=255u8);
        let leaf_index = rng.gen::<u32>();
        let prove_by_index = rng.gen_bool(0.5);
        let root_index = rng.gen::<u16>();
        let output_merkle_tree_index = rng.gen_range(0..=255u8);

        // Derive compressed account address
        let compressed_account_address = derive_address(
            &mint_pda.to_bytes(),
            &address_merkle_tree.to_bytes(),
            &program_id.to_bytes(),
        );

        // Step 1: Create expected extensions
        let expected_extensions = create_random_extension_data(&mut rng, mint_pda);

        // Step 2: Create expected input and output accounts
        let expected_input_account = create_expected_input_account(
            mint_pda,
            input_supply,
            decimals,
            is_decompressed,
            mint_authority,
            freeze_authority,
            version,
            expected_extensions.clone(),
            compressed_account_address,
            merkle_tree_pubkey_index,
            queue_pubkey_index,
            leaf_index,
            prove_by_index,
            root_index,
        );

        let expected_output_account = create_expected_output_account(
            mint_pda,
            output_supply,
            decimals,
            mint_authority,
            freeze_authority,
            version,
            expected_extensions.clone(),
            compressed_account_address,
            program_id,
            output_merkle_tree_index,
        );

        // Step 3: Convert expected accounts to instruction data
        let (extensions, extensions_config) =
            create_instruction_data_from_expected(expected_extensions);

        // Step 4: Create allocations and mint config
        let mint_config = CompressedMintConfig {
            mint_authority: (true, ()), // Always true like in cpi_bytes_config and mint_to_compressed
            freeze_authority: (freeze_authority.is_some(), ()),
            extensions: (extensions.is_some(), extensions_config.clone()),
        };

        let config_input = CpiConfigInput {
            input_accounts: arrayvec::ArrayVec::new(),
            output_accounts: arrayvec::ArrayVec::new(),
            has_proof: false,
            compressed_mint: true,
            compressed_mint_with_freeze_authority: freeze_authority.is_some(),
            extensions_config: extensions_config.clone(),
        };

        let config = cpi_bytes_config(config_input);
        let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);
        let (mut cpi_instruction_struct, _) =
            light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly::new_zero_copy(
                &mut cpi_bytes[8..],
                config,
            )
            .unwrap();

        // Step 5: Create actual input and output data
        let input_account = &mut cpi_instruction_struct.input_compressed_accounts[0];
        let output_account = &mut cpi_instruction_struct.output_compressed_accounts[0];

        // Create input data
        use light_compressed_account::compressed_account::PackedMerkleContext;
        use light_compressed_token::mint_to_compressed::instructions::CompressedMintInputs;
        use light_compressed_token::shared::context::TokenContext;
        use light_zero_copy::borsh::Deserialize;

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
                    extensions: extensions.clone(),
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

        let input_data = input_compressed_mint.try_to_vec().unwrap();
        let (z_compressed_mint_inputs, _) =
            CompressedMintInputs::zero_copy_at(&input_data).unwrap();

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
        let extensions_data = if let Some(ref extensions) = extensions {
            use borsh::BorshSerialize;
            let mut extensions_data = Vec::new();
            for extension in extensions {
                extension.serialize(&mut extensions_data).unwrap();
            }
            Some(extensions_data)
        } else {
            None
        };

        let z_extensions = if let Some(ref extensions_data) = extensions_data {
            let mut z_extensions = Vec::new();
            let mut offset = 0;
            for _ in extensions.as_ref().unwrap() {
                let (z_ext, remaining) =
                    ExtensionInstructionData::zero_copy_at(&extensions_data[offset..]).unwrap();
                z_extensions.push(z_ext);
                offset = extensions_data.len() - remaining.len();
            }
            Some(z_extensions)
        } else {
            None
        };

        // Create output data
        let base_mint_config = CompressedMintConfig {
            mint_authority: mint_config.mint_authority,
            freeze_authority: mint_config.freeze_authority,
            extensions: (false, vec![]), // No extensions for base size
        };
        let base_mint_len = CompressedMint::byte_len(&base_mint_config);

        create_output_compressed_mint_account(
            output_account,
            mint_pda,
            decimals,
            freeze_authority,
            mint_authority,
            output_supply.into(),
            &program_id,
            mint_config,
            compressed_account_address,
            output_merkle_tree_index,
            version,
            z_extensions.as_deref(),
            base_mint_len,
        )
        .unwrap();

        // Step 6: Assert created data vs expected
        let cpi_borsh =
            InstructionDataInvokeCpiWithReadOnly::deserialize(&mut &cpi_bytes[8..]).unwrap();

        let expected = InstructionDataInvokeCpiWithReadOnly {
            input_compressed_accounts: vec![expected_input_account],
            output_compressed_accounts: vec![expected_output_account],
            ..Default::default()
        };

        assert_eq!(cpi_borsh, expected);
    }
}

#[test]
fn test_compressed_mint_borsh_zero_copy_compatibility() {
    use light_zero_copy::borsh::Deserialize;

    // Create CompressedMint with token metadata extension
    let token_metadata = TokenMetadata {
        update_authority: Some(Pubkey::new_from_array([1; 32])),
        mint: Pubkey::new_from_array([2; 32]),
        metadata: Metadata {
            name: b"TestToken".to_vec(),
            symbol: b"TT".to_vec(),
            uri: b"https://test.com".to_vec(),
        },
        additional_metadata: vec![],
        version: 0,
    };

    let compressed_mint = CompressedMint {
        spl_mint: Pubkey::new_from_array([3; 32]),
        supply: 1000u64,
        decimals: 6u8,
        is_decompressed: false,
        mint_authority: Some(Pubkey::new_from_array([4; 32])),
        freeze_authority: None,
        version: 1u8,
        extensions: Some(vec![ExtensionStruct::TokenMetadata(token_metadata)]),
    };

    // Serialize with Borsh
    let borsh_bytes = borsh::to_vec(&compressed_mint).unwrap();

    // Deserialize with zero_copy_at
    let (zc_mint, remaining) = CompressedMint::zero_copy_at(&borsh_bytes).unwrap();
    assert!(remaining.is_empty());

    // Verify data matches - zero-copy fields vs original fields
    assert_eq!(zc_mint.spl_mint, compressed_mint.spl_mint);
    assert_eq!(u64::from(zc_mint.supply), compressed_mint.supply);
    assert_eq!(zc_mint.decimals, compressed_mint.decimals);
    assert_eq!(zc_mint.version, compressed_mint.version);

    // Check extensions match
    if let Some(ref zc_extensions) = zc_mint.extensions {
        if let Some(ref orig_extensions) = compressed_mint.extensions {
            for (z_extension, extension) in zc_extensions.iter().zip(orig_extensions.iter()) {
                match (z_extension, extension) {
                    (
                        ZExtensionStruct::TokenMetadata(z_metadata),
                        ExtensionStruct::TokenMetadata(metadata),
                    ) => {
                        assert_eq!(z_metadata.metadata.name, metadata.metadata.name.as_slice());
                        assert_eq!(
                            z_metadata.metadata.symbol,
                            metadata.metadata.symbol.as_slice()
                        );
                        assert_eq!(z_metadata.metadata.uri, metadata.metadata.uri.as_slice());
                        assert_eq!(*z_metadata.mint, metadata.mint);
                        assert_eq!(
                            z_metadata.update_authority.map(|x| *x),
                            metadata.update_authority
                        );
                        assert_eq!(z_metadata.version, metadata.version);
                    }
                    _ => panic!("Mismatched extension types"),
                }
            }
        }
    }

    println!("Borsh/zero-copy compatibility test passed");
}
