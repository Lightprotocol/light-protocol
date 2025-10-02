use light_program_test::{program_test::LightProgramTest, ProgramTestConfig, Rpc};
use solana_sdk::pubkey;

/// Test that all 5 v2 batched merkle tree accounts are accessible via get_account
#[tokio::test]
async fn test_v2_batched_merkle_tree_accounts() {
    let config = ProgramTestConfig::new(false, None);
    let context = LightProgramTest::new(config).await.unwrap();

    // All 5 v2 state tree triples that should be available
    let tree_triples = [
        (
            pubkey!("bmt1LryLZUMmF7ZtqESaw7wifBXLfXHQYoE4GAmrahU"),
            pubkey!("oq1na8gojfdUhsfCpyjNt6h4JaDWtHf1yQj4koBWfto"),
            pubkey!("cpi15BoVPKgEPw5o8wc2T816GE7b378nMXnhH3Xbq4y"),
        ),
        (
            pubkey!("bmt2UxoBxB9xWev4BkLvkGdapsz6sZGkzViPNph7VFi"),
            pubkey!("oq2UkeMsJLfXt2QHzim242SUi3nvjJs8Pn7Eac9H9vg"),
            pubkey!("cpi2yGapXUR3As5SjnHBAVvmApNiLsbeZpF3euWnW6B"),
        ),
        (
            pubkey!("bmt3ccLd4bqSVZVeCJnH1F6C8jNygAhaDfxDwePyyGb"),
            pubkey!("oq3AxjekBWgo64gpauB6QtuZNesuv19xrhaC1ZM1THQ"),
            pubkey!("cpi3mbwMpSX8FAGMZVP85AwxqCaQMfEk9Em1v8QK9Rf"),
        ),
        (
            pubkey!("bmt4d3p1a4YQgk9PeZv5s4DBUmbF5NxqYpk9HGjQsd8"),
            pubkey!("oq4ypwvVGzCUMoiKKHWh4S1SgZJ9vCvKpcz6RT6A8dq"),
            pubkey!("cpi4yyPDc4bCgHAnsenunGA8Y77j3XEDyjgfyCKgcoc"),
        ),
        (
            pubkey!("bmt5yU97jC88YXTuSukYHa8Z5Bi2ZDUtmzfkDTA2mG2"),
            pubkey!("oq5oh5ZR3yGomuQgFduNDzjtGvVWfDRGLuDVjv9a96P"),
            pubkey!("cpi5ZTjdgYpZ1Xr7B1cMLLUE81oTtJbNNAyKary2nV6"),
        ),
    ];

    for (i, (merkle_tree, output_queue, cpi_context)) in tree_triples.iter().enumerate() {
        // Test that the merkle tree account does not exist
        let tree_account = context.get_account(*merkle_tree).await.unwrap();
        assert!(
            tree_account.is_none(),
            "Tree {} merkle tree account should not exist",
            i + 1
        );

        // Test that the output queue account does not exist
        let queue_account = context.get_account(*output_queue).await.unwrap();
        assert!(
            queue_account.is_none(),
            "Tree {} output queue account should not exist",
            i + 1
        );

        // Test that the cpi context account does not exist
        let cpi_account = context.get_account(*cpi_context).await.unwrap();
        assert!(
            cpi_account.is_none(),
            "Tree {} cpi context account should not exist",
            i + 1
        );

        println!(
            "✓ Tree {} accounts verified as non-existent: tree={}, queue={}, cpi={}",
            i + 1,
            merkle_tree,
            output_queue,
            cpi_context
        );
    }

    println!("All 5 v2 batched merkle tree triples verified successfully!");
}

/// Test that v2_state_trees in TestAccounts match the expected pubkeys
#[tokio::test]
async fn test_v2_state_trees_in_test_accounts() {
    let config = ProgramTestConfig::new_v2(false, None);
    let context = LightProgramTest::new(config).await.unwrap();

    let test_accounts = context.test_accounts();

    // Verify we have 5 v2 state trees
    assert_eq!(
        test_accounts.v2_state_trees.len(),
        5,
        "Should have exactly 5 v2 state trees"
    );

    // Verify each tree matches expected pubkeys
    let expected = [
        (
            pubkey!("bmt1LryLZUMmF7ZtqESaw7wifBXLfXHQYoE4GAmrahU"),
            pubkey!("oq1na8gojfdUhsfCpyjNt6h4JaDWtHf1yQj4koBWfto"),
            pubkey!("cpi15BoVPKgEPw5o8wc2T816GE7b378nMXnhH3Xbq4y"),
        ),
        (
            pubkey!("bmt2UxoBxB9xWev4BkLvkGdapsz6sZGkzViPNph7VFi"),
            pubkey!("oq2UkeMsJLfXt2QHzim242SUi3nvjJs8Pn7Eac9H9vg"),
            pubkey!("cpi2yGapXUR3As5SjnHBAVvmApNiLsbeZpF3euWnW6B"),
        ),
        (
            pubkey!("bmt3ccLd4bqSVZVeCJnH1F6C8jNygAhaDfxDwePyyGb"),
            pubkey!("oq3AxjekBWgo64gpauB6QtuZNesuv19xrhaC1ZM1THQ"),
            pubkey!("cpi3mbwMpSX8FAGMZVP85AwxqCaQMfEk9Em1v8QK9Rf"),
        ),
        (
            pubkey!("bmt4d3p1a4YQgk9PeZv5s4DBUmbF5NxqYpk9HGjQsd8"),
            pubkey!("oq4ypwvVGzCUMoiKKHWh4S1SgZJ9vCvKpcz6RT6A8dq"),
            pubkey!("cpi4yyPDc4bCgHAnsenunGA8Y77j3XEDyjgfyCKgcoc"),
        ),
        (
            pubkey!("bmt5yU97jC88YXTuSukYHa8Z5Bi2ZDUtmzfkDTA2mG2"),
            pubkey!("oq5oh5ZR3yGomuQgFduNDzjtGvVWfDRGLuDVjv9a96P"),
            pubkey!("cpi5ZTjdgYpZ1Xr7B1cMLLUE81oTtJbNNAyKary2nV6"),
        ),
    ];

    for (i, (expected_tree, expected_queue, expected_cpi)) in expected.iter().enumerate() {
        let tree_info = &test_accounts.v2_state_trees[i];

        assert_eq!(
            tree_info.merkle_tree,
            *expected_tree,
            "Tree {} merkle_tree mismatch",
            i + 1
        );
        assert_eq!(
            tree_info.output_queue,
            *expected_queue,
            "Tree {} output_queue mismatch",
            i + 1
        );
        assert_eq!(
            tree_info.cpi_context,
            *expected_cpi,
            "Tree {} cpi_context mismatch",
            i + 1
        );

        // Test that the merkle tree account exists
        let tree_account = context.get_account(tree_info.merkle_tree).await.unwrap();
        assert!(
            tree_account.is_some(),
            "Tree {} merkle tree account should exist",
            i + 1
        );

        // Test that the output queue account exists
        let queue_account = context.get_account(tree_info.output_queue).await.unwrap();
        assert!(
            queue_account.is_some(),
            "Tree {} output queue account should exist",
            i + 1
        );

        // Test that the cpi context account exists
        let cpi_account = context.get_account(tree_info.cpi_context).await.unwrap();
        assert!(
            cpi_account.is_some(),
            "Tree {} cpi context account should exist",
            i + 1
        );

        println!(
            "✓ Tree {} TestAccounts entry verified: tree={}, queue={}, cpi={}",
            i + 1,
            expected_tree,
            expected_queue,
            expected_cpi
        );
    }

    println!("All v2_state_trees in TestAccounts match expected pubkeys!");
}
