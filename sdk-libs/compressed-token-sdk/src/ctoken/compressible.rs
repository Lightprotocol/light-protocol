use light_ctoken_types::{
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
            pre_pay_num_epochs: 2,
            lamports_per_write: Some(100),
            compress_to_account_pubkey: None,
            token_account_version: TokenDataVersion::ShaFlat,
        }
    }
}

impl CompressibleParams {
    pub fn new(lamports_per_write: u32, pre_pay_num_epochs: u8) -> Self {
        Self {
            lamports_per_write: Some(lamports_per_write),
            pre_pay_num_epochs,
            ..Default::default()
        }
    }

    pub fn compress_to_pubkey(mut self, compress_to: CompressToPubkey) -> Self {
        self.compress_to_account_pubkey = Some(compress_to);
        self
    }
}

pub struct CompressibleParamsInfos<'info> {
    pub compressible_config: AccountInfo<'info>,
    pub rent_sponsor: AccountInfo<'info>,
    pub pre_pay_num_epochs: u8,
    pub lamports_per_write: Option<u32>,
    pub compress_to_account_pubkey: Option<CompressToPubkey>,
    pub token_account_version: TokenDataVersion,
}
