use light_zero_copy::{num_trait::ZeroCopyNumTrait, ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

pub const SLOTS_PER_EPOCH: u64 = 432_000;
// TODO: add token account version
// TODO: consider adding externally funded mode
/// Compressible extension for token accounts
/// Contains timing data for compression/decompression and rent authority
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut,
)]
#[repr(C)]
pub struct CompressibleExtension {
    pub version: u8, // version 0 is uninitialized, default is 1
    pub rent_authority: Option<[u8; 32]>,
    pub rent_recipient: Option<[u8; 32]>,
    pub last_claimed_slot: u64,
    pub lamports_at_last_claimed_slot: u64,
    pub write_top_up_lamports: Option<u32>,
}

pub const COMPRESSION_COST: u64 = 10_000;
pub const COMPRESSION_INCENTIVE: u64 = 1000;

pub const MIN_RENT: u64 = 1220;

pub fn rent_curve_per_epoch(bytes: u64) -> u64 {
    MIN_RENT + bytes * 10
}

pub fn get_rent(bytes: u64, epochs: u64) -> u64 {
    rent_curve_per_epoch(bytes) * epochs
}

pub fn get_rent_with_compression_cost(bytes: u64, epochs: u64) -> u64 {
    rent_curve_per_epoch(bytes) * epochs + COMPRESSION_COST + COMPRESSION_INCENTIVE
}

macro_rules! impl_is_compressible {
    ($struct_name:ty $(, $deref:tt)?) => {
        impl $struct_name {
            /// current - last epoch = num epochs due
            /// rent_due
            /// available_balance = current_lamports - last_lamports
            ///     (we can never claim more lamports than rent is due)
            /// remaining_balance = available_balance - rent_due
            pub fn is_compressible(
                &self,
                bytes: u64,
                current_slot: u64,
                current_lamports: u64,
            ) -> (bool, u64) {
                calculate_rent_and_balance(
                    bytes,
                    current_slot,
                    current_lamports,
                    $($deref)? self.last_claimed_slot,
                    $($deref)? self.lamports_at_last_claimed_slot,
                )
            }
        }
    };
}

impl_is_compressible!(CompressibleExtension);

impl_is_compressible!(ZCompressibleExtension<'_>, *);

impl_is_compressible!(ZCompressibleExtensionMut<'_>, *);

#[inline(always)]
fn calculate_rent_and_balance(
    bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    lamports_at_last_claimed_slot: impl ZeroCopyNumTrait,
) -> (bool, u64) {
    let (required_epochs, rent_per_epoch, epochs_paid, unutilized_lamports) =
        calculate_rent_details(
            bytes,
            current_slot,
            current_lamports,
            last_claimed_slot,
            lamports_at_last_claimed_slot,
        );
    let is_compressible = epochs_paid < required_epochs;
    if is_compressible {
        let epochs_payable = required_epochs.saturating_sub(epochs_paid);
        let payable = epochs_payable * rent_per_epoch + COMPRESSION_COST + COMPRESSION_INCENTIVE;
        let net_payable = payable.saturating_sub(unutilized_lamports);
        (true, net_payable)
    } else {
        (false, 0)
    }
}

#[inline(always)]
fn calculate_rent_details(
    bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    lamports_at_last_claimed_slot: impl ZeroCopyNumTrait,
) -> (u64, u64, u64, u64) {
    let available_balance = current_lamports
        .checked_sub(lamports_at_last_claimed_slot.into())
        .unwrap();
    let current_epoch = current_slot / SLOTS_PER_EPOCH + 1;
    let last_claimed_epoch: u64 = last_claimed_slot.into() / SLOTS_PER_EPOCH;
    let required_epochs = current_epoch.saturating_sub(last_claimed_epoch);

    let rent_per_epoch = rent_curve_per_epoch(bytes);
    let epochs_paid = available_balance / rent_per_epoch;
    let unutilized_lamports = available_balance % rent_per_epoch;
    (
        required_epochs,
        rent_per_epoch,
        epochs_paid,
        unutilized_lamports,
    )
}

#[inline(always)]
pub fn calculate_close_lamports(
    bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    lamports_at_last_claimed_slot: impl ZeroCopyNumTrait,
) -> (u64, u64) {
    let (_, _, _, unutilized_lamports) = calculate_rent_details(
        bytes,
        current_slot,
        current_lamports,
        last_claimed_slot,
        lamports_at_last_claimed_slot,
    );
    (current_lamports - unutilized_lamports, unutilized_lamports)
}

#[cfg(test)]
mod test {
    use super::*;

    const TEST_BYTES: u64 = 261;
    const RENT_PER_EPOCH: u64 = 3830;
    const FULL_COMPRESSION_COSTS: u64 = COMPRESSION_COST + COMPRESSION_INCENTIVE;

    #[derive(Debug)]
    struct TestInput {
        current_slot: u64,
        current_lamports: u64,
        last_claimed_slot: u64,
        lamports_at_last_claimed_slot: u64,
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
                    current_lamports: 1000,
                    last_claimed_slot: 0,
                    lamports_at_last_claimed_slot: 1000,
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
                    current_lamports: 1000 + RENT_PER_EPOCH + FULL_COMPRESSION_COSTS,
                    last_claimed_slot: 0,
                    lamports_at_last_claimed_slot: 1000 + FULL_COMPRESSION_COSTS,
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
                    current_lamports: 1000 + RENT_PER_EPOCH + FULL_COMPRESSION_COSTS,
                    last_claimed_slot: 0,
                    lamports_at_last_claimed_slot: 1000 + FULL_COMPRESSION_COSTS,
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
                    current_lamports: 1000 + RENT_PER_EPOCH + FULL_COMPRESSION_COSTS,
                    last_claimed_slot: 0,
                    lamports_at_last_claimed_slot: 1000 + FULL_COMPRESSION_COSTS,
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
                    current_lamports: 1000 + (RENT_PER_EPOCH * 3) + FULL_COMPRESSION_COSTS,
                    last_claimed_slot: 0,
                    lamports_at_last_claimed_slot: 1000 + FULL_COMPRESSION_COSTS,
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
                    current_lamports: 1000 + (RENT_PER_EPOCH * 2) - 1 + FULL_COMPRESSION_COSTS,
                    last_claimed_slot: 0,
                    lamports_at_last_claimed_slot: 1000 + FULL_COMPRESSION_COSTS,
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
                    current_lamports: 1000 + RENT_PER_EPOCH + FULL_COMPRESSION_COSTS,
                    last_claimed_slot: 0,
                    lamports_at_last_claimed_slot: 1000 + FULL_COMPRESSION_COSTS,
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
                    current_lamports: 1000 + (RENT_PER_EPOCH * 3 / 2) + FULL_COMPRESSION_COSTS,
                    last_claimed_slot: 0,
                    lamports_at_last_claimed_slot: 1000 + FULL_COMPRESSION_COSTS,
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
                    current_lamports: 1000,
                    last_claimed_slot: SLOTS_PER_EPOCH,
                    lamports_at_last_claimed_slot: 1000,
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
                    current_lamports: 1000 + (RENT_PER_EPOCH * 2) + FULL_COMPRESSION_COSTS,
                    last_claimed_slot: 0,
                    lamports_at_last_claimed_slot: 1000 + FULL_COMPRESSION_COSTS,
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
                    current_lamports: 1000 + (RENT_PER_EPOCH * 2) + FULL_COMPRESSION_COSTS,
                    last_claimed_slot: 0,
                    lamports_at_last_claimed_slot: 1000 + FULL_COMPRESSION_COSTS,
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
                    current_lamports: 1000 + (RENT_PER_EPOCH * 500) + FULL_COMPRESSION_COSTS,
                    last_claimed_slot: 0,
                    lamports_at_last_claimed_slot: 1000 + FULL_COMPRESSION_COSTS,
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
                    current_lamports: 1000 + RENT_PER_EPOCH + FULL_COMPRESSION_COSTS,
                    last_claimed_slot: 0,
                    lamports_at_last_claimed_slot: 1000 + FULL_COMPRESSION_COSTS,
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
                    current_lamports: 1000 + (RENT_PER_EPOCH * 2) + FULL_COMPRESSION_COSTS,
                    last_claimed_slot: SLOTS_PER_EPOCH, // Created in epoch 1
                    lamports_at_last_claimed_slot: 1000 + FULL_COMPRESSION_COSTS,
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
                    current_lamports: 1000 + (RENT_PER_EPOCH / 2) + FULL_COMPRESSION_COSTS,
                    last_claimed_slot: SLOTS_PER_EPOCH * 3,
                    lamports_at_last_claimed_slot: 1000 + FULL_COMPRESSION_COSTS,
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
                    current_lamports: 1000 + (RENT_PER_EPOCH * 100) + FULL_COMPRESSION_COSTS,
                    last_claimed_slot: 0,
                    lamports_at_last_claimed_slot: 1000 + FULL_COMPRESSION_COSTS,
                },
                expected: TestExpected {
                    is_compressible: false, // Has 100 epochs, only needs 2
                    deficit: 0,
                },
            },
        ];

        for test_case in test_cases {
            let (is_compressible, deficit) = calculate_rent_and_balance(
                TEST_BYTES,
                test_case.input.current_slot,
                test_case.input.current_lamports,
                test_case.input.last_claimed_slot,
                test_case.input.lamports_at_last_claimed_slot,
            );

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
}
