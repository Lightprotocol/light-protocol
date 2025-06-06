mod address;
mod common;
mod error;
mod state;

use common::BatchProcessor;
use error::Result;
use light_client::rpc::Rpc;
use tracing::{instrument, trace};

#[instrument(
    level = "debug",
    fields(
        epoch = context.epoch,
        tree = %context.merkle_tree,
        tree_type = ?tree_type
    ),
    skip(context)
)]
pub async fn process_batched_operations<R: Rpc, I: Indexer + IndexerType<R>>(
    context: BatchContext<R, I>,
    tree_type: TreeType,
) -> Result<usize> {
    trace!("process_batched_operations");
    let processor = BatchProcessor::new(context, tree_type);
    processor.process().await
}

pub use common::BatchContext;
pub use error::BatchProcessError;
use light_client::indexer::Indexer;
use light_compressed_account::TreeType;

use crate::indexer_type::IndexerType;
