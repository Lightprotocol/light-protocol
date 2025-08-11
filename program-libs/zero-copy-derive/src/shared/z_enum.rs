use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataEnum, Fields, Ident};

/// Generate the zero-copy enum definition with type aliases for pattern matching
pub fn generate_z_enum(z_enum_name: &Ident, enum_data: &DataEnum) -> syn::Result<TokenStream> {
    // Collect type aliases for complex variants
    let mut type_aliases = Vec::new();

    let variants = enum_data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;

        match &variant.fields {
            Fields::Unit => {
                // Unit variant: Placeholder0,
                Ok(quote! { #variant_name })
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                // Single unnamed field: TokenMetadata(TokenMetadataInstructionData)
                let field_type = &fields.unnamed.first()
                    .ok_or_else(|| syn::Error::new_spanned(
                        fields,
                        "Internal error: expected exactly one unnamed field but found none"
                    ))?
                    .ty;

                // Create a type alias for this variant to enable pattern matching
                let alias_name = format_ident!("{}Type", variant_name);
                type_aliases.push(quote! {
                    pub type #alias_name<'a> = <#field_type as light_zero_copy::borsh::Deserialize<'a>>::Output;
                });

                Ok(quote! { #variant_name(#alias_name<'a>) })
            }
            Fields::Named(_) => {
                // Named fields - not commonly used in enums but we can support it
                Err(syn::Error::new_spanned(
                    variant,
                    "Named fields in enum variants are not supported yet",
                ))
            }
            Fields::Unnamed(fields) if fields.unnamed.len() > 1 => {
                // Multiple unnamed fields - not common but we can support it
                Err(syn::Error::new_spanned(
                    variant,
                    "Multiple unnamed fields in enum variants are not supported yet",
                ))
            }
            _ => {
                Err(syn::Error::new_spanned(
                    variant,
                    "Unsupported enum variant format",
                ))
            }
        }
    }).collect::<Result<Vec<_>, _>>()?;

    Ok(quote! {
        // Generate type aliases for complex variants
        #(#type_aliases)*

        #[derive(Debug, Clone, PartialEq)]
        pub enum #z_enum_name<'a> {
            #(#variants,)*
        }
    })
}

/// Generate the deserialize implementation for the enum
pub fn generate_enum_deserialize_impl(
    original_name: &Ident,
    z_enum_name: &Ident,
    enum_data: &DataEnum,
) -> syn::Result<TokenStream> {
    // Generate match arms for each variant
    let match_arms_result: Result<Vec<TokenStream>, syn::Error> = enum_data.variants.iter().enumerate().map(|(index, variant)| {
        let variant_name = &variant.ident;
        let discriminant = index as u8; // Borsh uses sequential discriminants starting from 0

        match &variant.fields {
            Fields::Unit => {
                // Unit variant
                Ok(quote! {
                    #discriminant => {
                        Ok((#z_enum_name::#variant_name, remaining_data))
                    }
                })
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                // Single unnamed field
                let field_type = &fields.unnamed.first()
                    .ok_or_else(|| syn::Error::new_spanned(
                        fields,
                        "Internal error: expected exactly one unnamed field but found none"
                    ))?
                    .ty;
                Ok(quote! {
                    #discriminant => {
                        let (value, remaining_bytes) =
                            <#field_type as light_zero_copy::borsh::Deserialize>::zero_copy_at(remaining_data)?;
                        Ok((#z_enum_name::#variant_name(value), remaining_bytes))
                    }
                })
            }
            _ => {
                // Other cases already handled in generate_z_enum
                Ok(quote! {
                    #discriminant => {
                        Err(light_zero_copy::errors::ZeroCopyError::InvalidConversion)
                    }
                })
            }
        }
    }).collect();
    let match_arms = match_arms_result?;

    Ok(quote! {
        impl<'a> light_zero_copy::borsh::Deserialize<'a> for #original_name {
            type Output = #z_enum_name<'a>;

            fn zero_copy_at(
                data: &'a [u8],
            ) -> Result<(Self::Output, &'a [u8]), light_zero_copy::errors::ZeroCopyError> {
                // Read discriminant (first 1 byte for borsh enum)
                if data.is_empty() {
                    return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        data.len(),
                    ));
                }

                let discriminant = data[0];
                let remaining_data = &data[1..];

                match discriminant {
                    #(#match_arms)*
                    _ => Err(light_zero_copy::errors::ZeroCopyError::InvalidConversion),
                }
            }
        }
    })
}

/// Generate the ZeroCopyStructInner implementation for the enum
pub fn generate_enum_zero_copy_struct_inner(
    original_name: &Ident,
    z_enum_name: &Ident,
) -> syn::Result<TokenStream> {
    Ok(quote! {
        impl light_zero_copy::borsh::ZeroCopyStructInner for #original_name {
            type ZeroCopyInner = #z_enum_name<'static>;
        }
    })
}
