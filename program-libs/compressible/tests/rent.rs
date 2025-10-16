use light_compressible::rent::{
    AccountRentState, RentConfig, COMPRESSION_COST, COMPRESSION_INCENTIVE, SLOTS_PER_EPOCH,
};

const TEST_BYTES: u64 = 260;
const RENT_PER_EPOCH: u64 = 260 + 128;
const FULL_COMPRESSION_COSTS: u64 = (COMPRESSION_COST + COMPRESSION_INCENTIVE) as u64;

fn test_rent_config() -> RentConfig {
    RentConfig::default()
}

pub fn get_rent_exemption_lamports(_num_bytes: u64) -> u64 {
    // Standard rent-exempt balance for tests: 890880 + 6.96 * bytes
    // This matches Solana's rent calculation
    // 890_880 + ((696 * _num_bytes + 99) / 100)
    2700480
}
#[derive(Debug)]
struct TestInput {
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: u64,
}

#[derive(Debug)]
struct TestExpected {
    is_compressible: bool,
    deficit: u64,
}

#[derive(Debug)]
struct TestCase {
    name: &'static str,
    input: TestInput,
    expected: TestExpected,
}

#[test]
fn test_calculate_rent_and_balance() {
    let test_cases = vec![
        TestCase {
            name: "account creation instant compressible",
            input: TestInput {
                current_slot: 0,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES) + FULL_COMPRESSION_COSTS,
                last_claimed_slot: 0,
            },
            expected: TestExpected {
                is_compressible: true,
                deficit: RENT_PER_EPOCH + FULL_COMPRESSION_COSTS, // Full rent for 1 epoch
            },
        },
        TestCase {
            name: "account creation in epoch 0 paid rent for one epoch (epoch 0)",
            input: TestInput {
                current_slot: 0,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES)
                    + RENT_PER_EPOCH
                    + FULL_COMPRESSION_COSTS,
                last_claimed_slot: 0,
            },
            expected: TestExpected {
                is_compressible: false,
                deficit: 0,
            },
        },
        TestCase {
            name: "account paid one epoch rent, last slot of epoch 0",
            input: TestInput {
                current_slot: SLOTS_PER_EPOCH - 1,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES)
                    + RENT_PER_EPOCH
                    + FULL_COMPRESSION_COSTS,
                last_claimed_slot: 0,
            },
            expected: TestExpected {
                is_compressible: false,
                deficit: 0,
            },
        },
        TestCase {
            name: "account paid one epoch, in epoch 1",
            input: TestInput {
                current_slot: SLOTS_PER_EPOCH + 1,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES)
                    + RENT_PER_EPOCH
                    + FULL_COMPRESSION_COSTS,
                last_claimed_slot: 0,
            },
            expected: TestExpected {
                is_compressible: true,
                deficit: RENT_PER_EPOCH + FULL_COMPRESSION_COSTS,
            },
        },
        TestCase {
            name: "account with 3 epochs prepaid, checked in epoch 2",
            input: TestInput {
                current_slot: SLOTS_PER_EPOCH * 2,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES)
                    + (RENT_PER_EPOCH * 3)
                    + FULL_COMPRESSION_COSTS,
                last_claimed_slot: 0,
            },
            expected: TestExpected {
                is_compressible: false, // Has 3 epochs, needs 3 for epoch 2
                deficit: 0,
            },
        },
        TestCase {
            name: "one lamport short of required rent in epoch 1",
            input: TestInput {
                current_slot: SLOTS_PER_EPOCH,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES) + (RENT_PER_EPOCH * 2)
                    - 1
                    + FULL_COMPRESSION_COSTS,
                last_claimed_slot: 0,
            },
            expected: TestExpected {
                is_compressible: true,
                deficit: 1 + FULL_COMPRESSION_COSTS,
            },
        },
        TestCase {
            name: "account untouched for 10 epochs",
            input: TestInput {
                current_slot: SLOTS_PER_EPOCH * 10,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES)
                    + RENT_PER_EPOCH
                    + FULL_COMPRESSION_COSTS,
                last_claimed_slot: 0,
            },
            expected: TestExpected {
                is_compressible: true,
                deficit: (RENT_PER_EPOCH * 10) + FULL_COMPRESSION_COSTS, // Needs 11 epochs, has 1
            },
        },
        TestCase {
            name: "account with 1.5 epochs of rent in epoch 1",
            input: TestInput {
                current_slot: SLOTS_PER_EPOCH,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES)
                    + (RENT_PER_EPOCH * 3 / 2)
                    + FULL_COMPRESSION_COSTS,
                last_claimed_slot: 0,
            },
            expected: TestExpected {
                is_compressible: true, // Has 1.5 epochs (rounds down to 1), needs 2
                deficit: (RENT_PER_EPOCH / 2) + FULL_COMPRESSION_COSTS,
            },
        },
        TestCase {
            name: "account created in epoch 1 with no rent",
            input: TestInput {
                current_slot: SLOTS_PER_EPOCH,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES) + FULL_COMPRESSION_COSTS,
                last_claimed_slot: SLOTS_PER_EPOCH,
            },
            expected: TestExpected {
                is_compressible: true, // Created with no rent, instantly compressible
                deficit: RENT_PER_EPOCH + FULL_COMPRESSION_COSTS,
            },
        },
        TestCase {
            name: "last slot of epoch 1 with 2 epochs paid",
            input: TestInput {
                current_slot: SLOTS_PER_EPOCH * 2 - 1,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES)
                    + (RENT_PER_EPOCH * 2)
                    + FULL_COMPRESSION_COSTS,
                last_claimed_slot: 0,
            },
            expected: TestExpected {
                is_compressible: false, // Still in epoch 1, has 2 epochs
                deficit: 0,
            },
        },
        TestCase {
            name: "first slot of epoch 2 with 2 epochs paid",
            input: TestInput {
                current_slot: SLOTS_PER_EPOCH * 2,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES)
                    + (RENT_PER_EPOCH * 2)
                    + FULL_COMPRESSION_COSTS,
                last_claimed_slot: 0,
            },
            expected: TestExpected {
                is_compressible: true, // Now in epoch 2, needs 3 epochs
                deficit: RENT_PER_EPOCH + FULL_COMPRESSION_COSTS,
            },
        },
        TestCase {
            name: "very large epoch number",
            input: TestInput {
                current_slot: SLOTS_PER_EPOCH * 1000,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES)
                    + (RENT_PER_EPOCH * 500)
                    + FULL_COMPRESSION_COSTS,
                last_claimed_slot: 0,
            },
            expected: TestExpected {
                is_compressible: true, // Has 500 epochs, needs 1001
                deficit: (RENT_PER_EPOCH * 501) + FULL_COMPRESSION_COSTS,
            },
        },
        TestCase {
            name: "tracking compressibility transition - not yet compressible",
            input: TestInput {
                current_slot: SLOTS_PER_EPOCH - 1, // Last slot of epoch 0
                current_lamports: get_rent_exemption_lamports(TEST_BYTES)
                    + RENT_PER_EPOCH
                    + FULL_COMPRESSION_COSTS,
                last_claimed_slot: 0,
            },
            expected: TestExpected {
                is_compressible: false, // In epoch 0, has 1 epoch (more than needed)
                deficit: 0,
            },
        },
        TestCase {
            name: "account with exactly 2 epochs at epoch boundary",
            input: TestInput {
                current_slot: SLOTS_PER_EPOCH * 2,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES)
                    + (RENT_PER_EPOCH * 2)
                    + FULL_COMPRESSION_COSTS,
                last_claimed_slot: SLOTS_PER_EPOCH, // Created in epoch 1
            },
            expected: TestExpected {
                is_compressible: false, // In epoch 2, from epoch 1, needs 2 epochs, has 2
                deficit: 0,
            },
        },
        TestCase {
            name: "account with partial rent in later epoch",
            input: TestInput {
                current_slot: SLOTS_PER_EPOCH * 5,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES)
                    + (RENT_PER_EPOCH / 2)
                    + FULL_COMPRESSION_COSTS,
                last_claimed_slot: SLOTS_PER_EPOCH * 3,
            },
            expected: TestExpected {
                is_compressible: true, // From epoch 3 to 5, needs 3 epochs, has 0.5
                deficit: (RENT_PER_EPOCH * 3 - RENT_PER_EPOCH / 2) + FULL_COMPRESSION_COSTS,
            },
        },
        TestCase {
            name: "account with massive prepayment",
            input: TestInput {
                current_slot: SLOTS_PER_EPOCH,
                current_lamports: get_rent_exemption_lamports(TEST_BYTES)
                    + (RENT_PER_EPOCH * 100)
                    + FULL_COMPRESSION_COSTS,
                last_claimed_slot: 0,
            },
            expected: TestExpected {
                is_compressible: false, // Has 100 epochs, only needs 2
                deficit: 0,
            },
        },
    ];

    let rent_config = test_rent_config();
    let rent_exemption_lamports = get_rent_exemption_lamports(TEST_BYTES);

    for test_case in test_cases {
        let state = AccountRentState {
            num_bytes: TEST_BYTES,
            current_slot: test_case.input.current_slot,
            current_lamports: test_case.input.current_lamports,
            last_claimed_slot: test_case.input.last_claimed_slot,
        };

        let deficit_option = state.is_compressible(&rent_config, rent_exemption_lamports);
        let (is_compressible, deficit) = match deficit_option {
            Some(d) => (true, d),
            None => (false, 0),
        };

        assert_eq!(
            deficit, test_case.expected.deficit,
            "Test '{}' failed: deficit mismatch {:?}",
            test_case.name, test_case
        );
        assert_eq!(
            is_compressible, test_case.expected.is_compressible,
            "Test '{}' failed: is_compressible mismatch test case {:?}",
            test_case.name, test_case
        );
    }
}

#[test]
fn test_claimable_lamports() {
    // Test claiming rent for completed epochs only
    let rent_config = test_rent_config();
    let rent_exemption_lamports = get_rent_exemption_lamports(TEST_BYTES);

    // Scenario 1: No completed epochs (same epoch)
    let state = AccountRentState {
        num_bytes: TEST_BYTES,
        current_slot: 100, // Slot in epoch 0
        current_lamports: rent_exemption_lamports + RENT_PER_EPOCH + FULL_COMPRESSION_COSTS,
        last_claimed_slot: 0, // Last claimed in epoch 0
    };
    let claimable = state.calculate_claimable_rent(&rent_config, rent_exemption_lamports);
    assert_eq!(claimable, Some(0), "Should not claim in same epoch");

    // Scenario 2: One completed epoch
    let state = AccountRentState {
        num_bytes: TEST_BYTES,
        current_slot: SLOTS_PER_EPOCH + 100, // Slot in epoch 1
        current_lamports: rent_exemption_lamports + RENT_PER_EPOCH * 2 + FULL_COMPRESSION_COSTS,
        last_claimed_slot: 0, // Last claimed in epoch 0
    };
    let claimable = state.calculate_claimable_rent(&rent_config, rent_exemption_lamports);
    assert_eq!(
        claimable,
        Some(RENT_PER_EPOCH),
        "Should claim for epoch 0 when in epoch 1"
    );

    // Scenario 3: Two epochs passed, one claimable
    let state = AccountRentState {
        num_bytes: TEST_BYTES,
        current_slot: SLOTS_PER_EPOCH * 2 + 100, // Slot in epoch 2
        current_lamports: rent_exemption_lamports + (RENT_PER_EPOCH * 3) + FULL_COMPRESSION_COSTS,
        last_claimed_slot: 0, // Last claimed in epoch 0
    };
    let claimable = state.calculate_claimable_rent(&rent_config, rent_exemption_lamports);
    assert_eq!(
        claimable,
        Some(2 * RENT_PER_EPOCH),
        "Should claim rent for epoch 1 only"
    );

    // Scenario 4: Multiple completed epochs
    let state = AccountRentState {
        num_bytes: TEST_BYTES,
        current_slot: SLOTS_PER_EPOCH * 4 + 100, // Slot in epoch 5
        current_lamports: rent_exemption_lamports + (RENT_PER_EPOCH * 5) + FULL_COMPRESSION_COSTS,
        last_claimed_slot: 0, // Last claimed in epoch 0
    };
    let claimable = state.calculate_claimable_rent(&rent_config, rent_exemption_lamports);
    assert_eq!(
        claimable,
        Some(RENT_PER_EPOCH * 4),
        "Should claim rent for epochs 1-4"
    );

    // Scenario 5: Account is compressible (insufficient rent)
    let state = AccountRentState {
        num_bytes: TEST_BYTES,
        current_slot: SLOTS_PER_EPOCH * 5 + 100, // Slot in epoch 5
        current_lamports: rent_exemption_lamports + RENT_PER_EPOCH + FULL_COMPRESSION_COSTS, // Only 1 epoch of rent available
        last_claimed_slot: 0, // Last claimed in epoch 0
    };
    let claimable = state.calculate_claimable_rent(&rent_config, rent_exemption_lamports);
    assert_eq!(claimable, None, "Should only claim available rent");
}
