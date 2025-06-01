use solana_account_decoder_client_types::UiDataSliceConfig;
use solana_pubkey::Pubkey;

pub struct GetCompressedTokenAccountsByOwnerOrDelegateOptions {
    pub mint: Option<PublicKey>,
    pub cursor: Option<String>,
    pub limit: Option<u64>,
}

/// **Cursor** is a unique identifier for a page of results by which the next page can be fetched.
///
/// **Limit** is the maximum number of results to return per page.
pub struct PaginatedOptions {
    pub cursor: Option<String>,
    pub limit: Option<u64>,
}

pub struct GetCompressedAccountsByOwnerConfig {
    pub filters: Option<Vec<GetCompressedAccountsFilter>>,
    pub data_slice: Option<UiDataSliceConfig>,
    pub cursor: Option<String>,
    pub limit: Option<u64>,
}

pub struct GetCompressedAccountsFilter {
    bytes: Vec<u8>,
    offset: u64,
}
