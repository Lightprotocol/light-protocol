// Integration tests for compressed token account operations
// This file serves as the entry point for the ctoken test module

// Declare submodules from the ctoken/ directory
#[path = "ctoken/shared.rs"]
mod shared;

#[path = "ctoken/create.rs"]
mod create;

#[path = "ctoken/transfer.rs"]
mod transfer;

#[path = "ctoken/functional_ata.rs"]
mod functional_ata;

#[path = "ctoken/functional.rs"]
mod functional;

#[path = "ctoken/compress_and_close.rs"]
mod compress_and_close;

#[path = "ctoken/close.rs"]
mod close;

#[path = "ctoken/create_ata.rs"]
mod create_ata;

#[path = "ctoken/create_ata2.rs"]
mod create_ata2;

#[path = "ctoken/spl_instruction_compat.rs"]
mod spl_instruction_compat;
