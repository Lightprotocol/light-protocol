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
    // Parse the input DeriveInput
    let input: DeriveInput = syn::parse(fn_input.clone())?;

    let hasher = false;

    // Process the input to extract struct information
    let (name, z_struct_name, z_struct_meta_name, fields) = utils::process_input(&input)?;

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

    // Parse the input DeriveInput
    let input: DeriveInput = syn::parse(fn_input)?;

    // Process the input to extract struct information
    let (name, _z_struct_name, _z_struct_meta_name, fields) = utils::process_input(&input)?;

    // Use the same field processing logic as other derive macros for consistency
    let (meta_fields, struct_fields) = utils::process_fields(fields);

    // Analyze only struct fields for ZeroCopyNew (meta fields are always fixed-size)
    let struct_field_types = analyze_struct_fields(&struct_fields)?;
    
    // Always generate ZeroCopyNew, but use unit config for fixed-size types
    // This follows zerocopy-derive pattern of always implementing traits
    let has_dynamic_fields = struct_field_types.iter().any(|ft| {
        use crate::shared::zero_copy_new::requires_config;
        requires_config(ft)
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

    // Combine all mutable implementations
    let expanded = quote! {
        #config_struct

        #init_mut_impl

        #meta_struct_def_mut

        #z_struct_def_mut

        #zero_copy_struct_inner_impl_mut

        #deserialize_impl_mut
    };

    Ok(expanded)
}
