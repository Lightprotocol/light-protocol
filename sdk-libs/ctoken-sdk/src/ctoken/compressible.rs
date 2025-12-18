use light_ctoken_interface::{
    instructions::extensions::compressible::CompressToPubkey, state::TokenDataVersion,
};
use solana_account_info::AccountInfo;
use solana_pubkey::Pubkey;

use crate::ctoken::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR};

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
/// use light_ctoken_sdk::ctoken::CompressibleParams;
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
    pub compressible_config: Pubkey,
    pub rent_sponsor: Pubkey,
    pub compression_only: bool,
}

impl Default for CompressibleParams {
    fn default() -> Self {
        Self {
            compressible_config: COMPRESSIBLE_CONFIG_V1,
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

    /// Sets the destination pubkey for compression.
    pub fn compress_to_pubkey(mut self, compress_to: CompressToPubkey) -> Self {
        self.compress_to_account_pubkey = Some(compress_to);
        self
    }
}

/// Parameters for creating compressible ctoken accounts via CPI.
///
/// # Example
/// ```rust,no_run
/// # use light_ctoken_sdk::ctoken::CompressibleParamsCpi;
/// # use solana_account_info::AccountInfo;
/// // Use ctoken::COMPRESSIBLE_CONFIG_V1 or ctoken::config_pda() to get the protocol config.
/// // Use ctoken::RENT_SPONSOR or ctoken::rent_sponsor_pda() to get the protocol rent sponsor.
/// # let compressible_config: AccountInfo = todo!();
/// # let rent_sponsor: AccountInfo = todo!();
/// # let system_program: AccountInfo = todo!();
/// let params = CompressibleParamsCpi::new(
///     compressible_config,
///     rent_sponsor,
///     system_program,
/// );
/// ```
pub struct CompressibleParamsCpi<'info> {
    pub compressible_config: AccountInfo<'info>,
    pub rent_sponsor: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub pre_pay_num_epochs: u8,
    pub lamports_per_write: Option<u32>,
    pub compress_to_account_pubkey: Option<CompressToPubkey>,
    pub token_account_version: TokenDataVersion,
    pub compression_only: bool,
}

impl<'info> CompressibleParamsCpi<'info> {
    pub fn new(
        compressible_config: AccountInfo<'info>,
        rent_sponsor: AccountInfo<'info>,
        system_program: AccountInfo<'info>,
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

    pub fn with_compress_to_pubkey(mut self, compress_to: CompressToPubkey) -> Self {
        self.compress_to_account_pubkey = Some(compress_to);
        self
    }
}
