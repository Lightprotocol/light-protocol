//! ComputeBudget program instruction decoder.
//!
//! This module provides a macro-derived decoder for the Solana ComputeBudget program,
//! which uses single-byte discriminators based on variant indices.

// Allow the macro-generated code to reference types from this crate
extern crate self as light_instruction_decoder;

use light_instruction_decoder_derive::InstructionDecoder;

/// ComputeBudget program instructions.
///
/// The ComputeBudget program uses a 1-byte discriminator (variant index).
/// Each variant's discriminator is its position in this enum (0, 1, 2, ...).
#[derive(InstructionDecoder)]
#[instruction_decoder(
    program_id = "ComputeBudget111111111111111111111111111111",
    program_name = "Compute Budget",
    discriminator_size = 1
)]
pub enum ComputeBudgetInstruction {
    /// Deprecated variant (index 0)
    Unused,

    /// Request a specific heap frame size (index 1)
    RequestHeapFrame { bytes: u32 },

    /// Set compute unit limit for the transaction (index 2)
    SetComputeUnitLimit { units: u32 },

    /// Set compute unit price in micro-lamports (index 3)
    SetComputeUnitPrice { micro_lamports: u64 },

    /// Set loaded accounts data size limit (index 4)
    SetLoadedAccountsDataSizeLimit { bytes: u32 },
}
