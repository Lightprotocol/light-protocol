mod address;
mod common;
mod state;

use common::BatchProcessor;
use light_client::rpc::Rpc;
use tracing::{instrument, trace};

use crate::Result;

#[instrument(
    level = "debug",
    fields(
        epoch = context.epoch,
        tree = %context.merkle_tree,
        tree_type = ?tree_type
    ),
    skip(context)
)]
pub async fn process_batched_operations<R: Rpc>(
    context: BatchContext<R>,
    tree_type: TreeType,
) -> Result<usize> {
    trace!("process_batched_operations");
    let processor = BatchProcessor::new(context, tree_type);
    processor.process().await
}

pub use common::BatchContext;
use light_compressed_account::TreeType;
