use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    bracketed, parse::Parse, punctuated::Punctuated, Attribute, DeriveInput, Expr, Field, Fields,
    GenericArgument, Ident, PathArguments, Result, Token, Type, TypePath,
};

/// Information about a compressible account field found in an instruction struct
#[derive(Debug, Clone)]
struct CompressibleFieldInfo {
    /// The account type (e.g., PoolState)
    account_type: Ident,
    /// The field name in the instruction struct (e.g., pool_state)
    field_name: Ident,
    /// The seeds expressions from the #[account] attribute
    seeds: Vec<Expr>,
    /// Whether the field has a bump constraint
    has_bump: bool,
}

/// Parse a derive input and generate compressible registry functions
pub(crate) fn derive_compressible(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    
    // Extract fields from the struct
    let fields = match &input.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    struct_name,
                    "Compressible can only be derived for structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                struct_name,
                "Compressible can only be derived for structs",
            ))
        }
    };

    // Find all fields that have init + seeds constraints
    let mut compressible_fields = Vec::new();
    
    for field in fields {
        if let Some(field_info) = extract_compressible_field_info(field)? {
            compressible_fields.push(field_info);
        }
    }

    if compressible_fields.is_empty() {
        return Err(syn::Error::new_spanned(
            struct_name,
            "No compressible fields found. Expected at least one field with #[account(init, seeds = [...], bump)]",
        ));
    }

    // Generate registry functions for each compressible field
    let mut generated_functions = Vec::new();
    
    for field_info in compressible_fields {
        let registry_fn = generate_seed_registry_function(&field_info)?;
        generated_functions.push(registry_fn);
    }

    Ok(quote! {
        #(#generated_functions)*
    })
}

/// Extract compressible field information from a struct field
fn extract_compressible_field_info(field: &Field) -> Result<Option<CompressibleFieldInfo>> {
    let field_name = field.ident.as_ref().ok_or_else(|| {
        syn::Error::new_spanned(field, "Field must have a name")
    })?;

    // Extract account type from the field type (e.g., Account<'info, PoolState> -> PoolState)
    let account_type = extract_account_type(&field.ty)?;
    
    if account_type.is_none() {
        // This field is not an Account type, skip it
        return Ok(None);
    }
    
    let account_type = account_type.unwrap();

    // Look for #[account] attribute with init and seeds
    for attr in &field.attrs {
        if attr.path().is_ident("account") {
            if let Some((has_init, seeds, has_bump)) = parse_account_attribute(attr)? {
                if has_init && !seeds.is_empty() {
                    return Ok(Some(CompressibleFieldInfo {
                        account_type,
                        field_name: field_name.clone(),
                        seeds,
                        has_bump,
                    }));
                }
            }
        }
    }

    Ok(None)
}

/// Extract the account type from a field type like Account<'info, T> -> T
fn extract_account_type(ty: &Type) -> Result<Option<Ident>> {
    match ty {
        Type::Path(type_path) => {
            if let Some(last_segment) = type_path.path.segments.last() {
                let segment_name = last_segment.ident.to_string();
                
                // Check for Account, Box<Account>, etc.
                if is_account_wrapper(&segment_name) {
                    return extract_account_type_from_generics(&last_segment.arguments);
                }
                
                // Handle Box<Account<...>>
                if segment_name == "Box" {
                    if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                        for arg in &args.args {
                            if let GenericArgument::Type(inner_type) = arg {
                                if let Some(account_type) = extract_account_type(inner_type)? {
                                    return Ok(Some(account_type));
                                }
                            }
                        }
                    }
                }
            }
        }
        Type::Reference(type_ref) => {
            // Handle &Account<...> or &mut Account<...>
            return extract_account_type(&type_ref.elem);
        }
        _ => {}
    }
    
    Ok(None)
}

/// Check if a type name is an account wrapper (Account, AccountLoader, InterfaceAccount, etc.)
fn is_account_wrapper(type_name: &str) -> bool {
    matches!(type_name, "Account" | "AccountLoader" | "InterfaceAccount")
}

/// Extract account type from generic arguments like Account<'info, PoolState> -> PoolState
fn extract_account_type_from_generics(args: &PathArguments) -> Result<Option<Ident>> {
    if let PathArguments::AngleBracketed(args) = args {
        // Look for the account type (usually the second generic argument after lifetime)
        for arg in &args.args {
            if let GenericArgument::Type(Type::Path(TypePath { path, .. })) = arg {
                if let Some(last_segment) = path.segments.last() {
                    // Skip lifetime parameters
                    if last_segment.ident.to_string().starts_with('_') || 
                       last_segment.ident.to_string() == "info" {
                        continue;
                    }
                    return Ok(Some(last_segment.ident.clone()));
                }
            }
        }
    }
    Ok(None)
}

/// Parse account attribute to extract init, seeds, and bump information
fn parse_account_attribute(attr: &Attribute) -> Result<Option<(bool, Vec<Expr>, bool)>> {
    if !attr.path().is_ident("account") {
        return Ok(None);
    }

    let mut has_init = false;
    let mut seeds = Vec::new();
    let mut has_bump = false;

    // Parse the attribute content
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("init") {
            has_init = true;
            Ok(())
        } else if meta.path.is_ident("bump") {
            has_bump = true;
            Ok(())
        } else if meta.path.is_ident("seeds") {
            // Parse seeds = [...]
            if meta.input.peek(Token![=]) {
                meta.input.parse::<Token![=]>()?; // Consume the equals sign
                let content;
                bracketed!(content in meta.input);
                let seed_exprs: Punctuated<Expr, Token![,]> =
                    content.parse_terminated(Expr::parse, Token![,])?;
                seeds = seed_exprs.into_iter().collect();
            }
            Ok(())
        } else {
            // Skip other attributes like payer, space, etc.
            if meta.input.peek(Token![=]) {
                meta.input.parse::<Token![=]>()?;
                meta.input.parse::<Expr>()?;
            }
            Ok(())
        }
    })?;

    Ok(Some((has_init, seeds, has_bump)))
}

/// Generate a seed registry function for a compressible field
fn generate_seed_registry_function(field_info: &CompressibleFieldInfo) -> Result<TokenStream> {
    let account_type = &field_info.account_type;
    let seeds = &field_info.seeds;
    let has_bump = field_info.has_bump;
    
    // Generate a module with a predictable name that the main macro can find
    let module_name = format_ident!("__compressible_seeds_{}", account_type.to_string().to_lowercase());
    
    Ok(quote! {
        #[doc(hidden)]
        #[allow(non_snake_case)]
        pub mod #module_name {
            use super::*;
            
            // Export the account type for verification
            pub type AccountType = super::#account_type;
            
            // Export the seed information in a format the main macro can parse
            pub const HAS_BUMP: bool = #has_bump;
            
            // Generate a function that returns the seeds
            // The main macro will look for this function signature and extract the seeds from its body
            pub fn get_seeds() -> Vec<()> {
                // The main macro will parse the expressions inside this block
                let _ = vec![#(#seeds),*];
                vec![]
            }
        }
    })
} 