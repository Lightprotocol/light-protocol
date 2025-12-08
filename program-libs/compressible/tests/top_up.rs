#![cfg(test)]
use light_compressible::{
    compression_info::CompressionInfo,
    rent::{RentConfig, COMPRESSION_COST, COMPRESSION_INCENTIVE, SLOTS_PER_EPOCH},
};

const TEST_BYTES: u64 = 260;
const RENT_PER_EPOCH: u64 = 260 + 128;
const FULL_COMPRESSION_COSTS: u64 = (COMPRESSION_COST + COMPRESSION_INCENTIVE) as u64;

fn test_rent_config() -> RentConfig {
    RentConfig::default()
}

pub fn get_rent_exemption_lamports(_num_bytes: u64) -> u64 {
    2700480
}

#[derive(Debug)]
struct TestCase {
    name: &'static str,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: u64,
    lamports_per_write: u32,
    expected_top_up: u64,
    description: &'static str,
}

#[test]
fn test_calculate_top_up_lamports() {
    let rent_exemption_lamports = get_rent_exemption_lamports(TEST_BYTES);
    let lamports_per_write = 5000u32;

    let test_cases = vec![
        // ============================================================
        // PATH 1: COMPRESSIBLE CASES (lamports_per_write + rent_deficit)
        // ============================================================
        TestCase {
            name: "instant compressibility - account created with only rent exemption + compression cost",
            current_slot: 0,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS,
            last_claimed_slot: 0,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64 + RENT_PER_EPOCH + FULL_COMPRESSION_COSTS,
            description: "Epoch 0: available_balance=0, required_epochs<true>=1, deficit includes 1 epoch + compression_cost",
        },
        TestCase {
            name: "partial epoch rent in epoch 0",
            current_slot: 100,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + (RENT_PER_EPOCH / 2),
            last_claimed_slot: 0,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64 + (RENT_PER_EPOCH / 2) + FULL_COMPRESSION_COSTS,
            description: "Epoch 0: available_balance=194 (0.5 epochs), required=388 (1 epoch), compressible with deficit of 0.5 epoch",
        },
        TestCase {
            name: "epoch boundary crossing - becomes compressible",
            current_slot: SLOTS_PER_EPOCH + 1,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + RENT_PER_EPOCH,
            last_claimed_slot: 0,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64 + RENT_PER_EPOCH + FULL_COMPRESSION_COSTS,
            description: "Epoch 1: available_balance=388 (1 epoch), required_epochs<true>=2 (epochs 1+2), deficit=1 epoch + compression_cost",
        },
        TestCase {
            name: "many epochs behind (10 epochs)",
            current_slot: SLOTS_PER_EPOCH * 10,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + RENT_PER_EPOCH,
            last_claimed_slot: 0,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64 + (RENT_PER_EPOCH * 10) + FULL_COMPRESSION_COSTS,
            description: "Epoch 10: available_balance=388 (1 epoch), required_epochs<true>=11 (epochs 0-10 + next), deficit=10 epochs + compression_cost",
        },
        TestCase {
            name: "extreme epoch gap - 10,000 epochs behind",
            current_slot: SLOTS_PER_EPOCH * 10_000,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS,
            last_claimed_slot: 0,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64 + (RENT_PER_EPOCH * 10_001) + FULL_COMPRESSION_COSTS,
            description: "Epoch 10,000: available_balance=0, required_epochs<true>=10,001, deficit includes all 10,001 epochs",
        },
        TestCase {
            name: "one lamport short of required rent",
            current_slot: SLOTS_PER_EPOCH,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + (RENT_PER_EPOCH * 2) - 1,
            last_claimed_slot: 0,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64 + 1 + FULL_COMPRESSION_COSTS,
            description: "Epoch 1: available_balance=775 (1.997 epochs), required=776 (2 epochs), compressible with 1 lamport deficit",
        },
        // ============================================================
        // PATH 2: NOT COMPRESSIBLE, NEEDS TOP-UP (lamports_per_write)
        // ============================================================
        TestCase {
            name: "exact boundary - lagging claim requires top-up",
            current_slot: SLOTS_PER_EPOCH,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + (RENT_PER_EPOCH * 2),
            last_claimed_slot: 0,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64,
            description: "Epoch 1: last_claimed=epoch 0, funded through epoch 1, epochs_funded_ahead=1 < max=2",
        },
        TestCase {
            name: "exactly 1 epoch funded (max is 2)",
            current_slot: 0,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + RENT_PER_EPOCH,
            last_claimed_slot: 0,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64,
            description: "Epoch 0: not compressible, epochs_funded_ahead=1 < max_funded_epochs=2, needs write top-up only",
        },
        TestCase {
            name: "1.5 epochs funded (rounds down to 1)",
            current_slot: 0,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + (RENT_PER_EPOCH * 3 / 2),
            last_claimed_slot: 0,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64,
            description: "Epoch 0: not compressible, epochs_funded_ahead=582/388=1 (rounds down) < max_funded_epochs=2",
        },
        TestCase {
            name: "fractional epoch - 1.99 epochs rounds down",
            current_slot: 0,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + (RENT_PER_EPOCH * 2) - 1,
            last_claimed_slot: 0,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64,
            description: "Epoch 0: not compressible, epochs_funded_ahead=775/388=1 (rounds down) < max_funded_epochs=2",
        },
        TestCase {
            name: "epoch boundary with 1 epoch funded",
            current_slot: SLOTS_PER_EPOCH - 1,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + RENT_PER_EPOCH,
            last_claimed_slot: 0,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64,
            description: "Last slot of epoch 0: not compressible, epochs_funded_ahead=1 < max_funded_epochs=2",
        },
        TestCase {
            name: "account created in later epoch with 1 epoch rent",
            current_slot: SLOTS_PER_EPOCH * 5,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + RENT_PER_EPOCH,
            last_claimed_slot: SLOTS_PER_EPOCH * 5,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64,
            description: "Epoch 5: created same epoch, not compressible, epochs_funded_ahead=1 < max_funded_epochs=2",
        },
        // ============================================================
        // PATH 3: WELL-FUNDED (0 lamports)
        // ============================================================
        TestCase {
            name: "2 epochs funded - needs top-up (only 1 ahead)",
            current_slot: 0,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + (RENT_PER_EPOCH * 2),
            last_claimed_slot: 0,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64,
            description: "Epoch 0: funded through epoch 1, epochs_funded_ahead=1 < max=2, needs top-up",
        },
        TestCase {
            name: "3 epochs when max is 2 - exactly at target",
            current_slot: 0,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + (RENT_PER_EPOCH * 3),
            last_claimed_slot: 0,
            lamports_per_write,
            expected_top_up: 0,
            description: "Epoch 0: funded through epoch 2, epochs_funded_ahead=2 >= max_funded_epochs=2",
        },
        TestCase {
            name: "2 epochs at epoch 1 boundary - needs top-up",
            current_slot: SLOTS_PER_EPOCH,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + (RENT_PER_EPOCH * 2),
            last_claimed_slot: SLOTS_PER_EPOCH,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64,
            description: "Epoch 1: funded through epoch 2, epochs_funded_ahead=1 < max_funded_epochs=2",
        },
        // ============================================================
        // EDGE CASES
        // ============================================================
        TestCase {
            name: "zero lamports_per_write - compressible case",
            current_slot: 0,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS,
            last_claimed_slot: 0,
            lamports_per_write: 0,
            expected_top_up: RENT_PER_EPOCH + FULL_COMPRESSION_COSTS,
            description: "Zero write fee + compressible state: top_up = 0 + deficit (rent + compression_cost)",
        },
        TestCase {
            name: "zero lamports_per_write - needs top-up but write fee is 0",
            current_slot: 0,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + (RENT_PER_EPOCH * 2),
            last_claimed_slot: 0,
            lamports_per_write: 0,
            expected_top_up: 0,
            description: "Zero write fee: epochs_funded_ahead=1 < max=2, but lamports_per_write=0 so top_up=0",
        },
        TestCase {
            name: "large lamports_per_write",
            current_slot: 0,
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + RENT_PER_EPOCH,
            last_claimed_slot: 0,
            lamports_per_write: 1_000_000,
            expected_top_up: 1_000_000,
            description: "Large write fee (1M): not compressible, epochs_funded_ahead=1 < max_funded_epochs=2",
        },
        TestCase {
            name: "underflow protection - zero available balance",
            current_slot: 0,
            current_lamports: rent_exemption_lamports, // NOTE: Invalid state - missing compression_cost, but tests saturating_sub behavior
            last_claimed_slot: 0,
            lamports_per_write,
            expected_top_up: lamports_per_write as u64 + RENT_PER_EPOCH + FULL_COMPRESSION_COSTS,
            description: "Invalid state: current_lamports < rent_exemption+compression_cost, saturating_sub → available_balance=0",
        },
        // ============================================================
        // LAGGING CLAIMS - verifies arrears don't count as "funded ahead"
        // ============================================================
        TestCase {
            name: "lagging claim - 5 epochs funded but only 1 ahead due to arrears",
            current_slot: SLOTS_PER_EPOCH * 3, // epoch 3
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + (RENT_PER_EPOCH * 5),
            last_claimed_slot: 0, // epoch 0 - lagging 3 epochs behind
            lamports_per_write,
            expected_top_up: lamports_per_write as u64,
            description: "Epoch 3: 5 epochs funded from epoch 0 covers through epoch 4. epochs_funded_ahead = 4 - 3 = 1 < max=2, needs top-up",
        },
        TestCase {
            name: "lagging claim - 6 epochs funded, exactly 2 ahead",
            current_slot: SLOTS_PER_EPOCH * 3, // epoch 3
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + (RENT_PER_EPOCH * 6),
            last_claimed_slot: 0, // epoch 0 - lagging 3 epochs behind
            lamports_per_write,
            expected_top_up: 0,
            description: "Epoch 3: 6 epochs funded from epoch 0 covers through epoch 5. epochs_funded_ahead = 5 - 3 = 2 >= max=2, no top-up",
        },
        TestCase {
            name: "lagging claim - just covers arrears plus current+next, no buffer",
            current_slot: SLOTS_PER_EPOCH * 3, // epoch 3
            current_lamports: rent_exemption_lamports + FULL_COMPRESSION_COSTS + (RENT_PER_EPOCH * 4),
            last_claimed_slot: 0, // epoch 0
            lamports_per_write,
            expected_top_up: lamports_per_write as u64,
            description: "Epoch 3: 4 epochs funded (0,1,2,3) covers through epoch 3. epochs_funded_ahead = 3 - 3 = 0 < max=2, needs top-up",
        },
    ];

    for test_case in test_cases {
        let extension = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            compression_authority: [0u8; 32],
            rent_sponsor: [0u8; 32],
            last_claimed_slot: test_case.last_claimed_slot,
            lamports_per_write: test_case.lamports_per_write,
            compress_to_pubkey: 0,
            rent_config: test_rent_config(),
        };

        let top_up = extension
            .calculate_top_up_lamports(
                TEST_BYTES,
                test_case.current_slot,
                test_case.current_lamports,
                rent_exemption_lamports,
            )
            .unwrap();

        assert_eq!(
            top_up, test_case.expected_top_up,
            "\nTest '{}' failed:\n  Description: {}\n  Expected: {}\n  Got: {}\n  Test case: {:?}",
            test_case.name, test_case.description, test_case.expected_top_up, top_up, test_case
        );
    }
}

#[test]
fn test_default_scenario_16_epochs_with_2_epoch_topup() {
    let rent_exemption_lamports = get_rent_exemption_lamports(TEST_BYTES);

    // Random starting slot (epoch 5)
    let start_slot = SLOTS_PER_EPOCH * 5 + 1234;

    let lamports_per_write = 388u32 * 2;
    let initial_rent_epochs = 16u64; // Create with 16 epochs rent
    let initial_lamports =
        rent_exemption_lamports + FULL_COMPRESSION_COSTS + (RENT_PER_EPOCH * initial_rent_epochs);

    let extension = CompressionInfo {
        account_version: 3,
        config_account_version: 1,
        compression_authority: [0u8; 32],
        rent_sponsor: [0u8; 32],
        last_claimed_slot: start_slot,
        lamports_per_write,
        compress_to_pubkey: 0,
        rent_config: test_rent_config(), // max_funded_epochs = 2
    };

    // Test each epoch from 0 to 13 - should NOT require top-up
    for epoch_offset in 0..=13 {
        let current_slot = start_slot + (SLOTS_PER_EPOCH * epoch_offset);

        let top_up = extension
            .calculate_top_up_lamports(
                TEST_BYTES,
                current_slot,
                initial_lamports,
                rent_exemption_lamports,
            )
            .unwrap();

        assert_eq!(
            top_up,
            0,
            "Epoch offset {} should NOT require top-up (still {} epochs ahead)",
            epoch_offset,
            15 - epoch_offset
        );
    }

    // Epoch 14 - should require top-up (only 1 epoch funded ahead < max_funded_epochs=2)
    let epoch_14_slot = start_slot + (SLOTS_PER_EPOCH * 14);
    let top_up = extension
        .calculate_top_up_lamports(
            TEST_BYTES,
            epoch_14_slot,
            initial_lamports,
            rent_exemption_lamports,
        )
        .unwrap();

    assert_eq!(
        top_up, lamports_per_write as u64,
        "Epoch 14 should require top-up (only 1 epoch funded ahead)"
    );

    // After write in epoch 14, lamports increase by top_up (lamports_per_write = 2 epochs rent)
    let lamports_after_write = initial_lamports + top_up;

    // Epochs 14 and 15 should NOT require top-up after the write
    for epoch_offset in 14..=15 {
        let current_slot = start_slot + (SLOTS_PER_EPOCH * epoch_offset);

        let top_up = extension
            .calculate_top_up_lamports(
                TEST_BYTES,
                current_slot,
                lamports_after_write,
                rent_exemption_lamports,
            )
            .unwrap();

        assert_eq!(
            top_up, 0,
            "Epoch offset {} should NOT require top-up after write (funded for 2 more epochs)",
            epoch_offset
        );
    }

    // Epoch 16 - should require top-up again
    let epoch_16_slot = start_slot + (SLOTS_PER_EPOCH * 16);
    let top_up = extension
        .calculate_top_up_lamports(
            TEST_BYTES,
            epoch_16_slot,
            lamports_after_write,
            rent_exemption_lamports,
        )
        .unwrap();

    assert_eq!(
        top_up, lamports_per_write as u64,
        "Epoch 16 should require top-up again"
    );
}
