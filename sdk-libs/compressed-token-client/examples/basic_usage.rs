//! Example demonstrating basic usage of the compressed-token-client library

use light_compressed_token_client::{
    batch_compress, compress, create_decompress_instruction, AccountState, CompressedAccount,
    DecompressParams, MerkleContext, TokenData, TreeType,
};
use solana_sdk::pubkey::Pubkey;

fn main() {
    // Example 1: Simple compression
    simple_compress_example();

    // Example 2: Batch compression
    batch_compress_example();

    // Example 3: Decompression
    decompress_example();
}

fn simple_compress_example() {
    println!("=== Simple Compress Example ===");

    let payer = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let source_token_account = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let output_state_tree = Pubkey::new_unique();

    let instruction = compress(
        payer,
        owner,
        source_token_account,
        mint,
        1000, // amount
        recipient,
        output_state_tree,
    )
    .expect("Failed to create compress instruction");

    println!("Created compress instruction:");
    println!("  Program ID: {}", instruction.program_id);
    println!("  Accounts: {} total", instruction.accounts.len());
    println!();
}

fn batch_compress_example() {
    println!("=== Batch Compress Example ===");

    let payer = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let source_token_account = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let output_state_tree = Pubkey::new_unique();

    let recipients = vec![
        (Pubkey::new_unique(), 500),
        (Pubkey::new_unique(), 300),
        (Pubkey::new_unique(), 200),
    ];

    let _instruction = batch_compress(
        payer,
        owner,
        source_token_account,
        mint,
        recipients.clone(),
        output_state_tree,
    )
    .expect("Failed to create batch compress instruction");

    println!(
        "Created batch compress instruction for {} recipients:",
        recipients.len()
    );
    for (recipient, amount) in recipients {
        println!("  {} -> {} tokens", recipient, amount);
    }
    println!();
}

fn decompress_example() {
    println!("=== Decompress Example ===");

    let payer = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let destination_token_account = Pubkey::new_unique();
    let merkle_tree = Pubkey::new_unique();
    let queue = Pubkey::new_unique();

    // Create mock compressed account data
    let compressed_account = CompressedAccount {
        owner: light_compressed_token_client::PROGRAM_ID,
        lamports: 0,
        data: None,
        address: None,
    };

    let token_data = TokenData {
        mint,
        owner,
        amount: 1000,
        delegate: None,
        state: AccountState::Initialized,
        tlv: None,
    };

    let merkle_context = MerkleContext {
        merkle_tree_pubkey: merkle_tree,
        queue_pubkey: queue,
        leaf_index: 0,
        prove_by_index: false,
        tree_type: TreeType::StateV2,
    };

    let params = DecompressParams {
        payer,
        input_compressed_token_accounts: vec![(compressed_account, token_data, merkle_context)],
        to_address: destination_token_account,
        amount: 500,
        recent_input_state_root_indices: vec![Some(0)],
        recent_validity_proof: None,
        output_state_tree: Some(merkle_tree),
        token_program_id: None,
    };

    let instruction =
        create_decompress_instruction(params).expect("Failed to create decompress instruction");

    println!("Created decompress instruction:");
    println!("  Program ID: {}", instruction.program_id);
    println!("  Decompressing 500 tokens (500 remain compressed)");
    println!();
}
