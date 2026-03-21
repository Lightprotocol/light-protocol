use crate::errors::MerkleTreeMetadataError;

/// Cap on lamports reimbursed to a forester per tree operation.
pub const FORESTER_REIMBURSEMENT_CAP: u64 = 5000;

/// Hardcoded rent exemption based on Solana's rent formula at deployment time.
/// Formula: `(data_len + ACCOUNT_STORAGE_OVERHEAD) * LAMPORTS_PER_BYTE_YEAR * EXEMPTION_THRESHOLD`
/// = `(data_len + 128) * 3480 * 2`
///
/// Hardcoded rather than queried via Sysvar because Solana's rent rate may
/// change after trees are initialized, and existing trees must retain correct reserves.
pub fn hardcoded_rent_exemption(data_len: u64) -> Option<u64> {
    const LAMPORTS_PER_BYTE: u64 = 6960; // 3480 * 2
    const ACCOUNT_STORAGE_OVERHEAD: u64 = 128;
    data_len
        .checked_add(ACCOUNT_STORAGE_OVERHEAD)?
        .checked_mul(LAMPORTS_PER_BYTE)
}

/// Computes excess lamports claimable from a tree/queue account.
///
/// Formula: `account_lamports - rent_exemption - rollover_fee * (capacity - next_index + 1)`
///
/// The first leaf (index 0) does not pay a rollover fee, so:
/// - paid fees = `next_index - 1`
/// - remaining unfunded = `capacity - next_index + 1`
///
/// Returns `None` if there is no excess (underflow in any step).
pub fn compute_claimable_excess(
    account_lamports: u64,
    rent_exemption: u64,
    rollover_fee: u64,
    capacity: u64,
    next_index: u64,
) -> Option<u64> {
    let remaining = capacity.checked_sub(next_index)?.checked_add(1)?;
    let reserved_rollover = rollover_fee.checked_mul(remaining)?;
    account_lamports
        .checked_sub(rent_exemption)?
        .checked_sub(reserved_rollover)
}

pub fn compute_rollover_fee(
    rollover_threshold: u64,
    tree_height: u32,
    rent: u64,
) -> Result<u64, MerkleTreeMetadataError> {
    let number_of_transactions = 1 << tree_height;
    if rollover_threshold > 100 {
        return Err(MerkleTreeMetadataError::InvalidRolloverThreshold);
    }
    if rollover_threshold == 0 {
        return Ok(rent);
    }
    // rent / (total_number_of_leaves * (rollover_threshold / 100))
    // (with ceil division)
    Ok((rent * 100).div_ceil(number_of_transactions * rollover_threshold))
}

#[test]
fn test_compute_rollover_fee() {
    let rollover_threshold = 100;
    let tree_height = 26;
    let rent = 1392890880;
    let total_number_of_leaves = 1 << tree_height;

    let fee = compute_rollover_fee(rollover_threshold, tree_height, rent).unwrap();
    // assert_ne!(fee, 0u64);
    assert!((fee + 1) * (total_number_of_leaves * 100 / rollover_threshold) > rent);

    let rollover_threshold = 50;
    let fee = compute_rollover_fee(rollover_threshold, tree_height, rent).unwrap();
    assert!((fee + 1) * (total_number_of_leaves * 100 / rollover_threshold) > rent);
    let rollover_threshold: u64 = 95;

    let fee = compute_rollover_fee(rollover_threshold, tree_height, rent).unwrap();
    assert!((fee + 1) * (total_number_of_leaves * 100 / rollover_threshold) > rent);
}

#[test]
fn test_concurrent_tree_compute_rollover_fee() {
    let merkle_tree_lamports = 9496836480;
    let queue_lamports = 2293180800;
    let cpi_context_lamports = 143487360;
    let rollover_threshold = 95;
    let height = 26;
    let rollover_fee = compute_rollover_fee(
        rollover_threshold,
        height,
        merkle_tree_lamports + cpi_context_lamports,
    )
    .unwrap()
        + compute_rollover_fee(rollover_threshold, height, queue_lamports).unwrap();
    let lifetime_lamports = rollover_fee * ((2u64.pow(height)) as f64 * 0.95) as u64;
    println!("rollover_fee: {}", rollover_fee);
    println!("lifetime_lamports: {}", lifetime_lamports);
    println!(
        "lifetime_lamports < total lamports: {}",
        lifetime_lamports > merkle_tree_lamports + queue_lamports + cpi_context_lamports
    );
    println!(
        "lifetime_lamports - total lamports: {}",
        lifetime_lamports - (merkle_tree_lamports + queue_lamports + cpi_context_lamports)
    );
    assert!(lifetime_lamports > (merkle_tree_lamports + queue_lamports + cpi_context_lamports));
}

#[test]
fn test_address_tree_compute_rollover_fee() {
    let merkle_tree_lamports = 9639711360;
    let queue_lamports = 2293180800;
    let rollover_threshold = 95;
    let height = 26;
    let rollover_fee = compute_rollover_fee(rollover_threshold, height, merkle_tree_lamports)
        .unwrap()
        + compute_rollover_fee(rollover_threshold, height, queue_lamports).unwrap();
    let lifetime_lamports = rollover_fee * ((2u64.pow(height)) as f64 * 0.95) as u64;
    println!("rollover_fee: {}", rollover_fee);
    println!("lifetime_lamports: {}", lifetime_lamports);
    println!(
        "lifetime_lamports < total lamports: {}",
        lifetime_lamports > merkle_tree_lamports + queue_lamports
    );
    println!(
        "lifetime_lamports - total lamports: {}",
        lifetime_lamports - (merkle_tree_lamports + queue_lamports)
    );
    assert!(lifetime_lamports > (merkle_tree_lamports + queue_lamports));
}

#[test]
fn test_hardcoded_rent_exemption() {
    // 8-byte discriminator account: (8 + 128) * 6960 = 946_560
    assert_eq!(hardcoded_rent_exemption(8), Some(946_560));
    // 0-byte account: (0 + 128) * 6960 = 890_880
    assert_eq!(hardcoded_rent_exemption(0), Some(890_880));
    // Large account
    let large = 10_000_000u64;
    assert_eq!(
        hardcoded_rent_exemption(large),
        Some((large + 128) * 6960)
    );
    // Overflow: u64::MAX
    assert_eq!(hardcoded_rent_exemption(u64::MAX), None);
}

#[test]
fn test_compute_claimable_excess_normal() {
    let rent = 1_000_000u64;
    let rollover_fee = 100u64;
    let capacity = 1000u64;
    let next_index = 500u64;
    // remaining = 1000 - 500 + 1 = 501
    // reserved = 100 * 501 = 50_100
    // excess = lamports - rent - reserved
    let lamports = rent + 50_100 + 5_000;
    assert_eq!(
        compute_claimable_excess(lamports, rent, rollover_fee, capacity, next_index),
        Some(5_000)
    );
}

#[test]
fn test_compute_claimable_excess_zero() {
    let rent = 1_000_000u64;
    let rollover_fee = 100u64;
    let capacity = 1000u64;
    let next_index = 500u64;
    let remaining = capacity - next_index + 1;
    let lamports = rent + rollover_fee * remaining;
    assert_eq!(
        compute_claimable_excess(lamports, rent, rollover_fee, capacity, next_index),
        Some(0)
    );
}

#[test]
fn test_compute_claimable_excess_negative_underflow() {
    let rent = 1_000_000u64;
    let rollover_fee = 100u64;
    let capacity = 1000u64;
    let next_index = 500u64;
    // Lamports less than rent alone
    let lamports = rent - 1;
    assert_eq!(
        compute_claimable_excess(lamports, rent, rollover_fee, capacity, next_index),
        None
    );
    // Lamports between rent and rent + reserved
    let remaining = capacity - next_index + 1;
    let lamports = rent + rollover_fee * remaining - 1;
    assert_eq!(
        compute_claimable_excess(lamports, rent, rollover_fee, capacity, next_index),
        None
    );
}

#[test]
fn test_compute_claimable_excess_next_index_zero() {
    // First leaf at index 0 does not pay rollover fee.
    // remaining = capacity - 0 + 1 = capacity + 1
    let rent = 1_000_000u64;
    let rollover_fee = 100u64;
    let capacity = 1000u64;
    let next_index = 0u64;
    let remaining = capacity + 1; // 1001
    let lamports = rent + rollover_fee * remaining + 42;
    assert_eq!(
        compute_claimable_excess(lamports, rent, rollover_fee, capacity, next_index),
        Some(42)
    );
}

#[test]
fn test_compute_claimable_excess_next_index_at_capacity() {
    // Tree is full: next_index == capacity
    // remaining = capacity - capacity + 1 = 1
    let rent = 1_000_000u64;
    let rollover_fee = 100u64;
    let capacity = 1000u64;
    let next_index = capacity;
    let remaining = 1; // one slot reserved
    let lamports = rent + rollover_fee * remaining + 99;
    assert_eq!(
        compute_claimable_excess(lamports, rent, rollover_fee, capacity, next_index),
        Some(99)
    );
}

#[test]
fn test_compute_claimable_excess_next_index_exceeds_capacity() {
    // Edge case: next_index > capacity should return None (checked_sub underflow)
    let rent = 1_000_000u64;
    let rollover_fee = 100u64;
    let capacity = 1000u64;
    let next_index = 1001u64;
    let lamports = rent + 999_999;
    assert_eq!(
        compute_claimable_excess(lamports, rent, rollover_fee, capacity, next_index),
        None
    );
}

#[test]
fn test_compute_claimable_excess_zero_rollover_fee() {
    // Accounts with rollover_fee == 0 (e.g. V1 nullifier queues)
    let rent = 1_000_000u64;
    let rollover_fee = 0u64;
    let capacity = 1000u64;
    let next_index = 500u64;
    let lamports = rent + 7777;
    // reserved = 0 * 501 = 0
    assert_eq!(
        compute_claimable_excess(lamports, rent, rollover_fee, capacity, next_index),
        Some(7777)
    );
}
