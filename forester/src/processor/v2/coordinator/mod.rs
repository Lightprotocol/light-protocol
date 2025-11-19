pub mod address_tree_coordinator;
pub mod batch_preparation;
pub mod batch_submission;
pub mod error;
pub mod proof_generation;
pub mod shared_state;
pub mod state_tree_coordinator;
pub mod tree_state;
pub mod types;

pub use address_tree_coordinator::AddressTreeCoordinator;
pub use shared_state::{create_shared_state, SharedState, SharedTreeState};
pub use state_tree_coordinator::StateTreeCoordinator;

/// Print combined performance summary for both state and address controllers.
pub async fn print_cumulative_performance_summary(label: &str) {
    state_tree_coordinator::print_cumulative_performance_summary(&format!("{} (State V2)", label))
        .await;
    address_tree_coordinator::print_cumulative_performance_summary(&format!(
        "{} (Address V2)",
        label
    ))
    .await;
}
