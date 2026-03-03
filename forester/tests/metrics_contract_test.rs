//! Integration test: ensures metrics-contract.json stays in sync with src/metrics.rs.
//!
//! Run with: cargo test --test metrics_contract_test

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

    let contract_names: HashSet<String> = contract["metrics"]
        .as_array()
        .expect("metrics must be an array")
        .iter()
        .map(|m| {
            m["name"]
                .as_str()
                .expect("metric.name must be a string")
                .to_string()
        })
        .collect();

    // --- read source ---
    let source_path = format!("{}/src/metrics.rs", manifest_dir);
    let source = std::fs::read_to_string(&source_path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", source_path, e));

    let code_names = extract_metric_names(&source);

    // --- compare ---
    let in_contract_not_code: Vec<_> = contract_names.difference(&code_names).collect();
    let in_code_not_contract: Vec<_> = code_names.difference(&contract_names).collect();

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

/// Scan Rust source for quoted strings that look like forester metric names.
fn extract_metric_names(source: &str) -> HashSet<String> {
    let prefixes = ["forester_", "queue_", "registered_"];
    let mut names = HashSet::new();

    for line in source.lines() {
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'"' {
                i += 1;
                let start = i;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += 1;
                }
                let word = &line[start..i];
                if prefixes.iter().any(|p| word.starts_with(p))
                    && word
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    names.insert(word.to_string());
                }
            }
            i += 1;
        }
    }
    names
}
