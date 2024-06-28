mod pipeline;
mod processor;

pub use pipeline::{setup_state_pipeline, PipelineStage};
pub use processor::{get_nullifier_queue, StateProcessor};
