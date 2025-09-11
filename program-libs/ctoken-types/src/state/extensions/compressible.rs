use light_zero_copy::{num_trait::ZeroCopyNumTrait, ZeroCopy, ZeroCopyMut};
use zerocopy::U64;

use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountError {
    FailedBorrowRentSysvar,
}

pub const SLOTS_PER_EPOCH: u64 = 432_000;
// TODO: add token account version
// TODO: consider adding externally funded mode
/// Compressible extension for token accounts
/// Contains timing data for compression/decompression and rent authority
#[derive(
    Debug,
    Clone,
    Hash,
    Copy,
    PartialEq,
    Eq,
    AnchorSerialize,
    AnchorDeserialize,
    ZeroCopy,
    ZeroCopyMut,
)]
#[repr(C)]
pub struct CompressibleExtension {
    pub version: u8, // version 0 is uninitialized, default is 1
    pub rent_authority: [u8; 32],
    pub rent_recipient: [u8; 32],
    pub last_claimed_slot: u64,
    pub write_top_up_lamports: u32,
    pub rent_config: RentConfig,
}

#[derive(
    Debug,
    Clone,
    Hash,
    Copy,
    PartialEq,
    Eq,
    AnchorSerialize,
    AnchorDeserialize,
    ZeroCopy,
    ZeroCopyMut,
)]
#[repr(C)]
pub struct RentConfig {
    pub min_rent: u16,
    pub full_compression_incentive: u16,
    pub rent_per_byte: u8,
    _place_holder_bytes: [u8; 3],
}

impl RentConfig {
    pub fn rent_curve_per_epoch(&self, bytes: u64) -> u64 {
        rent_curve_per_epoch(self.min_rent as u64, self.rent_per_byte as u64, bytes)
    }
    pub fn get_rent(&self, bytes: u64, epochs: u64) -> u64 {
        self.rent_curve_per_epoch(bytes) * epochs
    }
    pub fn get_rent_with_compression_cost(&self, bytes: u64, epochs: u64) -> u64 {
        self.rent_curve_per_epoch(bytes) * epochs + self.full_compression_incentive as u64
    }
}
pub fn rent_curve_per_epoch(min_rent: u64, rent_per_byte: u64, bytes: u64) -> u64 {
    min_rent + bytes * rent_per_byte
}

pub fn get_rent(min_rent: u64, rent_per_byte: u64, bytes: u64, epochs: u64) -> u64 {
    rent_curve_per_epoch(min_rent, rent_per_byte, bytes) * epochs
}
pub fn get_rent_with_compression_cost(
    min_rent: u64,
    rent_per_byte: u64,
    bytes: u64,
    epochs: u64,
    compression_costs: u64,
) -> u64 {
    get_rent(min_rent, rent_per_byte, bytes, epochs) + compression_costs
}

#[track_caller]
pub fn get_rent_exemption_lamports(_bytes: u64) -> Result<u64, AccountError> {
    #[cfg(target_os = "solana")]
    {
        use pinocchio::sysvars::Sysvar;
        pinocchio::sysvars::rent::Rent::get()
            .map(|rent| rent.minimum_balance(_bytes as usize))
            .map_err(|_| AccountError::FailedBorrowRentSysvar)
    }
    #[cfg(all(not(target_os = "solana"), feature = "solana"))]
    {
        use solana_sysvar::Sysvar;

        solana_sysvar::rent::Rent::get()
            .map(|rent| rent.minimum_balance(_bytes as usize))
            .map_err(|_| AccountError::FailedBorrowRentSysvar)
    }
    #[cfg(all(not(target_os = "solana"), not(feature = "solana")))]
    {
        #[cfg(test)]
        {
            // Standard rent-exempt balance for tests: 890880 + 6.96 * bytes
            // This matches Solana's rent calculation
            Ok(890_880 + ((696 * _bytes + 99) / 100))
        }
        #[cfg(not(test))]
        unimplemented!(
            "get_rent_exemption_lamports is only implemented for target os solana and tests"
        )
    }
}

impl Default for RentConfig {
    fn default() -> Self {
        Self {
            min_rent: MIN_RENT,
            full_compression_incentive: COMPRESSION_COST + COMPRESSION_INCENTIVE,
            rent_per_byte: RENT_PER_BYTE,
            _place_holder_bytes: [0; 3],
        }
    }
}

pub const COMPRESSION_COST: u16 = 10_000;
pub const COMPRESSION_INCENTIVE: u16 = 1000;

pub const MIN_RENT: u16 = 1220;
pub const RENT_PER_BYTE: u8 = 10;

// Unified macro for all compressible extension types
macro_rules! impl_is_compressible {
    ($struct_name:ty) => {
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
            ) -> Result<(bool, u64), AccountError> {
                let rent_exemption_lamports = get_rent_exemption_lamports(bytes)?;
                let min_rent: u64 = self.rent_config.min_rent.into();
                let rent_per_byte: u64 = self.rent_config.rent_per_byte.into();
                let full_compression_incentive: u64 =
                    self.rent_config.full_compression_incentive.into();

                Ok(calculate_rent_and_balance(
                    bytes,
                    current_slot,
                    current_lamports,
                    self.last_claimed_slot,
                    rent_exemption_lamports,
                    min_rent,
                    rent_per_byte,
                    full_compression_incentive,
                ))
            }
        }
    };
}

impl_is_compressible!(CompressibleExtension);
impl_is_compressible!(ZCompressibleExtension<'_>);
impl_is_compressible!(ZCompressibleExtensionMut<'_>);

/// Calculate the last epoch that has been paid for.
/// Returns the epoch number through which rent has been prepaid.
///
/// # Returns
/// The last epoch number that is covered by rent payments.
/// This is calculated as: last_claimed_epoch + epochs_paid - 1
///
/// # Example
/// If an account was created in epoch 0 and paid for 3 epochs of rent,
/// the last paid epoch would be 2 (epochs 0, 1, and 2 are covered).
#[inline(always)]
pub fn get_last_paid_epoch(
    bytes: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    min_rent: u64,
    rent_per_byte: u64,
    full_compression_incentive: u64,
) -> u64 {
    // Reuse the existing calculate_rent_inner function with INCLUDE_CURRENT=false
    // to get epochs_paid calculation
    let (_, _rent_per_epoch_calc, epochs_paid, _) = calculate_rent_inner::<false>(
        bytes,
        0, // current_slot not needed for epochs_paid calculation
        current_lamports,
        last_claimed_slot,
        rent_exemption_lamports,
        min_rent,
        rent_per_byte,
        full_compression_incentive,
    );

    let last_claimed_epoch: u64 = last_claimed_slot.into() / SLOTS_PER_EPOCH;

    // The last paid epoch is the last claimed epoch plus epochs paid minus 1
    // If no epochs are paid, the account is immediately compressible
    if epochs_paid > 0 {
        last_claimed_epoch + epochs_paid - 1
    } else {
        // No rent paid, last paid epoch is before last claimed
        last_claimed_epoch.saturating_sub(1)
    }
}

// Unified macro to implement get_last_paid_epoch for all extension types
macro_rules! impl_get_last_paid_epoch {
    ($struct_name:ty) => {
        impl $struct_name {
            /// Get the last epoch that has been paid for.
            /// Returns the epoch number through which rent has been prepaid.
            pub fn get_last_paid_epoch(
                &self,
                bytes: u64,
                current_lamports: u64,
                rent_exemption_lamports: u64,
            ) -> Result<u64, AccountError> {
                let min_rent: u64 = self.rent_config.min_rent.into();
                let rent_per_byte: u64 = self.rent_config.rent_per_byte.into();
                let full_compression_incentive: u64 =
                    self.rent_config.full_compression_incentive.into();

                Ok(get_last_paid_epoch(
                    bytes,
                    current_lamports,
                    self.last_claimed_slot,
                    rent_exemption_lamports,
                    min_rent,
                    rent_per_byte,
                    full_compression_incentive,
                ))
            }
        }
    };
}

impl_get_last_paid_epoch!(CompressibleExtension);
impl_get_last_paid_epoch!(ZCompressibleExtension<'_>);
impl_get_last_paid_epoch!(ZCompressibleExtensionMut<'_>);

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn calculate_rent_and_balance(
    bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    min_rent: u64,
    rent_per_byte: u64,
    full_compression_incentive: u64,
) -> (bool, u64) {
    let (required_epochs, rent_per_epoch, epochs_paid, unutilized_lamports) =
        calculate_rent_with_current_epoch(
            bytes,
            current_slot,
            current_lamports,
            last_claimed_slot,
            rent_exemption_lamports,
            min_rent,
            rent_per_byte,
            full_compression_incentive,
        );

    let is_compressible = epochs_paid < required_epochs;
    if is_compressible {
        let epochs_payable = required_epochs.saturating_sub(epochs_paid);
        let payable = epochs_payable * rent_per_epoch + full_compression_incentive;
        let net_payable = payable.saturating_sub(unutilized_lamports);
        (true, net_payable)
    } else {
        (false, 0)
    }
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn calculate_rent_with_current_epoch(
    bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    min_rent: u64,
    rent_per_byte: u64,
    full_compression_incentive: u64,
) -> (u64, u64, u64, u64) {
    calculate_rent_inner::<true>(
        bytes,
        current_slot,
        current_lamports,
        last_claimed_slot,
        rent_exemption_lamports,
        min_rent,
        rent_per_byte,
        full_compression_incentive,
    )
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn calculate_rent_inner<const INCLUDE_CURRENT: bool>(
    bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    min_rent: u64,
    rent_per_byte: u64,
    full_compression_incentive: u64,
) -> (u64, u64, u64, u64) {
    println!("current lamports: {}", current_lamports);
    let available_balance = current_lamports
        .checked_sub(rent_exemption_lamports + full_compression_incentive)
        .unwrap();
    let current_epoch = if INCLUDE_CURRENT {
        current_slot / SLOTS_PER_EPOCH + 1
    } else {
        current_slot / SLOTS_PER_EPOCH
    };
    let last_claimed_epoch: u64 = last_claimed_slot.into() / SLOTS_PER_EPOCH;
    let required_epochs = current_epoch.saturating_sub(last_claimed_epoch);

    let rent_per_epoch = rent_curve_per_epoch(min_rent, rent_per_byte, bytes);
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
#[allow(clippy::too_many_arguments)]
pub fn calculate_close_lamports(
    bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    min_rent: u64,
    rent_per_byte: u64,
    full_compression_incentive: u64,
) -> (u64, u64) {
    let (_, _, _, unutilized_lamports) = calculate_rent_with_current_epoch(
        bytes,
        current_slot,
        current_lamports,
        last_claimed_slot,
        rent_exemption_lamports,
        min_rent,
        rent_per_byte,
        full_compression_incentive,
    );
    (current_lamports - unutilized_lamports, unutilized_lamports)
}

/// Calculate how many lamports can be claimed for past completed epochs.
/// Only rent for fully completed epochs can be claimed, not the current ongoing epoch.
/// Returns None if the account is compressible (should be compressed instead of claimed).
/// Compression costs are never claimable.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn claimable_lamports(
    bytes: u64,
    current_slot: u64,
    current_lamports: u64,
    last_claimed_slot: impl ZeroCopyNumTrait,
    rent_exemption_lamports: u64,
    min_rent: u64,
    rent_per_byte: u64,
    full_compression_incentive: u64,
) -> Option<u64> {
    // First check if account is compressible
    let (is_compressible, _) = calculate_rent_and_balance(
        bytes,
        current_slot,
        current_lamports,
        last_claimed_slot,
        rent_exemption_lamports,
        min_rent,
        rent_per_byte,
        full_compression_incentive,
    );

    if is_compressible {
        // Account should be compressed, not claimed
        return None;
    }

    // Use calculate_rent_inner with INCLUDE_CURRENT=false to get only completed epochs
    let (completed_epochs, rent_per_epoch, _, _) = calculate_rent_inner::<false>(
        bytes,
        current_slot,
        current_lamports,
        last_claimed_slot,
        rent_exemption_lamports,
        min_rent,
        rent_per_byte,
        full_compression_incentive,
    );

    // Calculate how much rent we can claim for completed epochs
    Some(completed_epochs * rent_per_epoch)
}

impl ZCompressibleExtensionMut<'_> {
    /// Claim rent for past completed epochs and update the extension state.
    /// Returns the amount of lamports claimed, or None if account should be compressed.
    pub fn claim(
        &mut self,
        bytes: u64,
        current_slot: u64,
        current_lamports: u64,
        rent_exemption_lamports: u64,
    ) -> Result<Option<u64>, AccountError> {
        let min_rent: u64 = self.rent_config.min_rent.into();
        let rent_per_byte: u64 = self.rent_config.rent_per_byte.into();
        let full_compression_incentive: u64 = self.rent_config.full_compression_incentive.into();

        // Calculate claimable amount
        let claimed = claimable_lamports(
            bytes,
            current_slot,
            current_lamports,
            self.last_claimed_slot,
            rent_exemption_lamports,
            min_rent,
            rent_per_byte,
            full_compression_incentive,
        );

        if let Some(claimed_amount) = claimed {
            if claimed_amount > 0 {
                let (completed_epochs, _, _, _) = calculate_rent_inner::<false>(
                    bytes,
                    current_slot,
                    current_lamports,
                    self.last_claimed_slot,
                    rent_exemption_lamports,
                    min_rent,
                    rent_per_byte,
                    full_compression_incentive,
                );

                self.last_claimed_slot += U64::from(completed_epochs * SLOTS_PER_EPOCH);
                Ok(Some(claimed_amount))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod test {
    use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};

    use super::*;

    const TEST_BYTES: u64 = 261;
    const RENT_PER_EPOCH: u64 = 3830;
    const FULL_COMPRESSION_COSTS: u64 = (COMPRESSION_COST + COMPRESSION_INCENTIVE) as u64;

    fn test_rent_config() -> RentConfig {
        RentConfig::default()
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
                        + FULL_COMPRESSION_COSTS,
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
                        + (RENT_PER_EPOCH * 2)
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
                        + FULL_COMPRESSION_COSTS,
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
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
                    current_lamports: get_rent_exemption_lamports(TEST_BYTES).unwrap()
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
        let rent_exemption_lamports = get_rent_exemption_lamports(TEST_BYTES).unwrap();
        let min_rent = rent_config.min_rent as u64;
        let rent_per_byte = rent_config.rent_per_byte as u64;
        let full_compression_incentive = rent_config.full_compression_incentive as u64;

        for test_case in test_cases {
            let (is_compressible, deficit) = calculate_rent_and_balance(
                TEST_BYTES,
                test_case.input.current_slot,
                test_case.input.current_lamports,
                test_case.input.last_claimed_slot,
                rent_exemption_lamports,
                min_rent,
                rent_per_byte,
                full_compression_incentive,
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

    #[test]
    fn test_claimable_lamports() {
        // Test claiming rent for completed epochs only
        let rent_config = test_rent_config();
        let rent_exemption_lamports = get_rent_exemption_lamports(TEST_BYTES).unwrap();
        let min_rent = rent_config.min_rent as u64;
        let rent_per_byte = rent_config.rent_per_byte as u64;
        let full_compression_incentive = rent_config.full_compression_incentive as u64;

        // Scenario 1: No completed epochs (same epoch)
        let claimable = claimable_lamports(
            TEST_BYTES,
            100, // Slot in epoch 0
            rent_exemption_lamports + RENT_PER_EPOCH + FULL_COMPRESSION_COSTS,
            0, // Last claimed in epoch 0
            rent_exemption_lamports,
            min_rent,
            rent_per_byte,
            full_compression_incentive,
        );
        assert_eq!(claimable, Some(0), "Should not claim in same epoch");

        // Scenario 2: One completed epoch
        let claimable = claimable_lamports(
            TEST_BYTES,
            SLOTS_PER_EPOCH + 100, // Slot in epoch 1
            rent_exemption_lamports + RENT_PER_EPOCH * 2 + FULL_COMPRESSION_COSTS,
            0, // Last claimed in epoch 0
            rent_exemption_lamports,
            min_rent,
            rent_per_byte,
            full_compression_incentive,
        );
        assert_eq!(
            claimable,
            Some(3830),
            "Should not claim for current epoch 1 when last claimed was epoch 0"
        );

        // Scenario 3: Two epochs passed, one claimable
        let claimable = claimable_lamports(
            TEST_BYTES,
            SLOTS_PER_EPOCH * 2 + 100, // Slot in epoch 2
            rent_exemption_lamports + (RENT_PER_EPOCH * 3) + FULL_COMPRESSION_COSTS,
            0, // Last claimed in epoch 0
            rent_exemption_lamports,
            min_rent,
            rent_per_byte,
            full_compression_incentive,
        );
        assert_eq!(
            claimable,
            Some(2 * RENT_PER_EPOCH),
            "Should claim rent for epoch 1 only"
        );

        // Scenario 4: Multiple completed epochs
        let claimable = claimable_lamports(
            TEST_BYTES,
            SLOTS_PER_EPOCH * 4 + 100, // Slot in epoch 5
            rent_exemption_lamports + (RENT_PER_EPOCH * 5) + FULL_COMPRESSION_COSTS,
            0, // Last claimed in epoch 0
            rent_exemption_lamports,
            min_rent,
            rent_per_byte,
            full_compression_incentive,
        );
        assert_eq!(
            claimable,
            Some(RENT_PER_EPOCH * 4),
            "Should claim rent for epochs 1-4"
        );

        // Scenario 5: Account is compressible (insufficient rent)
        let claimable = claimable_lamports(
            TEST_BYTES,
            SLOTS_PER_EPOCH * 5 + 100, // Slot in epoch 5
            rent_exemption_lamports + RENT_PER_EPOCH + FULL_COMPRESSION_COSTS, // Only 1 epoch of rent available
            0, // Last claimed in epoch 0
            rent_exemption_lamports,
            min_rent,
            rent_per_byte,
            full_compression_incentive,
        );
        assert_eq!(claimable, None, "Should only claim available rent");
    }

    #[test]
    fn test_get_last_paid_epoch() {
        // Test the get_last_paid_epoch function with various scenarios

        // Test case 1: Account created in epoch 0 with 3 epochs of rent
        let extension = CompressibleExtension {
            version: 1,
            rent_authority: [0u8; 32],
            rent_recipient: [0u8; 32],
            last_claimed_slot: 0, // Created in epoch 0
            write_top_up_lamports: 0,
            rent_config: test_rent_config(),
        };

        // Has 3 epochs of rent
        let current_lamports = get_rent_exemption_lamports(TEST_BYTES).unwrap()
            + (RENT_PER_EPOCH * 3)
            + FULL_COMPRESSION_COSTS;
        let rent_exemption_lamports = get_rent_exemption_lamports(TEST_BYTES).unwrap();
        let last_paid = extension
            .get_last_paid_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();

        assert_eq!(
            last_paid, 2,
            "Should be paid through epoch 2 (epochs 0, 1, 2)"
        );

        // Test case 2: Account created in epoch 1 with 2 epochs of rent
        let extension = CompressibleExtension {
            version: 1,
            rent_authority: [0u8; 32],
            rent_recipient: [0u8; 32],
            last_claimed_slot: SLOTS_PER_EPOCH, // Created in epoch 1
            write_top_up_lamports: 0,
            rent_config: test_rent_config(),
        };

        let current_lamports = get_rent_exemption_lamports(TEST_BYTES).unwrap()
            + (RENT_PER_EPOCH * 2)
            + FULL_COMPRESSION_COSTS;
        let last_paid = extension
            .get_last_paid_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();
        assert_eq!(last_paid, 2, "Should be paid through epoch 2 (epochs 1, 2)");

        // Test case 3: Account with no rent paid (immediately compressible)
        let extension = CompressibleExtension {
            version: 1,
            rent_authority: [0u8; 32],
            rent_recipient: [0u8; 32],
            last_claimed_slot: SLOTS_PER_EPOCH * 2, // Created in epoch 2
            write_top_up_lamports: 0,
            rent_config: test_rent_config(),
        };

        let current_lamports =
            get_rent_exemption_lamports(TEST_BYTES).unwrap() + FULL_COMPRESSION_COSTS; // No rent paid
        let last_paid = extension
            .get_last_paid_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();
        assert_eq!(
            last_paid, 1,
            "With no rent, last paid epoch should be epoch 1 (before creation)"
        );

        // Test case 4: Account with 1 epoch of rent
        let extension = CompressibleExtension {
            version: 1,
            rent_authority: [0u8; 32],
            rent_recipient: [0u8; 32],
            last_claimed_slot: 0,
            write_top_up_lamports: 0,
            rent_config: test_rent_config(),
        };

        let current_lamports = get_rent_exemption_lamports(TEST_BYTES).unwrap()
            + RENT_PER_EPOCH
            + FULL_COMPRESSION_COSTS;
        let last_paid = extension
            .get_last_paid_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();
        assert_eq!(last_paid, 0, "Should be paid through epoch 0 only");

        // Test case 5: Account with massive prepayment (100 epochs)
        let extension = CompressibleExtension {
            version: 1,
            rent_authority: [0u8; 32],
            rent_recipient: [0u8; 32],
            last_claimed_slot: SLOTS_PER_EPOCH * 5, // Created in epoch 5
            write_top_up_lamports: 0,
            rent_config: test_rent_config(),
        };

        let current_lamports = get_rent_exemption_lamports(TEST_BYTES).unwrap()
            + (RENT_PER_EPOCH * 100)
            + FULL_COMPRESSION_COSTS;
        let last_paid = extension
            .get_last_paid_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();
        assert_eq!(
            last_paid, 104,
            "Should be paid through epoch 104 (5 + 100 - 1)"
        );

        // Test case 6: Account with partial epoch payment (1.5 epochs)
        let extension = CompressibleExtension {
            version: 1,
            rent_authority: [0u8; 32],
            rent_recipient: [0u8; 32],
            last_claimed_slot: 0,
            write_top_up_lamports: 0,
            rent_config: test_rent_config(),
        };

        let current_lamports = get_rent_exemption_lamports(TEST_BYTES).unwrap()
            + (RENT_PER_EPOCH * 3 / 2)
            + FULL_COMPRESSION_COSTS; // 1.5 epochs
        let last_paid = extension
            .get_last_paid_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();
        assert_eq!(
            last_paid, 0,
            "Partial epochs round down, so only epoch 0 is paid"
        );

        // Test case 7: Zero-copy version test
        let extension_data = CompressibleExtension {
            version: 1,
            rent_authority: [1; 32],
            rent_recipient: [2; 32],
            last_claimed_slot: SLOTS_PER_EPOCH * 3, // Epoch 3
            write_top_up_lamports: 100,
            rent_config: test_rent_config(),
        };

        let extension_bytes = extension_data.try_to_vec().unwrap();
        let (z_extension, _) = CompressibleExtension::zero_copy_at(&extension_bytes)
            .expect("Failed to create zero-copy extension");

        let current_lamports = get_rent_exemption_lamports(TEST_BYTES).unwrap()
            + (RENT_PER_EPOCH * 5)
            + FULL_COMPRESSION_COSTS;
        let last_paid = z_extension
            .get_last_paid_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();
        assert_eq!(last_paid, 7, "Should be paid through epoch 7 (3 + 5 - 1)");
    }

    #[test]
    fn test_claim_method() {
        // Test the claim method updates state correctly
        let extension_data = CompressibleExtension {
            version: 1,
            rent_authority: [1; 32],
            rent_recipient: [2; 32],
            last_claimed_slot: 0,
            write_top_up_lamports: 0,
            rent_config: test_rent_config(),
        };

        let mut extension_bytes = extension_data.try_to_vec().unwrap();
        let (mut z_extension, _) = CompressibleExtension::zero_copy_at_mut(&mut extension_bytes)
            .expect("Failed to create zero-copy extension");

        // Claim in epoch 2 (should claim for epochs 0 and 1)
        let current_slot = SLOTS_PER_EPOCH * 2 + 100;
        let current_lamports = get_rent_exemption_lamports(TEST_BYTES).unwrap()
            + RENT_PER_EPOCH * 3
            + FULL_COMPRESSION_COSTS; // Need 3 epochs: 0, 1, and current 2

        let claimed = z_extension
            .claim(
                TEST_BYTES,
                current_slot,
                current_lamports,
                get_rent_exemption_lamports(TEST_BYTES).unwrap(),
            )
            .unwrap();
        assert_eq!(
            claimed.unwrap(),
            RENT_PER_EPOCH * 2,
            "Should claim rent for epochs 0 and 1"
        );
        // assert_eq!(
        //     u64::from(*z_extension.last_claimed_slot),
        //     SLOTS_PER_EPOCH * 2 - 1, // Last slot of epoch 1 (last completed epoch)
        //     "Should update to last slot of last completed epoch"
        // );
        // assert_eq!(
        //     u64::from(*z_extension.rent_exemption_lamports_balance),
        //     RENT_PER_EPOCH,
        //     "Should update lamports after claim"
        // );
        // Try claiming again in same epoch (should return 0)
        let claimed_again = z_extension
            .claim(
                TEST_BYTES,
                current_slot,
                current_lamports - claimed.unwrap_or(0),
                get_rent_exemption_lamports(TEST_BYTES).unwrap(),
            )
            .unwrap();
        assert_eq!(claimed_again, None, "Should not claim again in same epoch");
        // Cannot claim the third epoch because the account is now compressible
        {
            let current_slot = SLOTS_PER_EPOCH * 3 + 100;
            let current_lamports = current_lamports - claimed.unwrap_or(0) + RENT_PER_EPOCH - 1;
            let claimed_again_in_third_epoch = z_extension
                .claim(
                    TEST_BYTES,
                    current_slot,
                    current_lamports,
                    get_rent_exemption_lamports(TEST_BYTES).unwrap(),
                )
                .unwrap();
            assert_eq!(
                claimed_again_in_third_epoch, None,
                "Cannot claim the third epoch because the account is now compressible"
            );
        }
        // Can claim after top up for one more epoch
        {
            let current_slot = SLOTS_PER_EPOCH * 3 + 100;
            let current_lamports = current_lamports - claimed.unwrap_or(0) + RENT_PER_EPOCH;
            let claimed_again_in_third_epoch = z_extension
                .claim(
                    TEST_BYTES,
                    current_slot,
                    current_lamports,
                    get_rent_exemption_lamports(TEST_BYTES).unwrap(),
                )
                .unwrap();
            assert_eq!(
                claimed_again_in_third_epoch,
                Some(RENT_PER_EPOCH),
                "Can claim the third epoch after top up"
            );
        }
        // Can claim for epoch four with top up for 10 more epochs
        {
            let current_slot = SLOTS_PER_EPOCH * 4 + 100;
            let current_lamports = current_lamports - claimed.unwrap_or(0) + 10 * RENT_PER_EPOCH;
            let claimed_again_in_third_epoch = z_extension
                .claim(
                    TEST_BYTES,
                    current_slot,
                    current_lamports,
                    get_rent_exemption_lamports(TEST_BYTES).unwrap(),
                )
                .unwrap();
            assert_eq!(
                claimed_again_in_third_epoch,
                Some(RENT_PER_EPOCH),
                "Can claim for epoch four with sufficient top up"
            );
        }
    }
}
