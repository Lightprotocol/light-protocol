//! Processor functions called by instruction handlers.
//!
//! This module demonstrates the nested processor pattern where
//! instruction handlers in the program module delegate to
//! processor functions in separate modules.

mod create_single_record;

pub use create_single_record::process_create_single_record;
