//! **aligned-sized** is a library providing the `aligned_sized` macro, which:
//!
//! * Calculates a size of the given struct and provides a `LEN` constant with
//!   that value.
//!
//! Future plans:
//!
//! * Ensuring that the struct is aligned, adding padding fields when
//!   neccessary.
//!
//! # Motivation
//!
//! Calculating the size of a struct is often a necessity when developing
//! project in Rust, in particular:
//!
//! * [Solana](https://solana.com/) programs, also when using
//!   [Anchor](https://www.anchor-lang.com/) framework.
//! * [eBPF](https://ebpf.io/) programs written in [Aya](https://aya-rs.dev/).
//!
//! This library provides a macro which automatically calculates the size,
//! also taking in account factors which make a straightforward use of
//! `core::mem::size_of::<T>` for the whole struct impossible (discriminants,
//! vectors etc.).

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemStruct};

mod expand;

#[proc_macro_attribute]
pub fn aligned_sized(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as expand::AlignedSizedArgs);
    let strct = parse_macro_input!(input as ItemStruct);
    expand::aligned_sized(args, strct)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
