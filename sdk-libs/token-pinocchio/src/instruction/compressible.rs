//! Compressible parameters for rent-free CToken accounts.

use light_token_interface::{instructions::extensions::CompressToPubkey, state::TokenDataVersion};
use pinocchio::account_info::AccountInfo;

use crate::constants::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR as RENT_SPONSOR};

/// Parameters for creating compressible ctoken accounts.
///
/// Compressible accounts have sponsored rent and can be compressed to compressed
/// token accounts when their lamports balance is insufficient.
///
/// Default values are:
/// - 24 hours rent
/// - lamports for 3 hours rent (paid on transfer when account rent is insufficient to cover the next 2 epochs)
/// - Protocol rent sponsor
/// - TokenDataVersion::ShaFlat token data hashing (only sha is supported for compressible accounts)
///
/// # Example
/// ```rust
/// use light_token_pinocchio::instruction::CompressibleParams;
///
/// let params = CompressibleParams::new();
/// ```
#[derive(Debug, Clone)]
pub struct CompressibleParams {
    pub token_account_version: TokenDataVersion,
    pub pre_pay_num_epochs: u8,
    /// Number of lamports transferred on a write operation (eg transfer) when account rent is insufficient to cover the next 2 rent-epochs.
    /// Default: 766 lamports for 3 hours rent.
    /// These lamports keep the ctoken account perpetually funded when used.
    pub lamports_per_write: Option<u32>,
    pub compress_to_account_pubkey: Option<CompressToPubkey>,
    pub compressible_config: [u8; 32],
    pub rent_sponsor: [u8; 32],
    pub compression_only: bool,
}

impl Default for CompressibleParams {
    fn default() -> Self {
        Self {
            compressible_config: LIGHT_TOKEN_CONFIG,
            rent_sponsor: RENT_SPONSOR,
            pre_pay_num_epochs: 16,
            lamports_per_write: Some(766),
            compress_to_account_pubkey: None,
            token_account_version: TokenDataVersion::ShaFlat,
            compression_only: false,
        }
    }
}

impl CompressibleParams {
    /// Creates a new `CompressibleParams` with default values.
    /// - 24 hours rent
    /// - 3 hours top up (paid on transfer when account rent is insufficient to cover the next 2 epochs)
    /// - Protocol rent sponsor
    /// - TokenDataVersion::ShaFlat token data hashing (only sha is supported for compressible accounts)
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates default params for ATAs (compression_only = true).
    /// ATAs are always compression_only.
    pub fn default_ata() -> Self {
        Self {
            compression_only: true,
            ..Self::default()
        }
    }

    /// Sets the destination pubkey for compression.
    pub fn compress_to_pubkey(mut self, compress_to: CompressToPubkey) -> Self {
        self.compress_to_account_pubkey = Some(compress_to);
        self
    }
}

/// Parameters for creating compressible ctoken accounts via CPI.
///
/// # Example
/// ```rust,ignore
/// use light_token_pinocchio::instruction::CompressibleParamsCpi;
/// use pinocchio::account_info::AccountInfo;
///
/// let params = CompressibleParamsCpi::new(
///     &compressible_config,
///     &rent_sponsor,
///     &system_program,
/// );
/// ```
pub struct CompressibleParamsCpi<'info> {
    pub compressible_config: &'info AccountInfo,
    pub rent_sponsor: &'info AccountInfo,
    pub system_program: &'info AccountInfo,
    pub pre_pay_num_epochs: u8,
    pub lamports_per_write: Option<u32>,
    pub compress_to_account_pubkey: Option<CompressToPubkey>,
    pub token_account_version: TokenDataVersion,
    pub compression_only: bool,
}

impl<'info> CompressibleParamsCpi<'info> {
    pub fn new(
        compressible_config: &'info AccountInfo,
        rent_sponsor: &'info AccountInfo,
        system_program: &'info AccountInfo,
    ) -> Self {
        let defaults = CompressibleParams::default();
        Self {
            compressible_config,
            rent_sponsor,
            system_program,
            pre_pay_num_epochs: defaults.pre_pay_num_epochs,
            lamports_per_write: defaults.lamports_per_write,
            compress_to_account_pubkey: None,
            token_account_version: defaults.token_account_version,
            compression_only: defaults.compression_only,
        }
    }

    pub fn new_ata(
        compressible_config: &'info AccountInfo,
        rent_sponsor: &'info AccountInfo,
        system_program: &'info AccountInfo,
    ) -> Self {
        let defaults = CompressibleParams::default_ata();
        Self {
            compressible_config,
            rent_sponsor,
            system_program,
            pre_pay_num_epochs: defaults.pre_pay_num_epochs,
            lamports_per_write: defaults.lamports_per_write,
            compress_to_account_pubkey: None,
            token_account_version: defaults.token_account_version,
            compression_only: defaults.compression_only,
        }
    }

    pub fn with_compress_to_pubkey(mut self, compress_to: CompressToPubkey) -> Self {
        self.compress_to_account_pubkey = Some(compress_to);
        self
    }
}
