use light_registry::account_compression_cpi::sdk::{
    create_nullify_state_v1_multi_instruction, nullify_state_v1_multi_lookup_table_accounts,
    CreateNullifyStateV1MultiInstructionInputs, NULLIFY_STATE_V1_MULTI_MAX_NODES,
};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    message::{v0, AddressLookupTableAccount, VersionedMessage},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::VersionedTransaction,
};

/// Validates that a nullify_state_v1_multi v0 transaction with ALT and SetComputeUnitLimit
/// fits within the 1232-byte Solana transaction size limit.
///
/// This is a pure serialization check -- no validator needed.
#[test]
fn test_nullify_state_v1_multi_v0_transaction_size() {
    let authority = Keypair::new();
    let merkle_tree = Pubkey::new_unique();
    let nullifier_queue = Pubkey::new_unique();

    // Worst case: MAX_NODES unique nodes
    let nullify_ix = create_nullify_state_v1_multi_instruction(
        CreateNullifyStateV1MultiInstructionInputs {
            authority: authority.pubkey(),
            nullifier_queue,
            merkle_tree,
            change_log_index: 1400,
            queue_indices: [100, 200, 300, 400],
            leaf_indices: [67_000_000, 67_000_001, 67_000_002, 67_000_003],
            proof_bitvecs: [0; 4],
            nodes: vec![[0xAA; 32]; NULLIFY_STATE_V1_MULTI_MAX_NODES],
            derivation: authority.pubkey(),
            is_metadata_forester: false,
        },
        0,
    );

    // Compute budget instructions (both are added by the forester's smart_transaction logic)
    let compute_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(600_000);
    let compute_price_ix = ComputeBudgetInstruction::set_compute_unit_price(1);

    // Build synthetic ALT with the known accounts (includes ComputeBudget program ID)
    let alt_accounts = nullify_state_v1_multi_lookup_table_accounts(merkle_tree, nullifier_queue);
    let alt_address = Pubkey::new_unique();
    let alt = AddressLookupTableAccount {
        key: alt_address,
        addresses: alt_accounts,
    };

    // Compile v0 message with all instructions
    let blockhash = Hash::default();
    let message = v0::Message::try_compile(
        &authority.pubkey(),
        &[compute_price_ix, compute_limit_ix, nullify_ix],
        &[alt],
        blockhash,
    )
    .expect("Failed to compile v0 message");

    // Create signed transaction
    let versioned_message = VersionedMessage::V0(message);
    let tx = VersionedTransaction::try_new(versioned_message, &[&authority])
        .expect("Failed to create versioned transaction");

    let serialized = tx.message.serialize();
    // Full tx = compact-u16 sig count (1) + signatures (64 * n) + serialized message
    let tx_size = 1 + tx.signatures.len() * 64 + serialized.len();

    let ix_data_size = 8 + 2 + 8 + 16 + 16 + 4 + NULLIFY_STATE_V1_MULTI_MAX_NODES * 32;

    println!(
        "nullify_state_v1_multi v0 transaction size: {} bytes (limit: 1232)",
        tx_size
    );
    println!(
        "  nullify_state_v1_multi instruction data: {} bytes",
        ix_data_size
    );
    println!(
        "  max_nodes: {} ({} bytes payload)",
        NULLIFY_STATE_V1_MULTI_MAX_NODES,
        NULLIFY_STATE_V1_MULTI_MAX_NODES * 32
    );
    println!("  margin: {} bytes", 1232_i64 - tx_size as i64);

    // Breakdown
    println!("\nTransaction breakdown:");
    println!("  signatures: {}", tx.signatures.len() * 64 + 1);
    let static_keys = tx.message.static_account_keys();
    println!("  static account keys: {}", static_keys.len());
    for (i, key) in static_keys.iter().enumerate() {
        let label = if *key == authority.pubkey() {
            "authority (signer)"
        } else if *key == light_registry::ID {
            "registry program"
        } else if *key == solana_sdk::compute_budget::ID {
            "compute budget program"
        } else {
            "unknown"
        };
        println!("    [{}] {} ({})", i, key, label);
    }
    if let VersionedMessage::V0(m) = &tx.message {
        println!("  address table lookups: {}", m.address_table_lookups.len());
        for alt_lookup in &m.address_table_lookups {
            println!("    writable indices: {:?}", alt_lookup.writable_indexes);
            println!("    readonly indices: {:?}", alt_lookup.readonly_indexes);
        }
    };

    assert!(
        tx_size <= 1232,
        "nullify_state_v1_multi v0 transaction is {} bytes, exceeds 1232 byte limit by {} bytes",
        tx_size,
        tx_size - 1232
    );
}
