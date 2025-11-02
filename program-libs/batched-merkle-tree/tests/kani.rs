#![cfg(kani)]
// Kani formal verification tests
// This file serves as the entry point for the kani test module
// cargo kani --tests --no-default-features -Z stubbing
#[path = "kani/batch.rs"]
mod batch;

#[path = "kani/zero_out_roots.rs"]
mod zero_out_roots;
