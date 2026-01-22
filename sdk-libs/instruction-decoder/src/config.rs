//! Configuration types for enhanced logging

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{registry::DecoderRegistry, InstructionDecoder};

/// Configuration for enhanced transaction logging
#[derive(Debug, Serialize, Deserialize)]
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
    /// Maximum CPI depth to display
    pub max_cpi_depth: usize,
    /// Show instruction data for account compression program
    pub show_compression_instruction_data: bool,
    /// Truncate byte arrays: Some((first, last)) shows first N and last N elements; None disables
    pub truncate_byte_arrays: Option<(usize, usize)>,
    /// Decoder registry containing built-in and custom decoders
    /// Wrapped in Arc so it can be shared across clones instead of being lost
    #[serde(skip)]
    decoder_registry: Option<Arc<DecoderRegistry>>,
}

impl Clone for EnhancedLoggingConfig {
    fn clone(&self) -> Self {
        // Arc clone shares the underlying DecoderRegistry across clones
        // This preserves custom decoders registered via with_decoders()
        Self {
            enabled: self.enabled,
            log_events: self.log_events,
            verbosity: self.verbosity,
            show_account_changes: self.show_account_changes,
            decode_light_instructions: self.decode_light_instructions,
            show_compute_units: self.show_compute_units,
            use_colors: self.use_colors,
            max_cpi_depth: self.max_cpi_depth,
            show_compression_instruction_data: self.show_compression_instruction_data,
            truncate_byte_arrays: self.truncate_byte_arrays,
            decoder_registry: self.decoder_registry.clone(),
        }
    }
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
            max_cpi_depth: 60,
            show_compression_instruction_data: false,
            truncate_byte_arrays: Some((2, 2)),
            decoder_registry: Some(Arc::new(DecoderRegistry::new())),
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
            max_cpi_depth: 60,
            show_compression_instruction_data: false,
            truncate_byte_arrays: Some((2, 2)),
            decoder_registry: Some(Arc::new(DecoderRegistry::new())),
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
            max_cpi_depth: 60,
            show_compression_instruction_data: false,
            truncate_byte_arrays: Some((2, 2)),
            decoder_registry: Some(Arc::new(DecoderRegistry::new())),
        }
    }

    /// Register custom decoders
    ///
    /// Note: Uses Arc::get_mut which works correctly in the builder pattern since
    /// there's only one Arc reference. If the Arc has been cloned, a new registry
    /// is created with built-in decoders plus the custom ones.
    pub fn with_decoders(mut self, decoders: Vec<Box<dyn InstructionDecoder>>) -> Self {
        if let Some(ref mut arc) = self.decoder_registry {
            if let Some(registry) = Arc::get_mut(arc) {
                registry.register_all(decoders);
                return self;
            }
        }
        // Create new registry if none exists or Arc has multiple references
        let mut registry = DecoderRegistry::new();
        registry.register_all(decoders);
        self.decoder_registry = Some(Arc::new(registry));
        self
    }

    /// Get or create the decoder registry
    pub fn get_decoder_registry(&mut self) -> &DecoderRegistry {
        if self.decoder_registry.is_none() {
            self.decoder_registry = Some(Arc::new(DecoderRegistry::new()));
        }
        self.decoder_registry.as_ref().unwrap()
    }

    /// Get the decoder registry if it exists (immutable access)
    pub fn decoder_registry(&self) -> Option<&DecoderRegistry> {
        self.decoder_registry.as_ref().map(|arc| arc.as_ref())
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
