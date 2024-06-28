mod pipeline;
mod processor;

pub use pipeline::{setup_address_pipeline, AddressPipelineStage};
pub use processor::{get_changelog_indices, AddressProcessor};
