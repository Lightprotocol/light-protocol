use light_registry::{
    account_compression_cpi::sdk::{
        create_nullify_2_instruction, nullify_2_lookup_table_accounts,
        CreateNullify2InstructionInputs,
    },
    utils::get_forester_epoch_pda_from_authority,
};
use solana_sdk::{
    hash::Hash,
    message::{v0, AddressLookupTableAccount, VersionedMessage},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::VersionedTransaction,
};

/// Validates that a nullify_2 v0 transaction with ALT fits within the
/// 1232-byte Solana transaction size limit.
///
/// This is a pure serialization check -- no validator needed.
/// If the serialized size is <= 1232 bytes, it will work on any validator.
#[test]
fn test_nullify_2_v0_transaction_size() {
    let authority = Keypair::new();
    let merkle_tree = Pubkey::new_unique();
    let nullifier_queue = Pubkey::new_unique();
    let epoch = 0u64;
    let forester_pda = get_forester_epoch_pda_from_authority(&authority.pubkey(), epoch).0;

    let ix = create_nullify_2_instruction(
        CreateNullify2InstructionInputs {
            authority: authority.pubkey(),
            nullifier_queue,
            merkle_tree,
            change_log_index: 1400,
            queue_index_0: 100,
            queue_index_1: 200,
            leaf_index_0: 67_000_000,
            leaf_index_1: 67_000_001,
            proof_0: [[0xAA; 32]; 15],
            proof_1: [[0xBB; 32]; 15],
            shared_proof_node: [0xCC; 32],
            derivation: authority.pubkey(),
            is_metadata_forester: false,
        },
        0,
    );

    // Build synthetic ALT with the known accounts
    let alt_accounts =
        nullify_2_lookup_table_accounts(merkle_tree, nullifier_queue, Some(forester_pda));
    let alt_address = Pubkey::new_unique();
    let alt = AddressLookupTableAccount {
        key: alt_address,
        addresses: alt_accounts,
    };

    // Compile v0 message
    let blockhash = Hash::default();
    let message = v0::Message::try_compile(&authority.pubkey(), &[ix], &[alt], blockhash)
        .expect("Failed to compile v0 message");

    // Create signed transaction
    let versioned_message = VersionedMessage::V0(message);
    let tx = VersionedTransaction::try_new(versioned_message, &[&authority])
        .expect("Failed to create versioned transaction");

    let serialized = tx.message.serialize();
    // Full tx = compact-u16 sig count (1) + signatures (64 * n) + serialized message
    let tx_size = 1 + tx.signatures.len() * 64 + serialized.len();

    println!(
        "nullify_2 v0 transaction size: {} bytes (limit: 1232)",
        tx_size
    );
    println!("  instruction data: {} bytes", 1007);
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
        "nullify_2 v0 transaction is {} bytes, exceeds 1232 byte limit by {} bytes",
        tx_size,
        tx_size - 1232
    );
}
