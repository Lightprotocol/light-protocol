// Integration tests for mint operations
// This file serves as the entry point for the mint test module

// Declare submodules from the mint/ directory
#[path = "mint/cpi_context.rs"]
mod cpi_context;

#[path = "mint/edge_cases.rs"]
mod edge_cases;

#[path = "mint/failing.rs"]
mod failing;

#[path = "mint/functional.rs"]
mod functional;

#[path = "mint/random.rs"]
mod random;
