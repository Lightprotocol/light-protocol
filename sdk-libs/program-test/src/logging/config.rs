//! Configuration types for enhanced logging

use serde::{Deserialize, Serialize};

/// Configuration for enhanced transaction logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedLoggingConfig {
    /// Whether enhanced logging is enabled
    pub enabled: bool,
    /// Whether to log events to console (file logging is always enabled when enhanced_logging.enabled = true)
    pub log_events: bool,
    /// Level of detail in logs
    pub verbosity: LogVerbosity,
    /// Show account changes before/after transaction
    pub show_account_changes: bool,
    /// Decode Light Protocol specific instructions
    pub decode_light_instructions: bool,
    /// Show compute units consumed per instruction
    pub show_compute_units: bool,
    /// Use ANSI colors in output
    pub use_colors: bool,
    /// Maximum number of inner instruction levels to display
    pub max_inner_instruction_depth: usize,
    /// Show instruction data for account compression program
    pub show_compression_instruction_data: bool,
}

impl Default for EnhancedLoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,     // Always enabled for processing
            log_events: false, // Don't log by default
            verbosity: LogVerbosity::Standard,
            show_account_changes: true,
            decode_light_instructions: true,
            show_compute_units: true,
            use_colors: true,
            max_inner_instruction_depth: 60,
            show_compression_instruction_data: false,
        }
    }
}

/// Verbosity levels for transaction logging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogVerbosity {
    /// Only instruction hierarchy and status
    Brief,
    /// + account changes and basic instruction info
    Standard,
    /// + parsed instruction data when available
    Detailed,
    /// + raw instruction data and internal debugging info
    Full,
}

impl EnhancedLoggingConfig {
    /// Create config optimized for debugging
    pub fn debug() -> Self {
        Self {
            enabled: true,
            log_events: true, // Enable logging for debug mode
            verbosity: LogVerbosity::Full,
            show_account_changes: true,
            decode_light_instructions: true,
            show_compute_units: true,
            use_colors: true,
            max_inner_instruction_depth: 60,
            show_compression_instruction_data: false,
        }
    }

    /// Create config optimized for CI/production
    pub fn minimal() -> Self {
        Self {
            enabled: true,
            log_events: false, // Don't log for minimal config
            verbosity: LogVerbosity::Brief,
            show_account_changes: false,
            decode_light_instructions: false,
            show_compute_units: false,
            use_colors: false,
            max_inner_instruction_depth: 60,
            show_compression_instruction_data: false,
        }
    }

    /// Create config based on environment - always enabled, debug level when RUST_BACKTRACE is set
    pub fn from_env() -> Self {
        if std::env::var("RUST_BACKTRACE").is_ok() {
            Self::debug()
        } else {
            // Always enabled but with standard verbosity when backtrace is not set
            Self::default()
        }
    }

    /// Enable event logging with current settings
    pub fn with_logging(mut self) -> Self {
        self.log_events = true;
        self
    }

    /// Disable event logging
    pub fn without_logging(mut self) -> Self {
        self.log_events = false;
        self
    }
}
