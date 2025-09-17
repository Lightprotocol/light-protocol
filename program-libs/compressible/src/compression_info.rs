use aligned_sized::aligned_sized;
use bytemuck::{Pod, Zeroable};
use light_zero_copy::{ZeroCopy, ZeroCopyMut};
use zerocopy::U64;

use crate::{
    error::CompressibleError,
    rent::{
        calculate_rent_and_balance, calculate_rent_inner, claimable_lamports, get_last_paid_epoch,
        get_rent_exemption_lamports, RentConfig, SLOTS_PER_EPOCH,
    },
    AnchorDeserialize, AnchorSerialize,
};

/// Compressible extension for ctoken accounts.
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
    Pod,
    Zeroable,
)]
#[repr(C)]
#[aligned_sized]
pub struct CompressionInfo {
    pub config_account_version: u16, // config_account_version 0 is uninitialized, default is 1
    /// Compress to account pubkey instead of account owner.
    pub compress_to_pubkey: u8,
    /// Version of the compressed token account when ctoken account is
    /// compressed and closed. (The account_version specifies the hashing scheme.)
    pub account_version: u8,
    /// Lamports amount the account is topped up with at every write
    /// by the fee payer.
    pub write_top_up_lamports: u32,
    /// Authority that can compress and close the account.
    pub rent_authority: [u8; 32],
    /// Recipient for rent exemption lamports up on account closure.
    pub rent_recipient: [u8; 32],
    /// Last slot rent was claimed from this account.
    pub last_claimed_slot: u64,
    /// Rent function parameters,
    /// used to calculate whether the account is compressible.
    pub rent_config: RentConfig,
}

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
            ) -> Result<(bool, u64), CompressibleError> {
                let rent_exemption_lamports = get_rent_exemption_lamports(bytes)?;
                let min_rent: u64 = self.rent_config.min_rent.into();
                let lamports_per_byte_per_epoch: u64 =
                    self.rent_config.lamports_per_byte_per_epoch.into();
                let full_compression_incentive: u64 =
                    self.rent_config.full_compression_incentive.into();

                Ok(calculate_rent_and_balance(
                    bytes,
                    current_slot,
                    current_lamports,
                    self.last_claimed_slot,
                    rent_exemption_lamports,
                    min_rent,
                    lamports_per_byte_per_epoch,
                    full_compression_incentive,
                ))
            }

            /// Converts the `compress_to_pubkey` field into a boolean.
            pub fn compress_to_pubkey(&self) -> bool {
                self.compress_to_pubkey != 0
            }
        }
    };
}
impl_is_compressible!(CompressionInfo);
impl_is_compressible!(ZCompressionInfo<'_>);
impl_is_compressible!(ZCompressionInfoMut<'_>);

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
            ) -> Result<u64, CompressibleError> {
                let min_rent: u64 = self.rent_config.min_rent.into();
                let lamports_per_byte_per_epoch: u64 =
                    self.rent_config.lamports_per_byte_per_epoch.into();
                let full_compression_incentive: u64 =
                    self.rent_config.full_compression_incentive.into();

                Ok(get_last_paid_epoch(
                    bytes,
                    current_lamports,
                    self.last_claimed_slot,
                    rent_exemption_lamports,
                    min_rent,
                    lamports_per_byte_per_epoch,
                    full_compression_incentive,
                ))
            }
        }
    };
}

impl_get_last_paid_epoch!(CompressionInfo);
impl_get_last_paid_epoch!(ZCompressionInfo<'_>);
impl_get_last_paid_epoch!(ZCompressionInfoMut<'_>);

impl ZCompressionInfoMut<'_> {
    /// Claim rent for past completed epochs and update the extension state.
    /// Returns the amount of lamports claimed, or None if account should be compressed.
    pub fn claim(
        &mut self,
        bytes: u64,
        current_slot: u64,
        current_lamports: u64,
        rent_exemption_lamports: u64,
    ) -> Result<Option<u64>, CompressibleError> {
        let min_rent: u64 = self.rent_config.min_rent.into();
        let lamports_per_byte_per_epoch: u64 = self.rent_config.lamports_per_byte_per_epoch.into();
        let full_compression_incentive: u64 = self.rent_config.full_compression_incentive.into();

        // Calculate claimable amount
        let claimed = claimable_lamports(
            bytes,
            current_slot,
            current_lamports,
            self.last_claimed_slot,
            rent_exemption_lamports,
            min_rent,
            lamports_per_byte_per_epoch,
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
                    lamports_per_byte_per_epoch,
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
    use crate::rent::{COMPRESSION_COST, COMPRESSION_INCENTIVE, SLOTS_PER_EPOCH};

    const TEST_BYTES: u64 = 261;
    const RENT_PER_EPOCH: u64 = 3830;
    const FULL_COMPRESSION_COSTS: u64 = (COMPRESSION_COST + COMPRESSION_INCENTIVE) as u64;

    fn test_rent_config() -> RentConfig {
        RentConfig::default()
    }

    #[test]
    fn test_claim_method() {
        // Test the claim method updates state correctly
        let extension_data = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            rent_authority: [1; 32],
            rent_recipient: [2; 32],
            last_claimed_slot: 0,
            write_top_up_lamports: 0,
            compress_to_pubkey: 0,
            rent_config: test_rent_config(),
        };

        let mut extension_bytes = extension_data.try_to_vec().unwrap();
        let (mut z_extension, _) = CompressionInfo::zero_copy_at_mut(&mut extension_bytes)
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

    #[test]
    fn test_get_last_paid_epoch() {
        // Test the get_last_paid_epoch function with various scenarios

        // Test case 1: Account created in epoch 0 with 3 epochs of rent
        let extension = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            rent_authority: [0u8; 32],
            rent_recipient: [0u8; 32],
            last_claimed_slot: 0, // Created in epoch 0
            write_top_up_lamports: 0,
            compress_to_pubkey: 0,
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
        let extension = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            rent_authority: [0u8; 32],
            rent_recipient: [0u8; 32],
            last_claimed_slot: SLOTS_PER_EPOCH, // Created in epoch 1
            write_top_up_lamports: 0,
            compress_to_pubkey: 0,
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
        let extension = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            rent_authority: [0u8; 32],
            rent_recipient: [0u8; 32],
            last_claimed_slot: SLOTS_PER_EPOCH * 2, // Created in epoch 2
            write_top_up_lamports: 0,
            compress_to_pubkey: 0,
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
        let extension = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            rent_authority: [0u8; 32],
            rent_recipient: [0u8; 32],
            last_claimed_slot: 0,
            write_top_up_lamports: 0,
            compress_to_pubkey: 0,
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
        let extension = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            rent_authority: [0u8; 32],
            rent_recipient: [0u8; 32],
            last_claimed_slot: SLOTS_PER_EPOCH * 5, // Created in epoch 5
            write_top_up_lamports: 0,
            compress_to_pubkey: 0,
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
        let extension = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            rent_authority: [0u8; 32],
            rent_recipient: [0u8; 32],
            last_claimed_slot: 0,
            write_top_up_lamports: 0,
            compress_to_pubkey: 0,
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

        // Test case 7: Zero-copy config_account_version test
        let extension_data = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            rent_authority: [1; 32],
            rent_recipient: [2; 32],
            last_claimed_slot: SLOTS_PER_EPOCH * 3, // Epoch 3
            write_top_up_lamports: 100,
            compress_to_pubkey: 0,
            rent_config: test_rent_config(),
        };

        let extension_bytes = extension_data.try_to_vec().unwrap();
        let (z_extension, _) = CompressionInfo::zero_copy_at(&extension_bytes)
            .expect("Failed to create zero-copy extension");

        let current_lamports = get_rent_exemption_lamports(TEST_BYTES).unwrap()
            + (RENT_PER_EPOCH * 5)
            + FULL_COMPRESSION_COSTS;
        let last_paid = z_extension
            .get_last_paid_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();
        assert_eq!(last_paid, 7, "Should be paid through epoch 7 (3 + 5 - 1)");
    }
}
