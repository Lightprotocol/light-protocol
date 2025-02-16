use crate::errors::MerkleTreeMetadataError;

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
