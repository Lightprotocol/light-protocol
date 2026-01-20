//! Derive macros for InstructionDecoder implementations
//!
//! This crate provides two macros:
//! 1. `#[derive(InstructionDecoder)]` - For instruction enums (native programs)
//! 2. `#[instruction_decoder]` - Attribute macro for Anchor program modules
//!
//! The attribute macro extracts function names from the program module and generates
//! an instruction enum with `#[derive(InstructionDecoder)]` applied.
//!
//! ## Enhanced InstructionDecoder for Anchor Programs
//!
//! The derive macro supports an enhanced mode that references Anchor-generated types
//! for account names and parameter decoding:
//!
//! ```rust,ignore
//! use light_instruction_decoder_derive::InstructionDecoder;
//!
//! #[derive(InstructionDecoder)]
//! #[instruction_decoder(
//!     program_id = "MyProgram111111111111111111111111111111111",
//!     program_name = "My Program"
//! )]
//! pub enum MyInstruction {
//!     #[instruction_decoder(accounts = CreateRecord, params = CreateRecordParams)]
//!     CreateRecord,
//!
//!     #[instruction_decoder(accounts = UpdateRecord)]
//!     UpdateRecord,
//! }
//! ```
//!
//! This generates a decoder that:
//! - Gets account names from `<AccountsType<'_>>::ACCOUNT_NAMES`
//! - Decodes instruction data using `ParamsType::try_from_slice()` with Debug output

extern crate proc_macro;

mod attribute_impl;
mod builder;
mod crate_context;
mod derive_impl;
mod parsing;
mod utils;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

use crate::utils::into_token_stream;

/// Derives an InstructionDecoder implementation for an Anchor instruction enum.
///
/// This macro generates a decoder struct and InstructionDecoder trait implementation
/// that can decode Anchor program instructions for logging purposes.
///
/// ## Usage
///
/// ```rust,ignore
/// use light_instruction_decoder_derive::InstructionDecoder;
///
/// #[derive(InstructionDecoder)]
/// #[instruction_decoder(
///     program_id = "MyProgramId111111111111111111111111111111111",
///     program_name = "My Program"
/// )]
/// pub enum MyInstruction {
///     CreateRecord,
///     UpdateRecord { score: u64 },
///     DeleteRecord,
/// }
/// ```
///
/// This generates a `MyInstructionDecoder` struct that implements `InstructionDecoder`.
#[proc_macro_derive(InstructionDecoder, attributes(instruction_decoder, discriminator))]
pub fn derive_instruction_decoder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    into_token_stream(derive_impl::derive_instruction_decoder_impl(input))
}

/// Attribute macro for generating InstructionDecoder from Anchor program modules.
///
/// This macro extracts function names from the program module and generates
/// an InstructionDecoder implementation automatically.
///
/// ## Usage
///
/// ```rust,ignore
/// use light_instruction_decoder_derive::instruction_decoder;
///
/// #[instruction_decoder]
/// #[program]
/// pub mod my_program {
///     pub fn create_record(ctx: Context<CreateRecord>) -> Result<()> { ... }
///     pub fn update_record(ctx: Context<UpdateRecord>) -> Result<()> { ... }
/// }
/// ```
///
/// This generates a `MyProgramInstructionDecoder` struct that implements `InstructionDecoder`.
/// The program_id can also be omitted if `declare_id!` is used inside the module.
#[proc_macro_attribute]
pub fn instruction_decoder(attr: TokenStream, item: TokenStream) -> TokenStream {
    into_token_stream(attribute_impl::instruction_decoder_attr(
        attr.into(),
        item.into(),
    ))
}
