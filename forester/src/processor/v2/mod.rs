pub mod common;
mod helpers;
mod processor;
mod proof_worker;
pub mod strategy;
mod tx_sender;

use light_client::rpc::Rpc;
use tracing::{instrument, trace};

use crate::{epoch_manager::ProcessingMetrics, Result};

pub use common::{BatchContext, ProverConfig, QueueWork};
use light_compressed_account::TreeType;

pub use processor::QueueProcessor;

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

#[derive(Debug, Clone, Default)]
pub struct ProcessingResult {
    pub items_processed: usize,
    pub metrics: ProcessingMetrics,
}
