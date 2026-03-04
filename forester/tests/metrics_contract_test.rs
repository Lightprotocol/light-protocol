//! Integration test: ensures metrics-contract.json stays in sync with src/metrics.rs.
//!
//! Run with: cargo test -p forester --test metrics_contract_test

use std::collections::HashSet;

#[test]
fn metrics_contract_matches_code() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    // --- read contract ---
    let contract_path = format!("{}/metrics-contract.json", manifest_dir);
    let contract_text = std::fs::read_to_string(&contract_path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", contract_path, e));
    let contract: serde_json::Value = serde_json::from_str(&contract_text)
        .unwrap_or_else(|e| panic!("Bad JSON in {}: {}", contract_path, e));

    let contract_set: HashSet<(String, Vec<String>)> = contract["metrics"]
        .as_array()
        .expect("metrics must be an array")
        .iter()
        .map(|m| {
            let name = m["name"]
                .as_str()
                .expect("metric.name must be a string")
                .to_string();
            let labels: Vec<String> = m["labels"]
                .as_array()
                .expect("metric.labels must be an array")
                .iter()
                .map(|l| l.as_str().expect("label must be a string").to_string())
                .collect();
            (name, labels)
        })
        .collect();

    // --- read code descriptors ---
    let code_set: HashSet<(String, Vec<String>)> = forester::metrics::METRIC_DESCRIPTORS
        .iter()
        .map(|(name, labels)| {
            (
                name.to_string(),
                labels.iter().map(|l| l.to_string()).collect(),
            )
        })
        .collect();

    // --- compare ---
    let in_contract_not_code: Vec<_> = contract_set.difference(&code_set).collect();
    let in_code_not_contract: Vec<_> = code_set.difference(&contract_set).collect();

    let mut errors = Vec::new();
    if !in_contract_not_code.is_empty() {
        errors.push(format!(
            "In contract but not in code: {:?}",
            in_contract_not_code
        ));
    }
    if !in_code_not_contract.is_empty() {
        errors.push(format!(
            "In code but not in contract: {:?}",
            in_code_not_contract
        ));
    }
    assert!(errors.is_empty(), "\n{}\n", errors.join("\n"));
}
