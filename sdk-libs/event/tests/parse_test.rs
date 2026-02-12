use borsh::BorshSerialize;
use light_compressed_account::{
    constants::{
        ACCOUNT_COMPRESSION_PROGRAM_ID, LIGHT_REGISTRY_PROGRAM_ID, REGISTERED_PROGRAM_PDA,
    },
    discriminators::DISCRIMINATOR_INSERT_INTO_QUEUES,
    Pubkey,
};
use light_event::parse::{
    extract_ata_owners, find_cpi_pattern, find_cpi_patterns, wrap_program_ids, Indices, ProgramId,
    TokenInstructionData,
};
use light_token_interface::{
    instructions::{
        extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
        transfer2::{CompressedTokenInstructionDataTransfer2, MultiTokenTransferOutputData},
    },
    LIGHT_TOKEN_PROGRAM_ID, TRANSFER2,
};
use rand::{
    rngs::{StdRng, ThreadRng},
    Rng, RngCore, SeedableRng,
};

fn get_rnd_program_id<R: Rng>(rng: &mut R, with_system_program: bool) -> ProgramId {
    let vec = [
        ProgramId::Unknown,
        ProgramId::AccountCompression,
        ProgramId::LightSystem,
    ];
    let len = if with_system_program { 3 } else { 2 };
    let index = rng.gen_range(0..len);
    vec[index]
}

fn get_rnd_program_ids<R: Rng>(
    rng: &mut R,
    len: usize,
    with_system_program: bool,
) -> Vec<ProgramId> {
    (0..len)
        .map(|_| get_rnd_program_id(rng, with_system_program))
        .collect()
}

/// Helper to create valid Transfer2 instruction data with ATA extensions
fn create_transfer2_with_ata(owner_index: u8, is_ata: bool) -> Vec<u8> {
    let transfer_data = CompressedTokenInstructionDataTransfer2 {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        output_queue: 0,
        max_top_up: u16::MAX, // No limit
        cpi_context: None,
        compressions: None,
        proof: None,
        in_token_data: vec![],
        out_token_data: vec![MultiTokenTransferOutputData {
            owner: owner_index,
            amount: 1000,
            has_delegate: false,
            delegate: 0,
            mint: 0,
            version: 3,
        }],
        in_lamports: None,
        out_lamports: None,
        in_tlv: None,
        out_tlv: Some(vec![vec![ExtensionInstructionData::CompressedOnly(
            CompressedOnlyExtensionInstructionData {
                delegated_amount: 0,
                withheld_transfer_fee: 0,
                is_frozen: false,
                compression_index: 0,
                is_ata,
                bump: 255,
                owner_index,
            },
        )]]),
    };
    let mut data = vec![TRANSFER2]; // discriminator
    data.extend(transfer_data.try_to_vec().unwrap());
    data
}

/// Helper to create Transfer2 instruction data with multiple outputs
fn create_transfer2_with_multiple_outputs(
    outputs: Vec<(u8, bool)>, // (owner_index, is_ata)
) -> Vec<u8> {
    let out_token_data: Vec<MultiTokenTransferOutputData> = outputs
        .iter()
        .map(|(owner_index, _)| MultiTokenTransferOutputData {
            owner: *owner_index,
            amount: 1000,
            has_delegate: false,
            delegate: 0,
            mint: 0,
            version: 3,
        })
        .collect();

    let out_tlv: Vec<Vec<ExtensionInstructionData>> = outputs
        .iter()
        .map(|(owner_index, is_ata)| {
            vec![ExtensionInstructionData::CompressedOnly(
                CompressedOnlyExtensionInstructionData {
                    delegated_amount: 0,
                    withheld_transfer_fee: 0,
                    is_frozen: false,
                    compression_index: 0,
                    is_ata: *is_ata,
                    bump: 255,
                    owner_index: *owner_index,
                },
            )]
        })
        .collect();

    let transfer_data = CompressedTokenInstructionDataTransfer2 {
        with_transaction_hash: false,
        with_lamports_change_account_merkle_tree_index: false,
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        output_queue: 0,
        max_top_up: u16::MAX, // No limit
        cpi_context: None,
        compressions: None,
        proof: None,
        in_token_data: vec![],
        out_token_data,
        in_lamports: None,
        out_lamports: None,
        in_tlv: None,
        out_tlv: Some(out_tlv),
    };
    let mut data = vec![TRANSFER2];
    data.extend(transfer_data.try_to_vec().unwrap());
    data
}

#[test]
fn test_rnd_functional() {
    let mut thread_rng = ThreadRng::default();
    let seed = thread_rng.next_u64();
    // Keep this print so that in case the test fails
    // we can use the seed to reproduce the error.
    println!("\n\ntest seed {}\n\n", seed);
    let mut rng = StdRng::seed_from_u64(seed);
    let num_iters = 100000;
    for _ in 0..num_iters {
        let len_pre = rng.gen_range(0..6);
        let rnd_vec_pre = get_rnd_program_ids(&mut rng, len_pre, false);
        let len_post = rng.gen_range(0..6);
        let rnd_vec_post = get_rnd_program_ids(&mut rng, len_post, false);
        let num_mid = rng.gen_range(1..6);

        let program_ids = [
            rnd_vec_pre.as_slice(),
            [ProgramId::LightSystem].as_slice(),
            vec![ProgramId::SolanaSystem; num_mid].as_slice(),
            [ProgramId::AccountCompression].as_slice(),
            rnd_vec_post.as_slice(),
        ]
        .concat();
        let start_index = program_ids.len() - 1 - len_post;
        let system_index = program_ids.len() - 1 - len_post - num_mid - 1;
        let vec = find_cpi_patterns(&program_ids);
        let expected = Indices {
            system: system_index,
            cpi: vec![],
            insert_into_queues: start_index,
            found_solana_system_program_instruction: true,
            found_system: true,
            token: None,
            found_registry: false,
        };
        assert!(
            vec.contains(&expected),
            "program ids {:?} parsed events  {:?} expected {:?} ",
            program_ids,
            vec,
            expected,
        );
    }

    for _ in 0..num_iters {
        let len_pre = rng.gen_range(0..6);
        let rnd_vec_pre = get_rnd_program_ids(&mut rng, len_pre, true);
        let len_post = rng.gen_range(0..6);
        let rnd_vec_post = get_rnd_program_ids(&mut rng, len_post, true);
        let num_mid = rng.gen_range(1..6);

        let program_ids = [
            rnd_vec_pre.as_slice(),
            [ProgramId::LightSystem].as_slice(),
            vec![ProgramId::SolanaSystem; num_mid].as_slice(),
            [ProgramId::AccountCompression].as_slice(),
            rnd_vec_post.as_slice(),
        ]
        .concat();
        let start_index = program_ids.len() - 1 - len_post;
        let system_index = program_ids.len() - 1 - len_post - num_mid - 1;
        let vec = find_cpi_patterns(&program_ids);
        let expected = Indices {
            system: system_index,
            cpi: vec![],
            insert_into_queues: start_index,
            found_solana_system_program_instruction: true,
            found_system: true,
            token: None,
            found_registry: false,
        };
        assert!(
            vec.iter().any(|x| x.system == expected.system
                && x.insert_into_queues == expected.insert_into_queues),
            "program ids {:?} parsed events  {:?} expected {:?} ",
            program_ids,
            vec,
            expected,
        );
    }
}

#[test]
fn test_rnd_failing() {
    let mut thread_rng = ThreadRng::default();
    let seed = thread_rng.next_u64();
    // Keep this print so that in case the test fails
    // we can use the seed to reproduce the error.
    println!("\n\ntest seed {}\n\n", seed);
    let mut rng = StdRng::seed_from_u64(seed);
    let num_iters = 100000;
    for _ in 0..num_iters {
        let len = rng.gen_range(0..20);
        let mut program_ids = get_rnd_program_ids(&mut rng, len, true);
        // if any ProgramId::LightSystem is followed by ProgramId::SolanaSystem overwrite ProgramId::SolanaSystem with ProgramId::Unknown
        for i in 0..program_ids.len().saturating_sub(1) {
            if matches!(program_ids[i], ProgramId::LightSystem)
                && matches!(program_ids[i + 1], ProgramId::SolanaSystem)
            {
                program_ids[i + 1] = ProgramId::Unknown;
            }
        }

        let vec = find_cpi_patterns(&program_ids);

        assert!(
            vec.is_empty(),
            "program_ids {:?} result {:?}",
            program_ids,
            vec
        );
    }
}

#[test]
fn test_find_two_patterns() {
    // Std pattern
    {
        let program_ids = vec![
            ProgramId::Unknown,
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
            ProgramId::Unknown,
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let vec = find_cpi_patterns(&program_ids);
        assert_eq!(vec.len(), 2);
        assert_eq!(
            vec[0],
            Indices {
                system: 5,
                cpi: vec![],
                insert_into_queues: 7,
                found_solana_system_program_instruction: true,
                found_system: true,
                token: None,
                found_registry: false,
            }
        );
        assert_eq!(
            vec[1],
            Indices {
                system: 1,
                cpi: vec![],
                insert_into_queues: 3,
                found_solana_system_program_instruction: true,
                found_system: true,
                token: None,
                found_registry: false,
            }
        );
        // Modify only second event is valid
        {
            let mut program_ids = program_ids.clone();
            program_ids[2] = ProgramId::Unknown;
            let vec = find_cpi_patterns(&program_ids);
            assert_eq!(vec.len(), 1);
            assert_eq!(
                vec[0],
                Indices {
                    system: 5,
                    cpi: vec![],
                    insert_into_queues: 7,
                    found_solana_system_program_instruction: true,
                    found_system: true,
                    token: None,
                    found_registry: false,
                }
            );
        }
        // Modify only first event is valid
        {
            let mut program_ids = program_ids;
            program_ids[6] = ProgramId::Unknown;
            let vec = find_cpi_patterns(&program_ids);
            assert_eq!(vec.len(), 1);
            assert_eq!(
                vec[0],
                Indices {
                    system: 1,
                    cpi: vec![],
                    insert_into_queues: 3,
                    found_solana_system_program_instruction: true,
                    found_system: true,
                    token: None,
                    found_registry: false,
                }
            );
        }
    }
}

#[test]
fn test_find_pattern() {
    // Std pattern
    {
        let program_ids = vec![
            ProgramId::Unknown,
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let (res, last_index) = find_cpi_pattern(3, &program_ids);
        assert_eq!(last_index, 0);
        assert_eq!(
            res,
            Some(Indices {
                system: 1,
                cpi: vec![],
                insert_into_queues: 3,
                found_solana_system_program_instruction: true,
                found_system: true,
                token: None,
                found_registry: false,
            })
        );
    }
    {
        let program_ids = vec![
            ProgramId::Unknown,
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::SolanaSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let start_index = program_ids.len() - 1;
        let (res, last_index) = find_cpi_pattern(start_index, &program_ids);
        assert_eq!(last_index, 0);
        assert_eq!(
            res,
            Some(Indices {
                system: 1,
                cpi: vec![],
                insert_into_queues: start_index,
                found_solana_system_program_instruction: true,
                found_system: true,
                token: None,
                found_registry: false,
            })
        );
    }
    {
        let program_ids = vec![
            ProgramId::Unknown,
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::Unknown,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let start_index = program_ids.len() - 1;
        let (res, last_index) = find_cpi_pattern(start_index, &program_ids);
        assert_eq!(last_index, 3);
        assert_eq!(res, None);
    }
    // With cpi context
    {
        let program_ids = vec![
            ProgramId::Unknown,
            ProgramId::LightSystem,
            ProgramId::Unknown,
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::SolanaSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let start_index = program_ids.len() - 1;
        let (res, last_index) = find_cpi_pattern(start_index, &program_ids);
        assert_eq!(last_index, 0);
        assert_eq!(
            res,
            Some(Indices {
                system: 3,
                cpi: vec![1],
                insert_into_queues: start_index,
                found_solana_system_program_instruction: true,
                found_system: true,
                token: None,
                found_registry: false,
            })
        );
        // Failing
        {
            let mut program_ids = program_ids;
            program_ids[5] = ProgramId::Unknown;
            let (res, last_index) = find_cpi_pattern(start_index, &program_ids);
            assert_eq!(last_index, 5);
            assert_eq!(res, None);
        }
    }
    // With cpi context
    {
        let program_ids = vec![
            ProgramId::Unknown,
            ProgramId::LightSystem,
            ProgramId::LightSystem,
            ProgramId::SolanaSystem,
            ProgramId::SolanaSystem,
            ProgramId::SolanaSystem,
            ProgramId::AccountCompression,
        ];
        let start_index = program_ids.len() - 1;
        let (res, last_index) = find_cpi_pattern(start_index, &program_ids);
        assert_eq!(last_index, 0);
        assert_eq!(
            res,
            Some(Indices {
                system: 2,
                cpi: vec![1],
                insert_into_queues: start_index,
                found_solana_system_program_instruction: true,
                found_system: true,
                token: None,
                found_registry: false,
            })
        );
        // Failing
        {
            let mut program_ids = program_ids;
            program_ids[4] = ProgramId::Unknown;
            let (res, last_index) = find_cpi_pattern(start_index, &program_ids);
            assert_eq!(last_index, 4);
            assert_eq!(res, None);
        }
    }
}

// ==========================================================================
// Tests for extract_ata_owners
// ==========================================================================

#[test]
fn test_extract_ata_owners_empty_data() {
    let token_instruction = TokenInstructionData {
        data: &[],
        accounts: &[],
    };
    let result = extract_ata_owners(&token_instruction);
    assert!(result.is_empty(), "Empty data should return empty vec");
}

#[test]
fn test_extract_ata_owners_wrong_discriminator() {
    let token_instruction = TokenInstructionData {
        data: &[0xFF, 0x00, 0x00], // Wrong discriminator
        accounts: &[],
    };
    let result = extract_ata_owners(&token_instruction);
    assert!(
        result.is_empty(),
        "Wrong discriminator should return empty vec"
    );
}

#[test]
fn test_extract_ata_owners_only_discriminator() {
    let token_instruction = TokenInstructionData {
        data: &[TRANSFER2], // Only discriminator, no data
        accounts: &[],
    };
    let result = extract_ata_owners(&token_instruction);
    assert!(
        result.is_empty(),
        "Only discriminator should return empty vec (deserialization fails)"
    );
}

#[test]
fn test_extract_ata_owners_malformed_data() {
    // Random garbage after discriminator
    let token_instruction = TokenInstructionData {
        data: &[TRANSFER2, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        accounts: &[],
    };
    let result = extract_ata_owners(&token_instruction);
    assert!(
        result.is_empty(),
        "Malformed data should return empty vec (deserialization fails)"
    );
}

#[test]
fn test_extract_ata_owners_valid_non_ata() {
    let data = create_transfer2_with_ata(0, false); // is_ata = false
    let accounts = vec![Pubkey::default(); 10];
    let token_instruction = TokenInstructionData {
        data: &data,
        accounts: &accounts,
    };
    let result = extract_ata_owners(&token_instruction);
    assert!(
        result.is_empty(),
        "Non-ATA accounts should not produce ATA owner info"
    );
}

#[test]
fn test_extract_ata_owners_valid_ata() {
    let owner_index = 2u8; // Index into packed_accounts
    let data = create_transfer2_with_ata(owner_index, true);

    // Create accounts array: 7 system accounts + packed_accounts
    // owner_index=2 means packed_accounts[2] = accounts[7+2] = accounts[9]
    let mut accounts = vec![Pubkey::default(); 10];
    let expected_owner = Pubkey::new_from_array([42u8; 32]);
    accounts[7 + owner_index as usize] = expected_owner;

    let token_instruction = TokenInstructionData {
        data: &data,
        accounts: &accounts,
    };
    let result = extract_ata_owners(&token_instruction);
    assert_eq!(result.len(), 1, "Should extract one ATA owner");
    assert_eq!(result[0].output_index, 0);
    assert_eq!(result[0].wallet_owner, expected_owner);
}

#[test]
fn test_extract_ata_owners_owner_index_out_of_bounds() {
    let owner_index = 100u8; // Way beyond accounts array
    let data = create_transfer2_with_ata(owner_index, true);

    // Only 10 accounts, but owner_index + 7 = 107
    let accounts = vec![Pubkey::default(); 10];

    let token_instruction = TokenInstructionData {
        data: &data,
        accounts: &accounts,
    };
    let result = extract_ata_owners(&token_instruction);
    assert!(
        result.is_empty(),
        "Out of bounds owner_index should be safely skipped"
    );
}

#[test]
fn test_extract_ata_owners_boundary_owner_index() {
    // Test with owner_index at the boundary
    let owner_index = 2u8;
    let data = create_transfer2_with_ata(owner_index, true);

    // Create exactly enough accounts: 7 system + 3 packed (indices 0, 1, 2)
    // owner_index=2 needs accounts[9], so we need 10 accounts total
    let mut accounts = vec![Pubkey::default(); 10];
    let expected_owner = Pubkey::new_from_array([99u8; 32]);
    accounts[9] = expected_owner;

    let token_instruction = TokenInstructionData {
        data: &data,
        accounts: &accounts,
    };
    let result = extract_ata_owners(&token_instruction);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].wallet_owner, expected_owner);

    // Now with one less account - should be skipped
    let accounts_short = vec![Pubkey::default(); 9];
    let token_instruction_short = TokenInstructionData {
        data: &data,
        accounts: &accounts_short,
    };
    let result_short = extract_ata_owners(&token_instruction_short);
    assert!(
        result_short.is_empty(),
        "Boundary case with insufficient accounts should be skipped"
    );
}

#[test]
fn test_extract_ata_owners_max_owner_index() {
    // Test with u8::MAX owner_index
    let owner_index = u8::MAX;
    let data = create_transfer2_with_ata(owner_index, true);

    // 255 + 7 = 262, need 263 accounts
    let accounts = vec![Pubkey::default(); 10]; // Way too few

    let token_instruction = TokenInstructionData {
        data: &data,
        accounts: &accounts,
    };
    let result = extract_ata_owners(&token_instruction);
    assert!(
        result.is_empty(),
        "u8::MAX owner_index with small accounts array should be safely skipped"
    );
}

// ==========================================================================
// Tests for wrap_program_ids with LightToken and Registry
// ==========================================================================

#[test]
fn test_wrap_program_ids_light_token_transfer2() {
    let program_ids = vec![Pubkey::from(LIGHT_TOKEN_PROGRAM_ID)];
    let mut instruction_data = vec![0u8; 12]; // Minimum size
    instruction_data[0] = TRANSFER2;
    let instructions = vec![instruction_data];
    let accounts = vec![vec![]];

    let result = wrap_program_ids(&program_ids, &instructions, &accounts);
    assert_eq!(result, vec![ProgramId::LightToken]);
}

#[test]
fn test_wrap_program_ids_light_token_non_transfer2() {
    let program_ids = vec![Pubkey::from(LIGHT_TOKEN_PROGRAM_ID)];
    let mut instruction_data = vec![0u8; 12];
    instruction_data[0] = 0xFF; // Not TRANSFER2
    let instructions = vec![instruction_data];
    let accounts = vec![vec![]];

    let result = wrap_program_ids(&program_ids, &instructions, &accounts);
    assert_eq!(result, vec![ProgramId::Unknown]);
}

#[test]
fn test_wrap_program_ids_registry() {
    let program_ids = vec![Pubkey::from(LIGHT_REGISTRY_PROGRAM_ID)];
    let instruction_data = vec![0u8; 12];
    let instructions = vec![instruction_data];
    let accounts = vec![vec![]];

    let result = wrap_program_ids(&program_ids, &instructions, &accounts);
    assert_eq!(result, vec![ProgramId::Registry]);
}

#[test]
fn test_wrap_program_ids_instruction_too_small() {
    let program_ids = vec![Pubkey::from(LIGHT_TOKEN_PROGRAM_ID)];
    let instruction_data = vec![TRANSFER2; 5]; // Less than 12 bytes
    let instructions = vec![instruction_data];
    let accounts = vec![vec![]];

    let result = wrap_program_ids(&program_ids, &instructions, &accounts);
    assert_eq!(
        result,
        vec![ProgramId::Unknown],
        "Instructions smaller than 12 bytes should be Unknown"
    );
}

// ==========================================================================
// Tests for find_cpi_pattern with Registry and Token tracking
// ==========================================================================

#[test]
fn test_find_cpi_pattern_with_registry_and_token() {
    // Pattern: Registry -> Token -> LightSystem -> SolanaSystem -> AccountCompression
    let program_ids = vec![
        ProgramId::Registry,
        ProgramId::LightToken,
        ProgramId::LightSystem,
        ProgramId::SolanaSystem,
        ProgramId::AccountCompression,
    ];
    let (res, _) = find_cpi_pattern(4, &program_ids);
    assert!(res.is_some());
    let indices = res.unwrap();
    assert!(indices.found_registry, "Should find registry");
    assert_eq!(
        indices.token,
        Some(1),
        "Should track token when registry is present"
    );
    assert_eq!(indices.system, 2);
}

#[test]
fn test_find_cpi_pattern_token_without_registry() {
    // Pattern: Token -> LightSystem -> SolanaSystem -> AccountCompression
    // No registry means token should NOT be tracked
    let program_ids = vec![
        ProgramId::LightToken,
        ProgramId::LightSystem,
        ProgramId::SolanaSystem,
        ProgramId::AccountCompression,
    ];
    let (res, _) = find_cpi_pattern(3, &program_ids);
    assert!(res.is_some());
    let indices = res.unwrap();
    assert!(!indices.found_registry, "Should not find registry");
    assert_eq!(
        indices.token, None,
        "Should NOT track token without registry"
    );
}

#[test]
fn test_find_cpi_pattern_registry_without_token() {
    // Registry can call LightSystem directly without Token
    // Pattern: Registry -> LightSystem -> SolanaSystem -> AccountCompression
    let program_ids = vec![
        ProgramId::Registry,
        ProgramId::LightSystem,
        ProgramId::SolanaSystem,
        ProgramId::AccountCompression,
    ];
    let (res, _) = find_cpi_pattern(3, &program_ids);
    assert!(res.is_some());
    let indices = res.unwrap();
    assert!(indices.found_registry, "Should find registry");
    assert_eq!(indices.token, None, "No token instruction in this pattern");
}

#[test]
fn test_find_cpi_pattern_multiple_tokens_only_first_tracked() {
    // Only the first (closest to system) token should be tracked
    // Pattern: Registry -> Token1 -> Token2 -> LightSystem -> SolanaSystem -> AccountCompression
    let program_ids = vec![
        ProgramId::Registry,
        ProgramId::LightToken, // Token1 - outer
        ProgramId::LightToken, // Token2 - inner, should be tracked
        ProgramId::LightSystem,
        ProgramId::SolanaSystem,
        ProgramId::AccountCompression,
    ];
    let (res, _) = find_cpi_pattern(5, &program_ids);
    assert!(res.is_some());
    let indices = res.unwrap();
    assert!(indices.found_registry);
    // The inner token (index 2) should be tracked as it's first when searching backwards
    assert_eq!(
        indices.token,
        Some(2),
        "Should track the token closest to system instruction"
    );
}

// ==========================================================================
// Additional ATA and Program ID filtering edge case tests
// ==========================================================================

#[test]
fn test_find_cpi_pattern_token_after_account_compression_not_tracked() {
    // Token appearing after AccountCompression should not be part of this pattern
    // Pattern: Registry -> LightSystem -> SolanaSystem -> AccountCompression -> Token
    let program_ids = vec![
        ProgramId::Registry,
        ProgramId::LightSystem,
        ProgramId::SolanaSystem,
        ProgramId::AccountCompression,
        ProgramId::LightToken, // After AccountCompression - not part of this pattern
    ];
    let (res, _) = find_cpi_pattern(3, &program_ids);
    assert!(res.is_some());
    let indices = res.unwrap();
    assert!(indices.found_registry);
    assert_eq!(
        indices.token, None,
        "Token after AccountCompression should not be tracked in this pattern"
    );
}

#[test]
fn test_find_cpi_pattern_registry_after_account_compression_not_found() {
    // Registry appearing after AccountCompression should not validate token tracking
    // Pattern: Token -> LightSystem -> SolanaSystem -> AccountCompression -> Registry
    let program_ids = vec![
        ProgramId::LightToken,
        ProgramId::LightSystem,
        ProgramId::SolanaSystem,
        ProgramId::AccountCompression,
        ProgramId::Registry, // After AccountCompression - not part of this pattern
    ];
    let (res, _) = find_cpi_pattern(3, &program_ids);
    assert!(res.is_some());
    let indices = res.unwrap();
    assert!(
        !indices.found_registry,
        "Registry after AccountCompression should not be found"
    );
    assert_eq!(
        indices.token, None,
        "Token should not be tracked without registry before AccountCompression"
    );
}

#[test]
fn test_find_cpi_pattern_token_between_unknown_programs() {
    // Token surrounded by Unknown programs, with Registry present
    // Pattern: Registry -> Unknown -> Token -> Unknown -> LightSystem -> SolanaSystem -> AccountCompression
    let program_ids = vec![
        ProgramId::Registry,
        ProgramId::Unknown,
        ProgramId::LightToken,
        ProgramId::Unknown,
        ProgramId::LightSystem,
        ProgramId::SolanaSystem,
        ProgramId::AccountCompression,
    ];
    let (res, _) = find_cpi_pattern(6, &program_ids);
    assert!(res.is_some());
    let indices = res.unwrap();
    assert!(indices.found_registry);
    assert_eq!(
        indices.token,
        Some(2),
        "Token should be tracked even with Unknown programs around it"
    );
}

#[test]
fn test_find_cpi_pattern_empty_program_ids() {
    let program_ids: Vec<ProgramId> = vec![];
    let patterns = find_cpi_patterns(&program_ids);
    assert!(
        patterns.is_empty(),
        "Empty program IDs should return no patterns"
    );
}

#[test]
fn test_find_cpi_pattern_single_account_compression() {
    let program_ids = vec![ProgramId::AccountCompression];
    let (res, _) = find_cpi_pattern(0, &program_ids);
    assert!(
        res.is_none(),
        "Single AccountCompression without system should not match"
    );
}

#[test]
fn test_find_cpi_pattern_registry_token_no_system() {
    // Registry and Token without LightSystem - invalid pattern
    let program_ids = vec![
        ProgramId::Registry,
        ProgramId::LightToken,
        ProgramId::SolanaSystem,
        ProgramId::AccountCompression,
    ];
    let (res, _) = find_cpi_pattern(3, &program_ids);
    assert!(
        res.is_none(),
        "Pattern without LightSystem should not match"
    );
}

#[test]
fn test_find_cpi_pattern_token_at_position_zero_not_tracked() {
    // Token at position 0 (outermost in CPI chain) - this is NOT a valid real-world pattern.
    // In the actual protocol, Registry is always the outermost caller (Registry -> Token -> LightSystem).
    // Pattern: Token -> Registry -> LightSystem -> SolanaSystem -> AccountCompression
    //
    // When searching backwards, we encounter Registry (index 1) BEFORE Token (index 0).
    // At the point we find Registry, tentative_token is still None, so we don't confirm a token.
    // Then we find Token at index 0, but Registry has already been processed.
    //
    // This behavior is CORRECT because Token being outermost is invalid - Registry must be outer.
    let program_ids = vec![
        ProgramId::LightToken, // Position 0 - invalid as outermost
        ProgramId::Registry,   // Position 1
        ProgramId::LightSystem,
        ProgramId::SolanaSystem,
        ProgramId::AccountCompression,
    ];
    let (res, _) = find_cpi_pattern(4, &program_ids);
    assert!(res.is_some());
    let indices = res.unwrap();
    assert!(indices.found_registry);
    // Token at position 0 is NOT tracked because it appears AFTER Registry in backwards search.
    // This is correct behavior - Token must be between Registry and LightSystem.
    assert_eq!(
        indices.token, None,
        "Token at position 0 (before Registry in array) should NOT be tracked - invalid CPI order"
    );
}

#[test]
fn test_find_cpi_pattern_multiple_registries() {
    // Multiple Registry programs - behavior verification
    // Pattern: Registry -> Registry -> Token -> LightSystem -> SolanaSystem -> AccountCompression
    let program_ids = vec![
        ProgramId::Registry, // First Registry
        ProgramId::Registry, // Second Registry
        ProgramId::LightToken,
        ProgramId::LightSystem,
        ProgramId::SolanaSystem,
        ProgramId::AccountCompression,
    ];
    let (res, _) = find_cpi_pattern(5, &program_ids);
    assert!(res.is_some());
    let indices = res.unwrap();
    assert!(indices.found_registry, "Should find at least one registry");
    assert_eq!(
        indices.token,
        Some(2),
        "Token should be tracked with registry present"
    );
}

#[test]
fn test_find_cpi_pattern_token_before_system_instruction() {
    // Token appearing before finding system instruction in backwards search
    // Pattern: LightSystem -> SolanaSystem -> Token -> AccountCompression
    // When searching backwards from AccountCompression, we find Token before system
    let program_ids = vec![
        ProgramId::LightSystem,
        ProgramId::SolanaSystem,
        ProgramId::LightToken, // Between SolanaSystem and AccountCompression
        ProgramId::AccountCompression,
    ];
    let (res, _) = find_cpi_pattern(3, &program_ids);
    // This should fail because we need SolanaSystem right before AccountCompression
    assert!(
        res.is_none(),
        "Token breaking the SolanaSystem -> AccountCompression chain should fail"
    );
}

#[test]
fn test_find_cpi_pattern_registry_between_system_and_solana_system() {
    // Registry between LightSystem and SolanaSystem
    // Pattern: Registry -> LightSystem -> Registry -> SolanaSystem -> AccountCompression
    let program_ids = vec![
        ProgramId::Registry,
        ProgramId::LightSystem,
        ProgramId::Registry, // Between LightSystem and SolanaSystem
        ProgramId::SolanaSystem,
        ProgramId::AccountCompression,
    ];
    let (res, _) = find_cpi_pattern(4, &program_ids);
    // Registry between should break the pattern
    assert!(
        res.is_none(),
        "Registry between LightSystem and SolanaSystem should break pattern"
    );
}

// ==========================================================================
// Additional extract_ata_owners edge case tests
// ==========================================================================

#[test]
fn test_extract_ata_owners_multiple_outputs_all_ata() {
    // Multiple outputs, all are ATAs
    let data = create_transfer2_with_multiple_outputs(vec![
        (0, true), // output 0: ATA with owner at packed_accounts[0]
        (1, true), // output 1: ATA with owner at packed_accounts[1]
        (2, true), // output 2: ATA with owner at packed_accounts[2]
    ]);

    let mut accounts = vec![Pubkey::default(); 12]; // 7 system + 5 packed
    let owner0 = Pubkey::new_from_array([10u8; 32]);
    let owner1 = Pubkey::new_from_array([11u8; 32]);
    let owner2 = Pubkey::new_from_array([12u8; 32]);
    accounts[7] = owner0;
    accounts[8] = owner1;
    accounts[9] = owner2;

    let token_instruction = TokenInstructionData {
        data: &data,
        accounts: &accounts,
    };
    let result = extract_ata_owners(&token_instruction);

    assert_eq!(result.len(), 3, "Should extract 3 ATA owners");
    assert_eq!(result[0].output_index, 0);
    assert_eq!(result[0].wallet_owner, owner0);
    assert_eq!(result[1].output_index, 1);
    assert_eq!(result[1].wallet_owner, owner1);
    assert_eq!(result[2].output_index, 2);
    assert_eq!(result[2].wallet_owner, owner2);
}

#[test]
fn test_extract_ata_owners_multiple_outputs_mixed() {
    // Mixed: some ATA, some not
    let data = create_transfer2_with_multiple_outputs(vec![
        (0, false), // output 0: NOT an ATA
        (1, true),  // output 1: ATA
        (2, false), // output 2: NOT an ATA
        (3, true),  // output 3: ATA
    ]);

    let mut accounts = vec![Pubkey::default(); 12];
    let owner1 = Pubkey::new_from_array([21u8; 32]);
    let owner3 = Pubkey::new_from_array([23u8; 32]);
    accounts[8] = owner1; // packed_accounts[1]
    accounts[10] = owner3; // packed_accounts[3]

    let token_instruction = TokenInstructionData {
        data: &data,
        accounts: &accounts,
    };
    let result = extract_ata_owners(&token_instruction);

    assert_eq!(result.len(), 2, "Should only extract ATA outputs");
    assert_eq!(result[0].output_index, 1);
    assert_eq!(result[0].wallet_owner, owner1);
    assert_eq!(result[1].output_index, 3);
    assert_eq!(result[1].wallet_owner, owner3);
}

#[test]
fn test_extract_ata_owners_multiple_outputs_none_ata() {
    // All outputs are non-ATA
    let data = create_transfer2_with_multiple_outputs(vec![(0, false), (1, false), (2, false)]);

    let accounts = vec![Pubkey::default(); 12];
    let token_instruction = TokenInstructionData {
        data: &data,
        accounts: &accounts,
    };
    let result = extract_ata_owners(&token_instruction);

    assert!(
        result.is_empty(),
        "Should not extract any owners when no ATAs"
    );
}

#[test]
fn test_extract_ata_owners_same_owner_multiple_atas() {
    // Multiple ATAs pointing to the same owner (same owner_index)
    let data = create_transfer2_with_multiple_outputs(vec![
        (0, true), // output 0: ATA with owner at packed_accounts[0]
        (0, true), // output 1: ATA with SAME owner
        (0, true), // output 2: ATA with SAME owner
    ]);

    let mut accounts = vec![Pubkey::default(); 10];
    let shared_owner = Pubkey::new_from_array([77u8; 32]);
    accounts[7] = shared_owner;

    let token_instruction = TokenInstructionData {
        data: &data,
        accounts: &accounts,
    };
    let result = extract_ata_owners(&token_instruction);

    assert_eq!(result.len(), 3, "Should extract all 3 ATA entries");
    assert!(
        result.iter().all(|r| r.wallet_owner == shared_owner),
        "All should have the same owner"
    );
    assert_eq!(result[0].output_index, 0);
    assert_eq!(result[1].output_index, 1);
    assert_eq!(result[2].output_index, 2);
}

#[test]
fn test_extract_ata_owners_partial_out_of_bounds() {
    // Some outputs have valid owner_index, some are out of bounds
    let data = create_transfer2_with_multiple_outputs(vec![
        (0, true),   // output 0: Valid owner_index
        (100, true), // output 1: Out of bounds
        (1, true),   // output 2: Valid owner_index
    ]);

    let mut accounts = vec![Pubkey::default(); 10];
    let owner0 = Pubkey::new_from_array([30u8; 32]);
    let owner1 = Pubkey::new_from_array([31u8; 32]);
    accounts[7] = owner0;
    accounts[8] = owner1;

    let token_instruction = TokenInstructionData {
        data: &data,
        accounts: &accounts,
    };
    let result = extract_ata_owners(&token_instruction);

    assert_eq!(result.len(), 2, "Should only extract valid owner indices");
    assert_eq!(result[0].output_index, 0);
    assert_eq!(result[0].wallet_owner, owner0);
    assert_eq!(result[1].output_index, 2);
    assert_eq!(result[1].wallet_owner, owner1);
}

#[test]
fn test_extract_ata_owners_zero_packed_accounts() {
    // Edge case: exactly 7 accounts (no packed_accounts at all)
    let data = create_transfer2_with_ata(0, true); // Wants packed_accounts[0] which doesn't exist

    let accounts = vec![Pubkey::default(); 7]; // Only system accounts

    let token_instruction = TokenInstructionData {
        data: &data,
        accounts: &accounts,
    };
    let result = extract_ata_owners(&token_instruction);

    assert!(
        result.is_empty(),
        "Should not extract ATA when no packed_accounts exist"
    );
}

#[test]
fn test_extract_ata_owners_exactly_one_packed_account() {
    // Edge case: exactly 8 accounts (only one packed_account at index 0)
    let data = create_transfer2_with_ata(0, true);

    let mut accounts = vec![Pubkey::default(); 8];
    let owner = Pubkey::new_from_array([55u8; 32]);
    accounts[7] = owner;

    let token_instruction = TokenInstructionData {
        data: &data,
        accounts: &accounts,
    };
    let result = extract_ata_owners(&token_instruction);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].wallet_owner, owner);
}

// ==========================================================================
// Tests for wrap_program_ids edge cases
// ==========================================================================

#[test]
fn test_wrap_program_ids_empty_instruction_data() {
    let program_ids = vec![Pubkey::from(LIGHT_TOKEN_PROGRAM_ID)];
    let instructions = vec![vec![]]; // Empty instruction data
    let accounts = vec![vec![]];

    let result = wrap_program_ids(&program_ids, &instructions, &accounts);
    assert_eq!(
        result,
        vec![ProgramId::Unknown],
        "Empty instruction should be Unknown"
    );
}

#[test]
fn test_wrap_program_ids_exactly_12_bytes() {
    // Boundary: exactly 12 bytes is valid
    let program_ids = vec![Pubkey::from(LIGHT_TOKEN_PROGRAM_ID)];
    let mut instruction_data = vec![0u8; 12];
    instruction_data[0] = TRANSFER2;
    let instructions = vec![instruction_data];
    let accounts = vec![vec![]];

    let result = wrap_program_ids(&program_ids, &instructions, &accounts);
    assert_eq!(result, vec![ProgramId::LightToken]);
}

#[test]
fn test_wrap_program_ids_11_bytes() {
    // Boundary: 11 bytes is too small
    let program_ids = vec![Pubkey::from(LIGHT_TOKEN_PROGRAM_ID)];
    let mut instruction_data = vec![0u8; 11];
    instruction_data[0] = TRANSFER2;
    let instructions = vec![instruction_data];
    let accounts = vec![vec![]];

    let result = wrap_program_ids(&program_ids, &instructions, &accounts);
    assert_eq!(result, vec![ProgramId::Unknown], "11 bytes is too small");
}

#[test]
fn test_wrap_program_ids_mixed_valid_invalid() {
    // Mix of valid and invalid instructions
    let program_ids = vec![
        Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
        Pubkey::from(LIGHT_REGISTRY_PROGRAM_ID),
        Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
        Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
    ];

    let mut valid_transfer = vec![0u8; 12];
    valid_transfer[0] = TRANSFER2;

    let instructions = vec![
        valid_transfer.clone(), // Valid Token + TRANSFER2
        vec![0u8; 12],          // Valid Registry (any 12+ bytes)
        vec![0xFF; 12],         // Token but not TRANSFER2
        vec![TRANSFER2; 5],     // Token + TRANSFER2 but too short
    ];
    let accounts = vec![vec![], vec![], vec![], vec![]];

    let result = wrap_program_ids(&program_ids, &instructions, &accounts);
    assert_eq!(
        result,
        vec![
            ProgramId::LightToken,
            ProgramId::Registry,
            ProgramId::Unknown,
            ProgramId::Unknown,
        ]
    );
}

#[test]
fn test_wrap_program_ids_account_compression_missing_registered_pda() {
    // AccountCompression with wrong registered PDA
    let program_ids = vec![Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID)];
    let mut instruction_data = vec![0u8; 12];
    instruction_data[0..8].copy_from_slice(&DISCRIMINATOR_INSERT_INTO_QUEUES);
    let instructions = vec![instruction_data];
    // accounts[1] should be REGISTERED_PROGRAM_PDA but we use a different pubkey
    let accounts = vec![vec![
        Pubkey::default(),
        Pubkey::new_from_array([99u8; 32]), // Wrong PDA
        Pubkey::default(),
    ]];

    let result = wrap_program_ids(&program_ids, &instructions, &accounts);
    assert_eq!(
        result,
        vec![ProgramId::Unknown],
        "AccountCompression with wrong registered PDA should be Unknown"
    );
}

#[test]
fn test_wrap_program_ids_account_compression_valid() {
    // AccountCompression with correct setup
    let program_ids = vec![Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID)];
    let mut instruction_data = vec![0u8; 12];
    instruction_data[0..8].copy_from_slice(&DISCRIMINATOR_INSERT_INTO_QUEUES);
    let instructions = vec![instruction_data];
    let accounts = vec![vec![
        Pubkey::default(),
        Pubkey::from(REGISTERED_PROGRAM_PDA), // Correct PDA
        Pubkey::default(),
    ]];

    let result = wrap_program_ids(&program_ids, &instructions, &accounts);
    assert_eq!(result, vec![ProgramId::AccountCompression]);
}

#[test]
fn test_wrap_program_ids_account_compression_insufficient_accounts() {
    // AccountCompression with too few accounts
    let program_ids = vec![Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID)];
    let mut instruction_data = vec![0u8; 12];
    instruction_data[0..8].copy_from_slice(&DISCRIMINATOR_INSERT_INTO_QUEUES);
    let instructions = vec![instruction_data];
    let accounts = vec![vec![Pubkey::default()]]; // Only 1 account, need 3

    let result = wrap_program_ids(&program_ids, &instructions, &accounts);
    assert_eq!(
        result,
        vec![ProgramId::Unknown],
        "AccountCompression with insufficient accounts should be Unknown"
    );
}
