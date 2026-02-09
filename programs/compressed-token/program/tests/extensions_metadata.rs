/// Randomized test for process_extensions_config_with_actions.
///
/// Validates that metadata add/remove action sequences produce correct
/// AdditionalMetadataConfig output, covering the add-remove-add bug
/// from audit issue #16.
use borsh::BorshSerialize;
use light_compressed_account::Pubkey;
use light_compressed_token::extensions::process_extensions_config_with_actions;
use light_token_interface::{
    instructions::mint_action::{
        Action, CpiContext, CreateMint, MintActionCompressedInstructionData, MintInstructionData,
        RemoveMetadataKeyAction, UpdateMetadataFieldAction,
    },
    state::{
        extensions::{AdditionalMetadataConfig, TokenMetadataConfig},
        AdditionalMetadata, ExtensionStruct, ExtensionStructConfig, MintMetadata, TokenMetadata,
    },
    MINT_ADDRESS_TREE,
};
use light_zero_copy::traits::ZeroCopyAt;
use rand::{rngs::StdRng, thread_rng, Rng, SeedableRng};

/// Small key pool to maximize add/remove/re-add collisions.
const KEY_POOL: &[&[u8]] = &[b"k0", b"k1", b"k2", b"k3", b"k4"];

fn random_key(rng: &mut StdRng) -> Vec<u8> {
    KEY_POOL[rng.gen_range(0..KEY_POOL.len())].to_vec()
}

fn random_value(rng: &mut StdRng) -> Vec<u8> {
    let len = rng.gen_range(1..20);
    (0..len).map(|_| rng.gen::<u8>()).collect()
}

fn random_additional_metadata(rng: &mut StdRng) -> Vec<AdditionalMetadata> {
    let count = rng.gen_range(0..=4);
    let mut items = Vec::new();
    let mut used_keys = Vec::new();
    for _ in 0..count {
        let key = random_key(rng);
        if used_keys.contains(&key) {
            continue; // no duplicate keys in initial metadata
        }
        used_keys.push(key.clone());
        items.push(AdditionalMetadata {
            key,
            value: random_value(rng),
        });
    }
    items
}

/// Generate random metadata actions (UpdateMetadataField with field_type=3
/// and RemoveMetadataKey), plus occasional name/symbol/uri updates.
fn random_metadata_actions(rng: &mut StdRng) -> Vec<Action> {
    let count = rng.gen_range(0..=10);
    let mut actions = Vec::with_capacity(count);
    for _ in 0..count {
        match rng.gen_range(0..5) {
            // Custom field update (field_type=3) targeting extension 0
            0 | 1 => actions.push(Action::UpdateMetadataField(UpdateMetadataFieldAction {
                extension_index: 0,
                field_type: 3,
                key: random_key(rng),
                value: random_value(rng),
            })),
            // Remove key targeting extension 0
            2 => actions.push(Action::RemoveMetadataKey(RemoveMetadataKeyAction {
                extension_index: 0,
                key: random_key(rng),
                idempotent: 1,
            })),
            // Name/symbol/uri update targeting extension 0
            3 => actions.push(Action::UpdateMetadataField(UpdateMetadataFieldAction {
                extension_index: 0,
                field_type: rng.gen_range(0..3), // 0=name, 1=symbol, 2=uri
                key: vec![],
                value: random_value(rng),
            })),
            // Action targeting a different extension (should be ignored)
            4 => actions.push(Action::UpdateMetadataField(UpdateMetadataFieldAction {
                extension_index: 1,
                field_type: 3,
                key: random_key(rng),
                value: random_value(rng),
            })),
            _ => unreachable!(),
        }
    }
    actions
}

/// Reference implementation: replay actions on a state map to compute expected config.
///
/// Uses a simple sequential approach: maintain an ordered list of (key, exists, value_len)
/// entries. Each action mutates the state in place. Original keys appear first (in their
/// original order), newly added keys are appended. At the end, filter to existing keys.
fn compute_expected_config(
    metadata: &TokenMetadata,
    actions: &[Action],
) -> TokenMetadataConfig {
    let extension_index = 0usize;

    // Track name/symbol/uri lengths (last update wins)
    let mut name_len = metadata.name.len();
    let mut symbol_len = metadata.symbol.len();
    let mut uri_len = metadata.uri.len();

    // State map: (key, exists, value_len) - preserves insertion order
    let mut state: Vec<(Vec<u8>, bool, usize)> = metadata
        .additional_metadata
        .iter()
        .map(|item| (item.key.clone(), true, item.value.len()))
        .collect();

    // Replay all actions sequentially
    for action in actions {
        match action {
            Action::UpdateMetadataField(update)
                if update.extension_index as usize == extension_index =>
            {
                match update.field_type {
                    0 => name_len = update.value.len(),
                    1 => symbol_len = update.value.len(),
                    2 => uri_len = update.value.len(),
                    3 => {
                        if let Some(entry) =
                            state.iter_mut().find(|(k, _, _)| *k == update.key)
                        {
                            entry.1 = true;
                            entry.2 = update.value.len();
                        } else {
                            state.push((
                                update.key.clone(),
                                true,
                                update.value.len(),
                            ));
                        }
                    }
                    _ => {}
                }
            }
            Action::RemoveMetadataKey(remove)
                if remove.extension_index as usize == extension_index =>
            {
                if let Some(entry) = state.iter_mut().find(|(k, _, _)| *k == remove.key) {
                    entry.1 = false;
                }
            }
            _ => {}
        }
    }

    TokenMetadataConfig {
        name: name_len as u32,
        symbol: symbol_len as u32,
        uri: uri_len as u32,
        additional_metadata: state
            .into_iter()
            .filter(|(_, exists, _)| *exists)
            .map(|(key, _, value_len)| AdditionalMetadataConfig {
                key: key.len() as u32,
                value: value_len as u32,
            })
            .collect(),
    }
}

/// Wrap actions in a MintActionCompressedInstructionData, serialize,
/// and zero-copy parse to get &[ZAction].
fn serialize_actions(actions: &[Action]) -> Vec<u8> {
    let instruction_data = MintActionCompressedInstructionData {
        create_mint: Some(CreateMint::default()),
        leaf_index: 0,
        prove_by_index: false,
        root_index: 0,
        max_top_up: 0,
        actions: actions.to_vec(),
        proof: None,
        cpi_context: Some(CpiContext {
            set_context: false,
            first_set_context: false,
            in_tree_index: 0,
            in_queue_index: 0,
            out_queue_index: 0,
            token_out_queue_index: 0,
            assigned_account_index: 0,
            read_only_address_trees: [0u8; 4],
            address_tree_pubkey: MINT_ADDRESS_TREE,
        }),
        mint: Some(MintInstructionData {
            supply: 0,
            decimals: 0,
            metadata: MintMetadata {
                version: 0,
                mint_decompressed: false,
                mint: Pubkey::default(),
                mint_signer: [0u8; 32],
                bump: 0,
            },
            mint_authority: None,
            freeze_authority: None,
            extensions: None,
        }),
    };
    instruction_data.try_to_vec().expect("Failed to serialize")
}

#[test]
fn test_metadata_config_with_actions_randomized() {
    let mut rng = thread_rng();
    let seed: u64 = rng.gen();
    println!("seed value: {}", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    for i in 0..1000 {
        let additional_metadata = random_additional_metadata(&mut rng);
        let token_metadata = TokenMetadata {
            update_authority: Pubkey::default(),
            mint: Pubkey::default(),
            name: random_value(&mut rng),
            symbol: random_value(&mut rng),
            uri: random_value(&mut rng),
            additional_metadata,
        };
        let actions = random_metadata_actions(&mut rng);
        let extensions = vec![ExtensionStruct::TokenMetadata(token_metadata.clone())];

        // Serialize and zero-copy parse to get ZAction slice
        let serialized = serialize_actions(&actions);
        let (zc_data, _) =
            MintActionCompressedInstructionData::zero_copy_at(&serialized)
                .unwrap_or_else(|e| panic!("iteration {i}, seed {seed}: zero_copy_at failed: {e}"));

        let (has_extensions, config_vec, _) =
            process_extensions_config_with_actions(Some(&extensions), &zc_data.actions)
                .unwrap_or_else(|e| {
                    panic!("iteration {i}, seed {seed}: process_extensions failed: {e:?}")
                });

        assert!(has_extensions, "iteration {i}, seed {seed}: expected has_extensions=true");
        assert_eq!(config_vec.len(), 1, "iteration {i}, seed {seed}: expected 1 config");

        let actual = match &config_vec[0] {
            ExtensionStructConfig::TokenMetadata(cfg) => cfg,
            other => panic!("iteration {i}, seed {seed}: unexpected config type: {other:?}"),
        };
        let expected = compute_expected_config(&token_metadata, &actions);

        assert_eq!(
            *actual, expected,
            "iteration {i}, seed {seed}\nactions: {actions:?}\nmetadata: {:?}",
            token_metadata.additional_metadata
        );
    }
}
