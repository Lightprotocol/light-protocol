#![cfg(test)]
use light_compressible::{
    compression_info::CompressionInfo,
    rent::{
        get_last_funded_epoch, AccountRentState, RentConfig, COMPRESSION_COST,
        COMPRESSION_INCENTIVE, SLOTS_PER_EPOCH,
    },
};
use rand::{rngs::StdRng, thread_rng, Rng, SeedableRng};

const TEST_BYTES: u64 = 260;
const RENT_PER_EPOCH: u64 = 260 + 128;
const FULL_COMPRESSION_COSTS: u64 = (COMPRESSION_COST + COMPRESSION_INCENTIVE) as u64;

fn test_rent_config() -> RentConfig {
    RentConfig::default()
}

pub fn get_rent_exemption_lamports(_num_bytes: u64) -> u64 {
    2700480
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RentState {
    completed_epochs: u64,
    is_compressible: bool,
    claimable: Option<u64>,
    epochs_ahead: u64,
    close_dist_to_sponsor: u64,
    close_dist_to_user: u64,
    last_funded: u64,
    top_up: u64,
}

impl RentState {
    fn from_account_rent_state(
        state: &AccountRentState,
        compression_info: &CompressionInfo,
        rent_config: &RentConfig,
        rent_exemption_lamports: u64,
    ) -> Self {
        let completed_epochs = state.get_completed_epochs();
        let is_compressible = state
            .is_compressible(rent_config, rent_exemption_lamports)
            .is_some();
        let claimable = state.calculate_claimable_rent(rent_config, rent_exemption_lamports);
        let epochs_ahead = state.epochs_funded_ahead(rent_config, rent_exemption_lamports);
        let close_dist = state.calculate_close_distribution(rent_config, rent_exemption_lamports);
        let last_funded = get_last_funded_epoch(
            state.num_bytes,
            state.current_lamports,
            state.last_claimed_slot,
            rent_config,
            rent_exemption_lamports,
        );
        let top_up = compression_info
            .calculate_top_up_lamports(state.num_bytes, state.current_slot, state.current_lamports)
            .unwrap();

        Self {
            completed_epochs,
            is_compressible,
            claimable,
            epochs_ahead,
            close_dist_to_sponsor: close_dist.to_rent_sponsor,
            close_dist_to_user: close_dist.to_user,
            last_funded,
            top_up,
        }
    }

    fn expected(
        current_slot: u64,
        current_lamports: u64,
        last_claimed_slot: u64,
        rent_exemption_lamports: u64,
        lamports_per_write: u32,
    ) -> Self {
        let current_epoch = current_slot / SLOTS_PER_EPOCH;
        let last_claimed_epoch = last_claimed_slot / SLOTS_PER_EPOCH;

        let available_rent = current_lamports - rent_exemption_lamports - FULL_COMPRESSION_COSTS;
        let total_epochs_fundable = available_rent / RENT_PER_EPOCH;
        let required_epochs = current_epoch - last_claimed_epoch + 1;
        let epochs_ahead = total_epochs_fundable.saturating_sub(required_epochs);
        let completed_epochs = current_epoch - last_claimed_epoch;

        let last_funded = if total_epochs_fundable > 0 {
            last_claimed_epoch + total_epochs_fundable - 1
        } else {
            last_claimed_epoch.saturating_sub(1)
        };

        let rent_needed = required_epochs * RENT_PER_EPOCH;
        let is_compressible = available_rent < rent_needed;

        let claimable = if is_compressible {
            None
        } else {
            Some(completed_epochs * RENT_PER_EPOCH)
        };

        // Calculate close distribution
        let rent_due = required_epochs * RENT_PER_EPOCH;
        let unutilized = available_rent.saturating_sub(rent_due);
        let close_dist_to_user = unutilized;
        let close_dist_to_sponsor = current_lamports - unutilized;

        // Top-up logic: max_funded_epochs = 2
        let top_up = if is_compressible {
            let deficit = rent_needed - available_rent + FULL_COMPRESSION_COSTS;
            lamports_per_write as u64 + deficit
        } else if epochs_ahead >= 2 {
            0
        } else {
            lamports_per_write as u64
        };

        Self {
            completed_epochs,
            is_compressible,
            claimable,
            epochs_ahead,
            close_dist_to_sponsor,
            close_dist_to_user,
            last_funded,
            top_up,
        }
    }
}

#[test]
fn test_consistency_16_epochs_progression() {
    let rent_exemption_lamports = get_rent_exemption_lamports(TEST_BYTES);
    let rent_config = test_rent_config();

    // Random starting slot (epoch 5)
    let start_slot = SLOTS_PER_EPOCH * 5 + 1234;

    // lamports_per_write = 2 epochs rent (matching top_up test)
    let lamports_per_write = (RENT_PER_EPOCH * 2) as u32;
    let initial_rent_epochs = 16u64;
    let initial_lamports =
        rent_exemption_lamports + FULL_COMPRESSION_COSTS + (RENT_PER_EPOCH * initial_rent_epochs);

    // Tracker variables - accumulate state across epochs
    let mut total_rent_claimed: u64 = 0;
    let mut total_top_up: u64 = 0;
    let mut current_lamports = initial_lamports;
    let mut current_slot = start_slot;
    let mut last_claimed_slot = start_slot;
    let mut epochs_funded_ahead = initial_rent_epochs - 1;
    let seed = thread_rng().gen();
    println!("Seed: {}", seed);
    // Continue with 1000 random slot increments (1-10 epochs)
    let mut rng = StdRng::seed_from_u64(seed);

    for _ in 0..1_000_000 {
        // Always advance by at least 1 slot, up to epochs_funded_ahead epochs
        let max_slots = (epochs_funded_ahead.max(1)) * SLOTS_PER_EPOCH;
        let slot_increment = rng.gen_range(0..max_slots);
        current_slot += slot_increment;
        let epoch_offset = current_slot / SLOTS_PER_EPOCH;
        let state = AccountRentState {
            num_bytes: TEST_BYTES,
            current_slot,
            current_lamports,
            last_claimed_slot,
        };

        // CompressionInfo for top-up calculation (recreate with current last_claimed_slot)
        let compression_info = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            compression_authority: [0u8; 32],
            rent_sponsor: [0u8; 32],
            last_claimed_slot,
            lamports_per_write,
            compress_to_pubkey: 0,
            rent_exemption_paid: rent_exemption_lamports as u32,
            _reserved: 0,
            rent_config: test_rent_config(),
        };

        let actual = RentState::from_account_rent_state(
            &state,
            &compression_info,
            &rent_config,
            rent_exemption_lamports,
        );

        let expected = RentState::expected(
            current_slot,
            current_lamports,
            last_claimed_slot,
            rent_exemption_lamports,
            lamports_per_write,
        );

        assert_eq!(actual, expected, "Epoch offset {}", epoch_offset);

        // Simulate rent claim if claimable - update trackers
        if let Some(rent_to_claim) = actual.claimable {
            if rent_to_claim > 0 {
                total_rent_claimed += rent_to_claim;
                current_lamports -= rent_to_claim;
                last_claimed_slot = current_slot;
            }
        }

        // Print state for debugging
        println!(
            "Epoch {}: slot={}, lamports={}, completed={}, compressible={}, claimable={:?}, ahead={}, last_funded={}, top_up={}, total_claimed={}",
            epoch_offset,
            current_slot,
            current_lamports,
            actual.completed_epochs,
            actual.is_compressible,
            actual.claimable,
            actual.epochs_ahead,
            actual.last_funded,
            actual.top_up,
            total_rent_claimed
        );
        // Always add top-up (simulates write operation)
        total_top_up += actual.top_up;
        current_lamports += actual.top_up;

        epochs_funded_ahead = actual.epochs_ahead;
    }

    // Verify balance equation: current = initial - claimed + top_ups
    assert_eq!(
        current_lamports,
        initial_lamports + total_top_up - total_rent_claimed,
        "Final lamports mismatch"
    );
}
