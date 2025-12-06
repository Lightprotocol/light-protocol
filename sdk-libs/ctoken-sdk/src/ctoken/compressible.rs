use light_ctoken_interface::{
    instructions::extensions::compressible::CompressToPubkey, state::TokenDataVersion,
};
use solana_account_info::AccountInfo;
use solana_pubkey::Pubkey;

use crate::ctoken::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR};

#[derive(Debug, Clone)]
pub struct CompressibleParams {
    pub token_account_version: TokenDataVersion,
    pub pre_pay_num_epochs: u8,
    pub lamports_per_write: Option<u32>,
    pub compress_to_account_pubkey: Option<CompressToPubkey>,
    pub compressible_config: Pubkey,
    pub rent_sponsor: Pubkey,
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
        }
    }
}

impl CompressibleParams {
    /// Creates a new `CompressibleParams` with default values.
    ///
    /// Use builder methods to customize:
    /// - [`compress_to_pubkey`](Self::compress_to_pubkey) - set the destination for compression
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the destination pubkey for compression.
    pub fn compress_to_pubkey(mut self, compress_to: CompressToPubkey) -> Self {
        self.compress_to_account_pubkey = Some(compress_to);
        self
    }
}

/// Account infos for compressible token accounts in CPI operations.
pub struct CompressibleParamsInfos<'info> {
    pub compressible_config: AccountInfo<'info>,
    pub rent_sponsor: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub pre_pay_num_epochs: u8,
    pub lamports_per_write: Option<u32>,
    pub compress_to_account_pubkey: Option<CompressToPubkey>,
    pub token_account_version: TokenDataVersion,
}

impl<'info> CompressibleParamsInfos<'info> {
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
            token_account_version: TokenDataVersion::ShaFlat,
        }
    }

    pub fn with_compress_to_pubkey(mut self, compress_to: CompressToPubkey) -> Self {
        self.compress_to_account_pubkey = Some(compress_to);
        self
    }
}
