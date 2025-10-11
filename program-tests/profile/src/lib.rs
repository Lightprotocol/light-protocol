use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn profile(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // If profile-program feature is not enabled, return the original function completely unchanged
    #[cfg(not(feature = "profile-program"))]
    {
        return item;
    }

    #[cfg(feature = "profile-program")]
    {
        use quote::quote;
        use syn::{parse_macro_input, ItemFn, ReturnType};
        let mut f = parse_macro_input!(item as ItemFn);
        let original_body = f.block.clone();
        let sig = f.sig.clone();
        let ident = sig.ident.clone();
        let fn_name_str = ident.to_string();
        let returns_value = !matches!(sig.output, ReturnType::Default);

        // Add padding before function name to align with program ID position
        // and padding after to align "consumed" with program's "consumed"
        let program_id_width = 43;
        let fn_name_len = fn_name_str.len();
        let front_padding = " ".repeat(8); // Same as "Program " length to align start position
        let back_padding = if fn_name_len < program_id_width {
            " ".repeat(program_id_width - fn_name_len) // Add 1 extra space to fix alignment
        } else {
            " ".to_string() // minimum one space
        };

        // Create profiling start and end calls with feature flag and compile-time caller info
        let profile_start = quote! {
            #[cfg(all(target_os = "solana", feature = "profile-program"))]
            {
                extern "C" {
                    fn sol_log_compute_units_start(id_addr: u64, id_len: u64, heap_value: u64, with_heap: u64, _arg5: u64);
                }
                // Dynamic padding calculated at compile time
                const PROFILE_ID: &str = concat!(#fn_name_str, "\n", #front_padding, file!(), ":", line!(), #back_padding);

                #[cfg(feature = "profile-heap")]
                unsafe {
                    sol_log_compute_units_start(
                        PROFILE_ID.as_ptr() as u64,
                        PROFILE_ID.len() as u64,
                     ::light_heap::GLOBAL_ALLOCATOR.get_used_heap(),
                        1u64,
                        0
                    );
                }

                #[cfg(not(feature = "profile-heap"))]
                unsafe {
                    sol_log_compute_units_start(
                        PROFILE_ID.as_ptr() as u64,
                        PROFILE_ID.len() as u64,
                        0u64,
                        0u64,
                        0
                    );
                }
            }
        };

        let profile_end = quote! {
                   #[cfg(all(target_os = "solana", feature = "profile-program"))]
                   {
                       extern "C" {
                           fn sol_log_compute_units_end(id_addr: u64, id_len: u64, heap_value: u64, with_heap: u64, _arg5: u64);
                       }
                       // Dynamic padding calculated at compile time
                       const PROFILE_ID: &str = concat!(#fn_name_str, "\n", #front_padding, file!(), ":", line!(), #back_padding);
                       #[cfg(feature = "profile-heap")]
                       unsafe {
                           sol_log_compute_units_end(
                               PROFILE_ID.as_ptr() as u64,
                               PROFILE_ID.len() as u64,
                               ::light_heap::GLOBAL_ALLOCATOR.get_used_heap(),
                               1u64,
                               0
                           );
                       }

                       #[cfg(target_os = "solana")]
        #[cfg(not(feature = "profile-heap"))]
                       unsafe {
                           sol_log_compute_units_end(
                               PROFILE_ID.as_ptr() as u64,
                               PROFILE_ID.len() as u64,
                               0u64,
                               0u64,
                               0
                           );
                       }
                   }
               };

        // Build the new function body by wrapping the original body with profiling calls
        let original_stmts = &original_body.stmts;
        let new_body = if returns_value {
            quote! {
                {
                    #profile_start
                    let __result = {
                        #(#original_stmts)*
                    };
                    #profile_end
                    __result
                }
            }
        } else {
            quote! {
                {
                    #profile_start
                    #(#original_stmts)*
                    #profile_end
                }
            }
        };

        // Filter out the profile attribute, add inline(always), and replace the function body
        f.attrs.retain(|a| !a.path().is_ident("profile"));
        // f.attrs.push(syn::parse_quote!(#[inline(always)]));
        f.block = syn::parse2(new_body).unwrap();

        TokenStream::from(quote! {
            #f
        })
    }
}
