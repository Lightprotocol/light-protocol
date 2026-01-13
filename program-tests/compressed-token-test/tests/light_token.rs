// Integration tests for compressed token account operations
// This file serves as the entry point for the light_token test module

// Declare submodules from the light_token/ directory
#[path = "light_token/shared.rs"]
mod shared;

#[path = "light_token/create.rs"]
mod create;

#[path = "light_token/transfer.rs"]
mod transfer;

#[path = "light_token/functional_ata.rs"]
mod functional_ata;

#[path = "light_token/functional.rs"]
mod functional;

#[path = "light_token/compress_and_close.rs"]
mod compress_and_close;

#[path = "light_token/close.rs"]
mod close;

#[path = "light_token/create_ata.rs"]
mod create_ata;

#[path = "light_token/create_ata2.rs"]
mod create_ata2;

#[path = "light_token/spl_instruction_compat.rs"]
mod spl_instruction_compat;

#[path = "light_token/extensions.rs"]
mod extensions;

#[path = "light_token/transfer_checked.rs"]
mod transfer_checked;

#[path = "light_token/freeze_thaw.rs"]
mod freeze_thaw;

#[path = "light_token/approve_revoke.rs"]
mod approve_revoke;

#[path = "light_token/burn.rs"]
mod burn;

#[path = "light_token/extensions_failing.rs"]
mod extensions_failing;
