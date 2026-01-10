//! The #[light_instruction] attribute macro.
//!
//! Wraps instruction handlers to automatically call:
//! - `light_pre_init()` at the START (creates mints via CPI context write)
//! - `light_finalize()` at the END (compresses PDAs and executes with proof)
//!
//! This two-phase design allows mints to be used during the instruction body.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Ident, ItemFn,
};

/// Arguments for #[light_instruction] or #[light_instruction(params_name)]
/// 
/// If no params_name is provided, defaults to `params`.
pub struct LightInstructionArgs {
    pub params_ident: Ident,
}

impl Parse for LightInstructionArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // If empty, default to "params"
        if input.is_empty() {
            return Ok(Self {
                params_ident: Ident::new("params", proc_macro2::Span::call_site()),
            });
        }
        // Otherwise parse the identifier: #[light_instruction(my_params)]
        let params_ident: Ident = input.parse()?;
        Ok(Self { params_ident })
    }
}

/// Generate the wrapped instruction function
pub fn light_instruction_impl(
    args: LightInstructionArgs,
    item: ItemFn,
) -> Result<TokenStream, syn::Error> {
    let params_ident = &args.params_ident;
    let fn_vis = &item.vis;
    let fn_sig = &item.sig;
    let fn_block = &item.block;
    let fn_attrs = &item.attrs;

    // Validate that the function has a Context parameter named `ctx`
    // and a parameter matching params_ident
    let mut has_ctx = false;
    let mut has_params = false;

    for input in &fn_sig.inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                if pat_ident.ident == "ctx" {
                    has_ctx = true;
                }
                if &pat_ident.ident == params_ident {
                    has_params = true;
                }
            }
        }
    }

    if !has_ctx {
        return Err(syn::Error::new(
            fn_sig.span(),
            "light_instruction requires a parameter named `ctx` (the Anchor Context)",
        ));
    }

    if !has_params {
        return Err(syn::Error::new(
            params_ident.span(),
            format!(
                "parameter `{}` not found in function signature",
                params_ident
            ),
        ));
    }

    // Generate the wrapped function with two-phase compression:
    // 1. light_pre_init() at START - creates mints via CPI context write
    // 2. light_finalize() at END - compresses PDAs and executes with proof
    Ok(quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            // Phase 1: Pre-init mints (writes to CPI context, does NOT execute yet)
            // This allows mint accounts to be used during the instruction body
            use light_sdk::compressible::{LightPreInit, LightFinalize};
            let __has_pre_init = ctx.accounts.light_pre_init(ctx.remaining_accounts, &#params_ident)
                .map_err(|e| {
                    let pe: solana_program_error::ProgramError = e.into();
                    pe
                })?;

            // Execute the original handler body in a closure
            let __light_handler_result = (|| #fn_block)();

            // Phase 2: On success, finalize compression (compresses PDAs + executes proof)
            // This runs BEFORE Anchor's exit() hook which serializes account data
            if __light_handler_result.is_ok() {
                ctx.accounts.light_finalize(ctx.remaining_accounts, &#params_ident, __has_pre_init)
                    .map_err(|e| {
                        let pe: solana_program_error::ProgramError = e.into();
                        pe
                    })?;
            }

            __light_handler_result
        }
    })
}
