mod address;
mod common;
mod error;
mod state;

use common::BatchProcessor;
use error::Result;
use forester_utils::{forester_epoch::TreeType};
use light_client::rpc::RpcConnection;
use tracing::{info, instrument};

#[instrument(
    level = "debug",
    fields(
        epoch = context.epoch,
        tree = %context.merkle_tree,
        tree_type = ?tree_type
    )
)]
pub async fn process_batched_operations<R: RpcConnection, I: Indexer<R> + IndexerType<R>>(
    context: BatchContext<R, I>,
    tree_type: TreeType,
) -> Result<usize> {
    info!("process_batched_operations");
    let processor = BatchProcessor::new(context, tree_type);
    processor.process().await
}

pub use common::BatchContext;
pub use error::BatchProcessError;
use light_client::indexer::Indexer;
use crate::indexer_type::IndexerType;
