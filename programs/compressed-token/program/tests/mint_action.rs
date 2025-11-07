/// Comprehensive randomized unit tests for MintAction AccountsConfig
///
/// Tests AccountsConfig::new() by generating random instruction data and verifying
/// that the derived configuration matches expected values based on instruction content.
use borsh::BorshSerialize;
use light_compressed_account::{instruction_data::compressed_proof::CompressedProof, Pubkey};
use light_compressed_token::mint_action::accounts::AccountsConfig;
use light_ctoken_types::{
    instructions::{
        extensions::{token_metadata::TokenMetadataInstructionData, ExtensionInstructionData},
        mint_action::{
            Action, CompressedMintInstructionData, CpiContext, CreateMint, CreateSplMintAction,
            DecompressedRecipient, MintActionCompressedInstructionData, MintToCTokenAction,
            MintToCompressedAction, Recipient, RemoveMetadataKeyAction, UpdateAuthority,
            UpdateMetadataAuthorityAction, UpdateMetadataFieldAction,
        },
    },
    state::CompressedMintMetadata,
    CMINT_ADDRESS_TREE,
};
use light_zero_copy::traits::ZeroCopyAt;
use rand::{rngs::StdRng, thread_rng, Rng, SeedableRng};

// ============================================================================
// Helper Functions for Random Data Generation
// ============================================================================

fn random_pubkey(rng: &mut StdRng) -> Pubkey {
    Pubkey::from(rng.gen::<[u8; 32]>())
}

fn random_optional_pubkey(rng: &mut StdRng, probability: f64) -> Option<Pubkey> {
    if rng.gen_bool(probability) {
        Some(random_pubkey(rng))
    } else {
        None
    }
}

fn random_compressed_mint_metadata(rng: &mut StdRng) -> CompressedMintMetadata {
    CompressedMintMetadata {
        version: rng.gen_range(1..=3) as u8,
        spl_mint_initialized: rng.gen_bool(0.5),
        mint: random_pubkey(rng),
    }
}

fn random_token_metadata_extension(rng: &mut StdRng) -> ExtensionInstructionData {
    ExtensionInstructionData::TokenMetadata(TokenMetadataInstructionData {
        update_authority: random_optional_pubkey(rng, 0.8),
        name: format!("Token{}", rng.gen::<u32>()).into_bytes(),
        symbol: format!("TK{}", rng.gen::<u16>()).into_bytes(),
        uri: format!("https://example.com/{}", rng.gen::<u32>()).into_bytes(),
        additional_metadata: Some(vec![]),
    })
}

fn random_mint_to_action(rng: &mut StdRng) -> MintToCompressedAction {
    let recipient_count = rng.gen_range(1..=3);
    let recipients = (0..recipient_count)
        .map(|_| Recipient {
            recipient: random_pubkey(rng),
            amount: rng.gen_range(1..=1_000_000),
        })
        .collect();

    MintToCompressedAction {
        token_account_version: rng.gen_range(0..=3) as u8,
        recipients,
    }
}

fn random_mint_to_decompressed_action(rng: &mut StdRng) -> MintToCTokenAction {
    MintToCTokenAction {
        recipient: DecompressedRecipient {
            amount: rng.gen_range(1..=1_000_000),
            account_index: rng.gen_range(1..=255),
        },
    }
}

fn random_update_authority_action(rng: &mut StdRng) -> UpdateAuthority {
    UpdateAuthority {
        new_authority: random_optional_pubkey(rng, 0.8),
    }
}

fn random_create_spl_mint_action(rng: &mut StdRng) -> CreateSplMintAction {
    CreateSplMintAction {
        mint_bump: rng.gen::<u8>(),
    }
}

fn random_update_metadata_field_action(rng: &mut StdRng) -> UpdateMetadataFieldAction {
    UpdateMetadataFieldAction {
        extension_index: rng.gen_range(0..=2) as u8,
        field_type: rng.gen_range(0..=3) as u8,
        key: format!("key_{}", rng.gen::<u32>()).into_bytes(),
        value: format!("value_{}", rng.gen::<u32>()).into_bytes(),
    }
}

fn random_update_metadata_authority_action(rng: &mut StdRng) -> UpdateMetadataAuthorityAction {
    UpdateMetadataAuthorityAction {
        extension_index: rng.gen_range(0..=2) as u8,
        new_authority: random_pubkey(rng), // Required field, not optional
    }
}

fn random_remove_metadata_key_action(rng: &mut StdRng) -> RemoveMetadataKeyAction {
    RemoveMetadataKeyAction {
        extension_index: rng.gen(),
        idempotent: rng.gen(),
        key: rng.gen::<[u8; 32]>().to_vec(),
    }
}

fn random_action(rng: &mut StdRng) -> Action {
    match rng.gen_range(0..8) {
        0 => Action::MintToCompressed(random_mint_to_action(rng)),
        1 => Action::UpdateMintAuthority(random_update_authority_action(rng)),
        2 => Action::UpdateFreezeAuthority(random_update_authority_action(rng)),
        3 => Action::CreateSplMint(random_create_spl_mint_action(rng)),
        4 => Action::MintToCToken(random_mint_to_decompressed_action(rng)),
        5 => Action::UpdateMetadataField(random_update_metadata_field_action(rng)),
        6 => Action::UpdateMetadataAuthority(random_update_metadata_authority_action(rng)),
        7 => Action::RemoveMetadataKey(random_remove_metadata_key_action(rng)),
        _ => unreachable!(),
    }
}

fn random_cpi_context(rng: &mut StdRng) -> CpiContext {
    CpiContext {
        set_context: rng.gen_bool(0.5),
        first_set_context: rng.gen_bool(0.5),
        in_tree_index: rng.gen::<u8>(),
        in_queue_index: rng.gen::<u8>(),
        out_queue_index: rng.gen::<u8>(),
        token_out_queue_index: rng.gen::<u8>(),
        assigned_account_index: rng.gen::<u8>(),
        read_only_address_trees: [0u8; 4],
        address_tree_pubkey: CMINT_ADDRESS_TREE,
    }
}

fn random_compressed_proof(rng: &mut StdRng) -> CompressedProof {
    CompressedProof {
        a: [rng.gen::<u8>(); 32],
        b: [rng.gen::<u8>(); 64],
        c: [rng.gen::<u8>(); 32],
    }
}

/// Generates random MintActionCompressedInstructionData with controllable parameters
fn generate_random_instruction_data(
    rng: &mut StdRng,
    force_create_mint: Option<bool>,
    force_cpi_context: Option<bool>,
    force_spl_initialized: Option<bool>,
    action_count_range: std::ops::Range<usize>,
) -> MintActionCompressedInstructionData {
    let create_mint = force_create_mint.unwrap_or_else(|| rng.gen_bool(0.3));
    let create_mint = if create_mint {
        Some(CreateMint {
            mint_bump: rng.gen(),
            ..Default::default()
        })
    } else {
        None
    };
    let has_cpi_context = force_cpi_context.unwrap_or_else(|| rng.gen_bool(0.4));

    let mut mint_metadata = random_compressed_mint_metadata(rng);
    if let Some(spl_init) = force_spl_initialized {
        mint_metadata.spl_mint_initialized = spl_init && create_mint.is_none();
    }

    // Generate actions
    let action_count = rng.gen_range(action_count_range);
    let mut actions = Vec::with_capacity(action_count);
    for _ in 0..action_count {
        actions.push(random_action(rng));
    }

    MintActionCompressedInstructionData {
        create_mint,
        leaf_index: rng.gen::<u32>(),
        prove_by_index: rng.gen_bool(0.5),
        root_index: rng.gen::<u16>(),
        compressed_address: rng.gen::<[u8; 32]>(),
        token_pool_bump: rng.gen::<u8>(),
        token_pool_index: rng.gen::<u8>(),
        actions,
        proof: if rng.gen_bool(0.6) {
            Some(random_compressed_proof(rng))
        } else {
            None
        },
        cpi_context: if has_cpi_context {
            Some(random_cpi_context(rng))
        } else {
            None
        },
        mint: CompressedMintInstructionData {
            supply: rng.gen_range(0..=1_000_000_000),
            decimals: rng.gen_range(0..=9),
            metadata: mint_metadata,
            mint_authority: random_optional_pubkey(rng, 0.9),
            freeze_authority: random_optional_pubkey(rng, 0.7),
            extensions: if rng.gen_bool(0.3) {
                Some(vec![random_token_metadata_extension(rng)])
            } else {
                None
            },
        },
    }
}

/// Computes expected AccountsConfig based on instruction data
fn compute_expected_config(data: &MintActionCompressedInstructionData) -> AccountsConfig {
    // 1. with_cpi_context
    let with_cpi_context = data.cpi_context.is_some();

    // 2. write_to_cpi_context
    let write_to_cpi_context = data
        .cpi_context
        .as_ref()
        .map(|ctx| ctx.first_set_context || ctx.set_context)
        .unwrap_or(false);

    // 3. has_mint_to_actions
    let has_mint_to_actions = data.actions.iter().any(|action| {
        matches!(
            action,
            Action::MintToCompressed(_) | Action::MintToCToken(_)
        )
    });

    // 4. create_spl_mint
    let create_spl_mint = data
        .actions
        .iter()
        .any(|action| matches!(action, Action::CreateSplMint(_)));

    // 5. spl_mint_initialized
    let spl_mint_initialized = data.mint.metadata.spl_mint_initialized || create_spl_mint;

    // 6. with_mint_signer
    let with_mint_signer = data.create_mint.is_some() || create_spl_mint;

    // 7. create_mint
    let create_mint = data.create_mint.is_some();

    AccountsConfig {
        with_cpi_context,
        write_to_cpi_context,
        spl_mint_initialized,
        has_mint_to_actions,
        with_mint_signer,
        create_mint,
    }
}

// ============================================================================
// Randomized Tests
// ============================================================================

#[test]
fn test_accounts_config_randomized() {
    let mut rng = thread_rng();
    let seed: u64 = rng.gen();
    println!("seed value: {}", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    for _ in 0..1000 {
        // Generate random instruction data
        let instruction_data = generate_random_instruction_data(
            &mut rng,
            Some(true), // Random create_mint
            Some(true), // Random cpi_context
            Some(true), // Random spl_initialized
            0..6,       // 1-5 actions
        );
        // Serialize to bytes then deserialize as zero-copy
        let serialized = instruction_data.try_to_vec().expect("Failed to serialize");
        let (zero_copy_data, _) = MintActionCompressedInstructionData::zero_copy_at(&serialized)
            .expect("Failed to deserialize as zero-copy");

        // Check if this configuration should error
        let should_error = check_if_config_should_error(&instruction_data);

        // Generate actual config
        let actual_config_result = AccountsConfig::new(&zero_copy_data);

        if should_error {
            // Verify that it returns the expected error
            assert!(
                actual_config_result.is_err(),
                "Expected error for instruction data but got Ok. CPI context: {:?}, Actions: {:?}",
                instruction_data.cpi_context,
                instruction_data.actions
            );

            // Verify the specific error code
            let error = actual_config_result.unwrap_err();
            assert_eq!(
                error,
                light_compressed_token::ErrorCode::CpiContextSetNotUsable.into(),
                "Expected CpiContextSetNotUsable error but got {:?}",
                error
            );
        } else {
            // Compute expected config
            let expected_config = compute_expected_config(&instruction_data);

            // Should succeed
            let actual_config =
                actual_config_result.expect("AccountsConfig::new failed unexpectedly");
            assert_eq!(expected_config, actual_config);
        }
    }
}

/// Check if the given instruction data should result in an error
fn check_if_config_should_error(instruction_data: &MintActionCompressedInstructionData) -> bool {
    // Check if write_to_cpi_context is true
    let write_to_cpi_context = instruction_data
        .cpi_context
        .as_ref()
        .map(|x| x.first_set_context || x.set_context)
        .unwrap_or_default();

    if write_to_cpi_context {
        // Check for MintToCToken actions
        let has_mint_to_ctoken = instruction_data
            .actions
            .iter()
            .any(|action| matches!(action, Action::MintToCToken(_)));

        // Check for CreateSplMint actions
        let create_spl_mint = instruction_data
            .actions
            .iter()
            .any(|action| matches!(action, Action::CreateSplMint(_)));

        // Check if SPL mint is initialized
        let spl_mint_initialized =
            instruction_data.mint.metadata.spl_mint_initialized || create_spl_mint;

        // Return true if any of these conditions are met
        has_mint_to_ctoken || create_spl_mint || spl_mint_initialized
    } else {
        false
    }
}
