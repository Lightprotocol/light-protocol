#![cfg(test)]
use borsh::BorshSerialize;
use light_compressible::{
    compression_info::CompressionInfo,
    rent::{RentConfig, COMPRESSION_COST, COMPRESSION_INCENTIVE, SLOTS_PER_EPOCH},
};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};

const TEST_BYTES: u64 = 260;
const RENT_PER_EPOCH: u64 = 260 + 128;
const FULL_COMPRESSION_COSTS: u64 = (COMPRESSION_COST + COMPRESSION_INCENTIVE) as u64;

fn test_rent_config() -> RentConfig {
    RentConfig::default()
}

pub fn get_rent_exemption_lamports(_num_bytes: u64) -> u64 {
    2700480
}

#[test]
fn test_claim_method() {
    // Test the claim method updates state correctly
    let rent_exemption = get_rent_exemption_lamports(TEST_BYTES) as u32;
    let extension_data = CompressionInfo {
        account_version: 3,
        config_account_version: 1,
        compression_authority: [1; 32],
        rent_sponsor: [2; 32],
        last_claimed_slot: 0,
        lamports_per_write: 0,
        compress_to_pubkey: 0,
        rent_exemption_paid: rent_exemption,
        _reserved: 0,
        rent_config: test_rent_config(),
    };

    let mut extension_bytes = extension_data.try_to_vec().unwrap();
    let (mut z_extension, _) = CompressionInfo::zero_copy_at_mut(&mut extension_bytes)
        .expect("Failed to create zero-copy extension");

    // Claim in epoch 2 (should claim for epochs 0 and 1)
    let current_slot = SLOTS_PER_EPOCH * 2 + 100;
    let current_lamports =
        get_rent_exemption_lamports(TEST_BYTES) + RENT_PER_EPOCH * 3 + FULL_COMPRESSION_COSTS; // Need 3 epochs: 0, 1, and current 2
    println!("Current lamports: {}", current_lamports);
    println!(
        "get_rent_exemption_lamports: {}",
        get_rent_exemption_lamports(TEST_BYTES)
    );
    let claimed = z_extension
        .claim(TEST_BYTES, current_slot, current_lamports)
        .unwrap();
    assert_eq!(
        claimed.unwrap(),
        RENT_PER_EPOCH * 2,
        "Should claim rent for epochs 0 and 1"
    );
    println!("post 1 assert");
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
        )
        .unwrap();
    assert_eq!(claimed_again, None, "Should not claim again in same epoch");
    // Cannot claim the third epoch because the account is now compressible
    {
        let current_slot = SLOTS_PER_EPOCH * 3 + 100;
        let current_lamports = current_lamports - claimed.unwrap_or(0) + RENT_PER_EPOCH - 1;
        let claimed_again_in_third_epoch = z_extension
            .claim(TEST_BYTES, current_slot, current_lamports)
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
            .claim(TEST_BYTES, current_slot, current_lamports)
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
            .claim(TEST_BYTES, current_slot, current_lamports)
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
    // Test the get_last_funded_epoch function with various scenarios
    let rent_exemption_lamports = get_rent_exemption_lamports(TEST_BYTES);

    // Test case 1: Account created in epoch 0 with 3 epochs of rent
    {
        let extension = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            compression_authority: [0u8; 32],
            rent_sponsor: [0u8; 32],
            last_claimed_slot: 0, // Created in epoch 0
            lamports_per_write: 0,
            compress_to_pubkey: 0,
            rent_exemption_paid: 0,
            _reserved: 0,
            rent_config: test_rent_config(),
        };

        // Has 3 epochs of rent
        let current_lamports =
            get_rent_exemption_lamports(TEST_BYTES) + (RENT_PER_EPOCH * 3) + FULL_COMPRESSION_COSTS;
        let last_funded_epoch = extension
            .get_last_funded_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();

        assert_eq!(
            last_funded_epoch, 2,
            "Should be paid through epoch 2 (epochs 0, 1, 2)"
        );
    }
    // Test case 1: Account created in epoch 0 with 3 epochs of rent
    {
        let extension = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            compression_authority: [0u8; 32],
            rent_sponsor: [0u8; 32],
            last_claimed_slot: SLOTS_PER_EPOCH - 1, // Created in epoch 0
            lamports_per_write: 0,
            compress_to_pubkey: 0,
            rent_exemption_paid: 0,
            _reserved: 0,
            rent_config: test_rent_config(),
        };

        // Has 3 epochs of rent
        let current_lamports =
            get_rent_exemption_lamports(TEST_BYTES) + (RENT_PER_EPOCH * 3) + FULL_COMPRESSION_COSTS;
        let last_funded_epoch = extension
            .get_last_funded_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();

        assert_eq!(
            last_funded_epoch, 2,
            "Should be paid through epoch 2 (epochs 0, 1, 2)"
        );
    }
    // Test case 2: Account created in epoch 1 with 2 epochs of rent
    {
        let extension = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            compression_authority: [0u8; 32],
            rent_sponsor: [0u8; 32],
            last_claimed_slot: SLOTS_PER_EPOCH, // Created in epoch 1
            lamports_per_write: 0,
            compress_to_pubkey: 0,
            rent_exemption_paid: 0,
            _reserved: 0,
            rent_config: test_rent_config(),
        };

        let current_lamports =
            get_rent_exemption_lamports(TEST_BYTES) + (RENT_PER_EPOCH * 2) + FULL_COMPRESSION_COSTS;
        let last_funded_epoch = extension
            .get_last_funded_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();
        assert_eq!(
            last_funded_epoch, 2,
            "Should be paid through epoch 2 (epochs 1, 2)"
        );
    }
    // Test case 3: Account with no rent paid (immediately compressible)
    let extension = CompressionInfo {
        account_version: 3,
        config_account_version: 1,
        compression_authority: [0u8; 32],
        rent_sponsor: [0u8; 32],
        last_claimed_slot: SLOTS_PER_EPOCH * 2, // Created in epoch 2
        lamports_per_write: 0,
        compress_to_pubkey: 0,
        rent_exemption_paid: 0,
        _reserved: 0,
        rent_config: test_rent_config(),
    };

    let current_lamports = get_rent_exemption_lamports(TEST_BYTES) + FULL_COMPRESSION_COSTS; // No rent paid
    let last_funded_epoch = extension
        .get_last_funded_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
        .unwrap();
    assert_eq!(
        last_funded_epoch, 1,
        "With no rent, last paid epoch should be epoch 1 (before creation)"
    );

    // Test case 4: Account with 1 epoch of rent
    {
        let extension = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            compression_authority: [0u8; 32],
            rent_sponsor: [0u8; 32],
            last_claimed_slot: 0,
            lamports_per_write: 0,
            compress_to_pubkey: 0,
            rent_exemption_paid: 0,
            _reserved: 0,
            rent_config: test_rent_config(),
        };

        let current_lamports =
            get_rent_exemption_lamports(TEST_BYTES) + RENT_PER_EPOCH + FULL_COMPRESSION_COSTS;
        let last_funded_epoch = extension
            .get_last_funded_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();
        assert_eq!(last_funded_epoch, 0, "Should be paid through epoch 0 only");
    }
    // Test case 5: Account with massive prepayment (100 epochs)
    {
        let extension = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            compression_authority: [0u8; 32],
            rent_sponsor: [0u8; 32],
            last_claimed_slot: SLOTS_PER_EPOCH * 5, // Created in epoch 5
            lamports_per_write: 0,
            compress_to_pubkey: 0,
            rent_exemption_paid: 0,
            _reserved: 0,
            rent_config: test_rent_config(),
        };

        let current_lamports = get_rent_exemption_lamports(TEST_BYTES)
            + (RENT_PER_EPOCH * 100)
            + FULL_COMPRESSION_COSTS;
        let last_funded_epoch = extension
            .get_last_funded_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();
        assert_eq!(
            last_funded_epoch, 104,
            "Should be paid through epoch 104 (5 + 100 - 1)"
        );
    }
    // Test case 6: Account with partial epoch payment (1.5 epochs)
    {
        let extension = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            compression_authority: [0u8; 32],
            rent_sponsor: [0u8; 32],
            last_claimed_slot: 0,
            lamports_per_write: 0,
            compress_to_pubkey: 0,
            rent_exemption_paid: 0,
            _reserved: 0,
            rent_config: test_rent_config(),
        };

        let current_lamports = get_rent_exemption_lamports(TEST_BYTES)
            + (RENT_PER_EPOCH * 3 / 2)
            + FULL_COMPRESSION_COSTS; // 1.5 epochs
        let last_funded_epoch = extension
            .get_last_funded_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();
        assert_eq!(
            last_funded_epoch, 0,
            "Partial epochs round down, so only epoch 0 is paid"
        );
    }

    // Test case 7: Zero-copy config_account_version test
    {
        let extension_data = CompressionInfo {
            account_version: 3,
            config_account_version: 1,
            compression_authority: [1; 32],
            rent_sponsor: [2; 32],
            last_claimed_slot: SLOTS_PER_EPOCH * 3, // Epoch 3
            lamports_per_write: 100,
            compress_to_pubkey: 0,
            rent_exemption_paid: 0,
            _reserved: 0,
            rent_config: test_rent_config(),
        };

        let extension_bytes = extension_data.try_to_vec().unwrap();
        let (z_extension, _) = CompressionInfo::zero_copy_at(&extension_bytes)
            .expect("Failed to create zero-copy extension");

        let current_lamports =
            get_rent_exemption_lamports(TEST_BYTES) + (RENT_PER_EPOCH * 5) + FULL_COMPRESSION_COSTS;
        let last_funded_epoch = z_extension
            .get_last_funded_epoch(TEST_BYTES, current_lamports, rent_exemption_lamports)
            .unwrap();
        assert_eq!(
            last_funded_epoch, 7,
            "Should be paid through epoch 7 (3 + 5 - 1)"
        );
    }
}
