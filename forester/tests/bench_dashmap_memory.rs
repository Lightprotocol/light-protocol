use std::time::Instant;

use dashmap::DashMap;
use light_compressed_account::Pubkey as LightPubkey;
use light_compressible::rent::RentConfig;
use light_ctoken_types::{
    state::{AccountState, CToken, CompressionInfo, ExtensionStruct},
    COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use rand::Rng;
use solana_sdk::pubkey::Pubkey;

/// Represents state of a compressible token account being tracked
#[derive(Clone, Debug)]
struct CompressibleAccountState {
    pub pubkey: Pubkey,
    pub account: CToken,
    pub lamports: u64,
}

fn create_mock_ctoken() -> CToken {
    let mut rng = rand::thread_rng();

    // Generate random pubkeys
    let mint_bytes: [u8; 32] = rng.gen();
    let owner_bytes: [u8; 32] = rng.gen();
    let compression_authority: [u8; 32] = rng.gen();
    let rent_sponsor: [u8; 32] = rng.gen();

    // Create mock compression info
    let compression_info = CompressionInfo {
        config_account_version: 1,
        compress_to_pubkey: 0,
        account_version: 1,
        lamports_per_write: 5000,
        compression_authority,
        rent_sponsor,
        last_claimed_slot: 0,
        rent_config: RentConfig {
            base_rent: 5000,
            compression_cost: 1000,
            lamports_per_byte_per_epoch: 34,
            max_funded_epochs: 100,
            _padding: [0; 2],
        },
    };

    CToken {
        mint: LightPubkey::new_from_array(mint_bytes),
        owner: LightPubkey::new_from_array(owner_bytes),
        amount: rng.gen_range(0..1_000_000),
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        extensions: Some(vec![ExtensionStruct::Compressible(compression_info)]),
    }
}

fn estimate_dashmap_memory(entries: usize) -> usize {
    use std::mem::size_of;

    // Pubkey key size
    let key_size = size_of::<Pubkey>();

    // CompressibleAccountState value size
    let value_size = size_of::<CompressibleAccountState>();

    // DashMap overhead (estimated: hash table buckets, internal locks, etc.)
    // DashMap uses RwLock per shard and a hash table
    let dashmap_overhead = entries * 8; // rough estimate for hash table pointers

    // Total = (key + value) * entries + overhead
    let total = (key_size + value_size) * entries + dashmap_overhead;

    total
}

#[test]
#[ignore] // Run with: cargo test --test bench_dashmap_memory -- --ignored --nocapture
fn bench_dashmap_insertions() {
    println!("\n=== DashMap Memory & Performance Benchmark ===");
    println!("Structure: DashMap<Pubkey, CompressibleAccountState>");
    println!("Value size: CToken with Compressible extension (~{} bytes on-chain)\n", COMPRESSIBLE_TOKEN_ACCOUNT_SIZE);

    let configs = vec![
        (1_000, "1K entries"),
        (10_000, "10K entries"),
        (100_000, "100K entries"),
    ];

    let map: DashMap<Pubkey, CompressibleAccountState> = DashMap::new();
    let mut rng = rand::thread_rng();

    use std::mem::size_of;
    println!("Size of Pubkey (key): {} bytes", size_of::<Pubkey>());
    println!("Size of CompressibleAccountState (value): {} bytes\n", size_of::<CompressibleAccountState>());

    let mut total_entries = 0;

    for (count, label) in configs {
        println!("--- {} ---", label);

        let entries_to_add = count - total_entries;
        let start_time = Instant::now();

        // Generate and insert entries
        for _ in 0..entries_to_add {
            let pubkey_bytes: [u8; 32] = rng.gen();
            let pubkey = Pubkey::new_from_array(pubkey_bytes);

            let state = CompressibleAccountState {
                pubkey,
                account: create_mock_ctoken(),
                lamports: rng.gen_range(2_000_000..10_000_000),
            };

            map.insert(pubkey, state);
        }

        let duration = start_time.elapsed();
        total_entries = count;

        // Measurements
        let time_ms = duration.as_millis();
        let time_per_insert_us = (duration.as_micros() as f64) / (entries_to_add as f64);
        let estimated_memory = estimate_dashmap_memory(total_entries);
        let estimated_memory_mb = estimated_memory as f64 / 1_048_576.0;

        println!("  Time: {}ms", time_ms);
        println!("  Avg per insert: {:.2}μs", time_per_insert_us);
        println!("  Rate: {:.0} inserts/sec", entries_to_add as f64 / duration.as_secs_f64());
        println!("  Map size: {} entries", map.len());
        println!("  Estimated memory: {:.2} MB ({} bytes)", estimated_memory_mb, estimated_memory);
        println!();
    }

    println!("=== Benchmark Complete ===\n");
}

#[test]
#[ignore] // Run with: cargo test --test bench_dashmap_memory bench_single_batch -- --ignored --nocapture
fn bench_dashmap_single_batch() {
    println!("\n=== DashMap Single Batch Insertion Benchmark ===");
    println!("Measuring fresh insertions for each batch size\n");

    let configs = vec![
        (1_000, "1K entries"),
        (10_000, "10K entries"),
        (100_000, "100K entries"),
    ];

    for (count, label) in configs {
        println!("--- {} ---", label);

        let map: DashMap<Pubkey, CompressibleAccountState> = DashMap::new();
        let mut rng = rand::thread_rng();

        let start_time = Instant::now();

        // Generate and insert entries
        for _ in 0..count {
            let pubkey_bytes: [u8; 32] = rng.gen();
            let pubkey = Pubkey::new_from_array(pubkey_bytes);

            let state = CompressibleAccountState {
                pubkey,
                account: create_mock_ctoken(),
                lamports: rng.gen_range(2_000_000..10_000_000),
            };

            map.insert(pubkey, state);
        }

        let duration = start_time.elapsed();

        // Measurements
        let time_ms = duration.as_millis();
        let time_per_insert_us = (duration.as_micros() as f64) / (count as f64);
        let estimated_memory = estimate_dashmap_memory(count);
        let estimated_memory_mb = estimated_memory as f64 / 1_048_576.0;

        println!("  Time: {}ms", time_ms);
        println!("  Avg per insert: {:.2}μs", time_per_insert_us);
        println!("  Rate: {:.0} inserts/sec", count as f64 / duration.as_secs_f64());
        println!("  Map size: {} entries", map.len());
        println!("  Estimated memory: {:.2} MB ({} bytes)", estimated_memory_mb, estimated_memory);
        println!();

        // Let the map drop to free memory before next test
        drop(map);
    }

    println!("=== Benchmark Complete ===\n");
}
