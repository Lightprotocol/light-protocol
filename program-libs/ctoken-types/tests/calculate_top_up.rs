use light_ctoken_types::calculate_top_up::{calculate_top_up, NUM_SLOTS_PER_TWO_YEARS};

const YIELD_PERCENTAGE_POINTS: u64 = 2000;
const TWO_YEAR_YIELD: u64 = 526176; // on 250 bytes account rent exemption * 0.2
#[test]
fn test_top_up_basic_calculation() {
    let num_bytes = 250;
    let last_slot = 0;
    let current_slot = 0;
    let compression_delay = NUM_SLOTS_PER_TWO_YEARS;
    let is_decompress = true;
    let mut two_year_yield = TWO_YEAR_YIELD;
    two_year_yield += 5000;
    let result = calculate_top_up(
        num_bytes,
        last_slot,
        current_slot,
        compression_delay,
        is_decompress,
        YIELD_PERCENTAGE_POINTS,
    );
    assert_eq!(result, two_year_yield);
}

// Slot Duration Tests
#[test]
fn test_slot_durations() {
    let last_slot = 1000;
    let current_slot = 1000;

    for compression_delay in 1..=1_000_000 {
        let expected = (((current_slot - last_slot).min(compression_delay) * 526176)
            / NUM_SLOTS_PER_TWO_YEARS)
            .max(1);
        let result = calculate_top_up(
            250,
            last_slot,
            current_slot,
            compression_delay,
            false,
            YIELD_PERCENTAGE_POINTS,
        );
        assert_eq!(result, expected, "compression_delay {}", compression_delay);
    }
}

#[test]
fn test_last_slot() {
    let compression_delay = 10000;
    let current_slot = 10000;
    for last_slot in 0..=current_slot {
        println!(
            "current_slot {} - last_slot {} + compression_delay{} {}",
            current_slot,
            last_slot,
            compression_delay,
            current_slot - last_slot + compression_delay
        );
        let expected = (((current_slot - last_slot).min(compression_delay) * 526176)
            / NUM_SLOTS_PER_TWO_YEARS)
            .max(1);
        let result = calculate_top_up(
            250,
            last_slot,
            current_slot,
            compression_delay,
            false,
            YIELD_PERCENTAGE_POINTS,
        );
        assert_eq!(result, expected, "compression_delay {}", compression_delay);
    }
}

// Slot Duration Tests
#[test]
fn test_zero_yield() {
    let last_slot = 1000;
    let current_slot = 1000;

    for compression_delay in 1..=1_000_000 {
        let expected = 1;
        let result = calculate_top_up(250, last_slot, current_slot, compression_delay, false, 0);
        assert_eq!(result, expected, "compression_delay {}", compression_delay);
    }
}
