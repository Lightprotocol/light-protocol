use photon_api::models::{FilterSelector, Memcmp};
use solana_account_decoder_client_types::UiDataSliceConfig;
use solana_pubkey::Pubkey;

#[derive(Debug, Clone, Default)]
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

/// Options for fetching queue elements (V2 with deduplicated nodes and address queue support).
#[derive(Debug, Clone, Default)]
pub struct QueueElementsV2Options {
    pub output_queue_start_index: Option<u64>,
    pub output_queue_limit: Option<u16>,
    pub output_queue_zkp_batch_size: Option<u16>,
    pub input_queue_start_index: Option<u64>,
    pub input_queue_limit: Option<u16>,
    pub input_queue_zkp_batch_size: Option<u16>,
    pub address_queue_start_index: Option<u64>,
    pub address_queue_limit: Option<u16>,
    pub address_queue_zkp_batch_size: Option<u16>,
}

impl QueueElementsV2Options {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_output_queue(mut self, start_index: Option<u64>, limit: Option<u16>) -> Self {
        self.output_queue_start_index = start_index;
        self.output_queue_limit = limit;
        self
    }

    pub fn with_output_queue_batch_size(mut self, batch_size: Option<u16>) -> Self {
        self.output_queue_zkp_batch_size = batch_size;
        self
    }

    pub fn with_input_queue(mut self, start_index: Option<u64>, limit: Option<u16>) -> Self {
        self.input_queue_start_index = start_index;
        self.input_queue_limit = limit;
        self
    }

    pub fn with_input_queue_batch_size(mut self, batch_size: Option<u16>) -> Self {
        self.input_queue_zkp_batch_size = batch_size;
        self
    }

    pub fn with_address_queue(mut self, start_index: Option<u64>, limit: Option<u16>) -> Self {
        self.address_queue_start_index = start_index;
        self.address_queue_limit = limit;
        self
    }

    pub fn with_address_queue_batch_size(mut self, batch_size: Option<u16>) -> Self {
        self.address_queue_zkp_batch_size = batch_size;
        self
    }
}

/// Authority type for querying compressed mints
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MintAuthorityType {
    /// Query by mint authority only
    MintAuthority,
    /// Query by freeze authority only
    FreezeAuthority,
    /// Query by both mint and freeze authority (default)
    #[default]
    Both,
}

/// Options for fetching compressed mints by authority.
#[derive(Debug, Clone, Default)]
pub struct GetCompressedMintsByAuthorityOptions {
    /// Which authority type to filter by (default: Both)
    pub authority_type: Option<MintAuthorityType>,
    /// Cursor for pagination
    pub cursor: Option<String>,
    /// Maximum number of results to return
    pub limit: Option<u16>,
}

impl GetCompressedMintsByAuthorityOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_authority_type(mut self, authority_type: MintAuthorityType) -> Self {
        self.authority_type = Some(authority_type);
        self
    }

    pub fn with_cursor(mut self, cursor: String) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn with_limit(mut self, limit: u16) -> Self {
        self.limit = Some(limit);
        self
    }
}
