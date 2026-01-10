//! The #[light_instruction] attribute macro.
//!
//! Wraps instruction handlers to automatically call `light_finalize()`
//! at the end of successful execution, before Anchor's exit hook.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Ident, ItemFn,
};

/// Arguments for #[light_instruction(params_name)]
/// 
/// The params_name identifies which function parameter contains the compression params.
pub struct LightInstructionArgs {
    pub params_ident: Ident,
}

impl Parse for LightInstructionArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse just the identifier: #[light_instruction(my_params)]
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

    // Generate the wrapped function
    //
    // Strategy: We wrap the original body in a closure to capture the result,
    // then call light_finalize on success before returning.
    //
    // IMPORTANT: `return` statements inside the original body will return from
    // the closure, not the outer function. This is acceptable because:
    // - Error returns (return Err(...)) will result in light_finalize NOT being called
    // - Success returns (return Ok(())) will result in light_finalize being called
    // - The `?` operator works correctly for error propagation
    //
    // Users should avoid explicit `return Ok(value)` for non-unit returns if they
    // have code after the return that shouldn't run. Use normal control flow instead.
    Ok(quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            // Execute the original handler body in a closure
            let __light_handler_result = (|| #fn_block)();

            // On success, call light_finalize before returning
            // This runs BEFORE Anchor's exit() hook which serializes account data
            if __light_handler_result.is_ok() {
                use light_sdk::compressible::LightFinalize;
                ctx.accounts.light_finalize(ctx.remaining_accounts, &#params_ident)
                    .map_err(|e| {
                        let pe: solana_program_error::ProgramError = e.into();
                        pe
                    })?;
            }

            __light_handler_result
        }
    })
}
