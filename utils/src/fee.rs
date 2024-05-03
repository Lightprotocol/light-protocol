use crate::UtilsError;

pub fn compute_rollover_fee(
    rollover_threshold: u64,
    tree_height: u32,
    rent: u64,
) -> Result<u64, UtilsError> {
    let number_of_transactions = 1 << tree_height;
    if rollover_threshold > 100 {
        return Err(UtilsError::InvalidRolloverThreshold);
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
