use std::time::Instant;

use dashmap::DashMap;
use light_compressed_account::Pubkey as LightPubkey;
use light_compressible::rent::RentConfig;
use light_ctoken_types::{
    state::{AccountState, CToken, CompressionInfo, ExtensionStruct},
    COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use rand::seq::SliceRandom;
use rand::Rng;
use solana_sdk::pubkey::Pubkey;

const SLOTS_PER_EPOCH: u64 = 6300;
const COMPRESSIBLE_TOKEN_RENT_EXEMPTION: u64 = 2_039_280;

/// Represents state of a compressible token account being tracked
#[derive(Clone, Debug)]
struct CompressibleAccountState {
    #[allow(dead_code)]
    pub pubkey: Pubkey,
    pub account: CToken,
    pub lamports: u64,
}

/// Creates a mock CToken with Compressible extension
fn create_mock_ctoken_with_compressible(last_claimed_slot: u64, _lamports: u64) -> CToken {
    let mut rng = rand::thread_rng();

    let mint_bytes: [u8; 32] = rng.gen();
    let owner_bytes: [u8; 32] = rng.gen();
    let compression_authority: [u8; 32] = rng.gen();
    let rent_sponsor: [u8; 32] = rng.gen();

    let compression_info = CompressionInfo {
        config_account_version: 1,
        compress_to_pubkey: 0,
        account_version: 1,
        lamports_per_write: 5000,
        compression_authority,
        rent_sponsor,
        last_claimed_slot,
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

/// Sets up a DashMap with the specified number of entries and compressibility percentage
/// Returns: (map, current_slot, expected_compressible_pubkeys)
fn setup_dashmap(
    entries: usize,
    compressible_percent: f64,
) -> (DashMap<Pubkey, CompressibleAccountState>, u64, Vec<Pubkey>) {
    let map: DashMap<Pubkey, CompressibleAccountState> = DashMap::new();
    let mut rng = rand::thread_rng();

    // Calculate how many should be compressible
    let num_compressible = (entries as f64 * compressible_percent) as usize;

    // Create indices and shuffle to randomize which accounts are compressible
    let mut indices: Vec<usize> = (0..entries).collect();
    indices.shuffle(&mut rng);

    let compressible_indices: std::collections::HashSet<usize> =
        indices.iter().take(num_compressible).copied().collect();

    let mut compressible_pubkeys = Vec::new();

    // Choose a current slot that will be used for filtering
    // Accounts with last_funded_slot < current_slot are compressible
    let current_slot = 1_000_000_u64;

    for i in 0..entries {
        let pubkey_bytes: [u8; 32] = rng.gen();
        let pubkey = Pubkey::new_from_array(pubkey_bytes);

        let is_compressible = compressible_indices.contains(&i);

        // Configure lamports and last_claimed_slot to control compressibility
        let (last_claimed_slot, lamports) = if is_compressible {
            // Set up so this account IS ready to compress
            // Make last_claimed_slot old enough that last_funded_slot < current_slot
            // Use minimal lamports so get_last_funded_epoch returns 0 or very low value
            let last_claimed_slot = current_slot.saturating_sub(SLOTS_PER_EPOCH * 20);
            let lamports = COMPRESSIBLE_TOKEN_RENT_EXEMPTION + 5000; // Just enough for rent exemption + compression cost, no extra epochs
            compressible_pubkeys.push(pubkey);
            (last_claimed_slot, lamports)
        } else {
            // Set up so this account is NOT ready to compress
            // Make last_claimed_slot recent and give LOTS of lamports for many epochs
            let last_claimed_slot = current_slot.saturating_sub(SLOTS_PER_EPOCH * 2);
            let lamports = COMPRESSIBLE_TOKEN_RENT_EXEMPTION + 5000 + 50_000_000; // Plenty for 100+ epochs
            (last_claimed_slot, lamports)
        };

        let state = CompressibleAccountState {
            pubkey,
            account: create_mock_ctoken_with_compressible(last_claimed_slot, lamports),
            lamports,
        };

        map.insert(pubkey, state);
    }

    (map, current_slot, compressible_pubkeys)
}

/// Benchmarks DashMap access using the production filtering logic
/// Returns: (duration, matched_count)
fn benchmark_access(
    map: &DashMap<Pubkey, CompressibleAccountState>,
    current_slot: u64,
) -> (std::time::Duration, usize) {
    let start = Instant::now();

    // This replicates the get_ready_to_compress() logic
    let matched: Vec<CompressibleAccountState> = map
        .iter()
        .filter(|entry| {
            let state = entry.value();
            if let Some(ExtensionStruct::Compressible(compressible_ext)) = state
                .account
                .extensions
                .as_ref()
                .and_then(|exts| {
                    exts.iter()
                        .find(|ext| matches!(ext, ExtensionStruct::Compressible(_)))
                })
            {
                let last_funded_epoch = compressible_ext
                    .get_last_funded_epoch(
                        COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
                        state.lamports,
                        COMPRESSIBLE_TOKEN_RENT_EXEMPTION,
                    )
                    .unwrap_or(0);

                let last_funded_slot = last_funded_epoch * SLOTS_PER_EPOCH;
                last_funded_slot < current_slot
            } else {
                false
            }
        })
        .map(|entry| entry.value().clone())
        .collect();

    let duration = start.elapsed();
    let matched_count = matched.len();

    (duration, matched_count)
}

#[test]
#[ignore] // Run with: cargo test --test bench_dashmap_access bench_dashmap_access_10_percent -- --ignored --nocapture
fn bench_dashmap_access_10_percent() {
    println!("\n=== DashMap Access Benchmark - 10% Compressible ===");
    println!("All entries have Compressible extension");
    println!("10% meet compressibility criteria (last_funded_slot < current_slot)\n");

    let configs = vec![
        (1_000, "1K entries"),
        (10_000, "10K entries"),
        (100_000, "100K entries"),
    ];

    for (count, label) in configs {
        println!("--- {} ---", label);

        let (map, current_slot, expected_compressible) = setup_dashmap(count, 0.10);

        let (duration, matched_count) = benchmark_access(&map, current_slot);

        let time_ms = duration.as_millis();
        let time_per_access_us = (duration.as_micros() as f64) / (count as f64);
        let access_rate = count as f64 / duration.as_secs_f64();
        let match_percent = (matched_count as f64 / count as f64) * 100.0;

        println!("  Time: {}ms", time_ms);
        println!("  Avg per access: {:.2}μs", time_per_access_us);
        println!("  Access rate: {:.0} entries/sec", access_rate);
        println!("  Matched: {} / {} ({:.1}%)", matched_count, count, match_percent);
        println!(
            "  Expected: {} ({:.1}%)",
            expected_compressible.len(),
            (expected_compressible.len() as f64 / count as f64) * 100.0
        );
        println!();
    }

    println!("=== Benchmark Complete ===\n");
}

#[test]
#[ignore] // Run with: cargo test --test bench_dashmap_access bench_dashmap_access_100_percent -- --ignored --nocapture
fn bench_dashmap_access_100_percent() {
    println!("\n=== DashMap Access Benchmark - 100% Compressible ===");
    println!("All entries have Compressible extension");
    println!("100% meet compressibility criteria (last_funded_slot < current_slot)\n");

    let configs = vec![
        (1_000, "1K entries"),
        (10_000, "10K entries"),
        (100_000, "100K entries"),
    ];

    for (count, label) in configs {
        println!("--- {} ---", label);

        let (map, current_slot, expected_compressible) = setup_dashmap(count, 1.0);

        let (duration, matched_count) = benchmark_access(&map, current_slot);

        let time_ms = duration.as_millis();
        let time_per_access_us = (duration.as_micros() as f64) / (count as f64);
        let access_rate = count as f64 / duration.as_secs_f64();
        let match_percent = (matched_count as f64 / count as f64) * 100.0;

        println!("  Time: {}ms", time_ms);
        println!("  Avg per access: {:.2}μs", time_per_access_us);
        println!("  Access rate: {:.0} entries/sec", access_rate);
        println!("  Matched: {} / {} ({:.1}%)", matched_count, count, match_percent);
        println!(
            "  Expected: {} ({:.1}%)",
            expected_compressible.len(),
            (expected_compressible.len() as f64 / count as f64) * 100.0
        );
        println!();
    }

    println!("=== Benchmark Complete ===\n");
}
