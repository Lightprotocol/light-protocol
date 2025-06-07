use photon_api::models::{FilterSelector, Memcmp};
use solana_account_decoder_client_types::UiDataSliceConfig;
use solana_pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct GetCompressedTokenAccountsByOwnerOrDelegateOptions {
    pub mint: Option<Pubkey>,
    pub cursor: Option<String>,
    pub limit: Option<u16>,
}

impl GetCompressedTokenAccountsByOwnerOrDelegateOptions {
    pub fn new(mint: Option<Pubkey>) -> Self {
        Self {
            mint,
            cursor: None,
            limit: None,
        }
    }
}

/// **Cursor** is a unique identifier for a page of results by which the next page can be fetched.
///
/// **Limit** is the maximum number of results to return per page.
pub struct PaginatedOptions {
    pub cursor: Option<String>,
    pub limit: Option<u16>,
}

pub struct GetCompressedAccountsByOwnerConfig {
    pub filters: Option<Vec<GetCompressedAccountsFilter>>,
    pub data_slice: Option<UiDataSliceConfig>,
    pub cursor: Option<String>,
    pub limit: Option<u16>,
}

#[derive(Clone)]
pub struct GetCompressedAccountsFilter {
    pub bytes: Vec<u8>,
    pub offset: u32,
}

#[allow(clippy::from_over_into)]
impl Into<FilterSelector> for GetCompressedAccountsFilter {
    fn into(self) -> FilterSelector {
        FilterSelector {
            memcmp: Some(Box::new(Memcmp {
                offset: self.offset,
                bytes: base64::encode(&self.bytes), // TODO: double check
            })),
        }
    }
}

impl GetCompressedAccountsByOwnerConfig {
    pub fn filters_to_photon(&self) -> Option<Vec<FilterSelector>> {
        self.filters
            .as_ref()
            .map(|filters| filters.iter().map(|f| f.clone().into()).collect())
    }
}
