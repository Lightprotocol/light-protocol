//! Enhanced logging system for light-program-test
//!
//! This module provides Solana Explorer-like transaction logging with:
//! - Hierarchical instruction display with inner instructions
//! - Account changes tracking
//! - Light Protocol specific parsing and formatting
//! - Configurable verbosity levels
//! - Color-coded output
//!
//! Logging behavior:
//! - File logging: Always enabled when `enhanced_logging.enabled = true` (default)
//! - Log file: Written to `target/light_program_test.log`
//! - Console output: Only when `RUST_BACKTRACE` is set AND `log_events = true`
//! - Log file is overwritten at session start, then appended for each transaction

pub mod config;
pub mod decoder;
pub mod formatter;
pub mod types;

use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono;
pub use config::{EnhancedLoggingConfig, LogVerbosity};
pub use formatter::TransactionFormatter;
use litesvm::types::TransactionResult;
use solana_sdk::{signature::Signature, transaction::Transaction};
pub use types::{
    AccountChange, EnhancedInstructionLog, EnhancedTransactionLog, ParsedInstructionData,
    TransactionStatus,
};

use crate::program_test::config::ProgramTestConfig;

static SESSION_STARTED: std::sync::Once = std::sync::Once::new();

/// Get the log file path in target directory
fn get_log_file_path() -> PathBuf {
    // Always use cargo workspace target directory
    use std::process::Command;
    if let Ok(output) = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version=1")
        .arg("--no-deps")
        .output()
    {
        if output.status.success() {
            if let Ok(metadata) = String::from_utf8(output.stdout) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&metadata) {
                    if let Some(target_directory) = json["target_directory"].as_str() {
                        let mut path = PathBuf::from(target_directory);
                        path.push("light_program_test.log");
                        return path;
                    }
                }
            }
        }
    }

    // Fallback to current directory's target
    let mut path = PathBuf::from("target");
    path.push("light_program_test.log");
    path
}

/// Initialize log file with session header (called only once per session)
fn initialize_log_file() {
    SESSION_STARTED.call_once(|| {
        let log_path = get_log_file_path();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Create new log file with session header
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)
        {
            // Format timestamp as readable date
            let datetime =
                chrono::DateTime::from_timestamp(timestamp as i64, 0).unwrap_or(chrono::Utc::now());
            let formatted_date = datetime.format("%Y-%m-%d %H:%M:%S UTC");

            let _ = writeln!(
                file,
                "=== Light Program Test Session Started at {} ===\n",
                formatted_date
            );
        }
    });
}

/// Strip ANSI escape codes from string for plain text log files
fn strip_ansi_codes(text: &str) -> String {
    // Simple regex-free approach to remove ANSI escape sequences
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Found escape character, skip until we find 'm' (end of color code)
            for next_ch in chars.by_ref() {
                if next_ch == 'm' {
                    break;
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Write log entry to file (append to existing session log)
fn write_to_log_file(content: &str) {
    // Ensure session is initialized
    initialize_log_file();

    let log_path = get_log_file_path();

    // Ensure parent directory exists
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Strip ANSI color codes for file output
    let clean_content = strip_ansi_codes(content);

    // Append transaction log to existing file
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = writeln!(file, "{}", clean_content);
    }
}

/// Main entry point for enhanced transaction logging
pub fn log_transaction_enhanced(
    config: &ProgramTestConfig,
    transaction: &Transaction,
    result: &TransactionResult,
    signature: &Signature,
    slot: u64,
    transaction_counter: usize,
) {
    log_transaction_enhanced_with_console(
        config,
        transaction,
        result,
        signature,
        slot,
        transaction_counter,
        false,
    )
}

/// Enhanced transaction logging with console output control
pub fn log_transaction_enhanced_with_console(
    config: &ProgramTestConfig,
    transaction: &Transaction,
    result: &TransactionResult,
    signature: &Signature,
    slot: u64,
    transaction_counter: usize,
    print_to_console: bool,
) {
    if !config.enhanced_logging.enabled {
        return;
    }

    let enhanced_log = EnhancedTransactionLog::from_transaction_result(
        transaction,
        result,
        signature,
        slot,
        &config.enhanced_logging,
    );

    let formatter = TransactionFormatter::new(&config.enhanced_logging);
    let formatted_log = formatter.format(&enhanced_log, transaction_counter);

    // Always write to log file when enhanced logging is enabled
    write_to_log_file(&formatted_log);

    // Print to console if requested
    if print_to_console {
        println!("{}", formatted_log);
    }
}

/// Check if enhanced logging should be used instead of basic logging
pub fn should_use_enhanced_logging(config: &ProgramTestConfig) -> bool {
    config.enhanced_logging.enabled && !config.no_logs
}
