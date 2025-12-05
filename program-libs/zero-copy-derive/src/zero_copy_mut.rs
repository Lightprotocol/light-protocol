use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

use crate::{
    shared::{
        meta_struct, utils,
        z_struct::{self, analyze_struct_fields},
        zero_copy_new::{generate_config_struct, generate_init_mut_impl},
    },
    zero_copy,
};

pub fn derive_zero_copy_mut_impl(fn_input: TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    // Parse the input DeriveInput once
    let input: DeriveInput = syn::parse(fn_input)?;

    // Validate that struct has #[repr(C)] attribute
    utils::validate_repr_c_required(&input.attrs, "ZeroCopyMut")?;

    let hasher = false;

    // Process the input to extract struct information
    let (name, z_struct_name, z_struct_meta_name, fields) = utils::process_input(&input)?;

    // Handle unit structs (no fields)
    let Some(fields) = fields else {
        // Unit struct - generate minimal mutable implementations
        let z_struct_name_mut = format_ident!("{}Mut", z_struct_name);

        let zero_copy_struct_inner_impl_mut =
            zero_copy::generate_zero_copy_struct_inner::<true>(name, &z_struct_name_mut)?;

        // Generate a simple unit ZStruct type alias for mut
        let z_struct_def_mut = quote::quote! {
            /// Zero-copy mutable reference type for unit struct #name
            pub type #z_struct_name_mut<'a> = &'a mut #name;
        };

        // Generate minimal deserialize impl for unit struct
        let deserialize_impl_mut = quote::quote! {
            impl<'a> ::light_zero_copy::traits::ZeroCopyAtMut<'a> for #name {
                type ZeroCopyAtMut = #z_struct_name_mut<'a>;
                fn zero_copy_at_mut(bytes: &'a mut [u8]) -> ::core::result::Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ::light_zero_copy::errors::ZeroCopyError> {
                    // For zero-sized types (ZSTs), Box::new does not allocate heap memory;
                    // it returns a dangling-but-aligned pointer, so leaking it is safe and
                    // does not cause a memory leak. This pattern avoids returning a mutable
                    // reference to a static for ZSTs, which would be unsound.
                    let unit: &'a mut #name = Box::leak(Box::new(#name));
                    Ok((unit, bytes))
                }
            }
        };

        // Generate unit type config
        let config_name = quote::format_ident!("{}Config", name);
        let config_struct = quote::quote! {
            pub type #config_name = ();
        };

        // Generate ZeroCopyNew impl for unit struct (specialized version)
        let init_mut_impl = quote::quote! {
            impl<'a> ::light_zero_copy::traits::ZeroCopyNew<'a> for #name {
                type ZeroCopyConfig = #config_name;
                type Output = #z_struct_name_mut<'a>;

                fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ::light_zero_copy::errors::ZeroCopyError> {
                    // Unit struct has 0 bytes
                    Ok(0)
                }

                fn new_zero_copy(
                    bytes: &'a mut [u8],
                    _config: Self::ZeroCopyConfig,
                ) -> Result<(Self::Output, &'a mut [u8]), ::light_zero_copy::errors::ZeroCopyError> {
                    // For zero-sized types (ZSTs), Box::new does not allocate heap memory;
                    // it returns a dangling-but-aligned pointer, so leaking it is safe and
                    // does not cause a memory leak.
                    let unit: &'a mut #name = Box::leak(Box::new(#name));
                    Ok((unit, bytes))
                }
            }
        };

        return Ok(quote::quote! {
            #config_struct
            #z_struct_def_mut

            const _: () = {
                #zero_copy_struct_inner_impl_mut
                #deserialize_impl_mut
                #init_mut_impl
            };
        });
    };

    // Process the fields to separate meta fields and struct fields
    let (meta_fields, struct_fields) = utils::process_fields(fields);

    let meta_struct_def_mut = if !meta_fields.is_empty() {
        meta_struct::generate_meta_struct::<true>(&z_struct_meta_name, &meta_fields, hasher)?
    } else {
        quote! {}
    };

    let z_struct_def_mut = z_struct::generate_z_struct::<true>(
        &z_struct_name,
        &z_struct_meta_name,
        &struct_fields,
        &meta_fields,
        hasher,
    )?;

    let zero_copy_struct_inner_impl_mut = zero_copy::generate_zero_copy_struct_inner::<true>(
        name,
        &format_ident!("{}Mut", z_struct_name),
    )?;

    let deserialize_impl_mut = zero_copy::generate_deserialize_impl::<true>(
        name,
        &z_struct_name,
        &z_struct_meta_name,
        &struct_fields,
        meta_fields.is_empty(),
        quote! {},
    )?;

    // Analyze only struct fields for ZeroCopyNew (meta fields are always fixed-size)
    let struct_field_types = analyze_struct_fields(&struct_fields)?;

    // Always generate ZeroCopyNew, but use unit config for fixed-size types
    // This follows zerocopy-derive pattern of always implementing traits
    let field_strategies: Vec<_> = struct_field_types
        .iter()
        .map(crate::shared::zero_copy_new::analyze_field_strategy)
        .collect();

    let has_dynamic_fields = !field_strategies.iter().all(|strategy| {
        matches!(
            strategy,
            crate::shared::zero_copy_new::FieldStrategy::FixedSize
        )
    });

    let (config_struct, init_mut_impl) = if has_dynamic_fields {
        // Generate complex config struct for dynamic fields
        let config = generate_config_struct(name, &struct_field_types)?;
        let init_impl = generate_init_mut_impl(name, &meta_fields, &struct_fields)?;
        (config, Some(init_impl))
    } else {
        // Generate unit type alias for fixed-size fields
        let config_name = quote::format_ident!("{}Config", name);
        let unit_config = Some(quote! {
            pub type #config_name = ();
        });
        let init_impl = generate_init_mut_impl(name, &meta_fields, &struct_fields)?;
        (unit_config, Some(init_impl))
    };

    // Combine all mutable implementations with selective hygiene isolation
    // Types must be public for trait associated types, but implementations are isolated
    let expanded = quote! {
        // Public types that need to be accessible from trait implementations
        #config_struct
        #meta_struct_def_mut
        #z_struct_def_mut

        // Isolate only the implementations to prevent pollution while keeping types accessible
        const _: () = {
            // Import all necessary items within the isolated scope
            #[allow(unused_imports)]
            use ::core::{mem::size_of, ops::Deref};

            #[allow(unused_imports)]
            use ::light_zero_copy::{
                errors::ZeroCopyError,
                slice_mut::ZeroCopySliceMutBorsh,
            };

            #[allow(unused_imports)]
            use ::zerocopy::{
                little_endian::{U16, U32, U64},
                FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned,
            };

            // Implementations are isolated to prevent helper function pollution
            #zero_copy_struct_inner_impl_mut
            #deserialize_impl_mut
            #init_mut_impl
        };
    };

    Ok(expanded)
}
