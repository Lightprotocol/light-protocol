pub mod address;
pub mod common;
pub mod state;
mod strategies;
mod unified;

use light_client::rpc::Rpc;
use tracing::{instrument, trace};

use crate::Result;

// Export unified architecture
pub use strategies::{AddressTreeStrategy, StateTreeStrategy};
pub use unified::UnifiedBatchProcessor;

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
    match tree_type {
        TreeType::AddressV2 => {
            trace!("AddressV2 processing should be handled through AddressSupervisor actor");
            Ok(0)
        }
        TreeType::StateV2 => {
            trace!("StateV2 processing should be handled through StateSupervisor actor");
            Ok(0)
        }
        _ => Ok(0),
    }
}

pub use common::{BatchContext, ProverConfig, QueueWork};
use light_compressed_account::TreeType;
