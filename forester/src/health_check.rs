use std::{str::FromStr, time::Instant};

use forester_utils::rpc_pool::{SolanaRpcPool, SolanaRpcPoolBuilder};
use light_client::rpc::{LightClient, Rpc};
use light_registry::{utils::get_forester_epoch_pda_from_authority, ForesterEpochPda};
use serde_json::json;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use tracing::error;

use crate::{cli::HealthArgs, errors::ForesterError, utils::get_protocol_config};

#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub check_name: String,
    pub success: bool,
    pub message: String,
    pub duration_ms: u64,
}

impl HealthCheckResult {
    fn new(check_name: &str, success: bool, message: String, duration_ms: u64) -> Self {
        Self {
            check_name: check_name.to_string(),
            success,
            message,
            duration_ms,
        }
    }
}

pub async fn run_health_check(args: &HealthArgs) -> Result<bool, ForesterError> {
    let mut results = Vec::new();
    let mut all_passed = true;

    let run_all = !args.check_balance && !args.check_registration;

    let rpc_pool = if args.check_balance || args.check_registration || run_all {
        if let Some(ref rpc_url) = args.rpc_url {
            Some(
                SolanaRpcPoolBuilder::<LightClient>::default()
                    .url(rpc_url.clone())
                    .commitment(solana_sdk::commitment_config::CommitmentConfig::confirmed())
                    .max_size(1)
                    .connection_timeout_secs(10)
                    .idle_timeout_secs(60)
                    .max_retries(100)
                    .initial_retry_delay_ms(1000)
                    .max_retry_delay_ms(16000)
                    .build()
                    .await
                    .map_err(ForesterError::RpcPool)?,
            )
        } else {
            None
        }
    } else {
        None
    };

    if args.check_balance || run_all {
        if let (Some(payer), Some(ref pool)) = (args.payer.as_ref(), &rpc_pool) {
            let result = check_wallet_balance(pool, payer, args.min_balance).await;
            all_passed &= result.success;
            results.push(result);
        } else if args.check_balance || run_all {
            results.push(HealthCheckResult::new(
                "balance",
                false,
                "No payer keypair or RPC URL provided".to_string(),
                0,
            ));
            all_passed = false;
        }
    }

    if args.check_registration || run_all {
        if let (Some(derivation), Some(ref pool)) = (args.derivation.as_ref(), &rpc_pool) {
            let result = check_epoch_registration(pool, derivation).await;
            all_passed &= result.success;
            results.push(result);
        } else if args.check_registration || run_all {
            results.push(HealthCheckResult::new(
                "registration",
                false,
                "Derivation pubkey and RPC URL required for registration check".to_string(),
                0,
            ));
            all_passed = false;
        }
    }

    match args.output.as_str() {
        "json" => {
            let json_output = json!({
                "success": all_passed,
                "checks": results.iter().map(|r| {
                    json!({
                        "name": r.check_name,
                        "success": r.success,
                        "message": r.message,
                        "duration_ms": r.duration_ms,
                    })
                }).collect::<Vec<_>>(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&json_output).unwrap_or_else(|_| "{}".to_string())
            );
        }
        _ => {
            for result in &results {
                let status = if result.success { "✓" } else { "✗" };
                let level = if result.success { "OK" } else { "FAIL" };
                println!(
                    "{} {} [{}ms] {}: {}",
                    status, result.check_name, result.duration_ms, level, result.message
                );
            }

            if !all_passed {
                println!("\nHealth check FAILED");
            } else {
                println!("\nHealth check PASSED");
            }
        }
    }

    Ok(all_passed)
}

async fn check_wallet_balance<R: Rpc>(
    pool: &SolanaRpcPool<R>,
    payer: &str,
    min_balance: f64,
) -> HealthCheckResult {
    let start = Instant::now();

    let payer_bytes: Vec<u8> = match serde_json::from_str(payer) {
        Ok(bytes) => bytes,
        Err(e) => {
            return HealthCheckResult::new(
                "balance",
                false,
                format!("Failed to parse payer JSON: {}", e),
                start.elapsed().as_millis() as u64,
            );
        }
    };

    let keypair = match Keypair::try_from(payer_bytes.as_slice()) {
        Ok(kp) => kp,
        Err(e) => {
            return HealthCheckResult::new(
                "balance",
                false,
                format!("Failed to create keypair from bytes: {}", e),
                start.elapsed().as_millis() as u64,
            );
        }
    };

    let pubkey = keypair.pubkey();

    match pool.get_connection().await {
        Ok(rpc) => {
            let balance_result = rpc.get_balance(&pubkey).await;

            match balance_result {
                Ok(lamports) => {
                    let sol_balance = lamports as f64 / LAMPORTS_PER_SOL as f64;

                    if sol_balance >= min_balance {
                        HealthCheckResult::new(
                            "balance",
                            true,
                            format!(
                                "Wallet balance: {:.4} SOL (minimum: {:.4} SOL)",
                                sol_balance, min_balance
                            ),
                            start.elapsed().as_millis() as u64,
                        )
                    } else {
                        HealthCheckResult::new(
                            "balance",
                            false,
                            format!(
                                "Wallet balance {:.4} SOL is below minimum {:.4} SOL",
                                sol_balance, min_balance
                            ),
                            start.elapsed().as_millis() as u64,
                        )
                    }
                }
                Err(e) => {
                    error!("Failed to get balance for {}: {}", pubkey, e);
                    HealthCheckResult::new(
                        "balance",
                        false,
                        format!("Failed to get balance: {}", e),
                        start.elapsed().as_millis() as u64,
                    )
                }
            }
        }
        Err(e) => HealthCheckResult::new(
            "balance",
            false,
            format!("Failed to get RPC connection: {}", e),
            start.elapsed().as_millis() as u64,
        ),
    }
}

async fn check_epoch_registration<R: Rpc>(
    pool: &SolanaRpcPool<R>,
    derivation: &str,
) -> HealthCheckResult {
    let start = Instant::now();

    // Parse derivation pubkey - it can be either a base58 string or a JSON array of bytes
    let derivation_pubkey = if derivation.starts_with('[') {
        // Parse as JSON array of bytes
        let bytes: Vec<u8> = match serde_json::from_str(derivation) {
            Ok(b) => b,
            Err(e) => {
                return HealthCheckResult::new(
                    "registration",
                    false,
                    format!("Failed to parse derivation pubkey JSON: {}", e),
                    start.elapsed().as_millis() as u64,
                );
            }
        };

        if bytes.len() != 32 {
            return HealthCheckResult::new(
                "registration",
                false,
                format!("Derivation pubkey must be 32 bytes, got {}", bytes.len()),
                start.elapsed().as_millis() as u64,
            );
        }

        Pubkey::new_from_array(bytes.try_into().unwrap())
    } else {
        match Pubkey::from_str(derivation) {
            Ok(pk) => pk,
            Err(e) => {
                return HealthCheckResult::new(
                    "registration",
                    false,
                    format!("Failed to parse derivation pubkey: {}", e),
                    start.elapsed().as_millis() as u64,
                );
            }
        }
    };

    let mut rpc = match pool.get_connection().await {
        Ok(r) => r,
        Err(e) => {
            return HealthCheckResult::new(
                "registration",
                false,
                format!("Failed to get RPC connection: {}", e),
                start.elapsed().as_millis() as u64,
            );
        }
    };

    let slot_result = rpc.get_slot().await;

    let slot = match slot_result {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to get current slot: {}", e);
            return HealthCheckResult::new(
                "registration",
                false,
                format!("Failed to get current slot: {}", e),
                start.elapsed().as_millis() as u64,
            );
        }
    };

    let protocol_config = get_protocol_config(&mut *rpc).await;
    let current_epoch = protocol_config.get_current_epoch(slot);
    let forester_epoch_pda_pubkey =
        get_forester_epoch_pda_from_authority(&derivation_pubkey, current_epoch).0;
    let registration_result = rpc
        .get_anchor_account::<ForesterEpochPda>(&forester_epoch_pda_pubkey)
        .await;

    match registration_result {
        Ok(Some(pda)) => {
            if pda.total_epoch_weight.is_some() {
                HealthCheckResult::new(
                    "registration",
                    true,
                    format!(
                        "Forester registered for epoch {} with weight {}",
                        current_epoch,
                        pda.total_epoch_weight.unwrap()
                    ),
                    start.elapsed().as_millis() as u64,
                )
            } else {
                HealthCheckResult::new(
                    "registration",
                    true,
                    format!(
                        "Forester registered for epoch {} (not yet finalized)",
                        current_epoch
                    ),
                    start.elapsed().as_millis() as u64,
                )
            }
        }
        Ok(None) => {
            if current_epoch > 0 {
                let prev_epoch = current_epoch - 1;
                let prev_pda_pubkey =
                    get_forester_epoch_pda_from_authority(&derivation_pubkey, prev_epoch).0;

                match rpc.get_anchor_account::<ForesterEpochPda>(&prev_pda_pubkey).await {
                    Ok(Some(_)) => {
                        HealthCheckResult::new(
                            "registration",
                            false,
                            format!(
                                "Forester not registered for current epoch {} (was registered for epoch {})",
                                current_epoch,
                                prev_epoch
                            ),
                            start.elapsed().as_millis() as u64,
                        )
                    }
                    _ => {
                        HealthCheckResult::new(
                            "registration",
                            false,
                            format!("Forester not registered for current epoch {}", current_epoch),
                            start.elapsed().as_millis() as u64,
                        )
                    }
                }
            } else {
                HealthCheckResult::new(
                    "registration",
                    false,
                    format!("Forester not registered for epoch {}", current_epoch),
                    start.elapsed().as_millis() as u64,
                )
            }
        }
        Err(e) => {
            error!(
                "Failed to check registration for {}: {}",
                forester_epoch_pda_pubkey, e
            );

            HealthCheckResult::new(
                "registration",
                false,
                format!("Failed to check registration: {}", e),
                start.elapsed().as_millis() as u64,
            )
        }
    }
}
