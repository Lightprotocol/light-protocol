use std::time::Instant;

use serde_json::json;
use solana_sdk::pubkey::Pubkey;

const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_ACCOUNT_SIZE: usize = 165;

#[tokio::test]
#[ignore] // Run with: cargo test --test bench_gpa -- --ignored --nocapture
async fn bench_gpa_mainnet_paginated() {
    let rpc_url = std::env::var("MAINNET_RPC_URL").expect(
        "MAINNET_RPC_URL must be set. Example: export MAINNET_RPC_URL='https://mainnet.helius-rpc.com/?api-key=YOUR_KEY'"
    );

    // Validate URL format
    assert!(
        rpc_url.contains("api-key="),
        "URL must contain api-key parameter"
    );
    assert!(!rpc_url.ends_with("api-key="), "API key cannot be empty");

    println!(
        "Using RPC URL: {}",
        rpc_url.split("api-key=").next().unwrap_or(&rpc_url)
    );

    let client = reqwest::Client::new();

    // Test basic connectivity first
    println!("Testing RPC connection...");
    let test_payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getVersion"
    });

    match client.post(&rpc_url).json(&test_payload).send().await {
        Ok(response) => {
            if response.status().is_success() {
                println!("Connected to Solana RPC");
            } else if response.status().as_u16() == 429 {
                eprintln!("Rate limited (429). Please wait or upgrade your plan.");
                eprintln!("This benchmark makes many requests and may exceed free tier limits.");
                panic!("Rate limit exceeded");
            } else {
                eprintln!("Failed to connect: HTTP {}", response.status());
                panic!("RPC connection failed");
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to RPC: {:?}", e);
            panic!("RPC connection failed");
        }
    }

    let token_program = SPL_TOKEN_PROGRAM_ID.parse::<Pubkey>().unwrap();

    println!("\n=== GetProgramAccountsV2 Paginated Benchmark ===");
    println!("Program: {}", SPL_TOKEN_PROGRAM_ID);
    println!("Account size filter: {} bytes", TOKEN_ACCOUNT_SIZE);
    println!("Using Helius getProgramAccountsV2 with cursor pagination\n");

    // Page size: 1K accounts per page (max 10K for V2)
    const PAGE_SIZE: usize = 10_000;

    // Benchmark configurations
    let configs = vec![
        (1_000, PAGE_SIZE, "1K (1K/page)"),
        (10_000, PAGE_SIZE, "10K (1K/page)"),
        (100_000, PAGE_SIZE, "100K (1K/page)"),
    ];

    for (total_limit, page_size, label) in configs {
        println!("--- Fetching {} ---", label);

        let mut total_fetched = 0;
        let mut total_duration = std::time::Duration::ZERO;
        let mut page_count = 0;
        let mut cursor: Option<String> = None;

        let overall_start = Instant::now();

        while total_fetched < total_limit {
            page_count += 1;

            let mut params = json!([
                token_program.to_string(),
                {
                    "encoding": "base64",
                    "commitment": "confirmed",
                    "filters": [
                        {"dataSize": TOKEN_ACCOUNT_SIZE}
                    ],
                    "limit": page_size
                }
            ]);

            // Add cursor for pagination
            if let Some(ref c) = cursor {
                params[1]["cursor"] = json!(c);
            }

            let payload = json!({
                "jsonrpc": "2.0",
                "id": page_count,
                "method": "getProgramAccountsV2",
                "params": params
            });

            let page_start = Instant::now();

            match client.post(&rpc_url).json(&payload).send().await {
                Ok(response) => {
                    let page_duration = page_start.elapsed();
                    total_duration += page_duration;

                    if !response.status().is_success() {
                        println!("  HTTP Error on page {}: {}", page_count, response.status());
                        break;
                    }

                    match response.json::<serde_json::Value>().await {
                        Ok(json_response) => {
                            // Debug: always print on first page
                            if page_count == 1 {
                                println!(
                                    "  Debug - Full JSON: {}",
                                    serde_json::to_string_pretty(&json_response)
                                        .unwrap_or_else(|_| "Failed to serialize".to_string())
                                );
                            }

                            if let Some(error) = json_response.get("error") {
                                println!("  RPC Error on page {}: {:?}", page_count, error);
                                break;
                            }

                            let result = json_response.get("result");

                            if let Some(result_obj) = result {
                                // V2 API returns {"accounts": [...], "cursor": "..."}
                                let accounts_array = if let Some(arr) =
                                    result_obj.get("accounts").and_then(|v| v.as_array())
                                {
                                    arr
                                } else if let Some(arr) = result_obj.as_array() {
                                    // Fallback: direct array
                                    arr
                                } else if let Some(value) =
                                    result_obj.get("value").and_then(|v| v.as_array())
                                {
                                    // Fallback: wrapped in "value"
                                    value
                                } else {
                                    println!("  Could not find accounts array");
                                    println!(
                                        "  Debug - Result keys: {:?}",
                                        result_obj
                                            .as_object()
                                            .map(|o| o.keys().collect::<Vec<_>>())
                                    );
                                    break;
                                };

                                let accounts = accounts_array.len();

                                if accounts == 0 {
                                    println!("  No more accounts available (0 accounts returned)");
                                    break;
                                }

                                let accounts_to_count = accounts.min(total_limit - total_fetched);
                                total_fetched += accounts_to_count;

                                // Get cursor for next page
                                cursor = result_obj
                                    .get("cursor")
                                    .and_then(|c| c.as_str())
                                    .map(|s| s.to_string());

                                println!(
                                    "  Page {}: {} accounts in {:?}{}",
                                    page_count,
                                    accounts_to_count,
                                    page_duration,
                                    if cursor.is_some() {
                                        " (has more)"
                                    } else {
                                        " (last page)"
                                    }
                                );

                                // If no cursor, we've reached the end
                                if cursor.is_none() {
                                    println!("  Reached end of results");
                                    break;
                                }
                            } else {
                                println!("  Unexpected response format on page {}", page_count);
                                break;
                            }
                        }
                        Err(e) => {
                            println!("  Failed to parse response on page {}: {:?}", page_count, e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    println!("  Request error on page {}: {:?}", page_count, e);
                    break;
                }
            }

            // Add delay between requests to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if total_fetched >= total_limit {
                break;
            }
        }

        let overall_duration = overall_start.elapsed();

        println!("  Total fetched: {} accounts", total_fetched);
        println!("  Pages: {}", page_count);
        println!("  Total time: {:?}", overall_duration);
        println!("  RPC time: {:?}", total_duration);
        if overall_duration.as_secs_f64() > 0.0 {
            println!(
                "  Rate: {:.2} accounts/sec",
                total_fetched as f64 / overall_duration.as_secs_f64()
            );
        }
        println!();

        // Add delay between benchmark runs
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    println!("=== Benchmark Complete ===\n");
}
