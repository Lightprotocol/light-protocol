#![cfg(kani)]
// Kani formal verification tests
// This file serves as the entry point for the kani test module
// cargo kani --tests --no-default-features -Z stubbing --features kani
#[path = "kani/batch.rs"]
mod batch;

#[path = "kani/address_tree_update.rs"]
mod address_tree_update;

#[path = "kani/ghost_state.rs"]
mod ghost_state;

#[path = "kani/utils.rs"]
pub mod utils;
